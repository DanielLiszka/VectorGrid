param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselinePath = "scripts/baselines/gpu_validation_perf_baseline.json",
  [string]$OutPath = "output/benchmarks/gpu_validation_perf_summary.json",
  [ValidateSet("release-authority", "validation-heavy")]
  [string]$WorkloadProfile = "release-authority",
  [double]$RegressionPct = 15.0,
  [double]$StructuredRegressionPct = 20.0,
  [switch]$SkipCriterion,
  [switch]$SkipStructuredProbe,
  [int]$StructuredProbeBars = 50000,
  [int]$StructuredProbeRepeats = 1,
  [string]$StructuredProbeCsv = "",
  [string]$CaseId = "",
  [string]$BenchName = "",
  [string]$BenchFilter = "",
  [string]$CriterionEstimatesPath = "",
  [string]$CriterionEstimatesGlob = "",
  [switch]$Quick,
  [switch]$ListCases,
  [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")

$SchemaVersion = 1
$CaseRegistryVersion = 1
$DefaultBaselinePath = "scripts/baselines/gpu_validation_perf_baseline.json"
$DefaultOutPath = "output/benchmarks/gpu_validation_perf_summary.json"
$OutPathWasDefault = $OutPath -eq $DefaultOutPath
$script:CompletedPerfProcessStartTicksById = @{}
$WorkloadProfileSlug = $WorkloadProfile.Replace("-", "_")
$StructuredProbeOutPath = "output/benchmarks/gpu_validation_${WorkloadProfileSlug}_perf_structured_probe.json"

function Get-ComparablePath {
  param([Parameter(Mandatory = $true)][string]$Path)

  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
}

if ($Quick) {
  $hasCustomCase = -not [string]::IsNullOrWhiteSpace($BenchFilter) `
    -or -not [string]::IsNullOrWhiteSpace($CriterionEstimatesPath) `
    -or -not [string]::IsNullOrWhiteSpace($CriterionEstimatesGlob) `
    -or -not [string]::IsNullOrWhiteSpace($BenchName)
  if ([string]::IsNullOrWhiteSpace($CaseId) -and -not $hasCustomCase) {
    $CaseId = "double_ma_gpu_validation_sparse"
  }
  $SkipStructuredProbe = $true
}

if ($WorkloadProfile -eq "validation-heavy") {
  if ($BaselinePath -eq $DefaultBaselinePath) {
    $BaselinePath = "scripts/baselines/gpu_validation_heavy_perf_baseline.json"
  }
}

if ($OutPathWasDefault) {
  if ($PlanOnly) {
    if ($Quick -and $WorkloadProfile -eq "validation-heavy") {
      $OutPath = "output/benchmarks/gpu_validation_heavy_quick_perf_plan.json"
    } elseif ($Quick) {
      $OutPath = "output/benchmarks/gpu_validation_quick_perf_plan.json"
    } elseif ($WorkloadProfile -eq "validation-heavy") {
      $OutPath = "output/benchmarks/gpu_validation_heavy_perf_plan.json"
    } else {
      $OutPath = "output/benchmarks/gpu_validation_perf_plan.json"
    }
  } elseif ($Quick) {
    if ($WorkloadProfile -eq "validation-heavy") {
      $OutPath = "output/benchmarks/gpu_validation_heavy_quick_perf_summary.json"
    } else {
      $OutPath = "output/benchmarks/gpu_validation_quick_perf_summary.json"
    }
  } elseif ($WorkloadProfile -eq "validation-heavy") {
    $OutPath = "output/benchmarks/gpu_validation_heavy_perf_summary.json"
  }
}

$WritesCandidateOutput = $Mode -eq "candidate" -and ((-not [bool]$ListCases) -or [bool]$PlanOnly)
if ($WritesCandidateOutput -and (Get-ComparablePath -Path $BaselinePath) -eq (Get-ComparablePath -Path $OutPath)) {
  throw "Candidate mode cannot use the same file for -BaselinePath and -OutPath: $OutPath"
}

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value,
    [int]$Depth = 24
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

function Get-ProcessStartTicks {
  param([Parameter(Mandatory = $true)]$Process)

  try {
    return [int64]$Process.StartTime.ToUniversalTime().Ticks
  } catch {
    return $null
  }
}

function Get-CimProcessStartTicks {
  param($CreationDate)

  if ($null -eq $CreationDate) {
    return $null
  }

  try {
    return [int64]([System.Management.ManagementDateTimeConverter]::ToDateTime($CreationDate).ToUniversalTime().Ticks)
  } catch {
    return $null
  }
}

function Get-ProcessCommandLine {
  param([Parameter(Mandatory = $true)][int]$ProcessId)

  try {
    return "$((Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction Stop).CommandLine)"
  } catch {
    return ""
  }
}

function Test-BenchmarkCommandLine {
  param([string]$CommandLine)

  if ([string]::IsNullOrWhiteSpace($CommandLine)) {
    return $false
  }
  if ($CommandLine -match '(^|[ \t])--no-run([ \t]|$)') {
    return $false
  }
  return $CommandLine -match '(^|[ \t])bench([ \t]|$)' `
    -or $CommandLine -match '(^|[ \t])--bench([ \t]|$)' `
    -or $CommandLine -match 'criterion'
}

function Test-KnownCompletedPerfProcess {
  param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    $StartTicks
  )

  $key = "$ProcessId"
  if (-not $script:CompletedPerfProcessStartTicksById.ContainsKey($key)) {
    return $false
  }
  if ($null -eq $StartTicks) {
    return $false
  }
  return [int64]$script:CompletedPerfProcessStartTicksById[$key] -eq [int64]$StartTicks
}

function Register-CompletedPerfProcess {
  param($Process)

  if ($null -eq $Process) {
    return
  }

  try {
    if ($Process.HasExited) {
      $startTicks = Get-ProcessStartTicks -Process $Process
      if ($null -ne $startTicks) {
        $script:CompletedPerfProcessStartTicksById["$($Process.Id)"] = $startTicks
      }
    }
  } catch {
  } finally {
    try {
      $Process.Dispose()
    } catch {
    }
  }
}

function Get-PerfProcesses() {
  $byId = [ordered]@{}

  foreach ($proc in @(Get-Process | Where-Object {
        $_.ProcessName -match '^(cargo|rustup|criterion|nsys|ncu|compute-sanitizer)$' `
          -or $_.ProcessName -like 'double_ma_validation_workflow*' `
          -or $_.ProcessName -like 'composite_validation_workflow*' `
          -or $_.ProcessName -like 'ta_double_ma_validation_cost*'
      })) {
    $commandLine = Get-ProcessCommandLine -ProcessId $proc.Id
    if ($proc.ProcessName -match '^(cargo|rustup)$' -and -not (Test-BenchmarkCommandLine -CommandLine $commandLine)) {
      continue
    }
    $startTicks = Get-ProcessStartTicks -Process $proc
    if (-not (Test-KnownCompletedPerfProcess -ProcessId $proc.Id -StartTicks $startTicks)) {
      $byId["$($proc.Id)"] = [pscustomobject]@{
        ProcessName = $proc.ProcessName
        Id = $proc.Id
        CPU = $proc.CPU
        StartTime = $proc.StartTime
        CommandLine = $commandLine
      }
    }
  }

  foreach ($proc in @(Get-CimInstance Win32_Process | Where-Object {
        $name = "$($_.Name)"
        $commandLine = "$($_.CommandLine)"
        $isShell = $name -match '^(pwsh|powershell|cmd|conhost|WindowsTerminal)\.exe$'
        -not $isShell -and (Test-BenchmarkCommandLine -CommandLine $commandLine)
      })) {
    $startTicks = Get-CimProcessStartTicks -CreationDate $proc.CreationDate
    if (-not (Test-KnownCompletedPerfProcess -ProcessId $proc.ProcessId -StartTicks $startTicks)) {
      $processName = [System.IO.Path]::GetFileNameWithoutExtension($proc.Name)
      $startTime = $null
      if ($null -ne $startTicks) {
        $startTime = ([DateTime]::new($startTicks, [DateTimeKind]::Utc)).ToLocalTime()
      }
      $byId["$($proc.ProcessId)"] = [pscustomobject]@{
        ProcessName = $processName
        Id = $proc.ProcessId
        CPU = $null
        StartTime = $startTime
        CommandLine = "$($proc.CommandLine)"
      }
    }
  }

  @($byId.Values)
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
          $commandLine = "$($_.CommandLine)"
          if ($commandLine.Length -gt 180) {
            $commandLine = $commandLine.Substring(0, 180) + "..."
          }
          if ([string]::IsNullOrWhiteSpace($commandLine)) {
            "{0}#{1}" -f $_.ProcessName, $_.Id
          } else {
            "{0}#{1} [{2}]" -f $_.ProcessName, $_.Id, $commandLine
          }
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
  $proc = $null
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
    $exitCode = $proc.ExitCode
    Register-CompletedPerfProcess -Process $proc
    $proc = $null
    if ($exitCode -ne 0) {
      throw "${FailureContext} (cargo exit code $exitCode)"
    }
  } finally {
    Register-CompletedPerfProcess -Process $proc
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
  $proc = $null
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
    $exitCode = $proc.ExitCode
    Register-CompletedPerfProcess -Process $proc
    $proc = $null
    if ($exitCode -ne 0) {
      throw "${FailureContext} ($FilePath exit code $exitCode)"
    }
    return $output
  } finally {
    Register-CompletedPerfProcess -Process $proc
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

function Convert-CriterionEstimateToCaseSummary {
  param(
    [Parameter(Mandatory = $true)]$CaseSpec
  )

  $estimatesPath = Resolve-EstimatePath -ExactPath $CaseSpec.estimates_path -GlobPath $CaseSpec.estimates_glob
  if (!(Test-Path $estimatesPath)) {
    throw "Missing criterion estimates file for case '$($CaseSpec.id)': $estimatesPath"
  }
  $criterion = Read-JsonFile -Path $estimatesPath
  return [ordered]@{
    id = $CaseSpec.id
    schema_version = $SchemaVersion
    timing_boundary = "Criterion"
    timing_boundary_note = "Criterion bench profile estimate for a deterministic GPU validation workflow case."
    build_profile = "bench"
    workload_profile = $WorkloadProfile
    workload_class = $CaseSpec.workload_class
    evidence_role = $CaseSpec.evidence_role
    bench_name = $CaseSpec.bench_name
    bench_filter = $CaseSpec.bench_filter
    estimates_json = $estimatesPath
    expected_backend_route = $CaseSpec.expected_backend_route
    bars = $CaseSpec.bars
    candidate_count = $CaseSpec.candidate_count
    validation_modules_gpu_native = $CaseSpec.validation_modules_gpu_native
    validation_modules_cpu_tail = $CaseSpec.validation_modules_cpu_tail
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

function Get-ValidationCaseByLabel {
  param(
    [Parameter(Mandatory = $true)]$Cases,
    [Parameter(Mandatory = $true)][string]$Label
  )
  return @($Cases | Where-Object { $_.label -eq $Label } | Select-Object -First 1)[0]
}

function Get-GpuValidationPerfCases {
  $hasCustomCase = -not [string]::IsNullOrWhiteSpace($BenchFilter) `
    -or -not [string]::IsNullOrWhiteSpace($CriterionEstimatesPath) `
    -or -not [string]::IsNullOrWhiteSpace($CriterionEstimatesGlob) `
    -or -not [string]::IsNullOrWhiteSpace($BenchName)

  if ($hasCustomCase) {
    if ([string]::IsNullOrWhiteSpace($BenchFilter) -or [string]::IsNullOrWhiteSpace($BenchName)) {
      throw "Custom GPU validation gate runs require -BenchName and -BenchFilter."
    }
    return @(
      [ordered]@{
        id = $(if ([string]::IsNullOrWhiteSpace($CaseId)) { "custom_gpu_validation_case" } else { $CaseId })
        bench_name = $BenchName
        bench_filter = $BenchFilter
        estimates_path = $CriterionEstimatesPath
        estimates_glob = $CriterionEstimatesGlob
        expected_backend_route = "GpuKernel"
        workload_class = "custom"
        evidence_role = "custom GPU validation gate case"
        bars = $null
        candidate_count = $null
        validation_modules_gpu_native = @()
        validation_modules_cpu_tail = @()
      }
    )
  }

  $criterionRoot = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) "target/criterion"))
  $releaseAuthorityCases = @(
    [ordered]@{
      id = "double_ma_gpu_validation_sparse"
      bench_name = "double_ma_validation_workflow"
      bench_filter = "double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates/wf_reopt_cscv_spa"
      estimates_path = ""
      estimates_glob = Join-Path $criterionRoot "*double_ma_validate_gpu_kernel_sparse_finalists_100k_8candidates*wf_reopt_cscv_spa*new*estimates.json"
      expected_backend_route = "GpuKernel"
      workload_class = "validation-heavy"
      evidence_role = "Double-MA sparse-finalist GPU validation route and module timing"
      bars = 100000
      candidate_count = 8
      validation_modules_gpu_native = @("in_sample", "walk_forward", "reopt_walk_forward", "cscv_pbo", "spa")
      validation_modules_cpu_tail = @("perturb", "trade_bootstrap")
    },
    [ordered]@{
      id = "composite_gpu_validation_asr_wf_cscv"
      bench_name = "composite_validation_workflow"
      bench_filter = "composite_validate/gpu_kernel/asr/100k/64candidates/wf_cscv"
      estimates_path = ""
      estimates_glob = Join-Path $criterionRoot "*composite_validate_gpu_kernel_asr_100k_64candidates*wf_cscv*new*estimates.json"
      expected_backend_route = "GpuKernel"
      workload_class = "route-authority"
      evidence_role = "ASR composite GPU validation route and walk-forward/CSCV timing"
      bars = 100000
      candidate_count = 64
      validation_modules_gpu_native = @("in_sample", "walk_forward", "cscv_pbo")
      validation_modules_cpu_tail = @()
    },
    [ordered]@{
      id = "composite_gpu_validation_vcet_wf_reopt_cscv_spa"
      bench_name = "composite_validation_workflow"
      bench_filter = "composite_validate/gpu_kernel/vcet/100000/64candidates/wf_reopt_cscv_spa"
      estimates_path = ""
      estimates_glob = Join-Path $criterionRoot "*composite_validate_gpu_kernel_vcet_100000_64candidates*wf_reopt_cscv_spa*new*estimates.json"
      expected_backend_route = "GpuKernel"
      workload_class = "validation-heavy"
      evidence_role = "VCET composite GPU validation heavy module timing"
      bars = 100000
      candidate_count = 64
      validation_modules_gpu_native = @("in_sample", "walk_forward", "reopt_walk_forward", "cscv_pbo", "spa")
      validation_modules_cpu_tail = @()
    }
  )

  $cases = $releaseAuthorityCases
  if ($WorkloadProfile -eq "validation-heavy") {
    $cases = @(
      $releaseAuthorityCases[0],
      $releaseAuthorityCases[2],
      [ordered]@{
        id = "composite_gpu_validation_cdmc_wf_reopt_cscv_spa"
        bench_name = "composite_validation_workflow"
        bench_filter = "composite_validate/gpu_kernel/cdmc/100000/32candidates/wf_reopt_cscv_spa"
        estimates_path = ""
        estimates_glob = Join-Path $criterionRoot "*composite_validate_gpu_kernel_cdmc_100000_32candidates*wf_reopt_cscv_spa*new*estimates.json"
        expected_backend_route = "GpuKernel"
        workload_class = "validation-heavy"
        evidence_role = "CDMC composite GPU validation heavy module timing"
        bars = 100000
        candidate_count = 32
        validation_modules_gpu_native = @("in_sample", "walk_forward", "reopt_walk_forward", "cscv_pbo", "spa")
        validation_modules_cpu_tail = @()
      }
    )
  }

  if (-not [string]::IsNullOrWhiteSpace($CaseId)) {
    $selected = @($cases | Where-Object { $_.id -eq $CaseId })
    if ($selected.Count -eq 0) {
      $available = ($cases | ForEach-Object { $_.id }) -join ", "
      throw "Unknown GPU validation case '$CaseId' for workload profile '$WorkloadProfile'. Available cases: $available"
    }
    return $selected
  }

  return $cases
}

function New-GpuValidationPerfPlan {
  param([Parameter(Mandatory = $true)]$Cases)

  return [ordered]@{
    schema_version = $SchemaVersion
    case_registry_version = $CaseRegistryVersion
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    mode = $Mode
    gate = "gpu_validation_perf"
    workload_profile = $WorkloadProfile
    quick_mode = [bool]$Quick
    criterion_enabled = -not [bool]$SkipCriterion
    structured_probe_enabled = -not [bool]$SkipStructuredProbe
    structured_probe = if (-not $SkipStructuredProbe) {
      [ordered]@{
        workflow_family = "conventional_double_ma_validation"
        out_path = $StructuredProbeOutPath
        bars = $StructuredProbeBars
        repeats = $StructuredProbeRepeats
        csv_path = $StructuredProbeCsv
        backend = "gpu-kernel"
        validates_route = "GpuKernel"
      }
    } else {
      $null
    }
    criterion_cases = $Cases
    baseline_path = $BaselinePath
    out_path = $OutPath
    regression_pct = $RegressionPct
    structured_regression_pct = $StructuredRegressionPct
  }
}

function Write-GpuValidationPerfPlan {
  param([Parameter(Mandatory = $true)]$Plan)

  Write-Host "GPU validation perf gate plan:"
  Write-Host "  mode=$($Plan.mode)"
  Write-Host "  workload_profile=$($Plan.workload_profile)"
  Write-Host "  quick_mode=$($Plan.quick_mode)"
  Write-Host "  criterion_enabled=$($Plan.criterion_enabled)"
  foreach ($case in $Plan.criterion_cases) {
    $modules = if ($case.validation_modules_gpu_native.Count -gt 0) { $case.validation_modules_gpu_native -join "+" } else { "unspecified" }
    $cpuTails = if ($case.validation_modules_cpu_tail.Count -gt 0) { $case.validation_modules_cpu_tail -join "+" } else { "none" }
    Write-Host "  case=$($case.id) bench=$($case.bench_name) filter=$($case.bench_filter) expected_route=$($case.expected_backend_route) gpu_native=$modules cpu_tail=$cpuTails"
  }
  Write-Host "  structured_probe_enabled=$($Plan.structured_probe_enabled)"
  if ($Plan.structured_probe_enabled) {
    Write-Host "  structured_probe=DoubleMa ${StructuredProbeBars} bars, ${StructuredProbeRepeats} repeat(s), expected_route=GpuKernel"
  }
}

function Invoke-GpuValidationBenchCase {
  param(
    [Parameter(Mandatory = $true)][string]$BenchExePath,
    [Parameter(Mandatory = $true)]$CaseSpec
  )

  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU validation benchmark launch"
  Write-Host "Running GPU validation benchmark ($($CaseSpec.bench_filter))..."
  $previousExactRoute = [Environment]::GetEnvironmentVariable("VTA_DOUBLE_MA_VALIDATE_REQUIRE_EXACT_ROUTE", "Process")
  try {
    if ($CaseSpec.bench_name -eq "double_ma_validation_workflow") {
      $env:VTA_DOUBLE_MA_VALIDATE_REQUIRE_EXACT_ROUTE = "1"
    }
    Invoke-ProcessAndEcho -FilePath $BenchExePath -ArgumentList @(
      $CaseSpec.bench_filter,
      "--bench"
    ) -FailureContext "GPU validation benchmark process failed" | Out-Null
  } finally {
    if ([string]::IsNullOrWhiteSpace($previousExactRoute)) {
      Remove-Item Env:VTA_DOUBLE_MA_VALIDATE_REQUIRE_EXACT_ROUTE -ErrorAction SilentlyContinue
    } else {
      $env:VTA_DOUBLE_MA_VALIDATE_REQUIRE_EXACT_ROUTE = $previousExactRoute
    }
  }
  Wait-NoPerfProcesses -TimeoutSeconds 900 -Context "after GPU validation benchmark completion"

  $estimatesPath = Resolve-EstimatePath -ExactPath $CaseSpec.estimates_path -GlobPath $CaseSpec.estimates_glob
  if (!(Test-Path $estimatesPath)) {
    throw "Expected criterion estimates were not generated for case '$($CaseSpec.id)': $estimatesPath"
  }
}

function Write-SyntheticCandlesCsv {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][int]$Bars
  )

  if ($Bars -lt 1000) {
    throw "StructuredProbeBars must be at least 1000"
  }

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }

  $culture = [System.Globalization.CultureInfo]::InvariantCulture
  $writer = [System.IO.StreamWriter]::new($Path, $false, [System.Text.Encoding]::UTF8)
  try {
    $writer.WriteLine("timestamp,open,high,low,close,volume")
    $px = 100.0
    $baseTs = [int64]1700000000000
    for ($i = 0; $i -lt $Bars; $i++) {
      $drift = 0.00004
      $noise = [Math]::Sin([double]$i * 0.019) * 0.0012
      $px = $px * (1.0 + $drift + $noise)
      $ts = $baseTs + ([int64]$i * 60000)
      $open = $px
      $high = $px * 1.001
      $low = $px * 0.999
      $close = $px
      $volume = 1000.0
      $writer.WriteLine(("{0},{1},{2},{3},{4},{5}" -f `
            $ts,
            $open.ToString("R", $culture),
            $high.ToString("R", $culture),
            $low.ToString("R", $culture),
            $close.ToString("R", $culture),
            $volume.ToString("R", $culture)))
    }
  } finally {
    $writer.Dispose()
  }
}

function Invoke-StructuredDoubleMaValidationProbe {
  param(
    [Parameter(Mandatory = $true)][string]$OutJson
  )

  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before structured validation probe"
  Invoke-CargoCommandAndEcho -ArgumentList @(
    "build",
    "-q",
    "-p", "ta_desktop_demo_app",
    "--bin", "ta_double_ma_validation_cost",
    "--release",
    "--features", "cuda,cuda-backtest-kernel"
  ) -FailureContext "Failed to build ta_double_ma_validation_cost"

  $csvPath = $StructuredProbeCsv
  if ([string]::IsNullOrWhiteSpace($csvPath)) {
    $csvPath = "output/benchmarks/gpu_validation_probe/synthetic_${StructuredProbeBars}.csv"
    Write-SyntheticCandlesCsv -Path $csvPath -Bars $StructuredProbeBars
  }

  $probeExe = Join-Path (Resolve-Path "target/release").Path "ta_double_ma_validation_cost.exe"
  Invoke-ProcessAndEcho -FilePath $probeExe -ArgumentList @(
    "--csv", $csvPath,
    "--backend", "gpu-kernel",
    "--fast-ma", "sma",
    "--slow-ma", "sma",
    "--fast-period", "7:43:4",
    "--slow-period", "41:173:12",
    "--optimize-top-k", "16",
    "--validate-max-candidates", "8",
    "--repeats", "$StructuredProbeRepeats",
    "--out-json", $OutJson
  ) -FailureContext "Structured Double-MA validation probe failed" | Out-Null
  Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "after structured validation probe"

  if (!(Test-Path $OutJson)) {
    throw "Structured Double-MA validation probe did not write expected JSON: $OutJson"
  }
  return Read-JsonFile -Path $OutJson
}

function Test-StructuredProbeRoutes {
  param([Parameter(Mandatory = $true)]$Probe)

  $ok = $true
  if ($null -eq $Probe.optimization) {
    throw "Structured probe is missing optimization summary"
  }
  $optRoute = "$($Probe.optimization.backend_route)"
  $optResidency = $Probe.optimization.residency
  $optResidencyOk = $true
  $optMessages = @()
  if ($optRoute -eq "GpuKernel") {
    if ($null -eq $optResidency) {
      $optResidencyOk = $false
      $optMessages += "missing_residency"
    } else {
      if ($optResidency.fully_gpu_resident -ne $true) {
        $optResidencyOk = $false
        $optMessages += "fully_gpu_resident=$($optResidency.fully_gpu_resident)"
      }
      if ($optResidency.cpu_fallback_used -eq $true) {
        $optResidencyOk = $false
        $optMessages += "cpu_fallback_used=true"
      }
      if ($optResidency.cpu_staged_indicators -eq $true) {
        $optResidencyOk = $false
        $optMessages += "cpu_staged_indicators=true"
      }
    }
  }
  if ($Probe.optimization.backend_fallback_reason) {
    $optResidencyOk = $false
    $optMessages += "optimization_fallback=$($Probe.optimization.backend_fallback_reason)"
  }
  if (-not $optResidencyOk) {
    $ok = $false
  }
  $optMessageText = if ($optMessages.Count -eq 0) { "route=$optRoute residency=ok" } else { $optMessages -join ", " }
  Write-PassFailLine "structured_probe_optimization_residency" $optResidencyOk $optMessageText

  if ($null -eq $Probe.validation_cases) {
    throw "Structured probe is missing validation_cases"
  }
  foreach ($case in $Probe.validation_cases) {
    $caseOk = $true
    $messages = @()
    if ("$($case.status)" -ne "ok") {
      $caseOk = $false
      $messages += "status=$($case.status)"
    }
    if ("$($case.backend_used)" -ne "Gpu") {
      $caseOk = $false
      $messages += "backend_used=$($case.backend_used)"
    }
    if ("$($case.backend_route)" -ne "GpuKernel") {
      $caseOk = $false
      $messages += "backend_route=$($case.backend_route)"
    }
    if ($null -ne $case.backend_fallback_reason -and -not [string]::IsNullOrWhiteSpace("$($case.backend_fallback_reason)")) {
      $caseOk = $false
      $messages += "fallback=$($case.backend_fallback_reason)"
    }
    if (-not $caseOk) {
      $ok = $false
    }
    $messageText = if ($messages.Count -eq 0) { "route=$($case.backend_route)" } else { $messages -join ", " }
    Write-PassFailLine "structured_probe_route_$($case.label)" $caseOk $messageText
  }
  return $ok
}

function Compare-StructuredProbeToBaseline {
  param(
    [Parameter(Mandatory = $true)]$Candidate,
    [Parameter(Mandatory = $true)]$Baseline
  )

  $budget = 1.0 + ($StructuredRegressionPct / 100.0)
  $ok = $true
  foreach ($case in $Candidate.validation_cases) {
    $baseCase = Get-ValidationCaseByLabel -Cases $Baseline.validation_cases -Label $case.label
    if ($null -eq $baseCase) {
      throw "Missing structured probe baseline case '$($case.label)'"
    }
    $candWall = [double]$case.avg_wall_ms
    $baseWall = [double]$baseCase.avg_wall_ms
    $casePass = $candWall -le ($baseWall * $budget)
    if (-not $casePass) {
      $ok = $false
    }
    Write-PassFailLine "structured_probe_$($case.label)_not_regressed_${StructuredRegressionPct}pct" $casePass "$candWall ms vs baseline $baseWall ms"
  }
  return $ok
}

$criterionCases = @()
if (-not $SkipCriterion) {
  $criterionCases = Get-GpuValidationPerfCases
}

if ($ListCases -or $PlanOnly) {
  $plan = New-GpuValidationPerfPlan -Cases $criterionCases
  Write-GpuValidationPerfPlan -Plan $plan
  if ($PlanOnly) {
    Write-JsonUtf8 -Path $OutPath -Value $plan
    Write-Host "GPU validation perf plan written to $OutPath"
  }
  exit 0
}

$gpuIdentity = Get-GpuIdentity
$structuredProbe = $null

$benchmarkLock = Enter-BenchmarkLock -Context "GPU validation perf gate"
try {
  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU validation perf gate start"
  try {
    if (-not $SkipCriterion) {
      $benchNames = @($criterionCases | ForEach-Object { $_.bench_name } | Sort-Object -Unique)
      $benchExecutables = @{}
      foreach ($benchNameItem in $benchNames) {
        Invoke-CargoCommandAndEcho -ArgumentList @(
          "bench",
          "-p", "ta_app_core",
          "--bench", "$benchNameItem",
          "--features", "cuda,cuda-backtest-kernel",
          "--no-run"
        ) -FailureContext "Failed to build GPU validation benchmark executable '$benchNameItem'"
        $benchExecutables[$benchNameItem] = (Find-LatestBenchExecutable -benchName $benchNameItem).FullName
      }

      foreach ($case in $criterionCases) {
        Invoke-GpuValidationBenchCase -BenchExePath $benchExecutables[$case.bench_name] -CaseSpec $case
      }
    }

    if (-not $SkipStructuredProbe) {
      $structuredProbe = Invoke-StructuredDoubleMaValidationProbe -OutJson $StructuredProbeOutPath
    }
  } finally {
    Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "after GPU validation perf gate work"
  }
} finally {
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
  quick_mode = [bool]$Quick
  baseline_path = $BaselinePath
  out_path = $OutPath
  environment = Get-EnvironmentFingerprint
  gpu = $gpuIdentity.raw
  gpu_info = $gpuIdentity
  criterion = if ($criterionCaseSummaries.Count -gt 0) { $criterionCaseSummaries[0] } else { $null }
  criterion_cases = $criterionCaseSummaries
  structured_probe_path = if ($null -ne $structuredProbe) { $StructuredProbeOutPath } else { $null }
  structured_probe = $structuredProbe
}

Write-JsonUtf8 -Path $OutPath -Value $summary
Write-Host "GPU validation perf summary written to $OutPath"

foreach ($case in $summary.criterion_cases) {
  $ciWidthPct = 0.0
  if ([double]$case.mean_ms -gt 0.0) {
    $ciWidthPct = (([double]$case.mean_ci_upper_ms - [double]$case.mean_ci_lower_ms) / [double]$case.mean_ms) * 100.0
  }
  if ($ciWidthPct -gt 10.0) {
    Write-Host "[WARN] High run variance detected for $($case.id) (mean CI width = $([math]::Round($ciWidthPct, 2))%). Consider stabilizing GPU clocks/load before trusting this baseline."
  }
}

$structuredRoutePass = $true
if ($null -ne $summary.structured_probe) {
  $structuredRoutePass = Test-StructuredProbeRoutes -Probe $summary.structured_probe
}

if ($Mode -eq "baseline") {
  Write-JsonUtf8 -Path $BaselinePath -Value $summary
  Write-Host "GPU validation perf baseline written to $BaselinePath"
  if (-not $structuredRoutePass) {
    throw "Structured probe route checks failed during baseline capture"
  }
  exit 0
}

if (!(Test-Path $BaselinePath)) {
  throw "Missing GPU validation perf baseline: $BaselinePath. Run with -Mode baseline first."
}

$baseline = Read-JsonFile -Path $BaselinePath
Assert-CompatibilityEqual -Name "gpu_validation.schema_version" -Candidate $summary.schema_version -Baseline $baseline.schema_version
Assert-CompatibilityEqual -Name "gpu_validation.case_registry_version" -Candidate $summary.case_registry_version -Baseline $baseline.case_registry_version
if ($baseline.PSObject.Properties.Name -contains "workload_profile") {
  Assert-CompatibilityEqual -Name "gpu_validation.workload_profile" -Candidate $summary.workload_profile -Baseline $baseline.workload_profile
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
  Assert-CompatibilityEqual -Name "$($case.id).bench_name" -Candidate $case.bench_name -Baseline $baseCase.bench_name
  Assert-CompatibilityEqual -Name "$($case.id).bench_filter" -Candidate $case.bench_filter -Baseline $baseCase.bench_filter
  Assert-CompatibilityEqual -Name "$($case.id).expected_backend_route" -Candidate $case.expected_backend_route -Baseline $baseCase.expected_backend_route

  $candMean = [double]$case.mean_ms
  $baseMean = [double]$baseCase.mean_ms
  $casePass = $candMean -le ($baseMean * $criterionBudget)
  if (-not $casePass) {
    $allCriterionPass = $false
  }
  Write-PassFailLine "gpu_validation_criterion_case_$($case.id)_not_regressed_${RegressionPct}pct" $casePass "$candMean ms vs baseline $baseMean ms"
}

$structuredPerfPass = $true
if ($null -ne $summary.structured_probe) {
  if ($null -eq $baseline.structured_probe) {
    throw "Candidate summary has structured_probe but baseline does not."
  }
  $structuredPerfPass = Compare-StructuredProbeToBaseline -Candidate $summary.structured_probe -Baseline $baseline.structured_probe
}

Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "before GPU validation perf gate exit"

if (!($allCriterionPass -and $structuredRoutePass -and $structuredPerfPass)) {
  throw "GPU validation performance gate failed"
}
