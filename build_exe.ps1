$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
    if (!(Test-Path "$PSScriptRoot\engine\target\release\airplay-hassy-engine.exe")) {
        & "$PSScriptRoot\build_engine.ps1"
    }
    python -m PyInstaller --noconfirm --clean "$PSScriptRoot\AirPlay-Hassy.spec"
    Write-Host "Packaged EXE ready: $PSScriptRoot\dist\AirPlay-Hassy.exe" -ForegroundColor Green
} finally {
    Pop-Location
}
