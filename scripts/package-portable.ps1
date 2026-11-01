param([string]$Version = "1.1.0")

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$binaryRoot = Join-Path $projectRoot 'src-tauri/target/release'
$appPath = Join-Path $binaryRoot 'keroro-wii-encoder.exe'
if (-not (Test-Path -LiteralPath $appPath)) { throw 'Build the release application first.' }
if ((Get-Item -LiteralPath $appPath).VersionInfo.ProductVersion -ne $Version) {
    throw "The release application does not match version $Version. Rebuild before packaging."
}

$releaseRoot = Join-Path $projectRoot "releases/v$Version"
$portableRoot = Join-Path $releaseRoot 'Keroro-Wii-Encoder'
New-Item -ItemType Directory -Path $portableRoot -Force | Out-Null
Copy-Item -LiteralPath $appPath -Destination (Join-Path $portableRoot 'keroro-wii-encoder.exe')
foreach ($tool in @('ffmpeg', 'ffprobe')) {
    $source = Join-Path $projectRoot "src-tauri/binaries/$tool-x86_64-pc-windows-msvc.exe"
    if (-not (Test-Path -LiteralPath $source)) { throw "Missing sidecar: $source" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $portableRoot "$tool.exe")
}
foreach ($document in @('FFmpeg-LICENSE.txt', 'FFmpeg-README.txt')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot "src-tauri/binaries/$document") -Destination (Join-Path $portableRoot $document)
}
@'
Keroro Wii Encoder 1.1

Extract the entire Keroro-Wii-Encoder folder and run keroro-wii-encoder.exe.
Keep ffmpeg.exe and ffprobe.exe beside the application. No Node.js, Vite,
Rust installation, or FFmpeg PATH setup is needed to run this package.
Microsoft Edge WebView2 Runtime must be installed.

Select separate input and output folders, choose quality, and start a batch.
Existing output files are preserved. Failed items appear in the status panel.
Supported input containers: MP4, MKV, AVI, MOV, WMV, FLV, WebM.
Folder paths, title, quality, theme, and mute save automatically.

Subtitles follow the anime folder until overridden. Use anime folder resets this.
Matching SRT/ASS/SSA/VTT files are burned in; clear the subtitle field to skip subtitles.
Audio uses AAC stereo targeting 128 kbps at 44.1 kHz without volume boosting.
Enter an Anime Title for sequential Title - E01 [Wii].mp4 output names.
Preview First 10 Seconds opens a temporary encoded clip in the video player.

UI sounds can be muted. Desktop notifications are opt-in; enable the checkbox
to request permission. Test Windows toast branding with the installed build.

Minimize to tray keeps encoding running. Closing during encoding asks whether
to cancel and exit. Closing while idle hides the app in the tray. Left-click
its tray icon to restore it, or choose Exit from the tray menu to quit.

The supplied FFmpeg license and build/source information accompany this app.
'@ | Set-Content -LiteralPath (Join-Path $portableRoot 'README.txt') -Encoding utf8

$archive = Join-Path $releaseRoot "Keroro-Wii-Encoder-$Version-win-x64.zip"
Compress-Archive -LiteralPath $portableRoot -DestinationPath $archive -Force
$artifacts = @($archive)
$installer = Get-ChildItem -LiteralPath (Join-Path $binaryRoot 'bundle/nsis') -File -Filter "*_${Version}_*-setup.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($installer) {
    $installerPath = Join-Path $releaseRoot "Keroro-Wii-Encoder-$Version-x64-setup.exe"
    Copy-Item -LiteralPath $installer.FullName -Destination $installerPath
    $artifacts += $installerPath
}
$checksums = foreach ($artifact in $artifacts) {
    $hash = Get-FileHash -LiteralPath $artifact -Algorithm SHA256
    "$($hash.Hash.ToLowerInvariant())  $([System.IO.Path]::GetFileName($artifact))"
}
$checksums | Set-Content -LiteralPath (Join-Path $releaseRoot 'SHA256SUMS.txt') -Encoding ascii
$artifacts | ForEach-Object { Get-Item -LiteralPath $_ | Select-Object FullName, Length }
