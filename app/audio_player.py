from __future__ import annotations

import collections
import threading
import av
import numpy as np
import sounddevice as sd

# Exact codec_data magic cookies from UxPlay renderers/audio_renderer.c
ALAC_EXTRADATA = bytes.fromhex(
    "00000024616c616300000000000001600010280a0e0200ff00000000000000000000ac44"
)
AAC_LC_EXTRADATA = bytes.fromhex("1210")
AAC_ELD_EXTRADATA = bytes.fromhex("f8e85000")


class AudioPlayer:
    """Low-latency AirPlay audio decoder (AAC-ELD / ALAC / AAC-LC / LPCM) + sounddevice output."""

    SAMPLE_RATE = 44100
    CHANNELS = 2

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._pcm_queue: collections.deque[np.ndarray] = collections.deque(maxlen=64)
        self._leftover = np.zeros((0, self.CHANNELS), dtype=np.float32)

        self._ctx: av.CodecContext | None = None
        self._resampler = av.AudioResampler(
            format="flt",
            layout="stereo",
            rate=self.SAMPLE_RATE,
        )
        self._ct: int = 0
        self._stream: sd.OutputStream | None = None

        self.enabled: bool = True
        self.local_volume: float = 0.85
        self.ios_volume: float = 1.0
        self.follow_ios_volume: bool = True

        self._pcm_listeners: list = []

    def add_pcm_listener(self, fn) -> None:
        self._pcm_listeners.append(fn)

    def remove_pcm_listener(self, fn) -> None:
        if fn in self._pcm_listeners:
            self._pcm_listeners.remove(fn)

    def _effective_gain(self) -> float:
        if not self.enabled:
            return 0.0
        gain = self.local_volume * (self.ios_volume if self.follow_ios_volume else 1.0)
        return max(0.0, min(1.5, gain))

    def _ensure_stream(self) -> None:
        if self._stream is not None:
            return
        try:
            self._stream = sd.OutputStream(
                samplerate=self.SAMPLE_RATE,
                channels=self.CHANNELS,
                dtype="float32",
                blocksize=512,
                latency="low",
                callback=self._sd_callback,
            )
            self._stream.start()
        except Exception as exc:
            print(f"[Audio] Failed to open audio output: {exc}")
            self._stream = None

    def _sd_callback(self, outdata: np.ndarray, frames: int, _time, _status) -> None:
        needed = frames
        out_pos = 0
        gain = self._effective_gain()

        with self._lock:
            while needed > 0:
                if len(self._leftover) == 0:
                    if not self._pcm_queue:
                        outdata[out_pos:].fill(0.0)
                        return
                    self._leftover = self._pcm_queue.popleft()

                take = min(needed, len(self._leftover))
                outdata[out_pos : out_pos + take] = self._leftover[:take] * gain
                self._leftover = self._leftover[take:]
                out_pos += take
                needed -= take

    def _open_codec(self, ct: int) -> None:
        if self._ctx is not None:
            try:
                self._ctx.close()
            except Exception:
                pass
            self._ctx = None
        self._ct = ct

        if ct == 1:
            # Raw LPCM S16LE
            return

        codec_name = "alac" if ct == 2 else "aac"
        ctx = av.CodecContext.create(codec_name, "r")
        ctx.sample_rate = self.SAMPLE_RATE
        ctx.layout = "stereo"
        if ct == 2:
            ctx.extradata = ALAC_EXTRADATA
        elif ct == 4:
            ctx.extradata = AAC_LC_EXTRADATA
        elif ct == 8:
            ctx.extradata = AAC_ELD_EXTRADATA
        ctx.open()
        self._ctx = ctx

    def feed(self, ct: int, payload: bytes, timestamp_ns: int) -> None:
        if ct != self._ct or (ct != 1 and self._ctx is None):
            try:
                self._open_codec(ct)
            except Exception as exc:
                print(f"[Audio] Codec init error (ct={ct}): {exc}")
                return

        self._ensure_stream()

        if ct == 1:
            # LPCM S16LE interleaved
            samples = np.frombuffer(payload, dtype=np.int16).astype(np.float32) / 32768.0
            if len(samples) % 2 == 0 and len(samples) > 0:
                pcm = samples.reshape(-1, 2)
                self._enqueue_pcm(pcm, timestamp_ns)
            return

        if self._ctx is None:
            return

        try:
            pkt = av.Packet(payload)
            frames = self._ctx.decode(pkt)
        except av.FFmpegError:
            return

        for frame in frames:
            try:
                resampled_list = self._resampler.resample(frame)
                for rf in resampled_list:
                    arr = rf.to_ndarray()
                    # Packed flt comes out as shape (1, samples*2) or (samples, 2)
                    if arr.ndim == 2 and arr.shape[0] == 1:
                        pcm = arr.reshape(-1, self.CHANNELS)
                    elif arr.ndim == 2 and arr.shape[0] == self.CHANNELS:
                        pcm = arr.T.copy()
                    else:
                        pcm = arr.reshape(-1, self.CHANNELS)
                    self._enqueue_pcm(pcm.astype(np.float32, copy=False), timestamp_ns)
            except Exception:
                continue

    def _enqueue_pcm(self, pcm: np.ndarray, timestamp_ns: int) -> None:
        with self._lock:
            self._pcm_queue.append(pcm)
        for listener in list(self._pcm_listeners):
            try:
                listener(pcm, timestamp_ns)
            except Exception:
                pass

    def flush(self) -> None:
        with self._lock:
            self._pcm_queue.clear()
            self._leftover = np.zeros((0, self.CHANNELS), dtype=np.float32)

    def close(self) -> None:
        self.flush()
        if self._stream is not None:
            try:
                self._stream.stop()
                self._stream.close()
            except Exception:
                pass
            self._stream = None
        if self._ctx is not None:
            try:
                self._ctx.close()
            except Exception:
                pass
            self._ctx = None
