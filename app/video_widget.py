from __future__ import annotations

import numpy as np
from PySide6 import QtCore, QtGui, QtWidgets

from app.ui_components import TOKENS

BG = TOKENS["bg"]
UI_FAMILY = "Microsoft YaHei UI"
MONO_FAMILY = "Cascadia Mono"


class VideoCanvas(QtWidgets.QWidget):
    """High-performance AirPlay video display surface with rotation, mirroring,
    sharpening, clean-feed mode, and live telemetry HUD overlay.
    """

    double_clicked = QtCore.Signal()
    context_menu_requested = QtCore.Signal(QtCore.QPoint)

    def __init__(self, parent: QtWidgets.QWidget | None = None) -> None:
        super().__init__(parent)
        self.setMinimumSize(480, 320)
        self.setSizePolicy(
            QtWidgets.QSizePolicy.Policy.Expanding,
            QtWidgets.QSizePolicy.Policy.Expanding,
        )
        self.setAttribute(QtCore.Qt.WidgetAttribute.WA_OpaquePaintEvent)

        self._pixmap: QtGui.QPixmap | None = None
        self._rotation: int = 0  # 0, 90, 180, 270
        self._mirror_h: bool = False
        self._smooth_scaling: bool = True
        self._sharpen: bool = False
        self._show_hud: bool = True
        self._clean_feed: bool = False

        # Idle screen info
        self.device_name: str = "AirPlay-Hassy"
        self.local_ip: str = "正在获取..."
        self.advertised_info: str = "2560×1440 @ 60fps (HEVC)"

        # Live stats HUD
        self.connected_client: str = ""
        self.stream_codec: str = "-"
        self.stream_resolution: str = "-"
        self.stream_fps: float = 0.0
        self.stream_mbps: float = 0.0
        self.decoder_label: str = "-"
        self.recording_text: str = ""
        self.panel_hidden: bool = False

        # Transient toast message (bottom-center)
        self._toast_text: str = ""
        self._toast_timer = QtCore.QTimer(self)
        self._toast_timer.setSingleShot(True)
        self._toast_timer.timeout.connect(self._clear_toast)

    def set_rotation(self, degrees: int) -> None:
        self._rotation = degrees % 360
        self.update()

    def rotate_cw(self) -> None:
        self.set_rotation(self._rotation + 90)

    def set_mirror_h(self, enabled: bool) -> None:
        self._mirror_h = enabled
        self.update()

    def set_smooth_scaling(self, smooth: bool) -> None:
        self._smooth_scaling = smooth
        self.update()

    def set_sharpen(self, sharpen: bool) -> None:
        self._sharpen = sharpen
        self.update()

    def set_show_hud(self, show: bool) -> None:
        self._show_hud = show
        self.update()

    def set_clean_feed(self, clean: bool) -> None:
        self._clean_feed = clean
        self.update()

    def clear_frame(self) -> None:
        self._pixmap = None
        self.connected_client = ""
        self.stream_fps = 0.0
        self.stream_mbps = 0.0
        self.update()

    def update_frame(self, rgb: np.ndarray) -> None:
        if self._sharpen:
            # Fast vectorized subtle unsharp mask on numpy array
            arr = rgb.astype(np.int16)
            blurred = (
                arr[1:-1, 1:-1] * 4
                + arr[:-2, 1:-1]
                + arr[2:, 1:-1]
                + arr[1:-1, :-2]
                + arr[1:-1, 2:]
            ) >> 3
            sharpened = arr.copy()
            sharpened[1:-1, 1:-1] = np.clip(
                arr[1:-1, 1:-1] + ((arr[1:-1, 1:-1] - blurred) >> 1), 0, 255
            )
            rgb = sharpened.astype(np.uint8)

        h, w, ch = rgb.shape
        if not rgb.flags["C_CONTIGUOUS"]:
            rgb = np.ascontiguousarray(rgb)
        qimg = QtGui.QImage(
            rgb.data, w, h, w * ch, QtGui.QImage.Format.Format_RGB888
        ).copy()

        if self._rotation != 0 or self._mirror_h:
            tr = QtGui.QTransform()
            if self._mirror_h:
                tr.scale(-1.0, 1.0)
            if self._rotation != 0:
                tr.rotate(self._rotation)
            qimg = qimg.transformed(tr, QtCore.Qt.TransformationMode.FastTransformation)

        self._pixmap = QtGui.QPixmap.fromImage(qimg)
        self.stream_resolution = f"{w}×{h}"
        self.update()

    def show_toast(self, text: str, ms: int = 3500) -> None:
        self._toast_text = text
        self._toast_timer.start(ms)
        self.update()

    def _clear_toast(self) -> None:
        self._toast_text = ""
        self.update()

    def mouseDoubleClickEvent(self, event: QtGui.QMouseEvent) -> None:
        if event.button() == QtCore.Qt.MouseButton.LeftButton:
            self.double_clicked.emit()
        super().mouseDoubleClickEvent(event)

    def contextMenuEvent(self, event: QtGui.QContextMenuEvent) -> None:
        self.context_menu_requested.emit(event.globalPos())

    def paintEvent(self, _event: QtGui.QPaintEvent) -> None:
        painter = QtGui.QPainter(self)
        painter.fillRect(self.rect(), QtGui.QColor(BG))

        if self._pixmap is None:
            if not self._clean_feed:
                self._paint_idle(painter)
                self._paint_overlays(painter)
            painter.end()
            return

        painter.setRenderHint(
            QtGui.QPainter.RenderHint.SmoothPixmapTransform, self._smooth_scaling
        )
        target = self._fit_rect(self._pixmap.size(), self.size())
        painter.drawPixmap(target, self._pixmap)

        if not self._clean_feed:
            if self._show_hud:
                self._paint_hud(painter)
            self._paint_overlays(painter)
        painter.end()

    def _fit_rect(self, src: QtCore.QSize, dst: QtCore.QSize) -> QtCore.QRect:
        if src.width() <= 0 or src.height() <= 0:
            return QtCore.QRect(0, 0, dst.width(), dst.height())
        scaled = src.scaled(dst, QtCore.Qt.AspectRatioMode.KeepAspectRatio)
        x = (dst.width() - scaled.width()) // 2
        y = (dst.height() - scaled.height()) // 2
        return QtCore.QRect(x, y, scaled.width(), scaled.height())

    # ------------------------------------------------------------ painting --

    @staticmethod
    def _font(family: str, px: int, weight=QtGui.QFont.Weight.Normal) -> QtGui.QFont:
        f = QtGui.QFont(family)
        f.setPixelSize(px)
        f.setWeight(weight)
        return f

    def _paint_idle(self, painter: QtGui.QPainter) -> None:
        painter.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        painter.setRenderHint(QtGui.QPainter.RenderHint.TextAntialiasing)
        w, h = self.width(), self.height()
        cx = w / 2
        connecting = bool(self.connected_client)
        accent = QtGui.QColor(TOKENS["ok"] if connecting else TOKENS["accent"])

        # Soft glow behind the content
        glow = QtGui.QRadialGradient(QtCore.QPointF(cx, h * 0.42), max(320.0, min(w, h) * 0.6))
        c0 = QtGui.QColor(accent)
        c0.setAlpha(26)
        c1 = QtGui.QColor(accent)
        c1.setAlpha(0)
        glow.setColorAt(0.0, c0)
        glow.setColorAt(1.0, c1)
        painter.fillRect(self.rect(), glow)

        block_h = 300
        top = max(24.0, (h - block_h) / 2 - 20)

        # Cast icon: screen outline + triangle
        icon_w, icon_h = 64.0, 44.0
        screen = QtCore.QRectF(cx - icon_w / 2, top, icon_w, icon_h)
        pen = QtGui.QPen(accent, 2.4)
        pen.setJoinStyle(QtCore.Qt.PenJoinStyle.RoundJoin)
        painter.setPen(pen)
        painter.setBrush(QtCore.Qt.BrushStyle.NoBrush)
        painter.drawRoundedRect(screen, 7, 7)
        painter.setPen(QtCore.Qt.PenStyle.NoPen)
        painter.setBrush(QtGui.QColor(BG))
        painter.drawRect(QtCore.QRectF(cx - 20, screen.bottom() - 2, 40, 6))
        tri = QtGui.QPolygonF(
            [
                QtCore.QPointF(cx, screen.bottom() - 10),
                QtCore.QPointF(cx - 15, screen.bottom() + 12),
                QtCore.QPointF(cx + 15, screen.bottom() + 12),
            ]
        )
        painter.setBrush(accent)
        painter.drawPolygon(tri)

        y = screen.bottom() + 40

        # Title
        painter.setFont(self._font(UI_FAMILY, 24, QtGui.QFont.Weight.DemiBold))
        painter.setPen(QtGui.QColor(TOKENS["text"]))
        title = f"正在连接 {self.connected_client}…" if connecting else "等待投屏"
        painter.drawText(QtCore.QRectF(0, y, w, 34), QtCore.Qt.AlignmentFlag.AlignCenter, title)
        y += 44

        # Steps
        painter.setFont(self._font(UI_FAMILY, 13))
        painter.setPen(QtGui.QColor(TOKENS["text_2"]))
        painter.drawText(
            QtCore.QRectF(20, y, w - 40, 22),
            QtCore.Qt.AlignmentFlag.AlignCenter,
            "在 iPhone / iPad 打开控制中心 → 屏幕镜像 → 选择",
        )
        y += 34

        # Device name pill
        name_font = self._font(UI_FAMILY, 16, QtGui.QFont.Weight.DemiBold)
        fm = QtGui.QFontMetrics(name_font)
        pill_w = fm.horizontalAdvance(self.device_name) + 48
        pill = QtCore.QRectF(cx - pill_w / 2, y, pill_w, 40)
        fill = QtGui.QColor(accent)
        fill.setAlpha(34)
        border = QtGui.QColor(accent)
        border.setAlpha(120)
        painter.setBrush(fill)
        painter.setPen(QtGui.QPen(border, 1))
        painter.drawRoundedRect(pill, 20, 20)
        painter.setFont(name_font)
        painter.setPen(QtGui.QColor(TOKENS["text"]))
        painter.drawText(pill, QtCore.Qt.AlignmentFlag.AlignCenter, self.device_name)
        y += 64

        # Meta chips
        chips = [("画质", self.advertised_info), ("IP", self.local_ip)]
        label_font = self._font(UI_FAMILY, 11)
        value_font = self._font(MONO_FAMILY, 12)
        lfm, vfm = QtGui.QFontMetrics(label_font), QtGui.QFontMetrics(value_font)
        widths = [lfm.horizontalAdvance(a) + vfm.horizontalAdvance(b) + 36 for a, b in chips]
        gap = 10
        total = sum(widths) + gap * (len(chips) - 1)
        x = cx - total / 2
        for (label, value), cw in zip(chips, widths):
            r = QtCore.QRectF(x, y, cw, 30)
            painter.setBrush(QtGui.QColor(TOKENS["card"]))
            painter.setPen(QtGui.QPen(QtGui.QColor(TOKENS["border"]), 1))
            painter.drawRoundedRect(r, 8, 8)
            lw = lfm.horizontalAdvance(label)
            painter.setFont(label_font)
            painter.setPen(QtGui.QColor(TOKENS["muted"]))
            painter.drawText(
                QtCore.QRectF(r.left() + 12, r.top(), lw + 2, r.height()),
                QtCore.Qt.AlignmentFlag.AlignVCenter,
                label,
            )
            painter.setFont(value_font)
            painter.setPen(QtGui.QColor(TOKENS["text_2"]))
            painter.drawText(
                QtCore.QRectF(r.left() + 12 + lw + 10, r.top(), r.width(), r.height()),
                QtCore.Qt.AlignmentFlag.AlignVCenter,
                value,
            )
            x += cw + gap

        # Shortcut hint at the bottom
        hint = "双击 全屏   ·   Tab 面板   ·   B 纯净模式   ·   R 旋转   ·   右键 更多"
        painter.setFont(self._font(UI_FAMILY, 11))
        painter.setPen(QtGui.QColor(TOKENS["muted"]))
        painter.drawText(
            QtCore.QRectF(0, h - 40, w, 20), QtCore.Qt.AlignmentFlag.AlignCenter, hint
        )

    def _paint_hud(self, painter: QtGui.QPainter) -> None:
        painter.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        title = self.connected_client or "AirPlay"
        lines = [
            f"{self.stream_resolution}  ·  {self.stream_codec}",
            f"{self.stream_fps:5.1f} fps  ·  {self.stream_mbps:5.2f} Mbps  ·  {self.decoder_label}",
        ]
        title_font = self._font(UI_FAMILY, 12, QtGui.QFont.Weight.DemiBold)
        mono = self._font(MONO_FAMILY, 11)
        tfm, mfm = QtGui.QFontMetrics(title_font), QtGui.QFontMetrics(mono)
        inner_w = max(
            [tfm.horizontalAdvance(title) + 14] + [mfm.horizontalAdvance(ln) for ln in lines]
        )
        rect = QtCore.QRectF(16, 16, inner_w + 28, 22 + 18 * len(lines) + 18)

        painter.setBrush(QtGui.QColor(12, 13, 17, 205))
        painter.setPen(QtGui.QPen(QtGui.QColor(255, 255, 255, 22), 1))
        painter.drawRoundedRect(rect, 10, 10)

        # Live dot + title
        painter.setPen(QtCore.Qt.PenStyle.NoPen)
        painter.setBrush(QtGui.QColor(TOKENS["ok"]))
        painter.drawEllipse(QtCore.QRectF(rect.left() + 14, rect.top() + 16, 7, 7))
        painter.setFont(title_font)
        painter.setPen(QtGui.QColor(TOKENS["text"]))
        painter.drawText(QtCore.QPointF(rect.left() + 28, rect.top() + 24), title)

        painter.setFont(mono)
        painter.setPen(QtGui.QColor(TOKENS["text_2"]))
        y = rect.top() + 44
        for line in lines:
            painter.drawText(QtCore.QPointF(rect.left() + 14, y), line)
            y += 18

    def _paint_overlays(self, painter: QtGui.QPainter) -> None:
        painter.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        # Recording badge (top-right)
        if self.recording_text:
            f = self._font(MONO_FAMILY, 12, QtGui.QFont.Weight.DemiBold)
            fm = QtGui.QFontMetrics(f)
            bw = fm.horizontalAdvance(self.recording_text) + 38
            r = QtCore.QRectF(self.width() - bw - 16, 16, bw, 30)
            painter.setPen(QtCore.Qt.PenStyle.NoPen)
            painter.setBrush(QtGui.QColor(TOKENS["danger"]))
            painter.drawRoundedRect(r, 15, 15)
            painter.setBrush(QtGui.QColor("#FFFFFF"))
            painter.drawEllipse(QtCore.QRectF(r.left() + 12, r.center().y() - 4, 8, 8))
            painter.setFont(f)
            painter.setPen(QtGui.QColor("#FFFFFF"))
            painter.drawText(
                QtCore.QRectF(r.left() + 26, r.top(), r.width() - 30, r.height()),
                QtCore.Qt.AlignmentFlag.AlignVCenter,
                self.recording_text,
            )

        # Toast (bottom-center)
        if self._toast_text:
            f = self._font(UI_FAMILY, 12)
            fm = QtGui.QFontMetrics(f)
            tw = fm.horizontalAdvance(self._toast_text) + 32
            r = QtCore.QRectF((self.width() - tw) / 2, self.height() - 84, tw, 34)
            painter.setBrush(QtGui.QColor(TOKENS["card"]))
            painter.setPen(QtGui.QPen(QtGui.QColor(TOKENS["border_strong"]), 1))
            painter.drawRoundedRect(r, 10, 10)
            painter.setFont(f)
            painter.setPen(QtGui.QColor(TOKENS["text"]))
            painter.drawText(r, QtCore.Qt.AlignmentFlag.AlignCenter, self._toast_text)

