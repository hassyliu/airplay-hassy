from __future__ import annotations

import sys
from PySide6 import QtWidgets

from app.main_window import MainWindow


def main() -> int:
    app = QtWidgets.QApplication(sys.argv)
    app.setApplicationName("AirPlay Hassy")
    window = MainWindow()
    window.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
