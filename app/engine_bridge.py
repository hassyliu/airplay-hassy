from __future__ import annotations

import hashlib
import json
import os
import shutil
import socket
import struct
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Callable

from PySide6 import QtCore

from app.config import AppConfig, ROOT_DIR

ENGINE_BIN_RELEASE = ROOT_DIR / "engine" / "target" / "release" / "airplay-hassy-engine.exe"
ENGINE_BIN_DEBUG = ROOT_DIR / "engine" / "target" / "debug" / "airplay-hassy-engine.exe"


def _file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _materialize_stable_engine(bundled: Path) -> Path:
    """Copy bundled Rust engine out of PyInstaller's ephemeral _MEIPASS temp dir
    to a permanent path under %LOCALAPPDATA%\\AirPlay-Hassy so Windows Firewall
    only asks for permission once.
    """
    local_app = os.environ.get("LOCALAPPDATA")
    stable_dir = (
        Path(local_app) / "AirPlay-Hassy"
        if local_app
        else Path.home() / ".airplay-hassy"
    )
    stable_dir.mkdir(parents=True, exist_ok=True)
    target = stable_dir / "airplay-hassy-engine.exe"

    needs_copy = not target.exists() or target.stat().st_size != bundled.stat().st_size
    if not needs_copy:
        try:
            needs_copy = _file_sha256(target) != _file_sha256(bundled)
        except OSError:
            needs_copy = True

    if needs_copy:
        try:
            shutil.copy2(bundled, target)
            return target
        except OSError:
            # If previous engine instance is still releasing lock, fall back to bundled path
            return target if target.exists() else bundled
    return target


def find_engine_binary() -> Path | None:
    if getattr(sys, "frozen", False):
        meipass = getattr(sys, "_MEIPASS", None)
        if meipass:
            bundled = Path(meipass) / "bin" / "airplay-hassy-engine.exe"
            if bundled.exists():
                return _materialize_stable_engine(bundled)
        exe_dir = Path(sys.executable).resolve().parent
        for candidate in (
            exe_dir / "airplay-hassy-engine.exe",
            exe_dir / "_internal" / "bin" / "airplay-hassy-engine.exe",
        ):
            if candidate.exists():
                return candidate

    if ENGINE_BIN_RELEASE.exists():
        return ENGINE_BIN_RELEASE
    if ENGINE_BIN_DEBUG.exists():
        return ENGINE_BIN_DEBUG
    return None


class EngineBridge(QtCore.QObject):
    """Manages the Rust airplay-hassy-engine subprocess and streams IPC packets."""

    event_received = QtCore.Signal(dict)
    log_received = QtCore.Signal(str)
    engine_stopped = QtCore.Signal(str)

    def __init__(
        self,
        on_video: Callable[[int, int, bytes], None],
        on_audio: Callable[[int, int, bytes], None],
        parent: QtCore.QObject | None = None,
    ) -> None:
        super().__init__(parent)
        self._on_video = on_video
        self._on_audio = on_audio
        self._proc: subprocess.Popen | None = None
        self._stop_event = threading.Event()
        self._reader_thread: threading.Thread | None = None
        self._stderr_thread: threading.Thread | None = None
        self._sock: socket.socket | None = None

    def start(self, cfg: AppConfig) -> None:
        self.stop()
        binary = find_engine_binary()
        if binary is None:
            self.engine_stopped.emit(
                "未找到 Rust 引擎可执行文件，请先在 engine 目录运行 cargo build --release"
            )
            return

        cmd = [
            str(binary),
            "--name", cfg.device_name,
            "--width", str(cfg.width),
            "--height", str(cfg.height),
            "--max-fps", str(cfg.max_fps),
            "--refresh-rate", str(cfg.refresh_rate),
            "--hevc", "true" if cfg.hevc else "false",
            "--overscanned", "true" if cfg.overscanned else "false",
            "--rtsp-port", str(cfg.rtsp_port),
            "--mirror-port", str(cfg.mirror_port),
            "--audio-data-port", str(cfg.audio_data_port),
            "--audio-control-port", str(cfg.audio_control_port),
            "--timing-port", str(cfg.timing_port),
            "--ipc-port", str(cfg.ipc_port),
            "--mac", cfg.mac_address,
        ]

        creationflags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
        self._stop_event.clear()
        self._proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            creationflags=creationflags,
        )

        self._stderr_thread = threading.Thread(
            target=self._read_stderr, args=(self._proc,), daemon=True
        )
        self._stderr_thread.start()

        self._reader_thread = threading.Thread(
            target=self._ipc_loop, args=(cfg.ipc_port,), daemon=True
        )
        self._reader_thread.start()

    def _read_stderr(self, proc: subprocess.Popen) -> None:
        assert proc.stderr is not None
        for raw in iter(proc.stderr.readline, b""):
            if self._stop_event.is_set():
                break
            line = raw.decode("utf-8", errors="replace").strip()
            if line:
                self.log_received.emit(line)

    def _recv_exact(self, sock: socket.socket, n: int) -> bytes | None:
        buf = bytearray(n)
        view = memoryview(buf)
        got = 0
        while got < n:
            chunk = sock.recv_into(view[got:], n - got)
            if chunk == 0:
                return None
            got += chunk
        return bytes(buf)

    def _ipc_loop(self, ipc_port: int) -> None:
        # Retry connecting while Rust engine boots up
        sock: socket.socket | None = None
        for _ in range(50):
            if self._stop_event.is_set():
                return
            try:
                s = socket.create_connection(("127.0.0.1", ipc_port), timeout=1.0)
                s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                s.settimeout(None)
                sock = s
                break
            except OSError:
                time.sleep(0.1)

        if sock is None:
            if not self._stop_event.is_set():
                self.engine_stopped.emit("无法连接到 Rust 引擎 IPC 端口")
            return

        self._sock = sock
        try:
            while not self._stop_event.is_set():
                hdr = self._recv_exact(sock, 14)
                if hdr is None:
                    break
                msg_type, codec_id, ts_ns, payload_len = struct.unpack("<BBQI", hdr)
                if payload_len > 32 * 1024 * 1024:
                    break
                payload = self._recv_exact(sock, payload_len) if payload_len > 0 else b""
                if payload is None:
                    break

                if msg_type == 1:
                    self._on_video(codec_id, ts_ns, payload)
                elif msg_type == 2:
                    self._on_audio(codec_id, ts_ns, payload)
                elif msg_type == 3:
                    try:
                        evt = json.loads(payload.decode("utf-8"))
                        self.event_received.emit(evt)
                    except Exception:
                        pass
        except OSError:
            pass
        finally:
            try:
                sock.close()
            except Exception:
                pass
            self._sock = None
            if not self._stop_event.is_set():
                self.engine_stopped.emit("Rust 引擎连接已断开")

    def stop(self) -> None:
        self._stop_event.set()
        if self._sock is not None:
            try:
                self._sock.shutdown(socket.SHUT_RDWR)
                self._sock.close()
            except Exception:
                pass
            self._sock = None
        if self._proc is not None:
            try:
                self._proc.terminate()
                self._proc.wait(timeout=2.0)
            except Exception:
                try:
                    self._proc.kill()
                except Exception:
                    pass
            self._proc = None
