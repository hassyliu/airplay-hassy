from __future__ import annotations

import datetime
import threading
import time
from fractions import Fraction
from pathlib import Path

import av
import numpy as np


class ScreenRecorder:
    """Records mirrored video + audio directly to local MP4.
    Uses NVIDIA NVENC (h264_nvenc) if available for near-zero CPU overhead,
    falling back to ultrafast libx264, plus AAC-LC audio for universal playback.
    """

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._recording = False
        self._container: av.container.OutputContainer | None = None
        self._vstream = None
        self._astream = None
        self._start_wall: float = 0.0
        self._video_size: tuple[int, int] | None = None
        self._audio_pts: int = 0
        self._last_video_pts: int = -1
        self.current_path: Path | None = None
        self._target_dir: Path | None = None

    @property
    def is_recording(self) -> bool:
        return self._recording

    @property
    def elapsed_seconds(self) -> float:
        if not self._recording:
            return 0.0
        return max(0.0, time.monotonic() - self._start_wall)

    def start(self, output_dir: Path) -> Path:
        with self._lock:
            if self._recording:
                assert self.current_path is not None
                return self.current_path
            output_dir.mkdir(parents=True, exist_ok=True)
            ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
            self.current_path = output_dir / f"AirPlay_{ts}.mp4"
            self._target_dir = output_dir
            self._container = None
            self._vstream = None
            self._astream = None
            self._video_size = None
            self._audio_pts = 0
            self._last_video_pts = -1
            self._start_wall = time.monotonic()
            self._recording = True
            return self.current_path

    def _init_container(self, width: int, height: int) -> None:
        assert self.current_path is not None
        # Ensure even dimensions for yuv420p
        w = width - (width % 2)
        h = height - (height % 2)
        self._video_size = (w, h)

        container = av.open(str(self.current_path), mode="w")

        # Try NVENC first for RTX GPU, else libx264 ultrafast
        vstream = None
        for codec_name, opts in [
            ("h264_nvenc", {"preset": "p4", "rc": "vbr", "cq": "19"}),
            ("libx264", {"preset": "ultrafast", "crf": "18"}),
        ]:
            try:
                vs = container.add_stream(codec_name, rate=60)
                vs.width = w
                vs.height = h
                vs.pix_fmt = "yuv420p"
                vs.time_base = Fraction(1, 90000)
                vs.options = opts
                # Trigger open check
                vstream = vs
                break
            except Exception:
                continue

        if vstream is None:
            container.close()
            raise RuntimeError("No H.264 encoder available for MP4 recording")

        astream = container.add_stream("aac", rate=44100)
        astream.layout = "stereo"
        astream.bit_rate = 192000

        self._container = container
        self._vstream = vstream
        self._astream = astream

    def write_video_rgb(self, rgb: np.ndarray) -> None:
        if not self._recording:
            return
        with self._lock:
            if not self._recording:
                return
            h, w, _ = rgb.shape
            if self._container is None:
                try:
                    self._init_container(w, h)
                except Exception as exc:
                    print(f"[Recorder] Init failed: {exc}")
                    self._recording = False
                    return

            assert self._container is not None
            assert self._vstream is not None
            assert self._video_size is not None

            target_w, target_h = self._video_size
            frame = av.VideoFrame.from_ndarray(rgb, format="rgb24")
            if frame.width != target_w or frame.height != target_h:
                frame = frame.reformat(width=target_w, height=target_h, format="yuv420p")
            else:
                frame = frame.reformat(format="yuv420p")

            pts = int((time.monotonic() - self._start_wall) * 90000)
            if pts <= self._last_video_pts:
                pts = self._last_video_pts + 1
            self._last_video_pts = pts
            frame.pts = pts
            frame.time_base = Fraction(1, 90000)

            try:
                for pkt in self._vstream.encode(frame):
                    self._container.mux(pkt)
            except Exception as exc:
                print(f"[Recorder] Video encode error: {exc}")

    def write_audio_pcm(self, pcm: np.ndarray, _timestamp_ns: int) -> None:
        if not self._recording:
            return
        with self._lock:
            if not self._recording or self._container is None or self._astream is None:
                return
            try:
                # pcm is float32 (N, 2) -> planar fltp (2, N)
                planar = np.ascontiguousarray(pcm.T, dtype=np.float32)
                frame = av.AudioFrame.from_ndarray(planar, format="fltp", layout="stereo")
                frame.sample_rate = 44100
                frame.pts = self._audio_pts
                frame.time_base = Fraction(1, 44100)
                self._audio_pts += frame.samples
                for pkt in self._astream.encode(frame):
                    self._container.mux(pkt)
            except Exception:
                pass

    def stop(self) -> Path | None:
        with self._lock:
            if not self._recording:
                return None
            self._recording = False
            saved_path = self.current_path
            if self._container is not None:
                try:
                    if self._vstream is not None:
                        for pkt in self._vstream.encode():
                            self._container.mux(pkt)
                    if self._astream is not None:
                        for pkt in self._astream.encode():
                            self._container.mux(pkt)
                    self._container.close()
                except Exception as exc:
                    print(f"[Recorder] Finalize error: {exc}")
                finally:
                    self._container = None
                    self._vstream = None
                    self._astream = None
            return saved_path
