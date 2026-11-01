param([switch]$DryRun)

$ErrorActionPreference = 'Stop'
$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$manifestPath = Join-Path $projectRoot 'src-tauri/Cargo.toml'
$metadataJson = & cargo metadata --offline --no-deps --format-version 1 --manifest-path $manifestPath
if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve the Cargo target directory; cleanup stopped.' }
$targetRoot = [System.IO.Path]::GetFullPath(($metadataJson | ConvertFrom-Json).target_directory)
$releaseRoot = Join-Path $targetRoot 'release'
$appPath = Join-Path $releaseRoot 'keroro-wii-encoder.exe'
$cleanupPaths = @((Join-Path $releaseRoot 'bundle'), (Join-Path $projectRoot 'dist'))

function Assert-ProjectPath([string]$Path) {
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    $prefix = $projectRoot.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolvedPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Cleanup target must stay inside this project: $resolvedPath"
    }
    # Do not follow junctions or symbolic links into another directory.
    $ancestor = $resolvedPath
    while ($ancestor -and $ancestor -ne $projectRoot) {
        if (Test-Path -LiteralPath $ancestor) {
            $item = Get-Item -LiteralPath $ancestor -Force
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Cleanup cannot follow a junction or symbolic link: $ancestor"
            }
        }
        $ancestor = Split-Path -Parent $ancestor
    }
    return $resolvedPath
}

$targetRoot = Assert-ProjectPath $targetRoot
$cleanupPaths = @($cleanupPaths | ForEach-Object { Assert-ProjectPath $_ })

Write-Host "Clean application release artifacts: cargo clean -p keroro-wii-encoder --release --manifest-path $manifestPath"
$cleanupPaths | ForEach-Object { Write-Host "Remove generated directory: $_" }
if ($DryRun) { return }

if (Test-Path -LiteralPath $appPath) {
    try {
        $handle = [System.IO.File]::Open($appPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
        $handle.Dispose()
    } catch {
        throw 'The release executable is locked. Exit Keroro Wii Encoder from its tray menu, then rebuild. Cleanup stopped.'
    }
}

& cargo clean -p keroro-wii-encoder --release --manifest-path $manifestPath
if ($LASTEXITCODE -ne 0) { throw 'Cargo release cleanup failed; build stopped.' }
foreach ($cleanupPath in $cleanupPaths) {
    if (Test-Path -LiteralPath $cleanupPath) {
        Remove-Item -LiteralPath $cleanupPath -Recurse -Force
    }
}
