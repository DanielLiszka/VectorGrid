param(
  [int]$SampleIntervalMs = 100,
  [string]$OutFile = "scripts/baselines/ui_capture/optimization_memory_metrics.json"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$exePath = Join-Path $repoRoot "target\debug\ta_workflow_regression.exe"

if (-not (Test-Path $exePath)) {
  Write-Host "Building ta_workflow_regression (debug)..." -ForegroundColor Yellow
  Push-Location $repoRoot
  try {
    cargo build -p ta_desktop_demo_app --bin ta_workflow_regression | Out-Host
  }
  finally {
    Pop-Location
  }
}

if (-not (Test-Path $exePath)) {
  throw "Missing executable: $exePath"
}

$outDir = Split-Path -Parent (Join-Path $repoRoot $OutFile)
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$summaryPath = Join-Path $repoRoot "target\workflow_regression_summary.json"
$arguments = @(
  "--out", $summaryPath,
  "--candles-len", "10000",
  "--train-assets", "10",
  "--holdout-assets", "6"
)

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exePath
$psi.WorkingDirectory = $repoRoot
$psi.Arguments = [string]::Join(' ', ($arguments | ForEach-Object {
      if ($_ -match '\s') { '"' + $_ + '"' } else { $_ }
    }))
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true

$proc = [System.Diagnostics.Process]::Start($psi)
if (-not $proc) {
  throw "Failed to start $exePath"
}

$startedAt = Get-Date
$maxWorkingSet = 0L
$maxPrivateBytes = 0L

while (-not $proc.HasExited) {
  $proc.Refresh()
  if ($proc.WorkingSet64 -gt $maxWorkingSet) {
    $maxWorkingSet = $proc.WorkingSet64
  }
  if ($proc.PrivateMemorySize64 -gt $maxPrivateBytes) {
    $maxPrivateBytes = $proc.PrivateMemorySize64
  }
  Start-Sleep -Milliseconds $SampleIntervalMs
}

$proc.WaitForExit()
$stdout = $proc.StandardOutput.ReadToEnd()
$stderr = $proc.StandardError.ReadToEnd()
$endedAt = Get-Date

$runtimeMs = [int]($endedAt - $startedAt).TotalMilliseconds

$payload = [ordered]@{
  generated_at = (Get-Date).ToString("o")
  sample_interval_ms = $SampleIntervalMs
  runtime_ms_wall = $runtimeMs
  peak_working_set_bytes = $maxWorkingSet
  peak_working_set_mb = [math]::Round($maxWorkingSet / 1MB, 2)
  peak_private_bytes = $maxPrivateBytes
  peak_private_mb = [math]::Round($maxPrivateBytes / 1MB, 2)
  workflow_summary_json = "target/workflow_regression_summary.json"
  command = @{
    exe = "target/debug/ta_workflow_regression.exe"
    args = $arguments
  }
}

$outPath = Join-Path $repoRoot $OutFile
$payload | ConvertTo-Json -Depth 6 | Set-Content -Path $outPath -Encoding UTF8

Write-Host "Optimization memory capture complete:"
Write-Host "- Peak working set: $([math]::Round($maxWorkingSet / 1MB, 2)) MB"
Write-Host "- Peak private bytes: $([math]::Round($maxPrivateBytes / 1MB, 2)) MB"
Write-Host "- Output: $OutFile"

if (-not [string]::IsNullOrWhiteSpace($stdout)) {
  Write-Host ""
  Write-Host "Command output (stdout):"
  Write-Host $stdout
}

if (-not [string]::IsNullOrWhiteSpace($stderr)) {
  Write-Host ""
  Write-Host "Command output (stderr):" -ForegroundColor Yellow
  Write-Host $stderr -ForegroundColor Yellow
}
