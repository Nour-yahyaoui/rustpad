<#
.SYNOPSIS
    Builds the standalone pure-Rust Windows executable (rustpad.exe) and optionally uploads it to GitHub Releases.
.DESCRIPTION
    Author: Nour Yahyaoui <nourryahyaoui@gmail.com> · https://github.com/Nour-yahyaoui/rustpad
.EXAMPLE
    .\scripts\build-windows-exe.ps1
    .\scripts\build-windows-exe.ps1 -ReleaseTag v1.0.0 -Upload
#>

param(
    [string]$ReleaseTag = "v1.0.0",
    [switch]$Upload
)

$ErrorActionPreference = "Stop"

Write-Host "==> Checking Rust toolchain..." -ForegroundColor Cyan
cargo --version
rustc --version

Write-Host "==> Running workspace tests (9 pure-Rust crates)..." -ForegroundColor Cyan
cargo test --workspace

Write-Host "==> Building stripped release binary (rustpad.exe)..." -ForegroundColor Cyan
cargo build --release -p rustpad

$OutDir = "target\dist"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$SrcExe = "target\release\rustpad.exe"
$VersionedExe = "$OutDir\rustpad-$ReleaseTag-windows-x86_64.exe"
Copy-Item -Force $SrcExe $VersionedExe
Copy-Item -Force $SrcExe "$OutDir\rustpad.exe"

$Hash = (Get-FileHash -Algorithm SHA256 $VersionedExe).Hash.ToLower()
$SumLine = "$Hash  rustpad-$ReleaseTag-windows-x86_64.exe"
$SumFile = "$OutDir\SHA256SUMS-windows.txt"
Set-Content -Path $SumFile -Value $SumLine -Encoding UTF8

$SizeMB = [math]::Round((Get-Item $VersionedExe).Length / 1MB, 2)
Write-Host ""
Write-Host "==> Built Windows executable successfully!" -ForegroundColor Green
Write-Host "    Binary : $VersionedExe ($SizeMB MB)"
Write-Host "    SHA256 : $Hash"

if ($Upload) {
    Write-Host "==> Uploading $VersionedExe to GitHub Release $ReleaseTag..." -ForegroundColor Cyan
    gh release upload $ReleaseTag $VersionedExe "$OutDir\rustpad.exe" $SumFile --clobber
    Write-Host "==> Uploaded! Check https://github.com/Nour-yahyaoui/rustpad/releases/tag/$ReleaseTag" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "Next step to publish on GitHub Releases:" -ForegroundColor Yellow
    Write-Host "    gh release upload $ReleaseTag $VersionedExe $OutDir\rustpad.exe $SumFile --clobber"
}
