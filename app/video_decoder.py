from __future__ import annotations

import av
import numpy as np


class VideoDecoder:
    """Low-latency H.264 / H.265 (HEVC) decoder backed by PyAV (FFmpeg).
    Attempts NVIDIA CUVID hardware decode first when enabled, with automatic
    low-delay multi-threaded software fallback.
    """

    def __init__(self, prefer_hw: bool = True) -> None:
        self.prefer_hw = prefer_hw
        self._codec_id: int = 0  # 1 = H.264, 2 = H.265/HEVC
        self._ctx: av.CodecContext | None = None
        self.active_decoder_name: str = "待机"
        self._hw_failed: bool = False

    def set_prefer_hw(self, prefer_hw: bool) -> None:
        if self.prefer_hw != prefer_hw:
            self.prefer_hw = prefer_hw
            self._hw_failed = False
            self.reset()

    def reset(self) -> None:
        if self._ctx is not None:
            try:
                self._ctx.close()
            except Exception:
                pass
        self._ctx = None
        self._codec_id = 0
        self.active_decoder_name = "待机"

    def _open_decoder(self, codec_id: int) -> None:
        self.reset()
        self._codec_id = codec_id

        candidates: list[tuple[str, str]] = []
        if codec_id == 2:
            if self.prefer_hw and not self._hw_failed:
                candidates.append(("hevc_cuvid", "NVDEC HEVC (GPU)"))
            candidates.append(("hevc", "FFmpeg HEVC (CPU)"))
        else:
            if self.prefer_hw and not self._hw_failed:
                candidates.append(("h264_cuvid", "NVDEC H.264 (GPU)"))
            candidates.append(("h264", "FFmpeg H.264 (CPU)"))

        last_err: Exception | None = None
        for name, label in candidates:
            try:
                ctx = av.CodecContext.create(name, "r")
                if not name.endswith("_cuvid"):
                    ctx.thread_type = "SLICE"
                    ctx.thread_count = 4
                low_delay = getattr(av.codec.context.Flags, "low_delay", None) or getattr(
                    av.codec.context.Flags, "LOW_DELAY", 0
                )
                if low_delay:
                    ctx.flags |= low_delay
                ctx.open()
                self._ctx = ctx
                self.active_decoder_name = label
                return
            except Exception as exc:
                last_err = exc
                continue
        raise RuntimeError(f"Failed to open video decoder for codec_id={codec_id}: {last_err}")

    def decode(self, codec_id: int, annexb_data: bytes, timestamp_ns: int) -> list[np.ndarray]:
        if self._ctx is None or self._codec_id != codec_id:
            self._open_decoder(codec_id)

        assert self._ctx is not None
        pkt = av.Packet(annexb_data)
        pkt.pts = timestamp_ns // 1000  # microseconds

        try:
            frames = self._ctx.decode(pkt)
        except av.FFmpegError:
            # If hardware decoder rejects stream profile (e.g., 10-bit or unusual SPS), fall back to SW
            if self.prefer_hw and not self._hw_failed and "GPU" in self.active_decoder_name:
                self._hw_failed = True
                self._open_decoder(codec_id)
                try:
                    frames = self._ctx.decode(pkt)  # type: ignore[union-attr]
                except av.FFmpegError:
                    return []
            else:
                return []

        rgb_frames: list[np.ndarray] = []
        for frame in frames:
            try:
                rgb = frame.to_ndarray(format="rgb24")
                rgb_frames.append(rgb)
            except Exception:
                continue
        return rgb_frames
