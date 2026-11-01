param(
    [string]$Vpk = 'vpk'
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$version = '0.0.100'
$source = Join-Path $repo 'src-tauri/target/release'
$sourceResources = Join-Path $repo 'src-tauri/resources'
$packDir = Join-Path $env:TEMP "AmplifiVpkPack-$version"
$outputDir = Join-Path $repo "release/$version"
$mainExe = Join-Path $source 'amplifi-app.exe'
$icon = Join-Path $repo 'src-tauri/icons/icon.ico'

if (-not (Test-Path -LiteralPath $mainExe -PathType Leaf)) {
    throw "Release executable not found. Build first with 'npm run tauri build'. Expected: $mainExe"
}
if (-not (Test-Path -LiteralPath (Join-Path $sourceResources 'bin/amplifi-engine.exe') -PathType Leaf)) {
    throw "Bundled engine resources not found in $sourceResources."
}

if (Test-Path -LiteralPath $packDir) {
    Remove-Item -LiteralPath $packDir -Recurse -Force
}
New-Item -ItemType Directory -Path $packDir | Out-Null
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
Copy-Item -LiteralPath $mainExe -Destination $packDir
$webViewLoader = Join-Path $source 'WebView2Loader.dll'
if (Test-Path -LiteralPath $webViewLoader -PathType Leaf) {
    Copy-Item -LiteralPath $webViewLoader -Destination $packDir
}
$packResources = Join-Path $packDir 'resources'
New-Item -ItemType Directory -Path $packResources -Force | Out-Null
Copy-Item -Path (Join-Path $sourceResources '*') -Destination $packResources -Recurse -Force
Get-ChildItem -LiteralPath $outputDir -File | Where-Object {
    $_.Name -in @('RELEASES', 'assets.win.json', 'releases.win.json', 'Amplifi-0.0.100-Windows-Folder-Installer.zip') -or
    $_.Name -like 'com.susenayw.amplifi-*'
} | Remove-Item -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'ChooseInstallLocation.ps1') -Destination $outputDir -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Install Amplifi.cmd') -Destination $outputDir -Force

& $Vpk pack `
    --packId com.susenayw.amplifi `
    --packVersion $version `
    --packDir $packDir `
    --mainExe 'amplifi-app.exe' `
    --runtime win-x64 `
    --packTitle 'Amplifi' `
    --packAuthors 'Amplifi contributors' `
    --icon $icon `
    --shortcuts 'StartMenuRoot,Desktop' `
    --instLicense (Join-Path $sourceResources 'licenses/AGPL-3.0.txt') `
    --msi `
    --instLocation Either `
    --outputDir $outputDir

if ($LASTEXITCODE -ne 0) {
    throw "Velopack packaging failed with exit code $LASTEXITCODE."
}

$setup = Join-Path $outputDir 'com.susenayw.amplifi-win-Setup.exe'
if (-not (Test-Path -LiteralPath $setup -PathType Leaf)) {
    throw "Velopack completed but the expected installer was not found: $setup"
}
$folderInstaller = Join-Path $outputDir 'Amplifi-0.0.100-Windows-Folder-Installer.zip'
if (Test-Path -LiteralPath $folderInstaller) {
    Remove-Item -LiteralPath $folderInstaller -Force
}
Compress-Archive -LiteralPath @(
    $setup,
    (Join-Path $PSScriptRoot 'Install Amplifi.cmd'),
    (Join-Path $PSScriptRoot 'ChooseInstallLocation.ps1')
) -DestinationPath $folderInstaller -CompressionLevel Optimal
Write-Host "Velopack release created in $outputDir"
Write-Host "To enable folder selection, distribute Install Amplifi.cmd and ChooseInstallLocation.ps1 beside com.susenayw.amplifi-win-Setup.exe."
Write-Host "Folder-picker installer bundle: $folderInstaller"
