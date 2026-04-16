param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselinePath = "scripts/baselines/cpu_perf_baseline.json",
  [string]$OutPath = "output/benchmarks/cpu_perf_summary.json",
  [double]$CriterionRegressionPct = 15.0,
  [bool]$CheckDynamicStrategyCpuRatio = $true,
  [string]$DynamicBenchFilter = "dynamic_strategy_cpu/full_run/2k/composite_mo",
  [string]$DynamicStaticEstimatesPath = "target/criterion/dynamic_strategy_cpu_full_run_2k/composite_mo_static_env_off/new/estimates.json",
  [string]$DynamicOnEstimatesPath = "target/criterion/dynamic_strategy_cpu_full_run_2k/composite_mo_dynamic_env_on/new/estimates.json",
  [double]$DynamicStrategyCpuRegressionPct = 20.0,
  [string]$WorkflowBaselinePath = "scripts/baselines/workflow_regression_baseline.json",
  [string]$WorkflowSummaryPath = "output/benchmarks/workflow_regression_summary.json",
  [ValidateSet("release", "dev")]
  [string]$WorkflowBuildProfile = "release"
)

$ErrorActionPreference = "Stop"

$SchemaVersion = 1
$CaseRegistryVersion = 1

function Write-PassFailLine([string]$name, [bool]$pass, [string]$msg) {
  if ($pass) {
    Write-Host "[PASS] ${name}: $msg"
  } else {
    Write-Host "[FAIL] ${name}: $msg"
  }
}

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

function Read-JsonFile {
  param([Parameter(Mandatory = $true)][string]$Path)
  return Get-Content -Raw $Path | ConvertFrom-Json
}

function Assert-CompatibilityEqual {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)]$Candidate,
    [Parameter(Mandatory = $true)]$Baseline
  )

  if ("$Candidate" -ne "$Baseline") {
    throw "Incompatible baseline for ${Name}: candidate='$Candidate' baseline='$Baseline'"
  }
}

function Get-EnvironmentFingerprint {
  $hostname = if ([string]::IsNullOrWhiteSpace($env:COMPUTERNAME)) { $env:HOSTNAME } else { $env:COMPUTERNAME }
  [ordered]@{
    hostname = "$hostname"
    os = [System.Environment]::OSVersion.VersionString
    architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
    powershell_edition = $PSVersionTable.PSEdition
    powershell_version = $PSVersionTable.PSVersion.ToString()
  }
}

function Convert-CriterionEstimateToCaseSummary {
  param(
    [Parameter(Mandatory = $true)]$CaseSpec
  )

  if (!(Test-Path $CaseSpec.estimates_path)) {
    throw "Missing criterion estimates file for case '$($CaseSpec.id)': $($CaseSpec.estimates_path)"
  }
  $criterion = Read-JsonFile -Path $CaseSpec.estimates_path
  return [ordered]@{
    id = $CaseSpec.id
    schema_version = $SchemaVersion
    timing_boundary = "Criterion"
    timing_boundary_note = "Criterion bench profile estimate for a single deterministic CPU optimization case."
    build_profile = "bench"
    bench_filter = $CaseSpec.bench_filter
    estimates_json = $CaseSpec.estimates_path
    mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
    mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
    mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
    median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  }
}

function Get-CaseById {
  param(
    [Parameter(Mandatory = $true)]$Cases,
    [Parameter(Mandatory = $true)][string]$Id
  )
  return @($Cases | Where-Object { $_.id -eq $Id } | Select-Object -First 1)[0]
}

$criterionCases = @(
  [ordered]@{
    id = "cpu_sma_small"
    bench_filter = "double_ma/cpu/sma_sma/200k/small"
    estimates_path = "target/criterion/double_ma_cpu_sma_sma_200k/small_1590pairs/new/estimates.json"
  },
  [ordered]@{
    id = "cpu_alma_large"
    bench_filter = "double_ma/cpu/alma_alma/200k/large"
    estimates_path = "target/criterion/double_ma_cpu_alma_alma_200k/large_58300pairs/new/estimates.json"
  }
)

$workflowMode = "candidate"
if (($Mode -eq "baseline") -and (!(Test-Path $WorkflowBaselinePath))) {
  $workflowMode = "baseline"
}
Write-Host "Running workflow regression gate ($workflowMode)..."
powershell -File "scripts/run_workflow_gate.ps1" `
  -Mode $workflowMode `
  -BaselinePath $WorkflowBaselinePath `
  -OutPath $WorkflowSummaryPath `
  -BuildProfile $WorkflowBuildProfile
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}
if (!(Test-Path $WorkflowSummaryPath)) {
  throw "Missing requested workflow summary: $WorkflowSummaryPath"
}

foreach ($case in $criterionCases) {
  Write-Host "Running criterion benchmark ($($case.bench_filter))..."
  $benchCmd = @(
    "bench", "-p", "ta_app_core", "--bench", "double_ma_opt", "--", $case.bench_filter
  )
  cargo @benchCmd
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
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

$criterionCaseSummaries = @()
foreach ($case in $criterionCases) {
  $criterionCaseSummaries += ,(Convert-CriterionEstimateToCaseSummary -CaseSpec $case)
}

if ($CheckDynamicStrategyCpuRatio) {
  if (!(Test-Path $DynamicStaticEstimatesPath)) {
    throw "Missing dynamic static estimates file: $DynamicStaticEstimatesPath"
  }
  if (!(Test-Path $DynamicOnEstimatesPath)) {
    throw "Missing dynamic on estimates file: $DynamicOnEstimatesPath"
  }
}

$workflow = Read-JsonFile -Path $WorkflowSummaryPath
$workflowTimingBoundary = if ($workflow.PSObject.Properties.Name -contains "timing_boundary") { $workflow.timing_boundary } else { "workflow-regression" }
$workflowTimingNote = if ($workflow.PSObject.Properties.Name -contains "timing_boundary_note") { $workflow.timing_boundary_note } else { "Warm workflow-regression wall-clock summary from ta_workflow_regression." }
$workflowProfile = if ($workflow.PSObject.Properties.Name -contains "build_profile") { $workflow.build_profile } else { $WorkflowBuildProfile }
$workflowBlock = [ordered]@{
  schema_version = if ($workflow.PSObject.Properties.Name -contains "schema_version") { $workflow.schema_version } else { $SchemaVersion }
  timing_boundary = $workflowTimingBoundary
  timing_boundary_note = $workflowTimingNote
  build_profile = $workflowProfile
  summary_path = $WorkflowSummaryPath
  baseline_path = $WorkflowBaselinePath
  runtime_ms = [double]$workflow.runtime_ms
  result = $workflow
}

$summary = [ordered]@{
  schema_version = $SchemaVersion
  case_registry_version = $CaseRegistryVersion
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  environment = Get-EnvironmentFingerprint
  workflow = $workflowBlock
  workflow_runtime_ms = [double]$workflow.runtime_ms
  criterion = $criterionCaseSummaries[0]
  criterion_cases = $criterionCaseSummaries
}

if ($CheckDynamicStrategyCpuRatio) {
  $dynStatic = Read-JsonFile -Path $DynamicStaticEstimatesPath
  $dynOn = Read-JsonFile -Path $DynamicOnEstimatesPath
  $dynOffMean = [math]::Round(([double]$dynStatic.mean.point_estimate) / 1000000.0, 6)
  $dynOnMean = [math]::Round(([double]$dynOn.mean.point_estimate) / 1000000.0, 6)
  $dynRatio = 0.0
  if ($dynOffMean -gt 0.0) {
    $dynRatio = [double]$dynOnMean / [double]$dynOffMean
  }
  $summary.dynamic_strategy_cpu = [ordered]@{
    checked = $true
    schema_version = $SchemaVersion
    timing_boundary = "Criterion"
    timing_boundary_note = "Criterion bench profile ratio between dynamic-strategy CPU full-run variants."
    build_profile = "bench"
    bench_filter = $DynamicBenchFilter
    static_estimates_json = $DynamicStaticEstimatesPath
    dynamic_estimates_json = $DynamicOnEstimatesPath
    static_mean_ms = $dynOffMean
    dynamic_mean_ms = $dynOnMean
    ratio_dynamic_over_static = [math]::Round($dynRatio, 6)
  }
}

Write-JsonUtf8 -Path $OutPath -Value $summary
Write-Host "CPU perf summary written to $OutPath"

if ($Mode -eq "baseline") {
  Write-JsonUtf8 -Path $BaselinePath -Value $summary
  Write-Host "CPU perf baseline written to $BaselinePath"
  exit 0
}

if (!(Test-Path $BaselinePath)) {
  throw "Missing CPU perf baseline: $BaselinePath. Run with -Mode baseline first."
}

$baseline = Read-JsonFile -Path $BaselinePath
Assert-CompatibilityEqual -Name "cpu_perf.schema_version" -Candidate $summary.schema_version -Baseline $baseline.schema_version
Assert-CompatibilityEqual -Name "cpu_perf.case_registry_version" -Candidate $summary.case_registry_version -Baseline $baseline.case_registry_version
Assert-CompatibilityEqual -Name "workflow.schema_version" -Candidate $summary.workflow.schema_version -Baseline $baseline.workflow.schema_version
Assert-CompatibilityEqual -Name "workflow.timing_boundary" -Candidate $summary.workflow.timing_boundary -Baseline $baseline.workflow.timing_boundary
Assert-CompatibilityEqual -Name "workflow.build_profile" -Candidate $summary.workflow.build_profile -Baseline $baseline.workflow.build_profile

$candWf = [double]$summary.workflow.runtime_ms
$baseWf = [double]$baseline.workflow.runtime_ms
$criterionBudget = 1.0 + ($CriterionRegressionPct / 100.0)
$workflowPass = $candWf -le ($baseWf * 1.5 + 1.0)
$allCriterionPass = $true

foreach ($case in $summary.criterion_cases) {
  $baseCase = Get-CaseById -Cases $baseline.criterion_cases -Id $case.id
  if ($null -eq $baseCase) {
    throw "Missing baseline criterion case '$($case.id)'"
  }
  Assert-CompatibilityEqual -Name "$($case.id).schema_version" -Candidate $case.schema_version -Baseline $baseCase.schema_version
  Assert-CompatibilityEqual -Name "$($case.id).timing_boundary" -Candidate $case.timing_boundary -Baseline $baseCase.timing_boundary
  Assert-CompatibilityEqual -Name "$($case.id).build_profile" -Candidate $case.build_profile -Baseline $baseCase.build_profile
  Assert-CompatibilityEqual -Name "$($case.id).bench_filter" -Candidate $case.bench_filter -Baseline $baseCase.bench_filter

  $candMean = [double]$case.mean_ms
  $baseMean = [double]$baseCase.mean_ms
  $casePass = $candMean -le ($baseMean * $criterionBudget)
  if (-not $casePass) {
    $allCriterionPass = $false
  }
  Write-PassFailLine "criterion_case_$($case.id)_not_regressed_${CriterionRegressionPct}pct" $casePass "$candMean ms vs baseline $baseMean ms"
}

$dynPass = $true
$dynCheckApplied = $false
if ($CheckDynamicStrategyCpuRatio) {
  if ($null -eq $baseline.dynamic_strategy_cpu) {
    throw "Incompatible baseline: dynamic_strategy_cpu block is missing."
  }
  Assert-CompatibilityEqual -Name "dynamic_strategy_cpu.schema_version" -Candidate $summary.dynamic_strategy_cpu.schema_version -Baseline $baseline.dynamic_strategy_cpu.schema_version
  Assert-CompatibilityEqual -Name "dynamic_strategy_cpu.timing_boundary" -Candidate $summary.dynamic_strategy_cpu.timing_boundary -Baseline $baseline.dynamic_strategy_cpu.timing_boundary
  Assert-CompatibilityEqual -Name "dynamic_strategy_cpu.build_profile" -Candidate $summary.dynamic_strategy_cpu.build_profile -Baseline $baseline.dynamic_strategy_cpu.build_profile
  Assert-CompatibilityEqual -Name "dynamic_strategy_cpu.bench_filter" -Candidate $summary.dynamic_strategy_cpu.bench_filter -Baseline $baseline.dynamic_strategy_cpu.bench_filter
  $candDynRatio = [double]$summary.dynamic_strategy_cpu.ratio_dynamic_over_static
  $baseDynRatio = [double]$baseline.dynamic_strategy_cpu.ratio_dynamic_over_static
  $dynBudget = 1.0 + ($DynamicStrategyCpuRegressionPct / 100.0)
  $dynPass = $candDynRatio -le ($baseDynRatio * $dynBudget)
  $dynCheckApplied = $true
}

Write-PassFailLine "workflow_runtime_not_regressed_50pct" $workflowPass "$candWf ms vs baseline $baseWf ms"
if ($CheckDynamicStrategyCpuRatio -and $dynCheckApplied) {
  $candDynRatio = [double]$summary.dynamic_strategy_cpu.ratio_dynamic_over_static
  $baseDynRatio = [double]$baseline.dynamic_strategy_cpu.ratio_dynamic_over_static
  Write-PassFailLine "cpu_dynamic_strategy_ratio_not_regressed_${DynamicStrategyCpuRegressionPct}pct" $dynPass "$candDynRatio vs baseline $baseDynRatio"
}

if (!($workflowPass -and $allCriterionPass -and $dynPass)) {
  throw "CPU performance gate failed"
}
