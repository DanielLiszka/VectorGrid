param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [ValidateSet("gpu-kernel", "gpu-ma")]
  [string]$WorkloadProfile = "gpu-kernel",
  [string]$BaselinePath = "",
  [string]$OutPath = "",
  [string]$CaseId = "",
  [string]$BenchFilter = "",
  [string]$CriterionEstimatesPath = "",
  [string]$CriterionEstimatesGlob = "",
  [double]$RegressionPct = 10.0,
  [bool]$CheckDynamicStrategyRatio = $true,
  [double]$DynamicStrategyRegressionPct = 20.0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")

$SchemaVersion = 1
$CaseRegistryVersion = 1

if ([string]::IsNullOrWhiteSpace($BaselinePath)) {
  $BaselinePath = if ($WorkloadProfile -eq "gpu-ma") {
    "scripts/baselines/gpu_ma_perf_baseline.json"
  } else {
    "scripts/baselines/gpu_perf_baseline.json"
  }
}
if ([string]::IsNullOrWhiteSpace($OutPath)) {
  $OutPath = if ($WorkloadProfile -eq "gpu-ma") {
    "output/benchmarks/gpu_ma_perf_summary.json"
  } else {
    "output/benchmarks/gpu_perf_summary.json"
  }
}
if ($WorkloadProfile -eq "gpu-ma" -and $CheckDynamicStrategyRatio) {
  throw "The dynamic-strategy ratio is a gpu-kernel authority and cannot be enabled for -WorkloadProfile gpu-ma."
}
$GpuFeatureSet = if ($WorkloadProfile -eq "gpu-ma") {
  "cuda"
} else {
  "cuda,cuda-backtest-kernel"
}
$GpuWorkloadLabel = if ($WorkloadProfile -eq "gpu-ma") {
  "GPU MA-sweep"
} else {
  "GPU kernel"
}
if ($Mode -eq "candidate" -and -not (Test-Path -LiteralPath $BaselinePath -PathType Leaf)) {
  throw "Missing $GpuWorkloadLabel perf baseline: $BaselinePath. Run with -Mode baseline first."
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

function Get-PerfProcesses() {
  @(Get-Process | Where-Object {
      $_.ProcessName -match '^(cargo|criterion|nsys|ncu)$' -or $_.ProcessName -like 'double_ma_opt*'
    } | Select-Object ProcessName, Id, CPU, StartTime)
}

function Wait-NoPerfProcesses(
  [int]$TimeoutSeconds = 300,
  [int]$PollMilliseconds = 500,
  [string]$Context = ""
) {
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ($true) {
    $procs = @(Get-PerfProcesses)
    if ($procs.Count -eq 0) {
      return
    }
    if ((Get-Date) -ge $deadline) {
      $summary = ($procs | ForEach-Object {
          "{0}#{1}" -f $_.ProcessName, $_.Id
        }) -join ", "
      if ([string]::IsNullOrWhiteSpace($Context)) {
        throw "Timed out waiting for benchmark/profiler processes to drain: $summary"
      }
      throw "Timed out waiting for benchmark/profiler processes to drain ${Context}: $summary"
    }
    Start-Sleep -Milliseconds $PollMilliseconds
  }
}

function Write-PassFailLine([string]$name, [bool]$pass, [string]$msg) {
  if ($pass) {
    Write-Host "[PASS] ${name}: $msg"
  } else {
    Write-Host "[FAIL] ${name}: $msg"
  }
}

function Invoke-CargoCommandAndEcho(
  [string[]]$ArgumentList,
  [string]$FailureContext
) {
  $stdoutPath = [System.IO.Path]::GetTempFileName()
  $stderrPath = [System.IO.Path]::GetTempFileName()
  try {
    $proc = Start-Process -FilePath "cargo" `
      -ArgumentList $ArgumentList `
      -NoNewWindow `
      -Wait `
      -PassThru `
      -RedirectStandardOutput $stdoutPath `
      -RedirectStandardError $stderrPath
    $output = @()
    if (Test-Path $stdoutPath) {
      $output += Get-Content $stdoutPath
    }
    if (Test-Path $stderrPath) {
      $output += Get-Content $stderrPath
    }
    foreach ($line in $output) {
      Write-Host $line
    }
    if ($proc.ExitCode -ne 0) {
      throw "${FailureContext} (cargo exit code $($proc.ExitCode))"
    }
  } finally {
    Remove-Item $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
  }
}

function Invoke-ProcessAndEcho(
  [string]$FilePath,
  [string[]]$ArgumentList,
  [string]$FailureContext
) {
  $stdoutPath = [System.IO.Path]::GetTempFileName()
  $stderrPath = [System.IO.Path]::GetTempFileName()
  try {
    $proc = Start-Process -FilePath $FilePath `
      -ArgumentList $ArgumentList `
      -NoNewWindow `
      -Wait `
      -PassThru `
      -RedirectStandardOutput $stdoutPath `
      -RedirectStandardError $stderrPath
    $output = @()
    if (Test-Path $stdoutPath) {
      $output += Get-Content $stdoutPath
    }
    if (Test-Path $stderrPath) {
      $output += Get-Content $stderrPath
    }
    foreach ($line in $output) {
      Write-Host $line
    }
    if ($proc.ExitCode -ne 0) {
      throw "${FailureContext} ($FilePath exit code $($proc.ExitCode))"
    }
    return $output
  } finally {
    Remove-Item $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
  }
}

function Find-LatestBenchExecutable([string]$benchName) {
  $benchExe = $null
  foreach ($candidate in Get-ChildItem -Path "target/release/deps" -File -Filter "${benchName}-*.exe") {
    if ($null -eq $benchExe -or $candidate.LastWriteTime -gt $benchExe.LastWriteTime) {
      $benchExe = $candidate
    }
  }
  if ($null -eq $benchExe) {
    throw "Could not find built benchmark executable for ${benchName} under target/release/deps"
  }
  return $benchExe
}

function Get-GpuIdentity {
  $raw = ""
  $name = ""
  $driverVersion = ""
  try {
    $raw = (& nvidia-smi --query-gpu=name,driver_version --format=csv,noheader | Select-Object -First 1).Trim()
    if (-not [string]::IsNullOrWhiteSpace($raw)) {
      $parts = @($raw.Split(",", 2))
      if ($parts.Count -ge 1) {
        $name = $parts[0].Trim()
      }
      if ($parts.Count -ge 2) {
        $driverVersion = $parts[1].Trim()
      }
    }
  } catch {
    $raw = ""
  }

  return [ordered]@{
    name = $name
    driver_version = $driverVersion
    raw = $raw
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

function Resolve-EstimatePath {
  param(
    [string]$ExactPath,
    [string]$GlobPath
  )

  if (-not [string]::IsNullOrWhiteSpace($ExactPath)) {
    if (Test-Path $ExactPath) {
      return (Resolve-Path $ExactPath).Path
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $ExactPath))
  }

  if (-not [string]::IsNullOrWhiteSpace($GlobPath)) {
    $matches = @(Get-ChildItem -Path "target/criterion" -Recurse -File -Filter "estimates.json" |
        Where-Object { $_.FullName -like $GlobPath })
    if ($matches.Count -eq 0) {
      throw "No criterion estimates file matched '$GlobPath'"
    }
    if ($matches.Count -gt 1) {
      $paths = ($matches | ForEach-Object { $_.FullName }) -join "; "
      throw "Ambiguous criterion estimates glob '$GlobPath': $paths"
    }
    return $matches[0].FullName
  }

  throw "Missing criterion estimates path; provide -CriterionEstimatesPath."
}

function Get-GpuPerfCases {
  param(
    [Parameter(Mandatory = $true)][string]$Profile,
    [string]$CustomCaseId,
    [string]$CustomBenchFilter,
    [string]$CustomEstimatesPath,
    [string]$CustomEstimatesGlob
  )

  $hasCustomCase = -not [string]::IsNullOrWhiteSpace($CustomBenchFilter) `
    -or -not [string]::IsNullOrWhiteSpace($CustomEstimatesPath) `
    -or -not [string]::IsNullOrWhiteSpace($CustomEstimatesGlob)

  if ($hasCustomCase) {
    if ([string]::IsNullOrWhiteSpace($CustomBenchFilter)) {
      throw "Custom GPU gate runs require -BenchFilter."
    }
    $resolvedEstimatePath = Resolve-EstimatePath -ExactPath $CustomEstimatesPath -GlobPath $CustomEstimatesGlob
    return @(
      [ordered]@{
        id = $(if ([string]::IsNullOrWhiteSpace($CustomCaseId)) { "custom_gpu_case" } else { $CustomCaseId })
        bench_filter = $CustomBenchFilter
        estimates_path = $resolvedEstimatePath
      }
    )
  }

  if ($Profile -eq "gpu-ma") {
    return @(
      [ordered]@{
        id = "gpu_ma_sma_small"
        bench_filter = "double_ma/gpu_sweep/sma_sma/200k/small"
        estimates_path = "target/criterion/double_ma_gpu_sweep_sma_sma_200k/small_1590pairs/new/estimates.json"
      },
      [ordered]@{
        id = "gpu_ma_alma_large"
        bench_filter = "double_ma/gpu_sweep/alma_alma/200k/large"
        estimates_path = "target/criterion/double_ma_gpu_sweep_alma_alma_200k/large_58300pairs/new/estimates.json"
      }
    )
  }

  return @(
    [ordered]@{
      id = "gpu_sma_small"
      bench_filter = "double_ma/gpu_kernel/sma_sma/200k/small"
      estimates_path = "target/criterion/double_ma_gpu_kernel_sma_sma_200k/small_1590pairs/new/estimates.json"
    },
    [ordered]@{
      id = "gpu_alma_large"
      bench_filter = "double_ma/gpu_kernel/alma_alma/200k/large"
      estimates_path = "target/criterion/double_ma_gpu_kernel_alma_alma_200k/large_58300pairs/new/estimates.json"
    }
  )
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
    timing_boundary_note = "Criterion bench profile estimate for a single deterministic $GpuWorkloadLabel optimization case."
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

function Invoke-GpuBenchCase {
  param(
    [Parameter(Mandatory = $true)][string]$BenchExePath,
    [Parameter(Mandatory = $true)]$CaseSpec
  )

  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU benchmark launch"
  Remove-Item Env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL -ErrorAction SilentlyContinue
  Write-Host "Running $GpuWorkloadLabel benchmark ($($CaseSpec.bench_filter)) with dynamic strategy preflight OFF..."
  Invoke-ProcessAndEcho -FilePath $BenchExePath -ArgumentList @(
    $CaseSpec.bench_filter,
    "--bench"
  ) -FailureContext "$GpuWorkloadLabel benchmark process failed" | Out-Null
  Wait-NoPerfProcesses -TimeoutSeconds 900 -Context "after GPU benchmark completion"

  if (!(Test-Path $CaseSpec.estimates_path)) {
    throw "Expected criterion estimates were not generated for case '$($CaseSpec.id)': $($CaseSpec.estimates_path)"
  }
}

function Invoke-DynamicStrategyProfileRun {
  param(
    [Parameter(Mandatory = $true)][string]$RunnerExePath,
    [bool]$DynamicStrategyEnabled,
    [int]$Iterations = 5
  )

  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before dynamic strategy profile run"
  if ($DynamicStrategyEnabled) {
    Write-Host "Running profile_gpu_dynamic_runner with dynamic strategy preflight ON..."
  } else {
    Write-Host "Running profile_gpu_dynamic_runner with dynamic strategy preflight OFF..."
  }

  $dynMode = if ($DynamicStrategyEnabled) { "on" } else { "off" }
  $output = Invoke-ProcessAndEcho -FilePath $RunnerExePath -ArgumentList @(
    "--strategy", "double_ma",
    "--workload", "small",
    "--iterations", "$Iterations",
    "--dynamic-mode", $dynMode,
    "--device-id", "0"
  ) -FailureContext "profile_gpu_dynamic_runner failed"
  Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "after dynamic strategy profile run"

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

$criterionCases = Get-GpuPerfCases `
  -Profile $WorkloadProfile `
  -CustomCaseId $CaseId `
  -CustomBenchFilter $BenchFilter `
  -CustomEstimatesPath $CriterionEstimatesPath `
  -CustomEstimatesGlob $CriterionEstimatesGlob

$prevDynamicFlag = [Environment]::GetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "Process")
$offDynRun = $null
$onDynRun = $null
$gpuIdentity = Get-GpuIdentity

$benchmarkLock = Enter-BenchmarkLock -Context "GPU perf gate"
try {
  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU perf gate start"
  try {
    Invoke-CargoCommandAndEcho -ArgumentList @(
      "bench",
      "-p", "ta_app_core",
      "--bench", "double_ma_opt",
      "--features", $GpuFeatureSet,
      "--no-run"
    ) -FailureContext "Failed to build $GpuWorkloadLabel benchmark executable"

    $benchExe = Find-LatestBenchExecutable -benchName "double_ma_opt"
    foreach ($case in $criterionCases) {
      Invoke-GpuBenchCase -BenchExePath $benchExe.FullName -CaseSpec $case
    }

    if ($CheckDynamicStrategyRatio) {
      Invoke-CargoCommandAndEcho -ArgumentList @(
        "build",
        "-q",
        "-p", "ta_app_core",
        "--bin", "profile_gpu_dynamic_runner",
        "--release",
        "--features", "cuda,cuda-backtest-kernel"
      ) -FailureContext "Failed to build profile_gpu_dynamic_runner"
      $runnerExe = Join-Path (Resolve-Path "target/release").Path "profile_gpu_dynamic_runner.exe"
      $offDynRun = Invoke-DynamicStrategyProfileRun -RunnerExePath $runnerExe -DynamicStrategyEnabled $false -Iterations 5
      $onDynRun = Invoke-DynamicStrategyProfileRun -RunnerExePath $runnerExe -DynamicStrategyEnabled $true -Iterations 5
    }
  } finally {
    Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "after GPU perf gate work"
  }
} finally {
  if ([string]::IsNullOrWhiteSpace($prevDynamicFlag)) {
    Remove-Item Env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL -ErrorAction SilentlyContinue
  } else {
    $env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL = $prevDynamicFlag
  }
  Exit-BenchmarkLock -Handle $benchmarkLock
}

$criterionCaseSummaries = @()
foreach ($case in $criterionCases) {
  $criterionCaseSummaries += ,(Convert-CriterionEstimateToCaseSummary -CaseSpec $case)
}

$summary = [ordered]@{
  schema_version = $SchemaVersion
  case_registry_version = $CaseRegistryVersion
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  workload_profile = $WorkloadProfile
  cargo_features = $GpuFeatureSet
  environment = Get-EnvironmentFingerprint
  gpu = $gpuIdentity.raw
  gpu_info = $gpuIdentity
  criterion = $criterionCaseSummaries[0]
  criterion_cases = $criterionCaseSummaries
  bench_filter = $criterionCaseSummaries[0].bench_filter
  criterion_estimates_json = $criterionCaseSummaries[0].estimates_json
  criterion_mean_ms = $criterionCaseSummaries[0].mean_ms
  criterion_mean_ci_lower_ms = $criterionCaseSummaries[0].mean_ci_lower_ms
  criterion_mean_ci_upper_ms = $criterionCaseSummaries[0].mean_ci_upper_ms
  criterion_median_ms = $criterionCaseSummaries[0].median_ms
}

if ($CheckDynamicStrategyRatio -and $null -ne $offDynRun -and $null -ne $onDynRun) {
  $ratio = 0.0
  if ($offDynRun.steady_state_wall_ms -gt 0.0) {
    $ratio = [double]$onDynRun.steady_state_wall_ms / [double]$offDynRun.steady_state_wall_ms
  }
  $summary.dynamic_strategy = [ordered]@{
    checked = $true
    schema_version = $SchemaVersion
    timing_boundary = "RunnerSteadyStateWallClock"
    timing_boundary_note = "Release-built profile_gpu_dynamic_runner steady-state wall-clock ratio between dynamic-strategy off and on runs."
    build_profile = "release"
    method = "profile_gpu_dynamic_runner_steady_state"
    runner = "profile_gpu_dynamic_runner"
    device_id = 0
    iterations = $offDynRun.iterations
    steady_state_iterations = $offDynRun.steady_state_iterations
    off_first_wall_ms = $offDynRun.first_wall_ms
    on_first_wall_ms = $onDynRun.first_wall_ms
    off_mean_ms = $offDynRun.steady_state_wall_ms
    on_mean_ms = $onDynRun.steady_state_wall_ms
    off_runtime_ms = $offDynRun.steady_state_runtime_ms
    on_runtime_ms = $onDynRun.steady_state_runtime_ms
    ratio_on_over_off = [math]::Round($ratio, 6)
  }
}

Write-JsonUtf8 -Path $OutPath -Value $summary
Write-Host "GPU perf summary written to $OutPath"

foreach ($case in $summary.criterion_cases) {
  $ciWidthPct = 0.0
  if ([double]$case.mean_ms -gt 0.0) {
    $ciWidthPct = (([double]$case.mean_ci_upper_ms - [double]$case.mean_ci_lower_ms) / [double]$case.mean_ms) * 100.0
  }
  if ($ciWidthPct -gt 10.0) {
    Write-Host "[WARN] High run variance detected for $($case.id) (mean CI width = $([math]::Round($ciWidthPct, 2))%). Consider stabilizing GPU clocks/load before trusting this baseline."
  }
}

if ($Mode -eq "baseline") {
  Write-JsonUtf8 -Path $BaselinePath -Value $summary
  Write-Host "GPU perf baseline written to $BaselinePath"
  exit 0
}

if (!(Test-Path $BaselinePath)) {
  throw "Missing $GpuWorkloadLabel perf baseline: $BaselinePath. Run with -Mode baseline first."
}

$baseline = Read-JsonFile -Path $BaselinePath
Assert-CompatibilityEqual -Name "gpu_perf.schema_version" -Candidate $summary.schema_version -Baseline $baseline.schema_version
Assert-CompatibilityEqual -Name "gpu_perf.case_registry_version" -Candidate $summary.case_registry_version -Baseline $baseline.case_registry_version
$baselineWorkloadProfile = if ($baseline.PSObject.Properties.Name -contains "workload_profile") {
  [string]$baseline.workload_profile
} else {
  "gpu-kernel"
}
Assert-CompatibilityEqual -Name "gpu_perf.workload_profile" -Candidate $summary.workload_profile -Baseline $baselineWorkloadProfile
if ($baseline.PSObject.Properties.Name -contains "cargo_features") {
  Assert-CompatibilityEqual -Name "gpu_perf.cargo_features" -Candidate $summary.cargo_features -Baseline $baseline.cargo_features
}

if (
  $summary.gpu_info.PSObject.Properties.Name -contains "name" -and
  $baseline.gpu_info -and
  $baseline.gpu_info.PSObject.Properties.Name -contains "name" -and
  -not [string]::IsNullOrWhiteSpace($summary.gpu_info.name) -and
  -not [string]::IsNullOrWhiteSpace($baseline.gpu_info.name)
) {
  Assert-CompatibilityEqual -Name "gpu_info.name" -Candidate $summary.gpu_info.name -Baseline $baseline.gpu_info.name
}

$criterionBudget = 1.0 + ($RegressionPct / 100.0)
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
  Write-PassFailLine "gpu_criterion_case_$($case.id)_not_regressed_${RegressionPct}pct" $casePass "$candMean ms vs baseline $baseMean ms"
}

$dynamicRatioPass = $true
$dynamicOffLatencyPass = $true
$dynamicCheckApplied = $false
if ($CheckDynamicStrategyRatio) {
  if ($null -eq $summary.dynamic_strategy) {
    throw "Dynamic strategy check was requested but the candidate summary is missing dynamic_strategy."
  }
  if ($null -eq $baseline.dynamic_strategy) {
    throw "Incompatible baseline: dynamic_strategy block is missing."
  }
  Assert-CompatibilityEqual -Name "dynamic_strategy.schema_version" -Candidate $summary.dynamic_strategy.schema_version -Baseline $baseline.dynamic_strategy.schema_version
  Assert-CompatibilityEqual -Name "dynamic_strategy.timing_boundary" -Candidate $summary.dynamic_strategy.timing_boundary -Baseline $baseline.dynamic_strategy.timing_boundary
  Assert-CompatibilityEqual -Name "dynamic_strategy.build_profile" -Candidate $summary.dynamic_strategy.build_profile -Baseline $baseline.dynamic_strategy.build_profile
  Assert-CompatibilityEqual -Name "dynamic_strategy.runner" -Candidate $summary.dynamic_strategy.runner -Baseline $baseline.dynamic_strategy.runner
  Assert-CompatibilityEqual -Name "dynamic_strategy.device_id" -Candidate $summary.dynamic_strategy.device_id -Baseline $baseline.dynamic_strategy.device_id

  $dynBudget = 1.0 + ($DynamicStrategyRegressionPct / 100.0)
  $candDynRatio = [double]$summary.dynamic_strategy.ratio_on_over_off
  $baseDynRatio = [double]$baseline.dynamic_strategy.ratio_on_over_off
  $dynamicRatioPass = $candDynRatio -le ($baseDynRatio * $dynBudget)

  $candDynOff = [double]$summary.dynamic_strategy.off_mean_ms
  $baseDynOff = [double]$baseline.dynamic_strategy.off_mean_ms
  $dynamicOffLatencyPass = $candDynOff -le ($baseDynOff * $dynBudget)
  $dynamicCheckApplied = $true
}

if ($CheckDynamicStrategyRatio -and $dynamicCheckApplied) {
  $candDynRatio = [double]$summary.dynamic_strategy.ratio_on_over_off
  $baseDynRatio = [double]$baseline.dynamic_strategy.ratio_on_over_off
  Write-PassFailLine "gpu_dynamic_strategy_ratio_not_regressed_${DynamicStrategyRegressionPct}pct" $dynamicRatioPass "$candDynRatio vs baseline $baseDynRatio"

  $candDynOff = [double]$summary.dynamic_strategy.off_mean_ms
  $baseDynOff = [double]$baseline.dynamic_strategy.off_mean_ms
  Write-PassFailLine "gpu_dynamic_strategy_static_off_latency_not_regressed_${DynamicStrategyRegressionPct}pct" $dynamicOffLatencyPass "$candDynOff ms vs baseline $baseDynOff ms"
}

Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "before GPU perf gate exit"

if (!($allCriterionPass -and $dynamicRatioPass -and $dynamicOffLatencyPass)) {
  throw "GPU performance gate failed"
}
