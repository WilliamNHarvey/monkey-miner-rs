$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RootDir

$DistDir = Join-Path "dist" "monkey-miner-windows-x86_64"

cargo build --release

if (Test-Path $DistDir) {
    Remove-Item $DistDir -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
Copy-Item "target\release\monkey-miner.exe" $DistDir
Copy-Item "assets" $DistDir -Recurse

Write-Host "Built $DistDir\monkey-miner.exe"
Write-Host "Run with: cd $DistDir; .\monkey-miner.exe"
