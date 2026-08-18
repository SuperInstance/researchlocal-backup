$ErrorActionPreference = 'Stop'

$root = (Resolve-Path 'C:\Users\casey\tzpro-agent').Path
$bat  = Join-Path $root 'fix_priority0.bat'
$icon = Join-Path $root 'assets\icon-shortcut-256.png'

if (-not (Test-Path $bat)) { throw "fix_priority0.bat missing at $bat" }

$deskCandidates = @(
    [Environment]::GetFolderPath('Desktop'),
    (Join-Path $env:USERPROFILE 'OneDrive\Desktop'),
    (Join-Path $env:USERPROFILE 'Desktop')
) | Sort-Object -Unique
$deskDir = $deskCandidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $deskDir) { throw "no writable Desktop folder found" }

# Remove any prior TZ Pro Agent tray shortcut; we're replacing with the fix-priority-0 launcher.
foreach ($name in @('TZ Pro Agent Tray.lnk','Fix TZ Pro Position.lnk')) {
    $existing = Join-Path $deskDir $name
    if (Test-Path $existing) { Remove-Item $existing -Force }
}

$link = Join-Path $deskDir 'Fix TZ Pro Position.lnk'
Write-Host "writing shortcut: $link"
Write-Host "  target:  $bat"
Write-Host "  workdir: $root"
Write-Host "  icon:    $icon"

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($link)
$shortcut.TargetPath       = $bat
$shortcut.WorkingDirectory = $root
$shortcut.WindowStyle      = 7   # SW_SHOWMINNOACTIVE
$shortcut.IconLocation     = "$icon,0"
$shortcut.Description      = "Fix TZ Pro: restart bridge (COM6->TCP:6006) + dashboard"
$shortcut.Save()
Write-Host "wrote $link"