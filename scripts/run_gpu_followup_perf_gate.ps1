param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselinePath = "scripts/baselines/gpu_followup_perf_baseline.json",
  [string]$OutPath = "output/benchmarks/gpu_followup_perf_summary.json",
  [double]$RegressionPct = 15.0,
  [string]$CaseId = "",
  [string]$BenchFilter = "",
  [string]$CriterionEstimatesPath = "",
  [string]$CriterionEstimatesGlob = ""
)

$ErrorActionPreference = "Stop"

$SchemaVersion = 1
$CaseRegistryVersion = 1

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value,
    [int]$Depth = 16
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

function Write-PassFailLine([string]$name, [bool]$pass, [string]$msg) {
  if ($pass) {
    Write-Host "[PASS] ${name}: $msg"
  } else {
    Write-Host "[FAIL] ${name}: $msg"
  }
}

function Get-PerfProcesses() {
  @(Get-Process | Where-Object {
      $_.ProcessName -match '^(cargo|criterion|nsys|ncu|compute-sanitizer)$' `
        -or $_.ProcessName -like 'double_ma_followup_workflow*'
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
  $benchExe = Get-ChildItem -Path "target/release/deps" -File -Filter "${benchName}-*.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
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
    $matches = @(Get-ChildItem -Path "target/criterion" -Recurse -File -Filter "estimates.json" -ErrorAction SilentlyContinue |
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

  throw "Missing criterion estimates path; provide -CriterionEstimatesPath or -CriterionEstimatesGlob."
}

function Get-GpuFollowupPerfCases {
  $hasCustomCase = -not [string]::IsNullOrWhiteSpace($BenchFilter) `
    -or -not [string]::IsNullOrWhiteSpace($CriterionEstimatesPath) `
    -or -not [string]::IsNullOrWhiteSpace($CriterionEstimatesGlob)

  if ($hasCustomCase) {
    if ([string]::IsNullOrWhiteSpace($BenchFilter)) {
      throw "Custom GPU follow-up gate runs require -BenchFilter."
    }
    return @(
      [ordered]@{
        id = $(if ([string]::IsNullOrWhiteSpace($CaseId)) { "custom_gpu_followup_case" } else { $CaseId })
        bench_name = "double_ma_followup_workflow"
        bench_filter = $BenchFilter
        bench_function = $BenchFilter
        estimates_path = $CriterionEstimatesPath
        estimates_glob = $CriterionEstimatesGlob
        expected_seed_backend_route = "GpuKernel"
      }
    )
  }

  $criterionRoot = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) "target/criterion"))
  return @(
    [ordered]@{
      id = "double_ma_gpu_followup_cold"
      bench_name = "double_ma_followup_workflow"
      bench_filter = "double_ma_followup/gpu_kernel/50k"
      bench_function = "drilldown_then_local_stability_cold"
      estimates_path = ""
      estimates_glob = Join-Path $criterionRoot "*double_ma_followup_gpu_kernel_50k*drilldown_then_local_stability_cold*new*estimates.json"
      expected_seed_backend_route = "GpuKernel"
    },
    [ordered]@{
      id = "double_ma_gpu_followup_cached"
      bench_name = "double_ma_followup_workflow"
      bench_filter = "double_ma_followup/gpu_kernel/50k"
      bench_function = "drilldown_then_local_stability_cached"
      estimates_path = ""
      estimates_glob = Join-Path $criterionRoot "*double_ma_followup_gpu_kernel_50k*drilldown_then_local_stability_cached*new*estimates.json"
      expected_seed_backend_route = "GpuKernel"
    }
  )
}

function Convert-CriterionEstimateToCaseSummary {
  param([Parameter(Mandatory = $true)]$CaseSpec)

  $estimatesPath = Resolve-EstimatePath -ExactPath $CaseSpec.estimates_path -GlobPath $CaseSpec.estimates_glob
  if (!(Test-Path $estimatesPath)) {
    throw "Missing criterion estimates file for case '$($CaseSpec.id)': $estimatesPath"
  }
  $criterion = Read-JsonFile -Path $estimatesPath
  return [ordered]@{
    id = $CaseSpec.id
    schema_version = $SchemaVersion
    workflow_family = "conventional_double_ma_followup"
    timing_boundary = "Criterion"
    timing_boundary_note = "Criterion bench profile estimate for Double-MA candidate detail plus local-stability follow-up. The benchmark asserts the seed optimization route remains resident GpuKernel."
    build_profile = "bench"
    bench_name = $CaseSpec.bench_name
    bench_filter = $CaseSpec.bench_filter
    bench_function = $CaseSpec.bench_function
    estimates_json = $estimatesPath
    expected_seed_backend_route = $CaseSpec.expected_seed_backend_route
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

function Invoke-GpuFollowupBenchFilter {
  param(
    [Parameter(Mandatory = $true)][string]$BenchExePath,
    [Parameter(Mandatory = $true)][string]$Filter
  )

  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU follow-up benchmark launch"
  Write-Host "Running GPU follow-up benchmark ($Filter)..."
  Invoke-ProcessAndEcho -FilePath $BenchExePath -ArgumentList @(
    $Filter,
    "--bench"
  ) -FailureContext "GPU follow-up benchmark process failed" | Out-Null
  Wait-NoPerfProcesses -TimeoutSeconds 900 -Context "after GPU follow-up benchmark completion"
}

$cases = Get-GpuFollowupPerfCases
$gpuIdentity = Get-GpuIdentity

Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU follow-up perf gate start"
try {
  Invoke-CargoCommandAndEcho -ArgumentList @(
    "bench",
    "-p", "ta_app_core",
    "--bench", "double_ma_followup_workflow",
    "--features", "cuda,cuda-backtest-kernel",
    "--no-run"
  ) -FailureContext "Failed to build GPU follow-up benchmark executable"

  $benchExe = Find-LatestBenchExecutable -benchName "double_ma_followup_workflow"
  $filters = @($cases | ForEach-Object { $_.bench_filter } | Sort-Object -Unique)
  foreach ($filter in $filters) {
    Invoke-GpuFollowupBenchFilter -BenchExePath $benchExe.FullName -Filter $filter
  }
} finally {
  Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "after GPU follow-up perf gate work"
}

$caseSummaries = @()
foreach ($case in $cases) {
  $caseSummaries += ,(Convert-CriterionEstimateToCaseSummary -CaseSpec $case)
}

$summary = [ordered]@{
  schema_version = $SchemaVersion
  case_registry_version = $CaseRegistryVersion
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  environment = Get-EnvironmentFingerprint
  gpu = $gpuIdentity.raw
  gpu_info = $gpuIdentity
  criterion_cases = $caseSummaries
}

Write-JsonUtf8 -Path $OutPath -Value $summary
Write-Host "GPU follow-up perf summary written to $OutPath"

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
  Write-Host "GPU follow-up perf baseline written to $BaselinePath"
  exit 0
}

if (!(Test-Path $BaselinePath)) {
  throw "Missing GPU follow-up perf baseline: $BaselinePath. Run with -Mode baseline first."
}

$baseline = Read-JsonFile -Path $BaselinePath
Assert-CompatibilityEqual -Name "gpu_followup.schema_version" -Candidate $summary.schema_version -Baseline $baseline.schema_version
Assert-CompatibilityEqual -Name "gpu_followup.case_registry_version" -Candidate $summary.case_registry_version -Baseline $baseline.case_registry_version

if (
  $summary.gpu_info.PSObject.Properties.Name -contains "name" -and
  $baseline.gpu_info -and
  $baseline.gpu_info.PSObject.Properties.Name -contains "name" -and
  -not [string]::IsNullOrWhiteSpace($summary.gpu_info.name) -and
  -not [string]::IsNullOrWhiteSpace($baseline.gpu_info.name)
) {
  Assert-CompatibilityEqual -Name "gpu_info.name" -Candidate $summary.gpu_info.name -Baseline $baseline.gpu_info.name
}

$budget = 1.0 + ($RegressionPct / 100.0)
$allPass = $true
foreach ($case in $summary.criterion_cases) {
  $baseCase = Get-CaseById -Cases $baseline.criterion_cases -Id $case.id
  if ($null -eq $baseCase) {
    throw "Missing baseline criterion case '$($case.id)'"
  }
  Assert-CompatibilityEqual -Name "$($case.id).schema_version" -Candidate $case.schema_version -Baseline $baseCase.schema_version
  Assert-CompatibilityEqual -Name "$($case.id).timing_boundary" -Candidate $case.timing_boundary -Baseline $baseCase.timing_boundary
  Assert-CompatibilityEqual -Name "$($case.id).build_profile" -Candidate $case.build_profile -Baseline $baseCase.build_profile
  Assert-CompatibilityEqual -Name "$($case.id).bench_name" -Candidate $case.bench_name -Baseline $baseCase.bench_name
  Assert-CompatibilityEqual -Name "$($case.id).bench_filter" -Candidate $case.bench_filter -Baseline $baseCase.bench_filter
  Assert-CompatibilityEqual -Name "$($case.id).bench_function" -Candidate $case.bench_function -Baseline $baseCase.bench_function
  Assert-CompatibilityEqual -Name "$($case.id).expected_seed_backend_route" -Candidate $case.expected_seed_backend_route -Baseline $baseCase.expected_seed_backend_route

  $candMean = [double]$case.mean_ms
  $baseMean = [double]$baseCase.mean_ms
  $casePass = $candMean -le ($baseMean * $budget)
  if (-not $casePass) {
    $allPass = $false
  }
  Write-PassFailLine "gpu_followup_criterion_case_$($case.id)_not_regressed_${RegressionPct}pct" $casePass "$candMean ms vs baseline $baseMean ms"
}

Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "before GPU follow-up perf gate exit"

if (-not $allPass) {
  throw "GPU follow-up performance gate failed"
}
