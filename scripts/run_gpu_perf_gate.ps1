param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselinePath = "scripts/baselines/gpu_perf_baseline.json",
  [string]$OutPath = "target/gpu_perf_summary.json",
  [string]$BenchFilter = "double_ma/gpu_kernel/sma_sma/200k/small",
  [string]$CriterionEstimatesGlob = "*double_ma_gpu_kernel_sma_sma_200k*\small_1590pairs\new\estimates.json",
  [double]$RegressionPct = 10.0
)

$ErrorActionPreference = "Stop"

function Write-PassFailLine([string]$name, [bool]$pass, [string]$msg) {
  if ($pass) {
    Write-Host "[PASS] ${name}: $msg"
  } else {
    Write-Host "[FAIL] ${name}: $msg"
  }
}

Write-Host "Running GPU kernel benchmark ($BenchFilter)..."
$benchCmd = @(
  "bench",
  "-p", "ta_app_core",
  "--bench", "double_ma_opt",
  "--features", "cuda,cuda-backtest-kernel",
  "--",
  $BenchFilter
)
cargo @benchCmd
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

$estimate = Get-ChildItem -Path "target/criterion" -Recurse -File -Filter "estimates.json" |
  Where-Object { $_.FullName -like $CriterionEstimatesGlob } |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
if ($null -eq $estimate) {
  Write-Host "No exact match for '$CriterionEstimatesGlob'. Falling back to most recent GPU-kernel estimate file."
  $estimate = Get-ChildItem -Path "target/criterion" -Recurse -File -Filter "estimates.json" |
    Where-Object { $_.FullName -like "*double_ma_gpu_kernel*\\new\\estimates.json" } |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if ($null -eq $estimate) {
    throw "Could not find any GPU-kernel criterion estimates.json file."
  }
}

$criterion = Get-Content -Raw $estimate.FullName | ConvertFrom-Json
$gpuInfo = ""
try {
  $gpuInfo = (& nvidia-smi --query-gpu=name,driver_version --format=csv,noheader | Select-Object -First 1).Trim()
} catch {
  $gpuInfo = ""
}

$summary = [ordered]@{
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  bench_filter = $BenchFilter
  criterion_estimates_json = $estimate.FullName
  criterion_mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
  criterion_mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
  criterion_mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
  criterion_median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  gpu = $gpuInfo
}

$outDir = Split-Path -Parent $OutPath
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}
$summary | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $OutPath
Write-Host "GPU perf summary written to $OutPath"

$ciWidthPct = 0.0
if ($summary.criterion_mean_ms -gt 0) {
  $ciWidthPct = (($summary.criterion_mean_ci_upper_ms - $summary.criterion_mean_ci_lower_ms) / $summary.criterion_mean_ms) * 100.0
}
if ($ciWidthPct -gt 10.0) {
  Write-Host "[WARN] High run variance detected (mean CI width = $([math]::Round($ciWidthPct, 2))%). Consider stabilizing GPU clocks/load before trusting this baseline."
}

if ($Mode -eq "baseline") {
  $baseDir = Split-Path -Parent $BaselinePath
  if ($baseDir) {
    New-Item -ItemType Directory -Force -Path $baseDir | Out-Null
  }
  $summary | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $BaselinePath
  Write-Host "GPU perf baseline written to $BaselinePath"
  exit 0
}

if (!(Test-Path $BaselinePath)) {
  throw "Missing GPU perf baseline: $BaselinePath. Run with -Mode baseline first."
}

$baseline = Get-Content -Raw $BaselinePath | ConvertFrom-Json
$candMean = [double]$summary.criterion_mean_ms
$baseMean = [double]$baseline.criterion_mean_ms
$budget = 1.0 + ($RegressionPct / 100.0)
$pass = $candMean -le ($baseMean * $budget)

Write-PassFailLine "gpu_criterion_mean_not_regressed_${RegressionPct}pct" $pass "$candMean ms vs baseline $baseMean ms"
if (!$pass) {
  throw "GPU performance gate failed"
}
