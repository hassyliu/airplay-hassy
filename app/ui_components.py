"""Reusable, self-painted UI building blocks for the AirPlay Hassy control panel."""
from __future__ import annotations

from PySide6 import QtCore, QtGui, QtWidgets

# ---------------------------------------------------------------------------
# Design tokens
# ---------------------------------------------------------------------------

TOKENS = {
    "bg": "#0C0D11",
    "sidebar": "#131419",
    "card": "#1A1B21",
    "card_hover": "#202128",
    "field": "#22232B",
    "border": "#2A2B33",
    "border_strong": "#363743",
    "text": "#ECEDF1",
    "text_2": "#A4A6B3",
    "muted": "#6E7080",
    "accent": "#6C8CFF",
    "accent_hover": "#8199FF",
    "accent_soft": "#262E4D",
    "ok": "#3DD68C",
    "warn": "#F5B544",
    "danger": "#FF5D5D",
    "danger_soft": "#3A1D22",
}

UI_FONT = '"Segoe UI Variable Text", "Microsoft YaHei UI", "Segoe UI", sans-serif'
MONO_FONT = '"Cascadia Mono", Consolas, monospace'


def _chevron_icon_path() -> str:
    """Render a small down-chevron PNG once (QSS can only reference image files)."""
    out_dir = QtCore.QDir(
        QtCore.QStandardPaths.writableLocation(QtCore.QStandardPaths.StandardLocation.TempLocation)
    )
    path = out_dir.filePath("airplay-hassy-chevron.png")
    pm = QtGui.QPixmap(40, 40)
    pm.fill(QtCore.Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    pen = QtGui.QPen(QtGui.QColor(TOKENS["text_2"]), 5)
    pen.setCapStyle(QtCore.Qt.PenCapStyle.RoundCap)
    pen.setJoinStyle(QtCore.Qt.PenJoinStyle.RoundJoin)
    p.setPen(pen)
    p.drawPolyline([QtCore.QPointF(8, 14), QtCore.QPointF(20, 26), QtCore.QPointF(32, 14)])
    p.end()
    pm.save(path, "PNG")
    return path.replace("\\", "/")


def build_stylesheet() -> str:
    qss = """
QMainWindow, QWidget#Root { background: @bg; }
QWidget { color: @text; font-family: @uifont; font-size: 13px; }
QToolTip { background: @field; color: @text; border: 1px solid @border_strong; padding: 4px 8px; border-radius: 6px; }

QFrame#Sidebar { background: @sidebar; border-right: 1px solid @border; }
QScrollArea, QScrollArea > QWidget > QWidget { background: transparent; border: none; }
QScrollBar:vertical { background: transparent; width: 8px; margin: 2px; }
QScrollBar::handle:vertical { background: @border_strong; border-radius: 3px; min-height: 24px; }
QScrollBar::add-line, QScrollBar::sub-line, QScrollBar::add-page, QScrollBar::sub-page { background: none; height: 0; }

QLabel#BrandTitle { font-size: 16px; font-weight: 600; color: @text; }
QLabel#BrandSub { font-size: 11px; color: @muted; }
QLabel#BrandMark { background: @accent; color: #0C0D11; border-radius: 9px; font-size: 16px; font-weight: 700; }
QLabel#CardTitle { font-size: 11px; font-weight: 600; color: @muted; letter-spacing: 0.6px; padding: 0 4px; }
QLabel#RowTitle { font-size: 13px; color: @text; }
QLabel#RowDesc { font-size: 11px; color: @muted; }
QLabel#Hint { font-size: 11px; color: @text_2; background: @accent_soft; border-radius: 8px; padding: 8px 10px; }
QLabel#ValueBadge { font-family: @monofont; font-size: 12px; color: @text_2; min-width: 40px; }

QFrame#Card { background: @card; border: 1px solid @border; border-radius: 12px; }
QFrame#Divider { background: @border; max-height: 1px; min-height: 1px; border: none; }

QFrame#StatusPill { background: @card; border: 1px solid @border; border-radius: 10px; }
QLabel#StatusText { font-size: 12px; color: @text; }
QLabel#StatusSub { font-size: 11px; color: @muted; font-family: @monofont; }

QFrame#Segmented { background: @bg; border: 1px solid @border; border-radius: 10px; }
QPushButton#SegBtn { background: transparent; border: none; border-radius: 7px; padding: 6px 0; color: @text_2; font-weight: 500; }
QPushButton#SegBtn:hover { color: @text; }
QPushButton#SegBtn:checked { background: @field; color: @text; }

QLineEdit, QSpinBox, QComboBox {
    background: @field; border: 1px solid transparent; border-radius: 8px;
    padding: 6px 10px; color: @text; selection-background-color: @accent;
}
QLineEdit:hover, QSpinBox:hover, QComboBox:hover { border-color: @border_strong; }
QLineEdit:focus, QSpinBox:focus, QComboBox:focus { border-color: @accent; }
QSpinBox { font-family: @monofont; }
QSpinBox::up-button, QSpinBox::down-button { width: 0; border: none; }
QSpinBox::up-arrow, QSpinBox::down-arrow { image: none; width: 0; height: 0; }
QComboBox { padding-right: 28px; }
QComboBox::drop-down { border: none; width: 28px; subcontrol-origin: padding; subcontrol-position: center right; }
QComboBox::down-arrow { image: url(@chevron); width: 10px; height: 10px; }
QComboBox QAbstractItemView {
    background: @field; border: 1px solid @border_strong; border-radius: 8px; padding: 4px;
    selection-background-color: @accent_soft; selection-color: @text; outline: none;
}

QPushButton {
    background: @field; border: 1px solid @border; border-radius: 8px;
    padding: 7px 12px; color: @text;
}
QPushButton:hover { background: @card_hover; border-color: @border_strong; }
QPushButton:pressed { background: @bg; }
QPushButton#Primary { background: @accent; border: none; color: #0C0D11; font-weight: 600; padding: 9px 12px; }
QPushButton#Primary:hover { background: @accent_hover; }
QPushButton#Record { background: @danger_soft; border: 1px solid #5A2A31; color: #FFB4B4; font-weight: 600; padding: 9px 12px; }
QPushButton#Record:hover { border-color: @danger; }
QPushButton#Record:checked { background: @danger; border-color: @danger; color: #FFFFFF; }
QToolButton#IconBtn {
    background: @field; border: 1px solid @border; border-radius: 8px; color: @text_2;
    min-width: 34px; min-height: 34px; max-width: 34px; max-height: 34px;
}
QToolButton#IconBtn:hover { color: @text; border-color: @border_strong; background: @card_hover; }
QToolButton#IconBtn:checked { color: @accent; border-color: @accent; background: @accent_soft; }

QSlider::groove:horizontal { height: 4px; background: @field; border-radius: 2px; }
QSlider::sub-page:horizontal { background: @accent; border-radius: 2px; }
QSlider::handle:horizontal { width: 14px; height: 14px; margin: -5px 0; border-radius: 7px; background: @text; }

QPlainTextEdit#Log {
    background: @bg; border: 1px solid @border; border-radius: 10px; padding: 6px;
    font-family: @monofont; font-size: 11px; color: @text_2;
}
QMenu { background: @card; border: 1px solid @border_strong; border-radius: 10px; padding: 6px; }
QMenu::item { padding: 7px 28px 7px 12px; border-radius: 6px; color: @text; }
QMenu::item:selected { background: @accent_soft; }
QMenu::separator { height: 1px; background: @border; margin: 5px 8px; }
"""
    qss = qss.replace("@uifont", UI_FONT).replace("@monofont", MONO_FONT)
    qss = qss.replace("@chevron", _chevron_icon_path())
    # Replace longer token names first so e.g. @accent_soft isn't clobbered by @accent.
    for key in sorted(TOKENS, key=len, reverse=True):
        qss = qss.replace(f"@{key}", TOKENS[key])
    return qss


# ---------------------------------------------------------------------------
# Icon glyphs (Windows Segoe Fluent / MDL2 icon font, with text fallback)
# ---------------------------------------------------------------------------

_ICON_FAMILY: str | None = None


def icon_font_family() -> str | None:
    global _ICON_FAMILY
    if _ICON_FAMILY is None:
        families = set(QtGui.QFontDatabase.families())
        for fam in ("Segoe Fluent Icons", "Segoe MDL2 Assets"):
            if fam in families:
                _ICON_FAMILY = fam
                break
        else:
            _ICON_FAMILY = ""
    return _ICON_FAMILY or None


GLYPHS = {
    "pin": ("", "顶"),
    "fullscreen": ("", "全"),
    "rotate": ("", "转"),
    "mirror": ("", "镜"),
    "clean": ("", "净"),
    "folder": ("", "夹"),
    "sidebar": ("", "≡"),
    "copy": ("", "复"),
    "clear": ("", "清"),
}


class IconButton(QtWidgets.QToolButton):
    def __init__(self, glyph: str, tooltip: str, checkable: bool = False, parent=None) -> None:
        super().__init__(parent)
        self.setObjectName("IconBtn")
        self.setCursor(QtCore.Qt.CursorShape.PointingHandCursor)
        self.setToolTip(tooltip)
        self.setCheckable(checkable)
        icon_char, fallback = GLYPHS.get(glyph, ("", glyph))
        family = icon_font_family()
        if family and icon_char:
            # Must go through the stylesheet: the global QSS font-family would
            # otherwise override setFont() and render the PUA glyphs as garbage.
            self.setStyleSheet(f'font-family: "{family}"; font-size: 14px;')
            self.setText(icon_char)
        else:
            self.setText(fallback)


# ---------------------------------------------------------------------------
# Toggle switch
# ---------------------------------------------------------------------------


class ToggleSwitch(QtWidgets.QAbstractButton):
    """iOS-style animated switch. Drop-in for a checkable button / checkbox."""

    def __init__(self, checked: bool = False, parent=None) -> None:
        super().__init__(parent)
        self.setCheckable(True)
        self.setCursor(QtCore.Qt.CursorShape.PointingHandCursor)
        self.setFixedSize(40, 22)
        self._offset = 1.0 if checked else 0.0
        self.setChecked(checked)
        self._anim = QtCore.QPropertyAnimation(self, b"offset", self)
        self._anim.setDuration(140)
        self._anim.setEasingCurve(QtCore.QEasingCurve.Type.OutCubic)
        self.toggled.connect(self._animate)

    def _get_offset(self) -> float:
        return self._offset

    def _set_offset(self, v: float) -> None:
        self._offset = v
        self.update()

    offset = QtCore.Property(float, _get_offset, _set_offset)

    def _animate(self, checked: bool) -> None:
        self._anim.stop()
        self._anim.setStartValue(self._offset)
        self._anim.setEndValue(1.0 if checked else 0.0)
        self._anim.start()

    def sizeHint(self) -> QtCore.QSize:
        return QtCore.QSize(40, 22)

    def paintEvent(self, _e) -> None:
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        off = QtGui.QColor(TOKENS["border_strong"])
        on = QtGui.QColor(TOKENS["accent"])
        t = self._offset
        track = QtGui.QColor(
            int(off.red() + (on.red() - off.red()) * t),
            int(off.green() + (on.green() - off.green()) * t),
            int(off.blue() + (on.blue() - off.blue()) * t),
        )
        if not self.isEnabled():
            track.setAlpha(90)
        r = self.rect().adjusted(0, 0, -1, -1)
        p.setPen(QtCore.Qt.PenStyle.NoPen)
        p.setBrush(track)
        p.drawRoundedRect(r, r.height() / 2, r.height() / 2)
        d = r.height() - 6
        x = 3 + (r.width() - d - 6) * t
        p.setBrush(QtGui.QColor("#FFFFFF"))
        p.drawEllipse(QtCore.QRectF(x, 3, d, d))
        p.end()


# ---------------------------------------------------------------------------
# Layout helpers
# ---------------------------------------------------------------------------


class Card(QtWidgets.QWidget):
    """Section title + rounded card that stacks rows with hairline dividers."""

    def __init__(self, title: str = "", parent=None) -> None:
        super().__init__(parent)
        outer = QtWidgets.QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(6)
        if title:
            lbl = QtWidgets.QLabel(title.upper())
            lbl.setObjectName("CardTitle")
            outer.addWidget(lbl)
        self.frame = QtWidgets.QFrame()
        self.frame.setObjectName("Card")
        self._lay = QtWidgets.QVBoxLayout(self.frame)
        self._lay.setContentsMargins(14, 4, 14, 4)
        self._lay.setSpacing(0)
        outer.addWidget(self.frame)
        self._count = 0

    def add(self, widget: QtWidgets.QWidget) -> QtWidgets.QWidget:
        if self._count:
            div = QtWidgets.QFrame()
            div.setObjectName("Divider")
            self._lay.addWidget(div)
        self._lay.addWidget(widget)
        self._count += 1
        return widget

    def add_row(self, title: str, control: QtWidgets.QWidget | None = None, desc: str = "") -> "SettingRow":
        row = SettingRow(title, control, desc)
        self.add(row)
        return row


class SettingRow(QtWidgets.QWidget):
    def __init__(self, title: str, control: QtWidgets.QWidget | None = None, desc: str = "", parent=None) -> None:
        super().__init__(parent)
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(0, 10, 0, 10)
        lay.setSpacing(12)
        text_col = QtWidgets.QVBoxLayout()
        text_col.setSpacing(2)
        t = QtWidgets.QLabel(title)
        t.setObjectName("RowTitle")
        text_col.addWidget(t)
        if desc:
            d = QtWidgets.QLabel(desc)
            d.setObjectName("RowDesc")
            d.setWordWrap(True)
            text_col.addWidget(d)
        lay.addLayout(text_col, 1)
        if control is not None:
            lay.addWidget(control, 0, QtCore.Qt.AlignmentFlag.AlignVCenter)
            if isinstance(control, QtWidgets.QAbstractButton):
                # Clicking anywhere on the row flips the switch.
                self._control = control
                self.setCursor(QtCore.Qt.CursorShape.PointingHandCursor)

    def mouseReleaseEvent(self, e: QtGui.QMouseEvent) -> None:
        ctl = getattr(self, "_control", None)
        if ctl is not None and e.button() == QtCore.Qt.MouseButton.LeftButton and ctl.isEnabled():
            ctl.toggle()
        super().mouseReleaseEvent(e)


class FieldBlock(QtWidgets.QWidget):
    """Label above a full-width control (for inputs that need room)."""

    def __init__(self, title: str, control: QtWidgets.QWidget | QtWidgets.QLayout, parent=None) -> None:
        super().__init__(parent)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 10, 0, 12)
        lay.setSpacing(6)
        t = QtWidgets.QLabel(title)
        t.setObjectName("RowDesc")
        lay.addWidget(t)
        if isinstance(control, QtWidgets.QLayout):
            lay.addLayout(control)
        else:
            lay.addWidget(control)


class SegmentedTabs(QtWidgets.QFrame):
    currentChanged = QtCore.Signal(int)

    def __init__(self, labels: list[str], parent=None) -> None:
        super().__init__(parent)
        self.setObjectName("Segmented")
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(3, 3, 3, 3)
        lay.setSpacing(2)
        self._group = QtWidgets.QButtonGroup(self)
        self._group.setExclusive(True)
        for i, text in enumerate(labels):
            b = QtWidgets.QPushButton(text)
            b.setObjectName("SegBtn")
            b.setCheckable(True)
            b.setCursor(QtCore.Qt.CursorShape.PointingHandCursor)
            self._group.addButton(b, i)
            lay.addWidget(b)
        self._group.button(0).setChecked(True)
        self._group.idClicked.connect(self.currentChanged.emit)

    def set_current(self, idx: int) -> None:
        btn = self._group.button(idx)
        if btn:
            btn.setChecked(True)
            self.currentChanged.emit(idx)


class StatusPill(QtWidgets.QFrame):
    STATES = {
        "starting": TOKENS["muted"],
        "broadcast": TOKENS["accent"],
        "connected": TOKENS["ok"],
        "warn": TOKENS["warn"],
        "error": TOKENS["danger"],
    }

    def __init__(self, parent=None) -> None:
        super().__init__(parent)
        self.setObjectName("StatusPill")
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(12, 9, 12, 9)
        lay.setSpacing(10)
        self._dot = QtWidgets.QLabel()
        self._dot.setFixedSize(8, 8)
        lay.addWidget(self._dot, 0, QtCore.Qt.AlignmentFlag.AlignVCenter)
        col = QtWidgets.QVBoxLayout()
        col.setSpacing(1)
        self._text = QtWidgets.QLabel()
        self._text.setObjectName("StatusText")
        self._sub = QtWidgets.QLabel()
        self._sub.setObjectName("StatusSub")
        col.addWidget(self._text)
        col.addWidget(self._sub)
        lay.addLayout(col, 1)
        self.set_state("starting", "正在启动引擎…", "")

    def set_state(self, state: str, text: str, sub: str | None = None) -> None:
        color = self.STATES.get(state, TOKENS["muted"])
        self._dot.setStyleSheet(f"background:{color}; border-radius:4px;")
        self._text.setText(text)
        if sub is not None:
            self._sub.setText(sub)
            self._sub.setVisible(bool(sub))
