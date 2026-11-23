from __future__ import annotations

import os
from pathlib import Path
from PySide6 import QtCore, QtGui, QtWidgets

from app.audio_player import AudioPlayer
from app.config import AppConfig, QUALITY_PRESETS
from app.engine_bridge import EngineBridge
from app.recorder import ScreenRecorder
from app.ui_components import (
    Card,
    FieldBlock,
    IconButton,
    SegmentedTabs,
    StatusPill,
    ToggleSwitch,
    build_stylesheet,
)
from app.video_decoder import VideoDecoder
from app.video_widget import VideoCanvas


class MainWindow(QtWidgets.QMainWindow):
    frame_ready = QtCore.Signal(object)  # numpy RGB ndarray

    def __init__(self) -> None:
        super().__init__()
        self.cfg = AppConfig.load()
        self.setWindowTitle("AirPlay Hassy")
        self.resize(1360, 840)
        self.setMinimumSize(900, 560)
        self.setStyleSheet(build_stylesheet())

        self._clean_feed = False
        self._saved_geometry: QtCore.QByteArray | None = None
        self._panel_was_visible = True
        self._applying_preset = False

        self.decoder = VideoDecoder(prefer_hw=self.cfg.hw_decode)
        self.audio = AudioPlayer()
        self.audio.enabled = self.cfg.audio_enabled
        self.audio.local_volume = self.cfg.volume
        self.audio.follow_ios_volume = self.cfg.follow_ios_volume

        self.recorder = ScreenRecorder()
        self.audio.add_pcm_listener(self.recorder.write_audio_pcm)

        self.bridge = EngineBridge(
            on_video=self._handle_video_packet,
            on_audio=self._handle_audio_packet,
            parent=self,
        )
        self.bridge.event_received.connect(self._on_engine_event)
        self.bridge.log_received.connect(self._append_log)
        self.bridge.engine_stopped.connect(self._on_engine_stopped)
        self.frame_ready.connect(self._on_frame_ready)

        self._build_ui()
        self._bind_shortcuts()

        # Timer for recording duration updates
        self._rec_timer = QtCore.QTimer(self)
        self._rec_timer.setInterval(300)
        self._rec_timer.timeout.connect(self._update_recording_indicator)
        self._rec_timer.start()

        # Apply initial window flags
        if self.cfg.always_on_top:
            self._apply_always_on_top(True)

        # Start Rust engine
        self._start_engine()

    # ------------------------------------------------------------------ UI --

    def _build_ui(self) -> None:
        central = QtWidgets.QWidget()
        central.setObjectName("Root")
        self.setCentralWidget(central)
        root_layout = QtWidgets.QHBoxLayout(central)
        root_layout.setContentsMargins(0, 0, 0, 0)
        root_layout.setSpacing(0)

        # ---------------- Sidebar ----------------
        self.side_panel = QtWidgets.QFrame()
        self.side_panel.setObjectName("Sidebar")
        self.side_panel.setFixedWidth(328)
        side = QtWidgets.QVBoxLayout(self.side_panel)
        side.setContentsMargins(16, 18, 16, 16)
        side.setSpacing(14)

        # Brand
        brand = QtWidgets.QHBoxLayout()
        brand.setSpacing(10)
        mark = QtWidgets.QLabel("H")
        mark.setObjectName("BrandMark")
        mark.setFixedSize(34, 34)
        mark.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
        brand.addWidget(mark)
        brand_text = QtWidgets.QVBoxLayout()
        brand_text.setSpacing(0)
        title = QtWidgets.QLabel("AirPlay Hassy")
        title.setObjectName("BrandTitle")
        subtitle = QtWidgets.QLabel("无线投屏接收端")
        subtitle.setObjectName("BrandSub")
        brand_text.addWidget(title)
        brand_text.addWidget(subtitle)
        brand.addLayout(brand_text, 1)
        self.btn_hide_panel = IconButton("sidebar", "收起面板 (Tab)")
        self.btn_hide_panel.clicked.connect(self._toggle_side_panel)
        brand.addWidget(self.btn_hide_panel)
        side.addLayout(brand)

        # Live connection status
        self.status_pill = StatusPill()
        side.addWidget(self.status_pill)

        # Tabs + pages
        self.tabs = SegmentedTabs(["画质", "画面", "音频", "日志"])
        side.addWidget(self.tabs)
        self.pages = QtWidgets.QStackedWidget()
        self.pages.addWidget(self._scroll_page(self._build_quality_page()))
        self.pages.addWidget(self._scroll_page(self._build_display_page()))
        self.pages.addWidget(self._scroll_page(self._build_audio_page()))
        self.pages.addWidget(self._build_log_page())
        self.tabs.currentChanged.connect(self.pages.setCurrentIndex)
        side.addWidget(self.pages, 1)

        # Pinned action for the quality tab (stays visible while the page scrolls).
        self.btn_apply = QtWidgets.QPushButton("应用并重启服务")
        self.btn_apply.setObjectName("Primary")
        self.btn_apply.setCursor(QtCore.Qt.CursorShape.PointingHandCursor)
        self.btn_apply.clicked.connect(self._apply_quality_and_restart)
        self.tabs.currentChanged.connect(lambda i: self.btn_apply.setVisible(i == 0))
        side.addWidget(self.btn_apply)

        # Footer: recording
        footer = QtWidgets.QHBoxLayout()
        footer.setSpacing(8)
        self.btn_rec = QtWidgets.QPushButton("●  开始录制")
        self.btn_rec.setObjectName("Record")
        self.btn_rec.setCheckable(True)
        self.btn_rec.setCursor(QtCore.Qt.CursorShape.PointingHandCursor)
        self.btn_rec.setToolTip("录制 MP4 (Ctrl+R)")
        self.btn_rec.toggled.connect(self._toggle_recording)
        footer.addWidget(self.btn_rec, 1)
        self.btn_open_rec = IconButton("folder", "打开录屏文件夹")
        self.btn_open_rec.clicked.connect(self._open_recordings_dir)
        footer.addWidget(self.btn_open_rec)
        side.addLayout(footer)

        # ---------------- Video canvas ----------------
        self.canvas = VideoCanvas()
        self.canvas.set_smooth_scaling(self.cfg.smooth_scaling)
        self.canvas.set_sharpen(self.cfg.sharpen)
        self.canvas.set_show_hud(self.cfg.show_hud)
        self.canvas.double_clicked.connect(self._toggle_fullscreen)
        self.canvas.context_menu_requested.connect(self._show_canvas_menu)

        root_layout.addWidget(self.side_panel)
        root_layout.addWidget(self.canvas, 1)

    @staticmethod
    def _scroll_page(content: QtWidgets.QWidget) -> QtWidgets.QScrollArea:
        scroll = QtWidgets.QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setHorizontalScrollBarPolicy(QtCore.Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        scroll.setWidget(content)
        return scroll

    @staticmethod
    def _page() -> tuple[QtWidgets.QWidget, QtWidgets.QVBoxLayout]:
        w = QtWidgets.QWidget()
        lay = QtWidgets.QVBoxLayout(w)
        lay.setContentsMargins(0, 2, 2, 6)
        lay.setSpacing(16)
        return w, lay

    def _build_quality_page(self) -> QtWidgets.QWidget:
        page, lay = self._page()

        dev = Card("设备")
        self.ed_name = QtWidgets.QLineEdit(self.cfg.device_name)
        self.ed_name.setPlaceholderText("AirPlay-Hassy")
        dev.add(FieldBlock("在 iPhone「屏幕镜像」列表中显示的名称", self.ed_name))
        lay.addWidget(dev)

        q = Card("画质协商")
        self.cb_preset = QtWidgets.QComboBox()
        self.cb_preset.addItems(list(QUALITY_PRESETS.keys()))
        # Don't let the longest preset name dictate the sidebar width.
        self.cb_preset.setSizeAdjustPolicy(
            QtWidgets.QComboBox.SizeAdjustPolicy.AdjustToMinimumContentsLengthWithIcon
        )
        self.cb_preset.setMinimumContentsLength(8)
        self.cb_preset.view().setMinimumWidth(300)
        self.cb_preset.setCurrentIndex(self._preset_index_for_cfg())
        self.cb_preset.currentTextChanged.connect(self._on_preset_changed)
        q.add(FieldBlock("预设", self.cb_preset))

        def spin(lo: int, hi: int, val: int, step: int = 1, suffix: str = "") -> QtWidgets.QSpinBox:
            s = QtWidgets.QSpinBox()
            s.setRange(lo, hi)
            s.setSingleStep(step)
            s.setValue(val)
            s.setSuffix(suffix)
            # Arrow buttons are hidden via QSS (NoButtons + QSS collapses the editor to 1px).
            s.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
            return s

        self.sp_width = spin(320, 7680, self.cfg.width, 16)
        self.sp_height = spin(240, 4320, self.cfg.height, 16)
        res_row = QtWidgets.QHBoxLayout()
        res_row.setSpacing(8)
        res_row.addWidget(self.sp_width, 1)
        times = QtWidgets.QLabel("×")
        times.setObjectName("RowDesc")
        res_row.addWidget(times)
        res_row.addWidget(self.sp_height, 1)
        q.add(FieldBlock("分辨率（像素）", res_row))

        self.sp_fps = spin(15, 240, self.cfg.max_fps, suffix=" fps")
        self.sp_rr = spin(15, 240, self.cfg.refresh_rate, suffix=" Hz")
        fps_row = QtWidgets.QHBoxLayout()
        fps_row.setSpacing(8)
        fps_row.addWidget(self.sp_fps, 1)
        fps_row.addWidget(self.sp_rr, 1)
        q.add(FieldBlock("最大帧率 / 刷新率", fps_row))

        self.chk_hevc = ToggleSwitch(self.cfg.hevc)
        q.add_row("H.265 / HEVC", self.chk_hevc, "同码率下更清晰，2K/4K 推荐开启")
        self.chk_overscan = ToggleSwitch(self.cfg.overscanned)
        q.add_row("过扫描标记", self.chk_overscan, "overscanned，一般保持关闭")
        lay.addWidget(q)

        # Let inputs shrink to the sidebar width instead of forcing horizontal overflow.
        for w in (self.ed_name, self.cb_preset, self.sp_width, self.sp_height, self.sp_fps, self.sp_rr):
            w.setSizePolicy(QtWidgets.QSizePolicy.Policy.Ignored, QtWidgets.QSizePolicy.Policy.Fixed)

        # Editing any field by hand switches the preset label to "自定义画质".
        for w in (self.sp_width, self.sp_height, self.sp_fps, self.sp_rr):
            w.valueChanged.connect(self._mark_custom_preset)
        self.chk_hevc.toggled.connect(self._mark_custom_preset)

        hint = QtWidgets.QLabel("应用后需在 iPhone 上断开并重新镜像一次，新画质才会生效。")
        hint.setObjectName("Hint")
        hint.setWordWrap(True)
        lay.addWidget(hint)
        lay.addStretch()
        return page

    def _build_display_page(self) -> QtWidgets.QWidget:
        page, lay = self._page()

        dec = Card("解码与画面")
        self.chk_hwdec = ToggleSwitch(self.cfg.hw_decode)
        self.chk_hwdec.toggled.connect(self._on_hwdec_toggled)
        dec.add_row("GPU 硬件解码", self.chk_hwdec, "NVIDIA NVDEC，不可用时自动回退 CPU")
        self.chk_smooth = ToggleSwitch(self.cfg.smooth_scaling)
        self.chk_smooth.toggled.connect(self._on_smooth_toggled)
        dec.add_row("平滑缩放", self.chk_smooth, "关闭后为像素锐利缩放")
        self.chk_sharpen = ToggleSwitch(self.cfg.sharpen)
        self.chk_sharpen.toggled.connect(self._on_sharpen_toggled)
        dec.add_row("锐化增强", self.chk_sharpen)
        self.chk_hud = ToggleSwitch(self.cfg.show_hud)
        self.chk_hud.toggled.connect(self._on_hud_toggled)
        dec.add_row("性能浮层", self.chk_hud, "FPS、码率、解码器 · 快捷键 I")
        lay.addWidget(dec)

        win = Card("窗口")
        self.btn_top = ToggleSwitch(self.cfg.always_on_top)
        self.btn_top.toggled.connect(self._apply_always_on_top)
        win.add_row("窗口置顶", self.btn_top, "快捷键 T")
        self.btn_mirror = ToggleSwitch(False)
        self.btn_mirror.toggled.connect(lambda v: self.canvas.set_mirror_h(v))
        win.add_row("水平镜像", self.btn_mirror, "快捷键 M")
        self.btn_clean = ToggleSwitch(False)
        self.btn_clean.toggled.connect(self._set_clean_feed)
        win.add_row("纯净模式", self.btn_clean, "无边框无浮层，适合 OBS 捕获 · B 切换")
        lay.addWidget(win)

        actions = QtWidgets.QHBoxLayout()
        actions.setSpacing(8)
        self.btn_rot = QtWidgets.QPushButton("旋转 90°")
        self.btn_rot.setToolTip("R")
        self.btn_rot.clicked.connect(lambda: self.canvas.rotate_cw())
        self.btn_full = QtWidgets.QPushButton("全屏")
        self.btn_full.setToolTip("F11 / 双击画面")
        self.btn_full.clicked.connect(self._toggle_fullscreen)
        for b in (self.btn_rot, self.btn_full):
            b.setCursor(QtCore.Qt.CursorShape.PointingHandCursor)
            actions.addWidget(b, 1)
        lay.addLayout(actions)
        lay.addStretch()
        return page

    def _build_audio_page(self) -> QtWidgets.QWidget:
        page, lay = self._page()
        a = Card("音频")
        self.chk_audio = ToggleSwitch(self.cfg.audio_enabled)
        self.chk_audio.toggled.connect(self._on_audio_toggled)
        a.add_row("播放 iOS 音频", self.chk_audio, "AAC-ELD / ALAC / AAC-LC")
        self.chk_follow_vol = ToggleSwitch(self.cfg.follow_ios_volume)
        self.chk_follow_vol.toggled.connect(self._on_follow_vol_toggled)
        a.add_row("跟随 iPhone 音量键", self.chk_follow_vol)

        vol_row = QtWidgets.QHBoxLayout()
        vol_row.setSpacing(10)
        self.sl_vol = QtWidgets.QSlider(QtCore.Qt.Orientation.Horizontal)
        self.sl_vol.setRange(0, 120)
        self.sl_vol.setValue(int(self.cfg.volume * 100))
        self.sl_vol.valueChanged.connect(self._on_volume_changed)
        self.lbl_vol = QtWidgets.QLabel(f"{int(self.cfg.volume * 100)}%")
        self.lbl_vol.setObjectName("ValueBadge")
        self.lbl_vol.setAlignment(
            QtCore.Qt.AlignmentFlag.AlignRight | QtCore.Qt.AlignmentFlag.AlignVCenter
        )
        vol_row.addWidget(self.sl_vol, 1)
        vol_row.addWidget(self.lbl_vol)
        a.add(FieldBlock("本地音量", vol_row))
        lay.addWidget(a)
        lay.addStretch()
        return page

    def _build_log_page(self) -> QtWidgets.QWidget:
        page = QtWidgets.QWidget()
        lay = QtWidgets.QVBoxLayout(page)
        lay.setContentsMargins(0, 2, 0, 0)
        lay.setSpacing(8)
        bar = QtWidgets.QHBoxLayout()
        lbl = QtWidgets.QLabel("引擎日志")
        lbl.setObjectName("CardTitle")
        bar.addWidget(lbl, 1)
        b_copy = IconButton("copy", "复制全部")
        b_copy.clicked.connect(
            lambda: QtWidgets.QApplication.clipboard().setText(self.log_box.toPlainText())
        )
        b_clear = IconButton("clear", "清空")
        b_clear.clicked.connect(lambda: self.log_box.clear())
        bar.addWidget(b_copy)
        bar.addWidget(b_clear)
        lay.addLayout(bar)
        self.log_box = QtWidgets.QPlainTextEdit()
        self.log_box.setObjectName("Log")
        self.log_box.setReadOnly(True)
        lay.addWidget(self.log_box, 1)
        return page

    def _set_status(self, state: str, text: str, sub: str | None = None) -> None:
        self.status_pill.set_state(state, text, sub)

    def _bind_shortcuts(self) -> None:
        QtGui.QShortcut(QtGui.QKeySequence("Tab"), self, activated=self._toggle_side_panel)
        QtGui.QShortcut(QtGui.QKeySequence("F11"), self, activated=self._toggle_fullscreen)
        QtGui.QShortcut(QtGui.QKeySequence("Escape"), self, activated=self._exit_special_modes)
        QtGui.QShortcut(QtGui.QKeySequence("R"), self, activated=lambda: self.canvas.rotate_cw())
        QtGui.QShortcut(QtGui.QKeySequence("M"), self, activated=lambda: self.btn_mirror.toggle())
        QtGui.QShortcut(QtGui.QKeySequence("T"), self, activated=lambda: self.btn_top.toggle())
        QtGui.QShortcut(QtGui.QKeySequence("B"), self, activated=lambda: self.btn_clean.toggle())
        QtGui.QShortcut(QtGui.QKeySequence("I"), self, activated=lambda: self.chk_hud.toggle())
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+R"), self, activated=lambda: self.btn_rec.toggle())

    def _show_canvas_menu(self, global_pos: QtCore.QPoint) -> None:
        menu = QtWidgets.QMenu(self)
        menu.setStyleSheet(self.styleSheet())
        act_panel = menu.addAction("显示 / 隐藏面板\tTab")
        act_clean = menu.addAction("纯净模式\tB")
        act_full = menu.addAction("全屏\tF11")
        act_top = menu.addAction("窗口置顶\tT")
        menu.addSeparator()
        act_rot = menu.addAction("顺时针旋转 90°\tR")
        act_mir = menu.addAction("水平镜像\tM")
        act_hud = menu.addAction("性能浮层\tI")
        menu.addSeparator()
        act_rec = menu.addAction(
            "停止录制\tCtrl+R" if self.recorder.is_recording else "开始录制\tCtrl+R"
        )
        for act, checked in (
            (act_clean, self._clean_feed),
            (act_full, self.isFullScreen()),
            (act_top, self.btn_top.isChecked()),
            (act_mir, self.btn_mirror.isChecked()),
            (act_hud, self.chk_hud.isChecked()),
        ):
            act.setCheckable(True)
            act.setChecked(checked)

        chosen = menu.exec(global_pos)
        if chosen == act_panel:
            self._toggle_side_panel()
        elif chosen == act_clean:
            self.btn_clean.toggle()
        elif chosen == act_full:
            self._toggle_fullscreen()
        elif chosen == act_top:
            self.btn_top.toggle()
        elif chosen == act_rot:
            self.canvas.rotate_cw()
        elif chosen == act_mir:
            self.btn_mirror.toggle()
        elif chosen == act_hud:
            self.chk_hud.toggle()
        elif chosen == act_rec:
            self.btn_rec.toggle()

    # ---- Config & Engine Management ----

    def _on_preset_changed(self, preset_name: str) -> None:
        spec = QUALITY_PRESETS.get(preset_name, {})
        if not spec:
            return
        self._applying_preset = True
        try:
            self.sp_width.setValue(spec["width"])
            self.sp_height.setValue(spec["height"])
            self.sp_fps.setValue(spec["max_fps"])
            self.sp_rr.setValue(spec["refresh_rate"])
            self.chk_hevc.setChecked(spec["hevc"])
        finally:
            self._applying_preset = False

    def _preset_index_for_cfg(self) -> int:
        """Pick the preset that matches the saved config (also migrates old long preset names)."""
        idx = self.cb_preset.findText(self.cfg.preset)
        if idx >= 0:
            return idx
        for name, spec in QUALITY_PRESETS.items():
            if spec and (
                spec["width"], spec["height"], spec["max_fps"], spec["refresh_rate"], spec["hevc"]
            ) == (self.cfg.width, self.cfg.height, self.cfg.max_fps, self.cfg.refresh_rate, self.cfg.hevc):
                return self.cb_preset.findText(name)
        return max(0, self.cb_preset.findText("自定义画质"))

    def _mark_custom_preset(self, *_args) -> None:
        if self._applying_preset:
            return
        spec = QUALITY_PRESETS.get(self.cb_preset.currentText(), {})
        if not spec:
            return
        if (
            spec["width"] != self.sp_width.value()
            or spec["height"] != self.sp_height.value()
            or spec["max_fps"] != self.sp_fps.value()
            or spec["refresh_rate"] != self.sp_rr.value()
            or spec["hevc"] != self.chk_hevc.isChecked()
        ):
            idx = self.cb_preset.findText("自定义画质")
            if idx >= 0:
                self.cb_preset.blockSignals(True)
                self.cb_preset.setCurrentIndex(idx)
                self.cb_preset.blockSignals(False)

    def _apply_quality_and_restart(self) -> None:
        self.cfg.device_name = self.ed_name.text().strip() or "AirPlay-Hassy"
        self.cfg.preset = self.cb_preset.currentText()
        self.cfg.width = self.sp_width.value()
        self.cfg.height = self.sp_height.value()
        self.cfg.max_fps = self.sp_fps.value()
        self.cfg.refresh_rate = self.sp_rr.value()
        self.cfg.hevc = self.chk_hevc.isChecked()
        self.cfg.overscanned = self.chk_overscan.isChecked()
        self.cfg.save()

        self.canvas.clear_frame()
        self.decoder.reset()
        self.audio.flush()
        self._append_log(
            f"[UI] 重启引擎: {self.cfg.device_name} {self.cfg.width}x{self.cfg.height}"
            f"@{self.cfg.max_fps}fps HEVC={self.cfg.hevc}"
        )
        self._start_engine()

    def _start_engine(self) -> None:
        codec_tag = "HEVC" if self.cfg.hevc else "H.264"
        self.canvas.device_name = self.cfg.device_name
        self.canvas.advertised_info = (
            f"{self.cfg.width}×{self.cfg.height} · {self.cfg.max_fps}fps · {codec_tag}"
        )
        self.canvas.update()
        self._set_status("starting", "正在启动引擎…", "")
        self.bridge.start(self.cfg)

    # ---- Media Handlers (called on high-priority IPC thread) ----

    def _handle_video_packet(self, codec_id: int, ts_ns: int, annexb: bytes) -> None:
        frames = self.decoder.decode(codec_id, annexb, ts_ns)
        for rgb in frames:
            if self.recorder.is_recording:
                self.recorder.write_video_rgb(rgb)
            self.frame_ready.emit(rgb)

    def _handle_audio_packet(self, ct: int, ts_ns: int, payload: bytes) -> None:
        self.audio.feed(ct, payload, ts_ns)

    @QtCore.Slot(object)
    def _on_frame_ready(self, rgb) -> None:
        self.canvas.decoder_label = self.decoder.active_decoder_name
        self.canvas.update_frame(rgb)

    @QtCore.Slot(dict)
    def _on_engine_event(self, evt: dict) -> None:
        etype = evt.get("type", "")
        if etype == "engine_ready":
            self.canvas.local_ip = evt.get("local_ip", "未知")
            self.canvas.device_name = evt.get("name", self.cfg.device_name)
            self.canvas.update()
            self._set_status(
                "broadcast",
                f"广播中 · {self.canvas.device_name}",
                f"{self.canvas.local_ip}:{evt.get('rtsp_port', 7000)}",
            )
        elif etype == "client_connected":
            name = evt.get("name") or evt.get("ip", "iOS 设备")
            model = evt.get("model", "")
            label = f"{name} ({model})" if model else name
            self.canvas.connected_client = label
            self.canvas.update()
            self._set_status("connected", f"已连接 · {name}", evt.get("ip", ""))
            self._append_log(f"[Connected] {label}")
        elif etype == "video_format":
            self.canvas.stream_codec = evt.get("codec", "-")
            w, h = evt.get("width", 0), evt.get("height", 0)
            if w and h:
                self.canvas.stream_resolution = f"{w}×{h}"
        elif etype == "video_stats":
            self.canvas.stream_codec = evt.get("codec", self.canvas.stream_codec)
            self.canvas.stream_fps = float(evt.get("fps", 0.0))
            self.canvas.stream_mbps = float(evt.get("bitrate_mbps", 0.0))
            self.canvas.decoder_label = self.decoder.active_decoder_name
            self.canvas.update()
        elif etype == "audio_format":
            self._append_log(f"[Audio] 音频流已建立: {evt.get('codec', '')} 44.1kHz Stereo")
        elif etype == "volume":
            self.audio.ios_volume = float(evt.get("linear", 1.0))
        elif etype == "audio_flush":
            self.audio.flush()
        elif etype in ("video_stopped", "client_disconnected"):
            self.canvas.clear_frame()
            self.decoder.reset()
            self.audio.flush()
            self._set_status(
                "broadcast",
                f"广播中 · {self.canvas.device_name}",
                f"{self.canvas.local_ip} · 上一台设备已断开",
            )

    @QtCore.Slot(str)
    def _append_log(self, line: str) -> None:
        self.log_box.appendPlainText(line)
        # Keep log buffer bounded
        doc = self.log_box.document()
        if doc.blockCount() > 400:
            cursor = QtGui.QTextCursor(doc)
            cursor.movePosition(QtGui.QTextCursor.MoveOperation.Start)
            cursor.movePosition(
                QtGui.QTextCursor.MoveOperation.Down,
                QtGui.QTextCursor.MoveMode.KeepAnchor,
                100,
            )
            cursor.removeSelectedText()

    @QtCore.Slot(str)
    def _on_engine_stopped(self, msg: str) -> None:
        self._set_status("error", "引擎异常", msg[:60])
        self._append_log(f"[Engine] {msg}")

    # ---- Real-time Render, Audio & Window Controls ----

    def _on_hwdec_toggled(self, checked: bool) -> None:
        self.cfg.hw_decode = checked
        self.cfg.save()
        self.decoder.set_prefer_hw(checked)

    def _on_smooth_toggled(self, checked: bool) -> None:
        self.cfg.smooth_scaling = checked
        self.cfg.save()
        self.canvas.set_smooth_scaling(checked)

    def _on_sharpen_toggled(self, checked: bool) -> None:
        self.cfg.sharpen = checked
        self.cfg.save()
        self.canvas.set_sharpen(checked)

    def _on_hud_toggled(self, checked: bool) -> None:
        self.cfg.show_hud = checked
        self.cfg.save()
        self.canvas.set_show_hud(checked)

    def _on_audio_toggled(self, checked: bool) -> None:
        self.cfg.audio_enabled = checked
        self.cfg.save()
        self.audio.enabled = checked

    def _on_volume_changed(self, value: int) -> None:
        self.lbl_vol.setText(f"{value}%")
        self.cfg.volume = value / 100.0
        self.cfg.save()
        self.audio.local_volume = self.cfg.volume

    def _on_follow_vol_toggled(self, checked: bool) -> None:
        self.cfg.follow_ios_volume = checked
        self.cfg.save()
        self.audio.follow_ios_volume = checked

    def _toggle_side_panel(self) -> None:
        if self._clean_feed:
            self.btn_clean.setChecked(False)
            return
        if self.isFullScreen():
            return
        self.side_panel.setVisible(not self.side_panel.isVisible())
        self.canvas.panel_hidden = not self.side_panel.isVisible()
        self.canvas.update()

    def _apply_always_on_top(self, checked: bool) -> None:
        self.cfg.always_on_top = checked
        self.cfg.save()
        flags = self.windowFlags()
        if checked:
            flags |= QtCore.Qt.WindowType.WindowStaysOnTopHint
        else:
            flags &= ~QtCore.Qt.WindowType.WindowStaysOnTopHint
        self.setWindowFlags(flags)
        self.show()

    def _toggle_fullscreen(self) -> None:
        if self.isFullScreen():
            self.showNormal()
            if not self._clean_feed and self._panel_was_visible:
                self.side_panel.show()
        else:
            self._panel_was_visible = self.side_panel.isVisible()
            self.side_panel.hide()
            self.showFullScreen()
        self.canvas.panel_hidden = not self.side_panel.isVisible()

    def _set_clean_feed(self, enabled: bool) -> None:
        self._clean_feed = enabled
        self.canvas.set_clean_feed(enabled)
        flags = self.windowFlags()
        if enabled:
            self._saved_geometry = self.saveGeometry()
            self._panel_was_visible = self.side_panel.isVisible()
            self.side_panel.hide()
            flags |= QtCore.Qt.WindowType.FramelessWindowHint
            self.setWindowFlags(flags)
            self.show()
        else:
            flags &= ~QtCore.Qt.WindowType.FramelessWindowHint
            self.setWindowFlags(flags)
            if self._panel_was_visible:
                self.side_panel.show()
            self.show()
            if self._saved_geometry is not None:
                self.restoreGeometry(self._saved_geometry)
        self.canvas.panel_hidden = not self.side_panel.isVisible()

    def _exit_special_modes(self) -> None:
        if self.isFullScreen():
            self._toggle_fullscreen()
        elif self._clean_feed:
            self.btn_clean.setChecked(False)

    # ---- Recording ----

    def _toggle_recording(self, checked: bool) -> None:
        if checked:
            out_dir = Path(self.cfg.recordings_dir)
            path = self.recorder.start(out_dir)
            self.btn_rec.setText("■  停止录制  00:00")
            self._append_log(f"[Recorder] 开始录制 -> {path.name}")
        else:
            saved = self.recorder.stop()
            self.btn_rec.setText("●  开始录制")
            self.canvas.recording_text = ""
            self.canvas.update()
            if saved is not None and saved.exists():
                size_mb = saved.stat().st_size / (1024 * 1024)
                self._append_log(f"[Recorder] 已保存: {saved} ({size_mb:.1f} MB)")
                self.canvas.show_toast(f"录屏已保存 · {saved.name} · {size_mb:.1f} MB")

    def _update_recording_indicator(self) -> None:
        if not self.recorder.is_recording:
            return
        sec = int(self.recorder.elapsed_seconds)
        mm, ss = divmod(sec, 60)
        self.canvas.recording_text = f"REC {mm:02d}:{ss:02d}"
        self.btn_rec.setText(f"■  停止录制  {mm:02d}:{ss:02d}")
        self.canvas.update()

    def _open_recordings_dir(self) -> None:
        out_dir = Path(self.cfg.recordings_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        os.startfile(str(out_dir))  # type: ignore[attr-defined]

    def closeEvent(self, event: QtGui.QCloseEvent) -> None:
        if self.recorder.is_recording:
            self.recorder.stop()
        self.bridge.stop()
        self.audio.close()
        self.decoder.reset()
        super().closeEvent(event)
