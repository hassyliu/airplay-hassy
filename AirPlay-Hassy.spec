# -*- mode: python ; coding: utf-8 -*-
from pathlib import Path
from PyInstaller.utils.hooks import collect_all

ROOT = Path(SPECPATH).resolve()

av_datas, av_binaries, av_hidden = collect_all("av")
sd_datas, sd_binaries, sd_hidden = collect_all("_sounddevice_data")

datas = av_datas + sd_datas
binaries = (
    av_binaries
    + sd_binaries
    + [(str(ROOT / "engine" / "target" / "release" / "airplay-hassy-engine.exe"), "bin")]
)
hiddenimports = av_hidden + sd_hidden + ["sounddevice", "cffi"]

excludes = [
    "torch",
    "torchaudio",
    "torchvision",
    "cv2",
    "scipy",
    "matplotlib",
    "pandas",
    "PIL",
    "fastapi",
    "uvicorn",
    "websockets",
    "aiohttp",
    "boto3",
    "botocore",
    "ctranslate2",
    "pyannote",
    "alembic",
    "sqlalchemy",
    "tkinter",
    "unittest",
    "pytest",
    "PySide6.QtWebEngine",
    "PySide6.QtWebEngineCore",
    "PySide6.QtWebEngineWidgets",
    "PySide6.Qt3DCore",
    "PySide6.Qt3DRender",
    "PySide6.QtQuick",
    "PySide6.QtQml",
    "PySide6.QtPdf",
    "PySide6.QtCharts",
    "PySide6.QtBluetooth",
    "PySide6.QtDesigner",
    "PySide6.QtNetwork",
    "PySide6.QtSql",
    "PySide6.QtSvg",
    "PySide6.QtTest",
]

a = Analysis(
    [str(ROOT / "run.py")],
    pathex=[str(ROOT)],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=excludes,
    noarchive=False,
    optimize=1,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name="AirPlay-Hassy",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    icon=str(ROOT / "assets" / "icon.ico"),
)
