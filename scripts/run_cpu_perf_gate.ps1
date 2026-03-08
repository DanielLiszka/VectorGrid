param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselinePath = "scripts/baselines/cpu_perf_baseline.json",
  [string]$OutPath = "target/cpu_perf_summary.json",
  [string]$BenchFilter = "double_ma/cpu/sma_sma/200k/small",
  [string]$BenchEstimatesPath = "target/criterion/double_ma_cpu_sma_sma_200k/small_1590pairs/new/estimates.json",
  [double]$CriterionRegressionPct = 15.0,
  [bool]$CheckDynamicStrategyCpuRatio = $true,
  [string]$DynamicBenchFilter = "dynamic_strategy_cpu/full_run/2k/composite_mo",
  [string]$DynamicStaticEstimatesPath = "target/criterion/dynamic_strategy_cpu_full_run_2k/composite_mo_static_env_off/new/estimates.json",
  [string]$DynamicOnEstimatesPath = "target/criterion/dynamic_strategy_cpu_full_run_2k/composite_mo_dynamic_env_on/new/estimates.json",
  [double]$DynamicStrategyCpuRegressionPct = 20.0
)

$ErrorActionPreference = "Stop"

function Write-PassFailLine([string]$name, [bool]$pass, [string]$msg) {
  if ($pass) {
    Write-Host "[PASS] ${name}: $msg"
  } else {
    Write-Host "[FAIL] ${name}: $msg"
  }
}

$workflowBaselinePath = "scripts/baselines/workflow_regression_baseline.json"
$workflowMode = "candidate"
if (($Mode -eq "baseline") -and (!(Test-Path $workflowBaselinePath))) {
  # Bootstrap only when no workflow baseline exists yet.
  $workflowMode = "baseline"
}
Write-Host "Running workflow regression gate ($workflowMode)..."
powershell -File "scripts/run_workflow_gate.ps1" -Mode $workflowMode
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Write-Host "Running criterion benchmark ($BenchFilter)..."
$benchCmd = @(
  "bench", "-p", "ta_app_core", "--bench", "double_ma_opt", "--", $BenchFilter
)
cargo @benchCmd
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

if ($CheckDynamicStrategyCpuRatio) {
  Write-Host "Running dynamic strategy CPU benchmark ($DynamicBenchFilter)..."
  $dynBenchCmd = @(
    "bench", "-p", "ta_app_core", "--bench", "dynamic_strategy_cpu", "--", $DynamicBenchFilter
  )
  cargo @dynBenchCmd
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
}

if (!(Test-Path $BenchEstimatesPath)) {
  throw "Missing criterion estimates file: $BenchEstimatesPath"
}
if ($CheckDynamicStrategyCpuRatio) {
  if (!(Test-Path $DynamicStaticEstimatesPath)) {
    throw "Missing dynamic static estimates file: $DynamicStaticEstimatesPath"
  }
  if (!(Test-Path $DynamicOnEstimatesPath)) {
    throw "Missing dynamic on estimates file: $DynamicOnEstimatesPath"
  }
}
if (!(Test-Path "target/workflow_regression_summary.json")) {
  throw "Missing workflow summary: target/workflow_regression_summary.json"
}

$criterion = Get-Content -Raw $BenchEstimatesPath | ConvertFrom-Json
$dynStatic = $null
$dynOn = $null
if ($CheckDynamicStrategyCpuRatio) {
  $dynStatic = Get-Content -Raw $DynamicStaticEstimatesPath | ConvertFrom-Json
  $dynOn = Get-Content -Raw $DynamicOnEstimatesPath | ConvertFrom-Json
}
$workflow = Get-Content -Raw "target/workflow_regression_summary.json" | ConvertFrom-Json

$summary = [ordered]@{
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  workflow_runtime_ms = [double]$workflow.runtime_ms
  criterion = [ordered]@{
    bench_filter = $BenchFilter
    estimates_json = $BenchEstimatesPath
    mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
    mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
    mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
    median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  }
}
if ($CheckDynamicStrategyCpuRatio) {
  $dynOffMean = [math]::Round(([double]$dynStatic.mean.point_estimate) / 1000000.0, 6)
  $dynOnMean = [math]::Round(([double]$dynOn.mean.point_estimate) / 1000000.0, 6)
  $dynRatio = 0.0
  if ($dynOffMean -gt 0.0) {
    $dynRatio = [double]$dynOnMean / [double]$dynOffMean
  }
  $summary.dynamic_strategy_cpu = [ordered]@{
    checked = $true
    bench_filter = $DynamicBenchFilter
    static_estimates_json = $DynamicStaticEstimatesPath
    dynamic_estimates_json = $DynamicOnEstimatesPath
    static_mean_ms = $dynOffMean
    dynamic_mean_ms = $dynOnMean
    ratio_dynamic_over_static = [math]::Round($dynRatio, 6)
  }
}

$outDir = Split-Path -Parent $OutPath
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}
$summary | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 $OutPath
Write-Host "CPU perf summary written to $OutPath"

if ($Mode -eq "baseline") {
  $baseDir = Split-Path -Parent $BaselinePath
  if ($baseDir) {
    New-Item -ItemType Directory -Force -Path $baseDir | Out-Null
  }
  $summary | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 $BaselinePath
  Write-Host "CPU perf baseline written to $BaselinePath"
  exit 0
}

if (!(Test-Path $BaselinePath)) {
  throw "Missing CPU perf baseline: $BaselinePath. Run with -Mode baseline first."
}

$baseline = Get-Content -Raw $BaselinePath | ConvertFrom-Json
$candWf = [double]$summary.workflow_runtime_ms
$baseWf = [double]$baseline.workflow_runtime_ms
$candMean = [double]$summary.criterion.mean_ms
$baseMean = [double]$baseline.criterion.mean_ms

$criterionBudget = 1.0 + ($CriterionRegressionPct / 100.0)
$criterionPass = $candMean -le ($baseMean * $criterionBudget)
$workflowPass = $candWf -le ($baseWf * 1.5 + 1.0)
$dynPass = $true
$dynCheckApplied = $false
if ($CheckDynamicStrategyCpuRatio) {
  if ($null -eq $baseline.dynamic_strategy_cpu) {
    Write-Host "[WARN] Baseline is missing dynamic_strategy_cpu block; skipping dynamic ratio check in candidate mode. Re-run baseline mode to enable this gate."
  } else {
    $candDynRatio = [double]$summary.dynamic_strategy_cpu.ratio_dynamic_over_static
    $baseDynRatio = [double]$baseline.dynamic_strategy_cpu.ratio_dynamic_over_static
    $dynBudget = 1.0 + ($DynamicStrategyCpuRegressionPct / 100.0)
    $dynPass = $candDynRatio -le ($baseDynRatio * $dynBudget)
    $dynCheckApplied = $true
  }
}

Write-PassFailLine "workflow_runtime_not_regressed_50pct" $workflowPass "$candWf ms vs baseline $baseWf ms"
Write-PassFailLine "criterion_mean_not_regressed_${CriterionRegressionPct}pct" $criterionPass "$candMean ms vs baseline $baseMean ms"
if ($CheckDynamicStrategyCpuRatio -and $dynCheckApplied) {
  $candDynRatio = [double]$summary.dynamic_strategy_cpu.ratio_dynamic_over_static
  $baseDynRatio = [double]$baseline.dynamic_strategy_cpu.ratio_dynamic_over_static
  Write-PassFailLine "cpu_dynamic_strategy_ratio_not_regressed_${DynamicStrategyCpuRegressionPct}pct" $dynPass "$candDynRatio vs baseline $baseDynRatio"
}

if (!($workflowPass -and $criterionPass -and $dynPass)) {
  throw "CPU performance gate failed"
}
