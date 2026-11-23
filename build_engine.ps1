$ErrorActionPreference = "Stop"
$tc = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnullvm\lib\rustlib\x86_64-pc-windows-gnullvm\bin"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$tc;$env:PATH"
Push-Location "$PSScriptRoot\engine"
try {
    cargo build --release
    Write-Host "Built: $PSScriptRoot\engine\target\release\airplay-hassy-engine.exe" -ForegroundColor Green
} finally {
    Pop-Location
}
