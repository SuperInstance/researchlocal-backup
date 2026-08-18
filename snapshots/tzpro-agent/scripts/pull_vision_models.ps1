# pull_vision_models.ps1 — kick background ollama pulls for vision models.
# Detached, no console window. Logs append to logs/vision_pull.log.
# Use to retry moondream/llava when Starlink gives us bandwidth.

$ErrorActionPreference = "Continue"
$repo = "C:\Users\casey\tzpro-agent"
$logDir = "$repo\logs"
$logFile = "$logDir\vision_pull.log"
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }

$ollama = Join-Path $env:LOCALAPPDATA "Programs\Ollama\ollama.exe"
if (-not (Test-Path $ollama)) {
    Write-Host "ollama not found at $ollama"
    exit 1
}

foreach ($model in @("moondream:latest", "llava:7b")) {
    if (Get-Process | Where-Object { $_.CommandLine -like "*ollama pull*" -and $_.CommandLine -like "*$model*" } | Select-Object -First 1) {
        Write-Host "Already pulling $model — skipping"
        continue
    }
    Write-Host "Starting: ollama pull $model"
    $args = @("pull", $model)
    Start-Process -FilePath $ollama -ArgumentList $args -RedirectStandardOutput $logFile -RedirectStandardError $logFile -WindowStyle Hidden -WorkingDirectory $repo
    Start-Sleep 3
}
Write-Host "Pulls launched. Monitor with: Get-Content $logFile -Tail 20"
