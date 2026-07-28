param(
  [ValidateRange(32, 2000000)]
  [int]$CandlesLen = 20000,
  [ValidateRange(64, 512)]
  [int]$VramBudgetMb = 160,
  [ValidateRange(0, 64)]
  [int]$DeviceId = 0,
  [ValidateRange(0.0, 1.0)]
  [double]$MetricAbsoluteTolerance = 0.000001,
  [ValidateRange(0.0, 1.0)]
  [double]$MetricRelativeTolerance = 0.000001,
  [string]$OutPath = "output/benchmarks/cuda_graph_multitile_parity_summary.json",
  [switch]$PlanOnly,
  [switch]$FreshnessFailureCanary
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")

$schemaVersion = 1
$workload = "large"
$fastRange = [ordered]@{ start = 5; end = 200; step = 1; count = 196 }
$slowRange = [ordered]@{ start = 10; end = 400; step = 1; count = 391 }
$metricFields = @(
  "sharpe",
  "pnl",
  "sortino",
  "omega",
  "calmar",
  "max_dd",
  "gross_profit",
  "gross_loss"
)
$integerMetricFields = @(
  "trades",
  "closed_trades",
  "winning_trades",
  "losing_trades"
)
$timingContract = [ordered]@{
  schema_version = $schemaVersion
  clock = "std_monotonic_instant"
  capture_boundary = "cuStreamBeginCapture_v2_through_cuStreamEndCapture_including_node_recording"
  instantiate_boundary = "cuGraphInstantiate_v2_host_call"
  update_boundary = "not_applicable_immutable_graph_exec_no_cuGraphExecUpdate_call"
  update_applicable = $false
  replay_boundary = "cuGraphLaunch_host_submission_only_async_device_completion_excluded"
}

$commonRunnerArgs = @(
  "--strategy", "double_ma",
  "--workload", $workload,
  "--iterations", "1",
  "--warmup-iterations", "0",
  "--candles-len", "$CandlesLen",
  "--candle-shape", "synth",
  "--device-id", "$DeviceId",
  "--fast-ma", "sma",
  "--slow-ma", "sma",
  "--objective", "sharpe",
  "--metric-contract", "full17",
  "--top-k", "50",
  "--heatmap-bins", "0",
  "--include-all", "off",
  "--include-trade-metrics", "off",
  "--dynamic-mode", "off",
  "--nvtx", "off",
  "--emit-json"
)
$baselineRunnerArgs = $commonRunnerArgs + @(
  "--cuda-graphs", "off",
  "--cuda-graph-timing", "off"
)
$graphRunnerArgs = $commonRunnerArgs + @(
  "--cuda-graphs", "on",
  "--cuda-graph-timing", "on"
)

function Resolve-EvidencePath {
  param([Parameter(Mandatory = $true)][string]$Path)
  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function Prepare-FreshEvidenceDestination {
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) {
    return
  }
  throw (
    "Refusing stale or concurrently reserved CUDA Graph multi-tile parity OutPath. " +
    "Choose a fresh destination: $Path"
  )
}

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value,
    [switch]$AllowReplace
  )
  $absolute = Resolve-EvidencePath -Path $Path
  $parent = Split-Path -Parent $absolute
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
  $tempPath = Join-Path $parent (
    ".{0}.{1}.tmp" -f ([System.IO.Path]::GetFileName($absolute)), [guid]::NewGuid().ToString("N")
  )
  try {
    [System.IO.File]::WriteAllText(
      $tempPath,
      "$(ConvertTo-Json -InputObject $Value -Depth 30)`n",
      [System.Text.UTF8Encoding]::new($false)
    )
    if (Test-Path -LiteralPath $absolute -PathType Leaf) {
      if (-not $AllowReplace) {
        throw "Refusing to replace CUDA Graph multi-tile parity evidence without explicit authority: $absolute"
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

function Assert-OwnedRunningMarker {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$RunId
  )
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "CUDA Graph multi-tile parity running marker disappeared: $Path"
  }
  $item = Get-Item -LiteralPath $Path -Force
  if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "CUDA Graph multi-tile parity running marker became a reparse point: $Path"
  }
  try {
    $marker = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
  } catch {
    throw "CUDA Graph multi-tile parity running marker is not valid JSON: $Path"
  }
  if ("$($marker.status)" -ne "running" -or "$($marker.run_id)" -ne $RunId) {
    throw "CUDA Graph multi-tile parity running marker ownership changed: $Path"
  }
}

function Get-PerfProcesses {
  @(Get-Process | Where-Object {
      $_.ProcessName -match '^(cargo|rustc|criterion|nsys|ncu)$' -or
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

function Resolve-CargoTargetRoot {
  if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target"))
  }
  if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    return [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $env:CARGO_TARGET_DIR))
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
    if ($process.ExitCode -ne 0) {
      foreach ($line in $stderr) {
        Write-Host $line
      }
      throw "${FailureContext} (exit code $($process.ExitCode))"
    }
    return [pscustomobject]@{
      stdout = $stdout
      stderr = $stderr
    }
  } finally {
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
  }
}

function Get-OnlyMeasureRow {
  param(
    [Parameter(Mandatory = $true)]$Captured,
    [Parameter(Mandatory = $true)][string]$Label
  )
  $rows = @()
  foreach ($line in $Captured.stdout) {
    $trimmed = "$line".Trim()
    if ([string]::IsNullOrWhiteSpace($trimmed) -or -not $trimmed.StartsWith("{")) {
      continue
    }
    try {
      $rows += ,($trimmed | ConvertFrom-Json)
    } catch {
      throw "$Label runner emitted a non-parseable JSON line: $trimmed"
    }
  }
  $measureRows = @($rows | Where-Object { "$($_.phase)" -eq "measure" })
  if ($measureRows.Count -ne 1) {
    throw "$Label runner must emit exactly one measured JSON row; received $($measureRows.Count)"
  }
  return $measureRows[0]
}

function Get-TileGeometry {
  param(
    [Parameter(Mandatory = $true)]$Captured,
    [Parameter(Mandatory = $true)][string]$Label
  )
  $pattern = (
    'VECTORBT_KERNEL_TILE_DEBUG: budget=(?<budget>\d+)MiB ' +
    'full=(?<full>\d+)MiB chosen=(?<chosen>\d+)MiB ' +
    'pf_tile=(?<pf>\d+)/(?<fast_total>\d+) ' +
    'ps_tile=(?<ps>\d+)/(?<slow_total>\d+)'
  )
  $geometryMatches = @()
  foreach ($line in $Captured.stderr) {
    $match = [regex]::Match("$line", $pattern)
    if (-not $match.Success) {
      continue
    }
    $geometry = [ordered]@{
      budget_mib = [int]$match.Groups["budget"].Value
      estimated_full_mib = [int]$match.Groups["full"].Value
      estimated_chosen_mib = [int]$match.Groups["chosen"].Value
      fast_tile = [int]$match.Groups["pf"].Value
      fast_total = [int]$match.Groups["fast_total"].Value
      slow_tile = [int]$match.Groups["ps"].Value
      slow_total = [int]$match.Groups["slow_total"].Value
    }
    if (
      $geometry.fast_total -eq $fastRange.count -and
      $geometry.slow_total -eq $slowRange.count
    ) {
      $geometryMatches += ,$geometry
    }
  }
  if ($geometryMatches.Count -eq 0) {
    throw (
      "$Label runner did not attest a constrained, tiled large-workload geometry. " +
      "The VRAM budget may no longer force multiple tiles."
    )
  }
  $canonical = @(
    $geometryMatches | ForEach-Object { ConvertTo-Json -InputObject $_ -Compress }
  )
  $unique = @($canonical | Sort-Object -Unique)
  if ($unique.Count -ne 1) {
    throw "$Label runner emitted conflicting large-workload tile geometries"
  }
  return $geometryMatches[0]
}

function Get-RepeatedCoarseTileShapeEvidence {
  param([Parameter(Mandatory = $true)]$Geometry)
  $groups = @{}
  $fullTileCount = 0
  for (
    $fastOffset = 0;
    $fastOffset + $Geometry.fast_tile -le $Geometry.fast_total;
    $fastOffset += $Geometry.fast_tile
  ) {
    for (
      $slowOffset = 0;
      $slowOffset + $Geometry.slow_tile -le $Geometry.slow_total;
      $slowOffset += $Geometry.slow_tile
    ) {
      $validPairs = 0
      for ($fastLocal = 0; $fastLocal -lt $Geometry.fast_tile; $fastLocal++) {
        $fastPeriod = $fastRange.start + $fastOffset + $fastLocal
        for ($slowLocal = 0; $slowLocal -lt $Geometry.slow_tile; $slowLocal++) {
          $slowPeriod = $slowRange.start + $slowOffset + $slowLocal
          if ($fastPeriod -lt $slowPeriod) {
            $validPairs++
          }
        }
      }
      if ($validPairs -le 0) {
        continue
      }
      $fullTileCount++
      $densePairs = $Geometry.fast_tile * $Geometry.slow_tile
      $key = (
        "fast_rows={0};slow_rows={1};pair_rows={2};launch_pair_rows={3}" -f
        $Geometry.fast_tile,
        $Geometry.slow_tile,
        $densePairs,
        $validPairs
      )
      if (-not $groups.ContainsKey($key)) {
        $groups[$key] = 0
      }
      $groups[$key] = [int]$groups[$key] + 1
    }
  }
  $shapeGroups = @(
    $groups.GetEnumerator() |
      Sort-Object Name |
      ForEach-Object {
        [ordered]@{
          coarse_signature = $_.Name
          tile_count = [int]$_.Value
        }
      }
  )
  $maxMultiplicity = if ($shapeGroups.Count -gt 0) {
    [int](($shapeGroups | Measure-Object -Property tile_count -Maximum).Maximum)
  } else {
    0
  }
  return [ordered]@{
    eligible_full_tile_count = $fullTileCount
    coarse_signature_group_count = $shapeGroups.Count
    max_same_coarse_signature_multiplicity = $maxMultiplicity
    coarse_signature_groups = $shapeGroups
  }
}

function Convert-FiniteDouble {
  param(
    [Parameter(Mandatory = $true)]$Value,
    [Parameter(Mandatory = $true)][string]$Label
  )
  if ($null -eq $Value) {
    throw "$Label is null"
  }
  try {
    $number = [System.Convert]::ToDouble(
      $Value,
      [System.Globalization.CultureInfo]::InvariantCulture
    )
  } catch {
    throw "$Label is not numeric: $Value"
  }
  if ([double]::IsNaN($number) -or [double]::IsInfinity($number)) {
    throw "$Label is not finite: $number"
  }
  return $number
}

function Assert-ResultParity {
  param(
    [Parameter(Mandatory = $true)]$Baseline,
    [Parameter(Mandatory = $true)]$Graph
  )
  if ("$($Baseline.backend_used)" -ne "Gpu" -or "$($Graph.backend_used)" -ne "Gpu") {
    throw "Both parity rows must attest backend_used=Gpu"
  }
  if (
    [string]::IsNullOrWhiteSpace("$($Baseline.backend_route)") -or
    "$($Baseline.backend_route)" -ne "$($Graph.backend_route)"
  ) {
    throw "Graph-off/on backend routes must be present and identical"
  }
  if ($null -ne $Baseline.backend_fallback_reason -or $null -ne $Graph.backend_fallback_reason) {
    throw "Graph-off/on parity rows must not contain a backend fallback reason"
  }
  foreach ($field in @("fast_len", "slow_len")) {
    if ([int64]$Baseline.best_params.$field -ne [int64]$Graph.best_params.$field) {
      throw (
        "CUDA Graph parity changed exact best parameter ${field}: " +
        "$($Baseline.best_params.$field) vs $($Graph.best_params.$field)"
      )
    }
  }
  $metricComparisons = @()
  foreach ($field in $metricFields) {
    $baselineValue = Convert-FiniteDouble `
      -Value $Baseline.best_metrics.$field `
      -Label "baseline best_metrics.$field"
    $graphValue = Convert-FiniteDouble `
      -Value $Graph.best_metrics.$field `
      -Label "graph best_metrics.$field"
    $difference = [math]::Abs($baselineValue - $graphValue)
    $scale = [math]::Max(
      1.0,
      [math]::Max([math]::Abs($baselineValue), [math]::Abs($graphValue))
    )
    $limit = $MetricAbsoluteTolerance + ($MetricRelativeTolerance * $scale)
    if ($difference -gt $limit) {
      throw (
        "CUDA Graph parity exceeded tolerance for best_metrics.${field}: " +
        "baseline=$baselineValue graph=$graphValue difference=$difference limit=$limit"
      )
    }
    $metricComparisons += ,[ordered]@{
      field = $field
      baseline = $baselineValue
      graph = $graphValue
      absolute_difference = $difference
      tolerance_limit = $limit
    }
  }
  foreach ($field in $integerMetricFields) {
    if ($null -eq $Baseline.best_metrics.$field -or $null -eq $Graph.best_metrics.$field) {
      throw "CUDA Graph parity integer metric best_metrics.$field is null"
    }
    if ([int64]$Baseline.best_metrics.$field -ne [int64]$Graph.best_metrics.$field) {
      throw (
        "CUDA Graph parity changed exact integer best_metrics.${field}: " +
        "$($Baseline.best_metrics.$field) vs $($Graph.best_metrics.$field)"
      )
    }
  }
  return $metricComparisons
}

function Assert-BaselineGraphDisabled {
  param([Parameter(Mandatory = $true)]$Row)
  if ("$($Row.cuda_graphs)" -ne "off" -or "$($Row.cuda_graph_timing)" -ne "off") {
    throw "Baseline row did not attest cuda_graphs=off and cuda_graph_timing=off"
  }
  if ($null -eq $Row.gpu_kernel_timing_ms) {
    throw "Baseline row omitted GPU kernel timing"
  }
  $graphTimingProperty = $Row.gpu_kernel_timing_ms.PSObject.Properties[
    "cuda_graph_timing_us"
  ]
  if ($null -ne $graphTimingProperty -and $null -ne $graphTimingProperty.Value) {
    throw "Baseline row unexpectedly emitted CUDA Graph timing"
  }
  $legacyReplayProperty = $Row.gpu_kernel_timing_ms.PSObject.Properties[
    "graph_replay_count"
  ]
  if (
    $null -ne $legacyReplayProperty -and
    $null -ne $legacyReplayProperty.Value -and
    [uint64]$legacyReplayProperty.Value -ne 0
  ) {
    throw "Baseline row unexpectedly replayed a CUDA Graph"
  }
}

function Assert-GraphExecution {
  param([Parameter(Mandatory = $true)]$Row)
  if ("$($Row.cuda_graphs)" -ne "on" -or "$($Row.cuda_graph_timing)" -ne "on") {
    throw "Graph row did not attest cuda_graphs=on and cuda_graph_timing=on"
  }
  $graph = $Row.gpu_kernel_timing_ms.cuda_graph_timing_us
  if ($null -eq $graph) {
    throw "Graph row omitted gpu_kernel_timing_ms.cuda_graph_timing_us"
  }
  if (
    [int]$graph.schema_version -ne $schemaVersion -or
    -not [bool]$graph.timing_enabled -or
    -not [bool]$graph.graphs_requested -or
    -not [bool]$graph.graphs_eligible
  ) {
    throw "Graph row did not attest the expected enabled and eligible timing schema"
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
  $captureAttempts = [uint64]$graph.capture_attempt_count
  $captureSuccesses = [uint64]$graph.capture_success_count
  $instantiateAttempts = [uint64]$graph.instantiate_attempt_count
  $instantiateSuccesses = [uint64]$graph.instantiate_success_count
  $replayAttempts = [uint64]$graph.replay_attempt_count
  $replaySuccesses = [uint64]$graph.replay_success_count
  if ($captureAttempts -lt 2 -or $captureSuccesses -ne $captureAttempts) {
    throw "Every CUDA Graph capture must succeed, with at least two captures"
  }
  if (
    $instantiateAttempts -ne $captureAttempts -or
    $instantiateSuccesses -ne $instantiateAttempts
  ) {
    throw "Every CUDA Graph capture must be instantiated successfully"
  }
  if ($replayAttempts -lt 2 -or $replaySuccesses -ne $replayAttempts) {
    throw "Every CUDA Graph replay must succeed, with at least two replays"
  }
  $legacyReplayCount = [uint64]$Row.gpu_kernel_timing_ms.graph_replay_count
  if ($legacyReplayCount -lt 2) {
    throw "gpu_kernel_timing_ms.graph_replay_count must be at least two"
  }
  if ($legacyReplayCount -ne $replaySuccesses) {
    throw "Legacy graph replay count disagrees with replay-success timing"
  }
  if ($captureSuccesses -ne $replaySuccesses) {
    throw (
      "Pointer-aware immutable graphs must recapture each full tile: " +
      "capture_success_count=$captureSuccesses replay_success_count=$replaySuccesses"
    )
  }
  if (
    [bool]$graph.update_applicable -or
    [uint64]$graph.update_attempt_count -ne 0 -or
    [uint64]$graph.update_success_count -ne 0 -or
    [uint64]$graph.update_total_us -ne 0
  ) {
    throw "CUDA Graph update must remain not applicable for immutable graph executables"
  }
  return $graph
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
  report_kind = "cuda-graph-multitile-correctness-parity"
  status = "planned"
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  cargo_features = "cuda,cuda-backtest-kernel"
  runner = "profile_gpu_dynamic_runner"
  workload = [ordered]@{
    name = $workload
    candles_len = $CandlesLen
    candle_shape = "synth"
    vram_budget_mb_env = $VramBudgetMb
    device_id = $DeviceId
    fast_range = $fastRange
    slow_range = $slowRange
  }
  parity_contract = [ordered]@{
    exact_best_params = $true
    continuous_metric_absolute_tolerance = $MetricAbsoluteTolerance
    continuous_metric_relative_tolerance = $MetricRelativeTolerance
    exact_integer_metrics = $integerMetricFields
    repeated_coarse_full_tile_shape_required = $true
    minimum_graph_replays = 2
    capture_instantiation_replay_must_all_succeed = $true
    capture_success_count_must_equal_replay_success_count = $true
  }
  timing_contract = $timingContract
  baseline_runner_args = $baselineRunnerArgs
  graph_runner_args = $graphRunnerArgs
}

if ($PlanOnly -and $FreshnessFailureCanary) {
  throw "-FreshnessFailureCanary cannot be combined with -PlanOnly"
}
if ($FreshnessFailureCanary -and -not $PSBoundParameters.ContainsKey("OutPath")) {
  throw "-FreshnessFailureCanary requires an explicit fresh -OutPath"
}
$effectiveOutPath = if ($PlanOnly -and -not $PSBoundParameters.ContainsKey("OutPath")) {
  "output/benchmarks/cuda_graph_multitile_parity_plan.json"
} else {
  $OutPath
}
$resolvedOutPath = Resolve-EvidencePath -Path $effectiveOutPath
Prepare-FreshEvidenceDestination -Path $resolvedOutPath

if ($PlanOnly) {
  $written = Write-JsonUtf8 -Path $resolvedOutPath -Value $plan
  Write-Host "CUDA Graph multi-tile parity plan written to $written"
  return
}

$runId = [guid]::NewGuid().ToString("N")
$runningMarker = [ordered]@{
  schema_version = $schemaVersion
  report_kind = "cuda-graph-multitile-correctness-parity"
  status = "running"
  run_id = $runId
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  workload = $plan.workload
  parity_contract = $plan.parity_contract
}
Write-JsonUtf8 -Path $resolvedOutPath -Value $runningMarker | Out-Null
if ($FreshnessFailureCanary) {
  throw "Intentional CUDA Graph multi-tile parity freshness failure canary"
}

$previousPipelineProfile = $env:VECTORBT_KERNEL_PROFILE_PIPELINE
$previousVramBudget = $env:VECTORBT_KERNEL_VRAM_BUDGET_MB
$previousTileDebug = $env:VECTORBT_KERNEL_TILE_DEBUG
$benchmarkLock = $null
try {
  $benchmarkLock = Enter-BenchmarkLock -Context "CUDA Graph multi-tile correctness parity"
  Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "before CUDA Graph multi-tile parity"
  $env:VECTORBT_KERNEL_PROFILE_PIPELINE = "0"
  $env:VECTORBT_KERNEL_VRAM_BUDGET_MB = "$VramBudgetMb"
  $env:VECTORBT_KERNEL_TILE_DEBUG = "1"

  Push-Location $repoRoot
  try {
    & cargo build --locked --offline -q -p ta_app_core `
      --bin profile_gpu_dynamic_runner `
      --release `
      --features cuda,cuda-backtest-kernel
    if ($LASTEXITCODE -ne 0) {
      throw "Failed to build profile_gpu_dynamic_runner (cargo exit code $LASTEXITCODE)"
    }
    $runnerExe = Join-Path (Resolve-CargoTargetRoot) "release/profile_gpu_dynamic_runner.exe"
    if (-not (Test-Path -LiteralPath $runnerExe -PathType Leaf)) {
      throw "Built runner was not found at $runnerExe"
    }

    $baselineCaptured = Invoke-ProcessCaptured `
      -FilePath $runnerExe `
      -ArgumentList $baselineRunnerArgs `
      -FailureContext "CUDA Graph-off multi-tile parity runner failed"
    $graphCaptured = Invoke-ProcessCaptured `
      -FilePath $runnerExe `
      -ArgumentList $graphRunnerArgs `
      -FailureContext "CUDA Graph-on multi-tile parity runner failed"
  } finally {
    Pop-Location
  }
  Wait-NoPerfProcesses -TimeoutSeconds 300 -Context "after CUDA Graph multi-tile parity"

  $baselineRow = Get-OnlyMeasureRow -Captured $baselineCaptured -Label "Graph-off"
  $graphRow = Get-OnlyMeasureRow -Captured $graphCaptured -Label "Graph-on"
  Assert-BaselineGraphDisabled -Row $baselineRow
  $graphTiming = Assert-GraphExecution -Row $graphRow

  $baselineGeometry = Get-TileGeometry -Captured $baselineCaptured -Label "Graph-off"
  $graphGeometry = Get-TileGeometry -Captured $graphCaptured -Label "Graph-on"
  if (
    (ConvertTo-Json -InputObject $baselineGeometry -Compress) -ne
    (ConvertTo-Json -InputObject $graphGeometry -Compress)
  ) {
    throw "Graph-off/on runs did not use identical constrained tile geometry"
  }
  $repeatedShapeEvidence = Get-RepeatedCoarseTileShapeEvidence -Geometry $graphGeometry
  if ($repeatedShapeEvidence.eligible_full_tile_count -lt 2) {
    throw "Constrained geometry did not produce at least two graph-eligible full tiles"
  }
  if ($repeatedShapeEvidence.max_same_coarse_signature_multiplicity -lt 2) {
    throw (
      "Constrained geometry did not produce repeated coarse-identical full tiles; " +
      "the captured-pointer regression was not exercised"
    )
  }
  if (
    [uint64]$graphTiming.replay_success_count -lt
    [uint64]$repeatedShapeEvidence.max_same_coarse_signature_multiplicity
  ) {
    throw "Observed graph replays do not cover the repeated coarse tile-shape multiplicity"
  }

  $metricComparisons = Assert-ResultParity -Baseline $baselineRow -Graph $graphRow
  $runnerHash = (Get-FileHash -LiteralPath $runnerExe -Algorithm SHA256).Hash.ToLowerInvariant()
  $summary = [ordered]@{
    schema_version = $schemaVersion
    report_kind = "cuda-graph-multitile-correctness-parity"
    status = "passed"
    run_id = $runId
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    cargo_features = "cuda,cuda-backtest-kernel"
    runner = "profile_gpu_dynamic_runner"
    runner_path = [System.IO.Path]::GetFullPath($runnerExe)
    runner_sha256 = $runnerHash
    environment = Get-EnvironmentFingerprint
    workload = $plan.workload
    parity_contract = $plan.parity_contract
    timing_contract = $timingContract
    tile_geometry = $graphGeometry
    repeated_coarse_tile_shape_evidence = $repeatedShapeEvidence
    graph_counts = [ordered]@{
      capture_attempt_count = [uint64]$graphTiming.capture_attempt_count
      capture_success_count = [uint64]$graphTiming.capture_success_count
      capture_failure_count = (
        [uint64]$graphTiming.capture_attempt_count -
        [uint64]$graphTiming.capture_success_count
      )
      instantiate_attempt_count = [uint64]$graphTiming.instantiate_attempt_count
      instantiate_success_count = [uint64]$graphTiming.instantiate_success_count
      instantiate_failure_count = (
        [uint64]$graphTiming.instantiate_attempt_count -
        [uint64]$graphTiming.instantiate_success_count
      )
      replay_attempt_count = [uint64]$graphTiming.replay_attempt_count
      replay_success_count = [uint64]$graphTiming.replay_success_count
      replay_failure_count = (
        [uint64]$graphTiming.replay_attempt_count -
        [uint64]$graphTiming.replay_success_count
      )
      legacy_graph_replay_count = [uint64]$graphRow.gpu_kernel_timing_ms.graph_replay_count
    }
    result_parity = [ordered]@{
      exact_best_params_match = $true
      continuous_metrics_within_tolerance = $true
      exact_integer_metrics_match = $true
      best_params = $baselineRow.best_params
      metric_comparisons = $metricComparisons
      exact_integer_metrics = [ordered]@{
        trades = [int64]$baselineRow.best_metrics.trades
        closed_trades = [int64]$baselineRow.best_metrics.closed_trades
        winning_trades = [int64]$baselineRow.best_metrics.winning_trades
        losing_trades = [int64]$baselineRow.best_metrics.losing_trades
      }
    }
    baseline_runner_args = $baselineRunnerArgs
    graph_runner_args = $graphRunnerArgs
    baseline_measurement_row = $baselineRow
    graph_measurement_row = $graphRow
  }
  Assert-OwnedRunningMarker -Path $resolvedOutPath -RunId $runId
  $written = Write-JsonUtf8 -Path $resolvedOutPath -Value $summary -AllowReplace
  Write-Host "CUDA Graph multi-tile correctness parity evidence written to $written"
} finally {
  $env:VECTORBT_KERNEL_PROFILE_PIPELINE = $previousPipelineProfile
  $env:VECTORBT_KERNEL_VRAM_BUDGET_MB = $previousVramBudget
  $env:VECTORBT_KERNEL_TILE_DEBUG = $previousTileDebug
  Exit-BenchmarkLock -Handle $benchmarkLock
}
