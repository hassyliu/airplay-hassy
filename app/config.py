from __future__ import annotations

import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

if getattr(sys, "frozen", False):
    ROOT_DIR = Path(sys.executable).resolve().parent
else:
    ROOT_DIR = Path(__file__).resolve().parent.parent

CONFIG_PATH = ROOT_DIR / "config.json"
DEFAULT_RECORDINGS_DIR = ROOT_DIR / "recordings"

QUALITY_PRESETS: dict[str, dict] = {
    "4K 极致": {
        "width": 3840,
        "height": 2160,
        "max_fps": 60,
        "refresh_rate": 60,
        "hevc": True,
    },
    "2K 超清": {
        "width": 2560,
        "height": 1440,
        "max_fps": 60,
        "refresh_rate": 60,
        "hevc": True,
    },
    "1080p 高刷 120Hz": {
        "width": 1920,
        "height": 1080,
        "max_fps": 120,
        "refresh_rate": 120,
        "hevc": True,
    },
    "1080p 标准": {
        "width": 1920,
        "height": 1080,
        "max_fps": 60,
        "refresh_rate": 60,
        "hevc": False,
    },
    "720p 低延迟": {
        "width": 1280,
        "height": 720,
        "max_fps": 60,
        "refresh_rate": 60,
        "hevc": False,
    },
    "iPhone 竖屏原生": {
        "width": 1170,
        "height": 2532,
        "max_fps": 60,
        "refresh_rate": 60,
        "hevc": True,
    },
    "自定义画质": {},
}


@dataclass
class AppConfig:
    # Protocol / AirPlay advertised quality settings
    device_name: str = "AirPlay-Hassy"
    preset: str = "2K 超清"
    width: int = 2560
    height: int = 1440
    max_fps: int = 60
    refresh_rate: int = 60
    hevc: bool = True
    overscanned: bool = False

    # Network ports
    rtsp_port: int = 7000
    mirror_port: int = 7100
    audio_data_port: int = 6000
    audio_control_port: int = 6001
    timing_port: int = 7010
    ipc_port: int = 17890
    mac_address: str = "48:5D:60:7C:EE:22"

    # Decoding & Display settings
    hw_decode: bool = True
    smooth_scaling: bool = True
    sharpen: bool = False
    show_hud: bool = True
    always_on_top: bool = False

    # Audio settings
    audio_enabled: bool = True
    volume: float = 0.85
    follow_ios_volume: bool = True

    # Recording settings
    recordings_dir: str = str(DEFAULT_RECORDINGS_DIR)

    @classmethod
    def load(cls, path: Path = CONFIG_PATH) -> "AppConfig":
        if path.exists():
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
                valid_fields = {f.name for f in cls.__dataclass_fields__.values()}
                filtered = {k: v for k, v in data.items() if k in valid_fields}
                return cls(**filtered)
            except Exception:
                pass
        cfg = cls()
        cfg.save(path)
        return cfg

    def save(self, path: Path = CONFIG_PATH) -> None:
        path.write_text(
            json.dumps(asdict(self), indent=2, ensure_ascii=False),
            encoding="utf-8",
        )
