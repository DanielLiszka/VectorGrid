param(
  [ValidateSet("small", "medium", "large", "xlarge")]
  [string]$Workload = "medium",
  [ValidateRange(1, 100)]
  [int]$Iterations = 5,
  [ValidateRange(0, 100)]
  [int]$WarmupIterations = 1,
  [ValidateRange(1, 2000000)]
  [int]$CandlesLen = 100000,
  [ValidateRange(0, 64)]
  [int]$DeviceId = 0,
  [string]$OutPath = "output/benchmarks/cuda_graph_timing_summary.json",
  [switch]$PlanOnly,
  [switch]$FreshnessFailureCanary
)

$ErrorActionPreference = "Stop"
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")

$schemaVersion = 1
$timingContract = [ordered]@{
  schema_version = $schemaVersion
  clock = "std_monotonic_instant"
  capture_boundary = "cuStreamBeginCapture_v2_through_cuStreamEndCapture_including_node_recording"
  instantiate_boundary = "cuGraphInstantiate_v2_host_call"
  update_boundary = "not_applicable_immutable_graph_exec_no_cuGraphExecUpdate_call"
  update_applicable = $false
  replay_boundary = "cuGraphLaunch_host_submission_only_async_device_completion_excluded"
  interpretation = "Replay measures host submission latency only. Device completion remains inside route wall/synchronization boundaries and requires Nsight or CUDA-event evidence for kernel-duration claims."
}

$runnerArgs = @(
  "--strategy", "double_ma",
  "--workload", $Workload,
  "--iterations", "$Iterations",
  "--warmup-iterations", "$WarmupIterations",
  "--candles-len", "$CandlesLen",
  "--device-id", "$DeviceId",
  "--top-k", "50",
  "--heatmap-bins", "0",
  "--include-all", "off",
  "--dynamic-mode", "off",
  "--cuda-graphs", "on",
  "--cuda-graph-timing", "on",
  "--nvtx", "off",
  "--emit-json"
)

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value,
    [switch]$AllowReplace
  )
  $absolute = if ([System.IO.Path]::IsPathRooted($Path)) {
    [System.IO.Path]::GetFullPath($Path)
  } else {
    [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
  }
  $parent = Split-Path -Parent $absolute
  if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  $json = $Value | ConvertTo-Json -Depth 20
  $tempPath = Join-Path $parent (
    ".{0}.{1}.tmp" -f ([System.IO.Path]::GetFileName($absolute)), [guid]::NewGuid().ToString("N")
  )
  try {
    [System.IO.File]::WriteAllText(
      $tempPath,
      "$json`n",
      [System.Text.UTF8Encoding]::new($false)
    )
    if (Test-Path -LiteralPath $absolute -PathType Leaf) {
      if (-not $AllowReplace) {
        throw "Refusing to replace CUDA Graph timing evidence without explicit authority: $absolute"
      }
      [System.IO.File]::Replace($tempPath, $absolute, $null, $true)
    } else {
      [System.IO.File]::Move($tempPath, $absolute)
    }
  } finally {
    Remove-Item -LiteralPath $tempPath -Force -ErrorAction SilentlyContinue
  }
  return $absolute
}

function Resolve-EvidencePath {
  param([Parameter(Mandatory = $true)][string]$Path)
  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function Test-SafeReplaceableEvidencePath {
  param([Parameter(Mandatory = $true)][string]$Path)
  $safeRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "output/benchmarks"))
  if (Test-Path -LiteralPath $safeRoot) {
    $safeRootItem = Get-Item -LiteralPath $safeRoot -Force
    if (($safeRootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      return $false
    }
  }
  $parent = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::GetDirectoryName($Path)
  )
  return (
    $parent.Equals($safeRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
    [System.IO.Path]::GetExtension($Path).Equals(
      ".json",
      [System.StringComparison]::OrdinalIgnoreCase
    )
  )
}

function Prepare-FreshEvidenceDestination {
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) {
    return
  }
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "CUDA Graph timing output path already exists and is not a file: $Path"
  }
  if (-not (Test-SafeReplaceableEvidencePath -Path $Path)) {
    throw (
      "CUDA Graph timing output already exists outside the bounded " +
      "immediate output/benchmarks JSON evidence directory. Choose a fresh destination: $Path"
    )
  }
  $item = Get-Item -LiteralPath $Path -Force
  if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Refusing to replace reparse-point CUDA Graph timing evidence: $Path"
  }
  Remove-Item -LiteralPath $Path -Force
  if (Test-Path -LiteralPath $Path) {
    throw "Failed to clear stale CUDA Graph timing evidence: $Path"
  }
}

function Get-PerfProcesses {
  @(Get-Process | Where-Object {
      $_.ProcessName -match '^(cargo|criterion|nsys|ncu)$' -or
      $_.ProcessName -like 'double_ma_opt*' -or
      $_.ProcessName -eq 'profile_gpu_dynamic_runner'
    } | Select-Object ProcessName, Id, CPU, StartTime)
}

function Wait-NoPerfProcesses {
  param(
    [int]$TimeoutSeconds = 300,
    [string]$Context = ""
  )
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ($true) {
    $active = @(Get-PerfProcesses)
    if ($active.Count -eq 0) {
      return
    }
    if ((Get-Date) -ge $deadline) {
      $summary = ($active | ForEach-Object { "$($_.ProcessName)#$($_.Id)" }) -join ", "
      throw "Timed out waiting for benchmark/profiler processes to drain ${Context}: $summary"
    }
    Start-Sleep -Milliseconds 500
  }
}

function Invoke-ProcessCaptured {
  param(
    [Parameter(Mandatory = $true)][string]$FilePath,
    [Parameter(Mandatory = $true)][string[]]$ArgumentList,
    [Parameter(Mandatory = $true)][string]$FailureContext
  )
  $stdoutPath = [System.IO.Path]::GetTempFileName()
  $stderrPath = [System.IO.Path]::GetTempFileName()
  try {
    $process = Start-Process -FilePath $FilePath `
      -ArgumentList $ArgumentList `
      -NoNewWindow `
      -Wait `
      -PassThru `
      -RedirectStandardOutput $stdoutPath `
      -RedirectStandardError $stderrPath
    $stdout = if (Test-Path -LiteralPath $stdoutPath) {
      @(Get-Content -LiteralPath $stdoutPath)
    } else {
      @()
    }
    $stderr = if (Test-Path -LiteralPath $stderrPath) {
      @(Get-Content -LiteralPath $stderrPath)
    } else {
      @()
    }
    foreach ($line in $stderr) {
      Write-Host $line
    }
    if ($process.ExitCode -ne 0) {
      throw "${FailureContext} (exit code $($process.ExitCode))"
    }
    return $stdout
  } finally {
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
  }
}

function Resolve-CargoTargetRoot {
  if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target"))
  }
  if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    return [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $env:CARGO_TARGET_DIR))
}

function Get-EnvironmentFingerprint {
  $gpu = ""
  try {
    $gpu = (& nvidia-smi --query-gpu=name,driver_version --format=csv,noheader |
        Select-Object -First 1).Trim()
  } catch {
    $gpu = ""
  }
  return [ordered]@{
    gpu = $gpu
    rustc = (& rustc --version).Trim()
    cargo = (& cargo --version).Trim()
    os = [System.Environment]::OSVersion.VersionString
    processor_count = [System.Environment]::ProcessorCount
  }
}

$plan = [ordered]@{
  schema_version = $schemaVersion
  report_kind = "cuda-graph-timing-boundary"
  status = "planned"
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  cargo_features = "cuda,cuda-backtest-kernel"
  runner = "profile_gpu_dynamic_runner"
  workload = [ordered]@{
    name = $Workload
    iterations = $Iterations
    warmup_iterations = $WarmupIterations
    candles_len = $CandlesLen
    device_id = $DeviceId
  }
  runner_args = $runnerArgs
  timing_contract = $timingContract
}

if ($PlanOnly -and $FreshnessFailureCanary) {
  throw "-FreshnessFailureCanary cannot be combined with -PlanOnly"
}
if ($FreshnessFailureCanary -and -not $PSBoundParameters.ContainsKey("OutPath")) {
  throw "-FreshnessFailureCanary requires an explicit bounded -OutPath"
}
$effectiveOutPath = if ($PlanOnly -and -not $PSBoundParameters.ContainsKey("OutPath")) {
  "output/benchmarks/cuda_graph_timing_plan.json"
} else {
  $OutPath
}
$resolvedOutPath = Resolve-EvidencePath -Path $effectiveOutPath
Prepare-FreshEvidenceDestination -Path $resolvedOutPath

if ($PlanOnly) {
  $written = Write-JsonUtf8 -Path $resolvedOutPath -Value $plan
  Write-Host "CUDA Graph timing probe plan written to $written"
  return
}

$runningMarker = [ordered]@{
  schema_version = $schemaVersion
  report_kind = "cuda-graph-timing-boundary"
  status = "running"
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  workload = $plan.workload
  runner_args = $runnerArgs
  timing_contract = $timingContract
}
Write-JsonUtf8 -Path $resolvedOutPath -Value $runningMarker | Out-Null
if ($FreshnessFailureCanary) {
  throw "Intentional CUDA Graph timing freshness failure canary"
}

$previousPipelineProfile = [Environment]::GetEnvironmentVariable(
  "VECTORBT_KERNEL_PROFILE_PIPELINE",
  "Process"
)
$rows = @()
$benchmarkLock = Enter-BenchmarkLock -Context "CUDA Graph timing probe"
try {
  [Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_PROFILE_PIPELINE", "0", "Process")
  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before CUDA Graph timing probe"

  Push-Location $repoRoot
  try {
    & cargo build -q -p ta_app_core --bin profile_gpu_dynamic_runner --release --features cuda,cuda-backtest-kernel
    if ($LASTEXITCODE -ne 0) {
      throw "Failed to build profile_gpu_dynamic_runner (cargo exit code $LASTEXITCODE)"
    }
    $runnerExe = Join-Path (Resolve-CargoTargetRoot) "release/profile_gpu_dynamic_runner.exe"
    if (-not (Test-Path -LiteralPath $runnerExe -PathType Leaf)) {
      throw "Built runner was not found at $runnerExe"
    }
    $output = Invoke-ProcessCaptured `
      -FilePath $runnerExe `
      -ArgumentList $runnerArgs `
      -FailureContext "CUDA Graph timing runner failed"
    foreach ($line in $output) {
      $trimmed = "$line".Trim()
      if ([string]::IsNullOrWhiteSpace($trimmed) -or -not $trimmed.StartsWith("{")) {
        continue
      }
      try {
        $rows += ,($trimmed | ConvertFrom-Json)
      } catch {
        throw "Runner emitted a non-parseable JSON line: $trimmed"
      }
    }
  } finally {
    Pop-Location
  }
  Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "after CUDA Graph timing probe"
} finally {
  [Environment]::SetEnvironmentVariable(
    "VECTORBT_KERNEL_PROFILE_PIPELINE",
    $previousPipelineProfile,
    "Process"
  )
  Exit-BenchmarkLock -Handle $benchmarkLock
}

$measureRows = @($rows | Where-Object { "$($_.phase)" -eq "measure" })
if ($measureRows.Count -ne $Iterations) {
  throw "Expected $Iterations measured JSON rows, received $($measureRows.Count)"
}

$captureTotalUs = [uint64]0
$captureCount = [uint64]0
$instantiateTotalUs = [uint64]0
$instantiateCount = [uint64]0
$replaySubmitTotalUs = [uint64]0
$replayCount = [uint64]0
foreach ($row in $measureRows) {
  if ("$($row.cuda_graphs)" -ne "on" -or "$($row.cuda_graph_timing)" -ne "on") {
    throw "Runner row did not attest cuda_graphs=on and cuda_graph_timing=on"
  }
  $graph = $row.gpu_kernel_timing_ms.cuda_graph_timing_us
  if ($null -eq $graph) {
    throw "Runner row omitted gpu_kernel_timing_ms.cuda_graph_timing_us"
  }
  if ([int]$graph.schema_version -ne $schemaVersion) {
    throw "Unexpected CUDA Graph timing schema version: $($graph.schema_version)"
  }
  if (-not [bool]$graph.timing_enabled -or
      -not [bool]$graph.graphs_requested -or
      -not [bool]$graph.graphs_eligible) {
    throw "CUDA Graph timing row did not attest an enabled, requested, eligible graph route"
  }
  foreach ($field in @(
      "clock",
      "capture_boundary",
      "instantiate_boundary",
      "update_boundary",
      "replay_boundary"
    )) {
    if ("$($graph.$field)" -ne "$($timingContract.$field)") {
      throw "CUDA Graph timing boundary mismatch for ${field}: '$($graph.$field)'"
    }
  }
  if ([uint64]$graph.capture_attempt_count -lt 1 -or
      [uint64]$graph.capture_success_count -ne [uint64]$graph.capture_attempt_count) {
    throw "CUDA Graph capture did not complete successfully for every attempt"
  }
  if ([uint64]$graph.instantiate_attempt_count -lt 1 -or
      [uint64]$graph.instantiate_success_count -ne [uint64]$graph.instantiate_attempt_count) {
    throw "CUDA Graph instantiation did not complete successfully for every attempt"
  }
  if ([bool]$graph.update_applicable -or
      [uint64]$graph.update_attempt_count -ne 0 -or
      [uint64]$graph.update_success_count -ne 0 -or
      [uint64]$graph.update_total_us -ne 0) {
    throw "CUDA Graph update must remain explicitly not applicable for the immutable graph-exec path"
  }
  if ([uint64]$graph.replay_attempt_count -lt 1 -or
      [uint64]$graph.replay_success_count -ne [uint64]$graph.replay_attempt_count) {
    throw "CUDA Graph replay submission did not complete successfully for every attempt"
  }
  if ([uint64]$row.gpu_kernel_timing_ms.graph_replay_count -ne
      [uint64]$graph.replay_success_count) {
    throw "Legacy graph replay count disagrees with the instrumented replay-success count"
  }

  $captureTotalUs += [uint64]$graph.capture_total_us
  $captureCount += [uint64]$graph.capture_success_count
  $instantiateTotalUs += [uint64]$graph.instantiate_total_us
  $instantiateCount += [uint64]$graph.instantiate_success_count
  $replaySubmitTotalUs += [uint64]$graph.replay_submit_total_us
  $replayCount += [uint64]$graph.replay_success_count
}

$summary = [ordered]@{
  schema_version = $schemaVersion
  report_kind = "cuda-graph-timing-boundary"
  status = "passed"
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  cargo_features = "cuda,cuda-backtest-kernel"
  runner = "profile_gpu_dynamic_runner"
  environment = Get-EnvironmentFingerprint
  workload = $plan.workload
  runner_args = $runnerArgs
  timing_contract = $timingContract
  aggregates = [ordered]@{
    measured_iterations = $measureRows.Count
    capture_success_count = $captureCount
    capture_total_us = $captureTotalUs
    capture_mean_us = if ($captureCount -gt 0) {
      [math]::Round([double]$captureTotalUs / [double]$captureCount, 6)
    } else { $null }
    instantiate_success_count = $instantiateCount
    instantiate_total_us = $instantiateTotalUs
    instantiate_mean_us = if ($instantiateCount -gt 0) {
      [math]::Round([double]$instantiateTotalUs / [double]$instantiateCount, 6)
    } else { $null }
    update_applicable = $false
    update_attempt_count = 0
    update_total_us = 0
    replay_success_count = $replayCount
    replay_submit_total_us = $replaySubmitTotalUs
    replay_submit_mean_us = if ($replayCount -gt 0) {
      [math]::Round([double]$replaySubmitTotalUs / [double]$replayCount, 6)
    } else { $null }
  }
  measurement_rows = $measureRows
}

$written = Write-JsonUtf8 -Path $resolvedOutPath -Value $summary -AllowReplace
Write-Host "CUDA Graph timing boundary evidence written to $written"
