param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$ArtifactPath,
  [string]$DatasetId,
  [string]$ImportCsv,
  [string]$ImportRootSymbol,
  [string]$AppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OutDir = "output/options_expression_phase05_gpu_gate",
  [string]$BaselinePath = "scripts/baselines/options_expression_perf_baseline.json",
  [string]$SummaryOutPath = "output/benchmarks/options_expression_perf_summary.json",
  [ValidateSet("auto", "cpu_reference", "gpu_compiled_tensor")]
  [string]$BackendPreference = "auto",
  [ValidateSet("event_chronological", "session_chronological")]
  [string]$SplitAssignmentMode = "event_chronological",
  [int]$TopK = 100,
  [int]$SealedFinalistCount = 10,
  [int]$WarmRepeats = 1,
  [int]$BenchmarkIterations = 5,
  [string[]]$RequestArgs = @(),
  [double]$PreviewWallSpeedupGate = 1.0,
  [double]$RegressionPct = 20.0,
  [double]$PreviewWallRegressionAbsMs = 5.0,
  [double]$RankingRegressionAbsUs = 1000.0,
  [switch]$InspectTop
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")

function Resolve-RepoRoot {
  return (Split-Path -Parent $PSScriptRoot)
}

function Join-IfRelative {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Root
  )

  if ([System.IO.Path]::IsPathRooted($Path)) {
    return $Path
  }
  return Join-Path $Root $Path
}

function Format-Status([bool]$Passed) {
  if ($Passed) {
    return "PASS"
  }
  return "FAIL"
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
    throw "Incompatible options performance baseline for ${Name}: candidate='$Candidate' baseline='$Baseline'"
  }
}

function Write-PassFailLine([string]$Name, [bool]$Passed, [string]$Message) {
  Write-Host ("[{0}] {1}: {2}" -f (Format-Status $Passed), $Name, $Message)
}

function Invoke-CargoTestEvidence {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][string[]]$ArgumentList,
    [bool]$UsePrebuiltPtx = $false,
    [int]$MinimumPassedTests = 1
  )

  $startedAt = (Get-Date).ToUniversalTime()
  $previousPrebuiltPtx = $env:VECTORBT_USE_PREBUILT_PTX
  $passedTestCount = 0
  try {
    if ($UsePrebuiltPtx) {
      $env:VECTORBT_USE_PREBUILT_PTX = "1"
    }
    $outputLines = @(& cargo @ArgumentList 2>&1)
    $exitCode = $LASTEXITCODE
    foreach ($line in $outputLines) {
      Write-Host $line
      $testResultMatch = [regex]::Match(
        [string]$line,
        'test result:\s+ok\.\s+(\d+)\s+passed;'
      )
      if ($testResultMatch.Success) {
        $passedTestCount += [int]$testResultMatch.Groups[1].Value
      }
    }
    if ($exitCode -ne 0) {
      throw "$Name failed with cargo exit code $exitCode."
    }
    if ($passedTestCount -lt $MinimumPassedTests) {
      throw "$Name did not execute the required test count ($passedTestCount passed; minimum $MinimumPassedTests)."
    }
  } finally {
    if ($UsePrebuiltPtx) {
      if ($null -ne $previousPrebuiltPtx) {
        $env:VECTORBT_USE_PREBUILT_PTX = $previousPrebuiltPtx
      } else {
        Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
      }
    }
  }

  return [ordered]@{
    name = $Name
    command = "cargo " + ($ArgumentList -join " ")
    passed = $true
    passed_test_count = $passedTestCount
    started_at_utc = $startedAt.ToString("o")
    finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  }
}

function Get-OptionsPerfProcesses() {
  @(Get-Process | Where-Object {
      $_.ProcessName -match '^(cargo|criterion|nsys|ncu)$' -or
      $_.ProcessName -like 'ta_options_expression*'
    } | Select-Object ProcessName, Id, CPU, StartTime)
}

function Wait-NoOptionsPerfProcesses(
  [int]$TimeoutSeconds = 300,
  [int]$PollMilliseconds = 500,
  [string]$Context = ""
) {
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ($true) {
    $procs = @(Get-OptionsPerfProcesses)
    if ($procs.Count -eq 0) {
      return
    }
    if ((Get-Date) -ge $deadline) {
      $summary = ($procs | ForEach-Object {
          "{0}#{1}" -f $_.ProcessName, $_.Id
        }) -join ", "
      if ([string]::IsNullOrWhiteSpace($Context)) {
        throw "Timed out waiting for options benchmark/profiler processes to drain: $summary"
      }
      throw "Timed out waiting for options benchmark/profiler processes to drain ${Context}: $summary"
    }
    Start-Sleep -Milliseconds $PollMilliseconds
  }
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
  return [ordered]@{
    hostname = "$hostname"
    os = [System.Environment]::OSVersion.VersionString
    architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
    powershell_edition = $PSVersionTable.PSEdition
    powershell_version = $PSVersionTable.PSVersion.ToString()
  }
}

$repoRoot = Resolve-RepoRoot
$phase05Script = Join-Path $repoRoot "scripts\run_options_expression_phase05.ps1"
$resolvedOutRoot = Join-IfRelative -Path $OutDir -Root $repoRoot
$BaselinePath = Join-IfRelative -Path $BaselinePath -Root $repoRoot
$SummaryOutPath = Join-IfRelative -Path $SummaryOutPath -Root $repoRoot
if ($Mode -eq "candidate" -and -not (Test-Path -LiteralPath $BaselinePath -PathType Leaf)) {
  throw "Missing options expression performance baseline: $BaselinePath. Run with -Mode baseline first."
}
$timestamp = Get-Date -Format "yyyy-MM-ddTHH-mm-ss"
$runOutRoot = Join-Path $resolvedOutRoot $timestamp
$correctnessEvidence = @()

$phase05Args = @(
  "-ExecutionPolicy",
  "Bypass",
  "-File",
  $phase05Script,
  "-AppLocalDataDir",
  $AppLocalDataDir,
  "-OutDir",
  $runOutRoot,
  "-Cuda",
  "-BackendPreference",
  $BackendPreference,
  "-SplitAssignmentMode",
  $SplitAssignmentMode,
  "-TopK",
  [string][Math]::Max(1, $TopK),
  "-SealedFinalistCount",
  [string][Math]::Max(1, [Math]::Min($SealedFinalistCount, [Math]::Max(1, $TopK))),
  "-WarmRepeats",
  [string][Math]::Max(0, $WarmRepeats),
  "-BenchmarkIterations",
  [string][Math]::Max(1, $BenchmarkIterations),
  "-CompareBackends"
)

if ($ArtifactPath) {
  $phase05Args += @("-ArtifactPath", $ArtifactPath)
}
if ($DatasetId) {
  $phase05Args += @("-DatasetId", $DatasetId)
}
if ($ImportCsv) {
  $phase05Args += @("-ImportCsv", $ImportCsv)
}
if ($ImportRootSymbol) {
  $phase05Args += @("-ImportRootSymbol", $ImportRootSymbol)
}
if ($RequestArgs -and $RequestArgs.Count -gt 0) {
  $phase05Args += @("-RequestArgs") + $RequestArgs
}
if ($InspectTop) {
  $phase05Args += "-InspectTop"
}

Write-Host ("Options GPU promotion gate mode: {0}" -f $Mode)
$benchmarkLock = Enter-BenchmarkLock -Context "options GPU promotion gate"
$promotionExitCode = 0
try {
  Wait-NoOptionsPerfProcesses -TimeoutSeconds 120 -Context "before options GPU promotion gate start"

  Push-Location $repoRoot
  try {
    $previousPrebuiltPtx = $env:VECTORBT_USE_PREBUILT_PTX
    try {
      $env:VECTORBT_USE_PREBUILT_PTX = "1"
      & powershell @phase05Args
      $promotionExitCode = $LASTEXITCODE
    } finally {
      if ($null -ne $previousPrebuiltPtx) {
        $env:VECTORBT_USE_PREBUILT_PTX = $previousPrebuiltPtx
      } else {
        Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
      }
    }
  } finally {
    Pop-Location
  }

  Wait-NoOptionsPerfProcesses -TimeoutSeconds 300 -Context "after options GPU promotion phase completion"
} finally {
  Exit-BenchmarkLock -Handle $benchmarkLock
}
if ($promotionExitCode -ne 0) {
  exit $promotionExitCode
}

Push-Location $repoRoot
try {
  $correctnessEvidence += ,(Invoke-CargoTestEvidence `
    -Name "Options expression CPU test suite" `
    -ArgumentList @(
      "test",
      "-p", "ta_app_core",
      "--lib",
      "options_expression::tests::",
      "--",
      "--test-threads=1"
    ))
  $correctnessEvidence += ,(Invoke-CargoTestEvidence `
    -Name "Options expression GPU aggregate parity" `
    -UsePrebuiltPtx $true `
    -ArgumentList @(
      "test",
      "-p", "ta_app_core",
      "--lib",
      "--features", "cuda,cuda-backtest-kernel",
      "options_expression::tests::fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates",
      "--",
      "--exact",
      "--test-threads=1"
    ))
  $correctnessEvidence += ,(Invoke-CargoTestEvidence `
    -Name "Options expression CPU/GPU ranking parity" `
    -UsePrebuiltPtx $true `
    -ArgumentList @(
      "test",
      "-p", "ta_app_core",
      "--lib",
      "--features", "cuda,cuda-backtest-kernel",
      "options_expression::tests::gpu_combined_ranking_matches_cpu_topk_on_small_surface",
      "--",
      "--exact",
      "--test-threads=1"
    ))
} finally {
  Pop-Location
}

$summaryJson = Get-ChildItem $runOutRoot -Recurse -Filter "phase05_summary.json" -File |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1

if ($null -eq $summaryJson) {
  throw "Phase 0.5 summary JSON was not produced under $runOutRoot"
}

$summary = Get-Content -Raw $summaryJson.FullName | ConvertFrom-Json
$phase05 = $summary.phase05_gates
$comparison = $summary.backend_comparison
$report = $summary.response.report
$gpuIdentity = Get-GpuIdentity
$previewWallSpeedup = $null
if ($null -ne $comparison) {
  if ($null -ne $comparison.estimated_backend_preview_wall_speedup_vs_cpu) {
    $previewWallSpeedup = [double]$comparison.estimated_backend_preview_wall_speedup_vs_cpu
  } elseif ($null -ne $comparison.preview_wall_speedup_vs_cpu) {
    $previewWallSpeedup = [double]$comparison.preview_wall_speedup_vs_cpu
  }
}

$checks = @(
  [pscustomobject]@{
    Label = "Candidate density gate"
    Value = "{0:P1}" -f [double]$phase05.valid_slot_density_observed
    Detail = ">= {0:P1}" -f [double]$phase05.valid_slot_density_gate_min
    Passed = [bool]$phase05.valid_slot_density_pass
  },
  [pscustomobject]@{
    Label = "Invalid-slot gate"
    Value = "{0:P1}" -f [double]$phase05.invalid_slot_rate_observed
    Detail = "<= {0:P1}" -f [double]$phase05.invalid_slot_rate_gate_max
    Passed = [bool]$phase05.invalid_slot_rate_pass
  },
  [pscustomobject]@{
    Label = "Structural cache reuse gate"
    Value = if ($null -eq $phase05.structural_cache_reuse_observed) { "n/a" } else { "{0:P1}" -f [double]$phase05.structural_cache_reuse_observed }
    Detail = ">= {0:P1}" -f [double]$phase05.cache_reuse_gate_min
    Passed = [bool]($phase05.structural_cache_reuse_pass -eq $true)
  },
  [pscustomobject]@{
    Label = "Summary cache reuse gate"
    Value = if ($null -eq $phase05.summary_cache_reuse_observed) { "n/a" } else { "{0:P1}" -f [double]$phase05.summary_cache_reuse_observed }
    Detail = ">= {0:P1}" -f [double]$phase05.cache_reuse_gate_min
    Passed = [bool]($phase05.summary_cache_reuse_pass -eq $true)
  },
  [pscustomobject]@{
    Label = "GPU hybrid-ranking speed gate"
    Value = if ($null -eq $phase05.gpu_speedup_observed) { "n/a" } else { "{0:N2}x" -f [double]$phase05.gpu_speedup_observed }
    Detail = ">= {0:N2}x" -f [double]$phase05.gpu_speedup_gate_min
    Passed = [bool]($phase05.gpu_speedup_pass -eq $true)
  },
  [pscustomobject]@{
    Label = "Estimated backend preview wall gate"
    Value = if ($null -eq $previewWallSpeedup) { "n/a" } else { "{0:N2}x" -f $previewWallSpeedup }
    Detail = "> {0:N2}x" -f $PreviewWallSpeedupGate
    Passed = ($null -ne $previewWallSpeedup) -and ($previewWallSpeedup -gt $PreviewWallSpeedupGate)
  }
)

Write-Host ""
Write-Host "== Options GPU Promotion Gate =="
Write-Host ("summary: {0}" -f $summaryJson.FullName)
foreach ($check in $checks) {
  Write-Host ("[{0}] {1}: {2} ({3})" -f (Format-Status $check.Passed), $check.Label, $check.Value, $check.Detail)
}

$gateSummary = [ordered]@{
  schema_version = 1
  case_registry_version = 1
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  environment = Get-EnvironmentFingerprint
  gpu = $gpuIdentity.raw
  gpu_info = $gpuIdentity
  phase05_summary_json = $summaryJson.FullName
  requested_command_args = $phase05Args
  artifact_path = $summary.artifact_path
  options_dataset_id = $summary.options_dataset_id
  workload_identity = [ordered]@{
    options_dataset_id = $summary.options_dataset_id
    root_symbol = $summary.dataset_manifest.root_symbol
    provider_name = $summary.dataset_manifest.provider_name
    price_semantics = $summary.dataset_manifest.price_semantics
    row_count = $summary.dataset_manifest.row_count
    contract_count = $summary.dataset_manifest.contract_count
    split_assignment_mode = $SplitAssignmentMode
    compiled_tuple_count = $report.compiled_tuple_count
    per_event_candidate_slots = $report.per_event_candidate_slots
    per_candidate_exit_summaries = $report.per_candidate_exit_summaries
    candidate_slot_count = $report.candidate_slot_count
    compiled_candidate_count = $report.compiled_candidate_count
    best_available_backend = $report.best_available_backend
    recommended_backend = $report.auto_backend_decision.recommended_backend
    gpu_eligible = $report.auto_backend_decision.gpu_eligible
  }
  backend_preference = $BackendPreference
  split_assignment_mode = $SplitAssignmentMode
  top_k = [Math]::Max(1, $TopK)
  sealed_finalist_count = [Math]::Max(1, [Math]::Min($SealedFinalistCount, [Math]::Max(1, $TopK)))
  warm_repeats = [Math]::Max(0, $WarmRepeats)
  benchmark_iterations = [Math]::Max(1, $BenchmarkIterations)
  preview_wall_speedup_gate = $PreviewWallSpeedupGate
  regression_pct = $RegressionPct
  preview_wall_regression_abs_ms = $PreviewWallRegressionAbsMs
  ranking_regression_abs_us = $RankingRegressionAbsUs
  correctness_evidence = $correctnessEvidence
  checks = $checks
  phase05_gates = $phase05
  backend_comparison = $comparison
  estimated_backend_preview_wall_speedup_vs_cpu = $previewWallSpeedup
  baseline_comparison = $null
  recommendations = $summary.recommendations
}
Write-JsonUtf8 -Path $SummaryOutPath -Value $gateSummary
Write-Host ("summary: {0}" -f $SummaryOutPath)

if ($summary.recommendations) {
  Write-Host ""
  Write-Host "Recommendations:"
  foreach ($recommendation in $summary.recommendations) {
    Write-Host ("- {0}" -f $recommendation)
  }
}

$failedChecks = $checks | Where-Object { -not $_.Passed }
if ($failedChecks.Count -gt 0) {
  $failedLabels = ($failedChecks | ForEach-Object { $_.Label }) -join "; "
  throw "Options GPU promotion gate failed: $failedLabels"
}

if ($Mode -eq "baseline") {
  Write-JsonUtf8 -Path $BaselinePath -Value $gateSummary
  Write-Host ("baseline: {0}" -f $BaselinePath)
  return
}

if (!(Test-Path $BaselinePath)) {
  throw "Missing options expression performance baseline: $BaselinePath. Run with -Mode baseline first."
}

$baseline = Read-JsonFile -Path $BaselinePath
Assert-CompatibilityEqual -Name "schema_version" -Candidate $gateSummary.schema_version -Baseline $baseline.schema_version
Assert-CompatibilityEqual -Name "case_registry_version" -Candidate $gateSummary.case_registry_version -Baseline $baseline.case_registry_version
Assert-CompatibilityEqual -Name "backend_preference" -Candidate $gateSummary.backend_preference -Baseline $baseline.backend_preference
Assert-CompatibilityEqual -Name "split_assignment_mode" -Candidate $gateSummary.split_assignment_mode -Baseline $baseline.split_assignment_mode
Assert-CompatibilityEqual -Name "top_k" -Candidate $gateSummary.top_k -Baseline $baseline.top_k
Assert-CompatibilityEqual -Name "sealed_finalist_count" -Candidate $gateSummary.sealed_finalist_count -Baseline $baseline.sealed_finalist_count
Assert-CompatibilityEqual -Name "workload.options_dataset_id" -Candidate $gateSummary.workload_identity.options_dataset_id -Baseline $baseline.workload_identity.options_dataset_id
Assert-CompatibilityEqual -Name "workload.root_symbol" -Candidate $gateSummary.workload_identity.root_symbol -Baseline $baseline.workload_identity.root_symbol
Assert-CompatibilityEqual -Name "workload.price_semantics" -Candidate $gateSummary.workload_identity.price_semantics -Baseline $baseline.workload_identity.price_semantics
Assert-CompatibilityEqual -Name "workload.compiled_tuple_count" -Candidate $gateSummary.workload_identity.compiled_tuple_count -Baseline $baseline.workload_identity.compiled_tuple_count
Assert-CompatibilityEqual -Name "workload.per_event_candidate_slots" -Candidate $gateSummary.workload_identity.per_event_candidate_slots -Baseline $baseline.workload_identity.per_event_candidate_slots
Assert-CompatibilityEqual -Name "workload.per_candidate_exit_summaries" -Candidate $gateSummary.workload_identity.per_candidate_exit_summaries -Baseline $baseline.workload_identity.per_candidate_exit_summaries
Assert-CompatibilityEqual -Name "workload.candidate_slot_count" -Candidate $gateSummary.workload_identity.candidate_slot_count -Baseline $baseline.workload_identity.candidate_slot_count
Assert-CompatibilityEqual -Name "workload.compiled_candidate_count" -Candidate $gateSummary.workload_identity.compiled_candidate_count -Baseline $baseline.workload_identity.compiled_candidate_count
Assert-CompatibilityEqual -Name "workload.best_available_backend" -Candidate $gateSummary.workload_identity.best_available_backend -Baseline $baseline.workload_identity.best_available_backend

if (
  $gateSummary.gpu_info.PSObject.Properties.Name -contains "name" -and
  $baseline.gpu_info -and
  $baseline.gpu_info.PSObject.Properties.Name -contains "name" -and
  -not [string]::IsNullOrWhiteSpace($gateSummary.gpu_info.name) -and
  -not [string]::IsNullOrWhiteSpace($baseline.gpu_info.name)
) {
  Assert-CompatibilityEqual -Name "gpu_info.name" -Candidate $gateSummary.gpu_info.name -Baseline $baseline.gpu_info.name
}

$budget = 1.0 + ($RegressionPct / 100.0)
$speedupFloor = 1.0 - ($RegressionPct / 100.0)
$candidatePreview = $gateSummary.backend_comparison.accelerated_preview
$baselinePreview = $baseline.backend_comparison.accelerated_preview
$candidateCpu = $gateSummary.backend_comparison.cpu_reference_preview
$baselineCpu = $baseline.backend_comparison.cpu_reference_preview

$comparisonChecks = @()
if ($null -ne $candidatePreview -and $null -ne $baselinePreview) {
  $acceleratedPreviewBudgetMs = [Math]::Max(
    [double]$baselinePreview.median_estimated_backend_preview_ms * $budget,
    [double]$baselinePreview.median_estimated_backend_preview_ms + $PreviewWallRegressionAbsMs
  )
  $acceleratedRankingBudgetUs = [Math]::Max(
    [double]$baselinePreview.median_ranking_us * $budget,
    [double]$baselinePreview.median_ranking_us + $RankingRegressionAbsUs
  )
  $comparisonChecks += [pscustomobject]@{
    Label = "Accelerated estimated preview wall"
    Passed = ([double]$candidatePreview.median_estimated_backend_preview_ms -le $acceleratedPreviewBudgetMs)
    Detail = "{0} ms vs baseline {1} ms (budget {2} ms)" -f $candidatePreview.median_estimated_backend_preview_ms, $baselinePreview.median_estimated_backend_preview_ms, ([Math]::Round($acceleratedPreviewBudgetMs, 3))
  }
  $comparisonChecks += [pscustomobject]@{
    Label = "Accelerated ranking latency"
    Passed = ([double]$candidatePreview.median_ranking_us -le $acceleratedRankingBudgetUs)
    Detail = "{0} us vs baseline {1} us (budget {2} us)" -f $candidatePreview.median_ranking_us, $baselinePreview.median_ranking_us, ([Math]::Round($acceleratedRankingBudgetUs, 3))
  }
}
if ($null -ne $candidateCpu -and $null -ne $baselineCpu) {
  $cpuPreviewBudgetMs = [Math]::Max(
    [double]$baselineCpu.median_preview_ms * $budget,
    [double]$baselineCpu.median_preview_ms + $PreviewWallRegressionAbsMs
  )
  $comparisonChecks += [pscustomobject]@{
    Label = "CPU reference preview wall"
    Passed = ([double]$candidateCpu.median_preview_ms -le $cpuPreviewBudgetMs)
    Detail = "{0} ms vs baseline {1} ms (budget {2} ms)" -f $candidateCpu.median_preview_ms, $baselineCpu.median_preview_ms, ([Math]::Round($cpuPreviewBudgetMs, 3))
  }
}
if ($null -ne $gateSummary.estimated_backend_preview_wall_speedup_vs_cpu -and $null -ne $baseline.estimated_backend_preview_wall_speedup_vs_cpu) {
  $comparisonChecks += [pscustomobject]@{
    Label = "Estimated backend preview speedup"
    Passed = ([double]$gateSummary.estimated_backend_preview_wall_speedup_vs_cpu -ge ([double]$baseline.estimated_backend_preview_wall_speedup_vs_cpu * $speedupFloor))
    Detail = "{0:N2}x vs baseline {1:N2}x" -f [double]$gateSummary.estimated_backend_preview_wall_speedup_vs_cpu, [double]$baseline.estimated_backend_preview_wall_speedup_vs_cpu
  }
}
if ($null -ne $phase05.gpu_speedup_observed -and $null -ne $baseline.phase05_gates.gpu_speedup_observed) {
  $comparisonChecks += [pscustomobject]@{
    Label = "GPU ranking speedup"
    Passed = ([double]$phase05.gpu_speedup_observed -ge ([double]$baseline.phase05_gates.gpu_speedup_observed * $speedupFloor))
    Detail = "{0:N2}x vs baseline {1:N2}x" -f [double]$phase05.gpu_speedup_observed, [double]$baseline.phase05_gates.gpu_speedup_observed
  }
}

foreach ($check in $comparisonChecks) {
  Write-PassFailLine $check.Label $check.Passed $check.Detail
}

$gateSummary.baseline_comparison = $comparisonChecks
Write-JsonUtf8 -Path $SummaryOutPath -Value $gateSummary

$failedComparisonChecks = @($comparisonChecks | Where-Object { -not $_.Passed })
if ($failedComparisonChecks.Count -gt 0) {
  $failedLabels = ($failedComparisonChecks | ForEach-Object { $_.Label }) -join "; "
  throw "Options expression performance regression gate failed: $failedLabels"
}

return
