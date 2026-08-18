# scripts/_make_shortcut.ps1
#
# Create a desktop shortcut that launches the TZ Pro Agent tray.
# Idempotent: re-running replaces the existing .lnk rather than
# adding another one.

param(
    [switch]$Unlink
)

$ErrorActionPreference = 'Stop'

# Robust anchor: $PSScriptRoot is the dir of this script (`scripts/`).
# We want the repo root, which is one level up.  Resolve-Path normalizes
# the result and fails loudly if the path doesn't exist.
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

$batPath   = Join-Path $root 'start_capture_tray.bat'
$iconPath  = Join-Path $root 'assets\icon-shortcut-256.png'

# On this machine the Desktop folder is OneDrive-redirected (set in
# NTUSER.DAT under "Desktop" -> User Shell Folders). The Win32
# GetFolderPath call *does* honor that, but on a freshly-synced FS
# we sometimes need to also check OneDrive\Desktop explicitly.
$deskCandidates = @(
    [Environment]::GetFolderPath('Desktop'),
    (Join-Path $env:USERPROFILE 'OneDrive\Desktop'),
    (Join-Path $env:USERPROFILE 'Desktop')
) | Sort-Object -Unique

$deskDir   = $deskCandidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $deskDir) {
    throw "could not locate a writable Desktop folder; tried: $($deskCandidates -join '; ')"
}
$linkPath  = Join-Path $deskDir 'TZ Pro Agent Tray.lnk'
Write-Host "writing shortcut to: $linkPath"

if ($Unlink) {
    if (Test-Path $linkPath) { Remove-Item $linkPath -Force }
    Write-Host "removed $linkPath"
    exit 0
}

if (-not (Test-Path $batPath)) {
    throw "start_capture_tray.bat missing at $batPath"
}

$shell = New-Object -ComObject WScript.Shell

# Remove any existing link first so we don't end up with duplicates.
if (Test-Path $linkPath) { Remove-Item $linkPath -Force }

$shortcut = $shell.CreateShortcut($linkPath)
$shortcut.TargetPath       = $batPath
$shortcut.WorkingDirectory = $root
$shortcut.WindowStyle      = 7          # SW_SHOWMINNOACTIVE
$shortcut.IconLocation     = "$iconPath,0"
$shortcut.Description      = "TZ Pro Agent tray app - dashboard + daemon control"
$shortcut.Save()

Write-Host "wrote $linkPath -> $batPath"
