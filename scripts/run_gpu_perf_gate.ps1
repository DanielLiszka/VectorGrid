param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselinePath = "scripts/baselines/gpu_perf_baseline.json",
  [string]$OutPath = "target/gpu_perf_summary.json",
  [string]$BenchFilter = "double_ma/gpu_kernel/sma_sma/200k/small",
  [string]$CriterionEstimatesGlob = "*double_ma_gpu_kernel_sma_sma_200k*\small_1590pairs\new\estimates.json",
  [double]$RegressionPct = 10.0,
  [bool]$CheckDynamicStrategyRatio = $true,
  [double]$DynamicStrategyRegressionPct = 20.0
)

$ErrorActionPreference = "Stop"

function Write-PassFailLine([string]$name, [bool]$pass, [string]$msg) {
  if ($pass) {
    Write-Host "[PASS] ${name}: $msg"
  } else {
    Write-Host "[FAIL] ${name}: $msg"
  }
}

function Find-LatestGpuKernelEstimate([string]$glob) {
  $estimate = Get-ChildItem -Path "target/criterion" -Recurse -File -Filter "estimates.json" |
    Where-Object { $_.FullName -like $glob } |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if ($null -eq $estimate) {
    Write-Host "No exact match for '$glob'. Falling back to most recent GPU-kernel estimate file."
    $estimate = Get-ChildItem -Path "target/criterion" -Recurse -File -Filter "estimates.json" |
      Where-Object { $_.FullName -like "*double_ma_gpu_kernel*\\new\\estimates.json" } |
      Sort-Object LastWriteTime -Descending |
      Select-Object -First 1
  }
  if ($null -eq $estimate) {
    throw "Could not find any GPU-kernel criterion estimates.json file."
  }
  return $estimate
}

function Invoke-GpuKernelBench([string]$benchFilter, [bool]$dynamicStrategyEnabled, [string]$estimateSnapshotPath) {
  if ($dynamicStrategyEnabled) {
    $env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL = "1"
    Write-Host "Running GPU kernel benchmark ($benchFilter) with dynamic strategy preflight ON..."
  } else {
    Remove-Item Env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL -ErrorAction SilentlyContinue
    Write-Host "Running GPU kernel benchmark ($benchFilter) with dynamic strategy preflight OFF..."
  }

  $benchCmd = @(
    "bench",
    "-p", "ta_app_core",
    "--bench", "double_ma_opt",
    "--features", "cuda,cuda-backtest-kernel",
    "--",
    $benchFilter
  )
  cargo @benchCmd
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  $estimate = Find-LatestGpuKernelEstimate -glob $CriterionEstimatesGlob
  if (![string]::IsNullOrWhiteSpace($estimateSnapshotPath)) {
    $snapshotDir = Split-Path -Parent $estimateSnapshotPath
    if ($snapshotDir) {
      New-Item -ItemType Directory -Force -Path $snapshotDir | Out-Null
    }
    Copy-Item -Force $estimate.FullName $estimateSnapshotPath
  }
  $criterion = Get-Content -Raw $estimate.FullName | ConvertFrom-Json
  return [ordered]@{
    estimate_path = $estimate.FullName
    estimate_snapshot_path = $estimateSnapshotPath
    mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
    mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
    mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
    median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  }
}

function Invoke-DynamicStrategyProfileRun([bool]$dynamicStrategyEnabled, [int]$iterations = 5) {
  if ($dynamicStrategyEnabled) {
    Write-Host "Running profile_gpu_dynamic_runner with dynamic strategy preflight ON..."
  } else {
    Write-Host "Running profile_gpu_dynamic_runner with dynamic strategy preflight OFF..."
  }

  $dynMode = if ($dynamicStrategyEnabled) { "on" } else { "off" }
  $runnerCmd = @(
    "run",
    "-q",
    "-p", "ta_app_core",
    "--bin", "profile_gpu_dynamic_runner",
    "--features", "cuda,cuda-backtest-kernel",
    "--",
    "--strategy", "double_ma",
    "--workload", "small",
    "--iterations", "$iterations",
    "--dynamic-mode", $dynMode,
    "--device-id", "0"
  )
  $output = cargo @runnerCmd 2>&1
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  $iterRows = @()
  foreach ($item in $output) {
    $line = "$item".Trim()
    if ($line -match 'iter=(\d+).*runtime_ms=(\d+).*wall_ms=(\d+)') {
      $iterRows += [pscustomobject]@{
        iter = [int]$matches[1]
        runtime_ms = [double]$matches[2]
        wall_ms = [double]$matches[3]
      }
    }
  }
  if ($iterRows.Count -lt 1) {
    throw "profile_gpu_dynamic_runner did not emit parseable iteration rows"
  }

  $steadyRows = if ($iterRows.Count -gt 1) { $iterRows | Select-Object -Skip 1 } else { $iterRows }
  $steadyWallMs = [math]::Round((($steadyRows | Measure-Object -Property wall_ms -Average).Average), 6)
  $steadyRuntimeMs = [math]::Round((($steadyRows | Measure-Object -Property runtime_ms -Average).Average), 6)
  return [ordered]@{
    mode = $dynMode
    iterations = $iterRows.Count
    steady_state_iterations = $steadyRows.Count
    first_wall_ms = [double]$iterRows[0].wall_ms
    first_runtime_ms = [double]$iterRows[0].runtime_ms
    steady_state_wall_ms = $steadyWallMs
    steady_state_runtime_ms = $steadyRuntimeMs
  }
}

$prevDynamicFlag = [Environment]::GetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "Process")
$offSnapshotPath = "target/gpu_perf/off_estimates.json"
try {
  $offRun = Invoke-GpuKernelBench -benchFilter $BenchFilter -dynamicStrategyEnabled $false -estimateSnapshotPath $offSnapshotPath
  $offDynRun = $null
  $onDynRun = $null
  if ($CheckDynamicStrategyRatio) {
    $offDynRun = Invoke-DynamicStrategyProfileRun -dynamicStrategyEnabled $false -iterations 5
    $onDynRun = Invoke-DynamicStrategyProfileRun -dynamicStrategyEnabled $true -iterations 5
  }
} finally {
  if ([string]::IsNullOrWhiteSpace($prevDynamicFlag)) {
    Remove-Item Env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL -ErrorAction SilentlyContinue
  } else {
    $env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL = $prevDynamicFlag
  }
}

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
  criterion_estimates_json = $offRun.estimate_path
  criterion_mean_ms = $offRun.mean_ms
  criterion_mean_ci_lower_ms = $offRun.mean_ci_lower_ms
  criterion_mean_ci_upper_ms = $offRun.mean_ci_upper_ms
  criterion_median_ms = $offRun.median_ms
  gpu = $gpuInfo
}
if ($CheckDynamicStrategyRatio -and $null -ne $offDynRun -and $null -ne $onDynRun) {
  $ratio = 0.0
  if ($offDynRun.steady_state_wall_ms -gt 0.0) {
    $ratio = [double]$onDynRun.steady_state_wall_ms / [double]$offDynRun.steady_state_wall_ms
  }
  $summary.dynamic_strategy = [ordered]@{
    checked = $true
    method = "profile_gpu_dynamic_runner_steady_state"
    regression_pct = $DynamicStrategyRegressionPct
    off_first_wall_ms = $offDynRun.first_wall_ms
    on_first_wall_ms = $onDynRun.first_wall_ms
    off_mean_ms = $offDynRun.steady_state_wall_ms
    on_mean_ms = $onDynRun.steady_state_wall_ms
    off_runtime_ms = $offDynRun.steady_state_runtime_ms
    on_runtime_ms = $onDynRun.steady_state_runtime_ms
    ratio_on_over_off = [math]::Round($ratio, 6)
  }
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
$dynamicPass = $true
if ($CheckDynamicStrategyRatio -and $null -ne $summary.dynamic_strategy) {
  $offMean = [double]$summary.dynamic_strategy.off_mean_ms
  $onMean = [double]$summary.dynamic_strategy.on_mean_ms
  $dynamicBudget = 1.0 + ($DynamicStrategyRegressionPct / 100.0)
  $dynamicPass = $onMean -le ($offMean * $dynamicBudget)
  Write-PassFailLine "gpu_dynamic_strategy_ratio_not_regressed_${DynamicStrategyRegressionPct}pct" $dynamicPass "$onMean ms (on) vs $offMean ms (off)"
}

if (!($pass -and $dynamicPass)) {
  throw "GPU performance gate failed"
}
