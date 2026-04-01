param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$PackPath = "scripts/workflow_benchmark_pack.json",
  [string]$BaselinePath = "scripts/baselines/workflow_regression_baseline.json",
  [string]$OutPath = "target/workflow_regression_summary.json",
  [ValidateSet("release", "dev")]
  [string]$BuildProfile = "release",
  [bool]$CheckStageBudgets = $true,
  [double]$OptimizeStageRegressionPct = 100.0,
  [double]$ValidateStageRegressionPct = 100.0,
  [double]$MetaStageRegressionPct = 150.0,
  [double]$HoldoutStageRegressionPct = 150.0
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 12
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

if (!(Test-Path $PackPath)) {
  throw "Missing benchmark pack: $PackPath"
}

$pack = Get-Content -Raw $PackPath | ConvertFrom-Json
$candlesLen = [int]$pack.candles_len
$trainAssets = [int]$pack.train_assets
$holdoutAssets = [int]$pack.holdout_assets

$outDir = Split-Path -Parent $OutPath
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$checkStageBudgetsValue = if ($CheckStageBudgets) { "true" } else { "false" }

$cmd = @("run")
if ($BuildProfile -eq "release") {
  $cmd += "--release"
}
$cmd += @(
  "-p", "ta_desktop_demo_app",
  "--bin", "ta_workflow_regression",
  "--",
  "--out", $OutPath,
  "--candles-len", "$candlesLen",
  "--train-assets", "$trainAssets",
  "--holdout-assets", "$holdoutAssets",
  "--check-stage-budgets", $checkStageBudgetsValue,
  "--optimize-stage-regression-pct", "$OptimizeStageRegressionPct",
  "--validate-stage-regression-pct", "$ValidateStageRegressionPct",
  "--meta-stage-regression-pct", "$MetaStageRegressionPct",
  "--holdout-stage-regression-pct", "$HoldoutStageRegressionPct"
)

if ($Mode -eq "candidate") {
  if (!(Test-Path $BaselinePath)) {
    throw "Missing baseline summary: $BaselinePath. Run with -Mode baseline first."
  }
  $cmd += @("--baseline", $BaselinePath)
}

Write-Host "Running workflow gate ($Mode, build_profile=$BuildProfile)..."
cargo @cmd
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

if (!(Test-Path $OutPath)) {
  throw "Workflow gate did not produce the requested output summary: $OutPath"
}

if ($Mode -eq "baseline") {
  $baseDir = Split-Path -Parent $BaselinePath
  if ($baseDir) {
    New-Item -ItemType Directory -Force -Path $baseDir | Out-Null
  }
  Copy-Item -LiteralPath $OutPath -Destination $BaselinePath -Force
  Write-Host "Workflow baseline copied to $BaselinePath"
}

Write-Host "Summary written to $OutPath"
