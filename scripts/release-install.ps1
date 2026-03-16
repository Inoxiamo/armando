$ErrorActionPreference = "Stop"

$AppName = "armando"
$BundleDir = Split-Path -Parent $PSScriptRoot
$BinarySource = Join-Path $BundleDir "$AppName.exe"
$AppDataRoot = if ($env:APPDATA) { $env:APPDATA } else { Join-Path $HOME "AppData\Roaming" }
$LocalAppDataRoot = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { Join-Path $HOME "AppData\Local" }
$ConfigRoot = Join-Path $AppDataRoot $AppName
$DataRoot = Join-Path $LocalAppDataRoot $AppName
$BinDir = Join-Path $DataRoot "bin"
$ConfigDir = Join-Path $ConfigRoot "configs"
$ThemesDir = Join-Path $ConfigRoot "themes"
$LocalesDir = Join-Path $ConfigRoot "locales"
$AssetsDir = Join-Path $DataRoot "assets"

if (!(Test-Path $BinarySource)) {
    throw "Could not find $AppName.exe in bundle root: $BinarySource"
}

New-Item -ItemType Directory -Force -Path $BinDir, $ConfigDir, $ThemesDir, $LocalesDir, $AssetsDir | Out-Null
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

$BundleAssetsDir = Join-Path $BundleDir "assets"
if (Test-Path $BundleAssetsDir) {
    Copy-Item (Join-Path $BundleAssetsDir "*") $AssetsDir -Recurse -Force
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
Write-Host ""
Write-Host "Assets:"
Write-Host "  $AssetsDir"
