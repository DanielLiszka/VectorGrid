param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$PackPath = "scripts/workflow_benchmark_pack.json",
  [string]$BaselinePath = "scripts/baselines/workflow_regression_baseline.json",
  [string]$OutPath = "target/workflow_regression_summary.json"
)

$ErrorActionPreference = "Stop"

if (!(Test-Path $PackPath)) {
  throw "Missing benchmark pack: $PackPath"
}

$pack = Get-Content -Raw $PackPath | ConvertFrom-Json
$candlesLen = [int]$pack.candles_len
$trainAssets = [int]$pack.train_assets
$holdoutAssets = [int]$pack.holdout_assets

if ($Mode -eq "baseline") {
  $OutPath = $BaselinePath
}

$outDir = Split-Path -Parent $OutPath
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$cmd = @(
  "run", "-p", "ta_desktop_demo_app", "--bin", "ta_workflow_regression", "--",
  "--out", $OutPath,
  "--candles-len", "$candlesLen",
  "--train-assets", "$trainAssets",
  "--holdout-assets", "$holdoutAssets"
)

if ($Mode -eq "candidate") {
  if (!(Test-Path $BaselinePath)) {
    throw "Missing baseline summary: $BaselinePath. Run with -Mode baseline first."
  }
  $cmd += @("--baseline", $BaselinePath)
}

Write-Host "Running workflow gate ($Mode)..."
cargo @cmd
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Write-Host "Summary written to $OutPath"
