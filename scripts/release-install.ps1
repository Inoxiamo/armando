$ErrorActionPreference = "Stop"

$AppName = "armando"
$BundleDir = Split-Path -Parent $PSScriptRoot
$BinarySource = Join-Path $BundleDir "$AppName.exe"
$BinDir = Join-Path $HOME ".local\bin"
$ConfigRoot = Join-Path $HOME ".config\$AppName"
$ConfigDir = Join-Path $ConfigRoot "configs"
$ThemesDir = Join-Path $ConfigRoot "themes"
$LocalesDir = Join-Path $ConfigRoot "locales"

if (!(Test-Path $BinarySource)) {
    throw "Could not find $AppName.exe in bundle root: $BinarySource"
}

New-Item -ItemType Directory -Force -Path $BinDir, $ConfigDir, $ThemesDir, $LocalesDir | Out-Null
Copy-Item $BinarySource (Join-Path $BinDir "$AppName.exe") -Force

$DefaultConfig = Join-Path $BundleDir "configs\default.yaml"
if (Test-Path $DefaultConfig) {
    Copy-Item $DefaultConfig (Join-Path $ConfigDir "default.yaml") -Force
}

$ThemeFiles = Join-Path $BundleDir "themes\*.yaml"
if (Get-ChildItem $ThemeFiles -ErrorAction SilentlyContinue) {
    Copy-Item $ThemeFiles $ThemesDir -Force
}

$LocaleFiles = Join-Path $BundleDir "locales\*.yaml"
if (Get-ChildItem $LocaleFiles -ErrorAction SilentlyContinue) {
    Copy-Item $LocaleFiles $LocalesDir -Force
}

Write-Host "Installed $AppName"
Write-Host ""
Write-Host "Binary:"
Write-Host "  $(Join-Path $BinDir "$AppName.exe")"
Write-Host ""
Write-Host "Config:"
Write-Host "  $(Join-Path $ConfigDir "default.yaml")"
Write-Host ""
Write-Host "Themes:"
Write-Host "  $ThemesDir"
Write-Host ""
Write-Host "Locales:"
Write-Host "  $LocalesDir"
