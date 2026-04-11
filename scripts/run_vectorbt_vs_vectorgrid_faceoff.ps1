param(
  [string]$ScenarioPath = "scripts/vectorbt_double_ma_faceoff_scenario_real_workflow_250k.json",
  [string]$Csv = "",
  [string]$OutDir = "target/vectorbt_vs_vectorgrid_faceoff",
  [ValidateSet("app-path", "cli-cold")]
  [string]$VectorGridMode = "app-path",
  [ValidateSet("cpu", "gpu-kernel")]
  [string[]]$VectorGridBackends = @(),
  [int]$WarmupTrials = -1,
  [int]$Trials = -1,
  [string]$PythonExe = "py",
  [string[]]$PythonArgs = @(),
  [string]$VectorBtBenchmarkMode = "",
  [int]$VectorBtTimeoutSec = -1,
  [int]$VectorBtCacheMaxEntries = -1,
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-JsonPropertyOrDefault {
  param(
    [object]$Object,
    [string]$Name,
    $Default
  )

  if ($null -eq $Object) {
    return $Default
  }
  if ($Object.PSObject.Properties.Name -contains $Name) {
    return $Object.$Name
  }
  return $Default
}

function Read-JsonFile {
  param([string]$Path)
  return Get-Content -Path $Path -Raw | ConvertFrom-Json
}

function Parse-RangeSpec {
  param([string]$Spec)
  $parts = @($Spec.Split(':', [System.StringSplitOptions]::RemoveEmptyEntries))
  if ($parts.Count -ne 3) {
    throw "Range spec '$Spec' must be start:end:step."
  }
  return [ordered]@{
    start = [int]$parts[0]
    end = [int]$parts[1]
    step = [int]$parts[2]
    spec = $Spec
  }
}

function Count-ValidPairs {
  param(
    [hashtable]$FastRange,
    [hashtable]$SlowRange
  )

  $count = [int64]0
  for ($fast = [int]$FastRange.start; $fast -le [int]$FastRange.end; $fast += [int]$FastRange.step) {
    for ($slow = [int]$SlowRange.start; $slow -le [int]$SlowRange.end; $slow += [int]$SlowRange.step) {
      if ($fast -lt $slow) {
        $count += 1
      }
    }
  }
  return $count
}

function Get-BenchmarkModeContractInfo {
  param([string]$BenchmarkMode)

  if ($BenchmarkMode -eq "real_workflow_10row") {
    return [ordered]@{
      label = "real_workflow_10row"
      intended_metric_contract = "auto_product_reduced_10"
      metric_count = 10
    }
  }

  return [ordered]@{
    label = "search_only_legacy"
    intended_metric_contract = "search_only_objective"
    metric_count = 1
  }
}

function Get-ExpectedVectorGridMetricMetadata {
  param([string]$Objective)

  switch ($Objective.ToLowerInvariant()) {
    "pnl" { return [ordered]@{ label = "objective_minimal_1row"; count = 1 } }
    "maxdrawdown" { return [ordered]@{ label = "objective_drawdown_3row"; count = 3 } }
    default { return [ordered]@{ label = "auto_product_reduced_10row"; count = 10 } }
  }
}

function Get-PythonCommand {
  param(
    [string]$Exe,
    [string[]]$BaseArgs
  )

  $normalizedExe = [System.IO.Path]::GetFileName($Exe)
  if ($normalizedExe -and $normalizedExe -notin @("py", "py.exe") -and $BaseArgs.Count -eq 1 -and $BaseArgs[0] -match '^-3(\.\d+)?$') {
    $BaseArgs = @()
  }

  if ($BaseArgs.Count -eq 0) {
    return [ordered]@{ exe = $Exe; base_args = @(); display = $Exe }
  }

  return [ordered]@{
    exe = $Exe
    base_args = $BaseArgs
    display = "$Exe $($BaseArgs -join ' ')"
  }
}

function Resolve-ExistingPathOrValue {
  param([string]$Path)

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $null
  }
  if (Test-Path $Path) {
    return (Resolve-Path $Path).Path
  }
  return $Path
}

function Get-FileSha256OrNull {
  param([string]$Path)

  if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path $Path)) {
    return $null
  }

  return ((Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLowerInvariant())
}

function Get-VectorGridBoundaryMetadata {
  param(
    [string]$Mode,
    [object]$ParsedBatch
  )

  if ($Mode -eq "app-path") {
    return [ordered]@{
      timing_boundary = Get-JsonPropertyOrDefault -Object $ParsedBatch -Name "timing_boundary" -Default "AppPathSameThread"
      timing_boundary_note = Get-JsonPropertyOrDefault -Object $ParsedBatch -Name "timing_boundary_note" -Default "same-thread harness using AppState plus blocking optimizer call; not the full Tauri command path"
      build_profile = Get-JsonPropertyOrDefault -Object $ParsedBatch -Name "build_profile" -Default "release"
    }
  }

  return [ordered]@{
    timing_boundary = "CliColdWallClock"
    timing_boundary_note = "External vectorgrid CLI process wall-clock timing for a single cold invocation."
    build_profile = "release"
  }
}

function Get-VectorBtBoundaryMetadata {
  return [ordered]@{
    timing_boundary = "PythonProcessWallClock"
    timing_boundary_note = "External Python/vectorbt workflow process wall-clock timing."
    build_profile = "python"
  }
}

function Ensure-ParentDir {
  param([string]$Path)
  $parent = Split-Path -Parent $Path
  if ($parent -and -not (Test-Path $parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
}

function Ensure-Directory {
  param([string]$Path)
  if ($Path -and -not (Test-Path $Path)) {
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
  }
}

function Capture-PythonProvenance {
  param(
    [hashtable]$Python,
    [string]$OutDir,
    [string]$RepoRoot
  )

  Ensure-Directory -Path $OutDir

  $lockfilePath = Join-Path $RepoRoot "scripts/requirements-vectorbt-faceoff.lock.txt"
  $freezePath = Join-Path $OutDir "python-freeze.txt"
  $freezeStderrPath = Join-Path $OutDir "python-freeze.stderr.txt"
  $pipVersionPath = Join-Path $OutDir "python-pip-version.txt"
  $pipVersionStderrPath = Join-Path $OutDir "python-pip-version.stderr.txt"

  $freezeStatus = "not_run"
  $freezeError = $null
  try {
    Invoke-LoggedCommand -Exe $Python.exe -CmdArgs @($Python.base_args + @("-m", "pip", "freeze")) -StdoutPath $freezePath -StderrPath $freezeStderrPath | Out-Null
    $freezeStatus = "captured"
  } catch {
    $freezeStatus = "failed"
    $freezeError = $_.Exception.Message
  }

  $pipVersionStatus = "not_run"
  $pipVersionError = $null
  try {
    Invoke-LoggedCommand -Exe $Python.exe -CmdArgs @($Python.base_args + @("-m", "pip", "--version")) -StdoutPath $pipVersionPath -StderrPath $pipVersionStderrPath | Out-Null
    $pipVersionStatus = "captured"
  } catch {
    $pipVersionStatus = "failed"
    $pipVersionError = $_.Exception.Message
  }

  return [ordered]@{
    lockfile = [ordered]@{
      path = Resolve-ExistingPathOrValue -Path $lockfilePath
      sha256 = Get-FileSha256OrNull -Path $lockfilePath
    }
    pip = [ordered]@{
      freeze_status = $freezeStatus
      freeze_path = Resolve-ExistingPathOrValue -Path $freezePath
      freeze_stderr_path = Resolve-ExistingPathOrValue -Path $freezeStderrPath
      freeze_error = $freezeError
      version_status = $pipVersionStatus
      version_path = Resolve-ExistingPathOrValue -Path $pipVersionPath
      version_stderr_path = Resolve-ExistingPathOrValue -Path $pipVersionStderrPath
      version_error = $pipVersionError
    }
  }
}

function New-ProcessStartInfo {
  param(
    [string]$Exe,
    [string[]]$CmdArgs
  )

  $psi = [System.Diagnostics.ProcessStartInfo]::new()
  $psi.FileName = $Exe
  $psi.Arguments = [string]::Join(" ", ($CmdArgs | ForEach-Object {
    '"' + ([string]$_).Replace('"', '\"') + '"'
  }))
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.UseShellExecute = $false
  $psi.CreateNoWindow = $true
  $psi.WorkingDirectory = (Get-Location).Path
  return $psi
}

function Invoke-LoggedCommand {
  param(
    [string]$Exe,
    [string[]]$CmdArgs,
    [string]$StdoutPath,
    [string]$StderrPath
  )

  Ensure-ParentDir -Path $StdoutPath
  Ensure-ParentDir -Path $StderrPath

  $proc = [System.Diagnostics.Process]::new()
  $proc.StartInfo = New-ProcessStartInfo -Exe $Exe -CmdArgs $CmdArgs
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  [void]$proc.Start()
  $stdout = $proc.StandardOutput.ReadToEnd()
  $stderr = $proc.StandardError.ReadToEnd()
  $proc.WaitForExit()
  $sw.Stop()

  Set-Content -Path $StdoutPath -Value $stdout -Encoding UTF8
  Set-Content -Path $StderrPath -Value $stderr -Encoding UTF8

  if ($proc.ExitCode -ne 0) {
    $msg = "Command failed ($Exe $($CmdArgs -join ' ')) exit=$($proc.ExitCode)"
    if (-not [string]::IsNullOrWhiteSpace($stderr)) {
      $msg = "$msg`n$stderr"
    }
    throw $msg
  }

  return [ordered]@{
    stdout = $stdout
    stderr = $stderr
    elapsed_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 3)
    exit_code = $proc.ExitCode
  }
}

function Invoke-ObservedCommand {
  param(
    [string]$Exe,
    [string[]]$CmdArgs,
    [string]$StdoutPath,
    [string]$StderrPath,
    [int]$TimeoutSec = 0
  )

  Ensure-ParentDir -Path $StdoutPath
  Ensure-ParentDir -Path $StderrPath

  $proc = [System.Diagnostics.Process]::new()
  $proc.StartInfo = New-ProcessStartInfo -Exe $Exe -CmdArgs $CmdArgs
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  [void]$proc.Start()

  $peakWorkingSet = 0L
  $peakPrivateBytes = 0L
  $timedOut = $false
  while (-not $proc.HasExited) {
    try {
      $proc.Refresh()
      if ($proc.WorkingSet64 -gt $peakWorkingSet) { $peakWorkingSet = $proc.WorkingSet64 }
      if ($proc.PrivateMemorySize64 -gt $peakPrivateBytes) { $peakPrivateBytes = $proc.PrivateMemorySize64 }
    } catch {
    }

    if ($TimeoutSec -gt 0 -and $sw.Elapsed.TotalSeconds -ge $TimeoutSec) {
      $timedOut = $true
      try { $proc.Kill($true) } catch { }
      break
    }
    Start-Sleep -Milliseconds 200
  }

  try { $proc.WaitForExit() } catch { }
  $sw.Stop()
  try { $stdout = $proc.StandardOutput.ReadToEnd() } catch { $stdout = "" }
  try { $stderr = $proc.StandardError.ReadToEnd() } catch { $stderr = "" }
  if ($timedOut -and [string]::IsNullOrWhiteSpace($stderr)) {
    $stderr = "Process timed out after $TimeoutSec seconds."
  }

  Set-Content -Path $StdoutPath -Value $stdout -Encoding UTF8
  Set-Content -Path $StderrPath -Value $stderr -Encoding UTF8

  return [ordered]@{
    stdout = $stdout
    stderr = $stderr
    elapsed_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 3)
    exit_code = if ($timedOut) { -1 } else { $proc.ExitCode }
    timed_out = $timedOut
    peak_working_set_bytes = $peakWorkingSet
    peak_private_bytes = $peakPrivateBytes
  }
}

function Classify-ObservedStatus {
  param([object]$Run)
  if ($Run.timed_out) { return "timed_out" }
  if ([int]$Run.exit_code -eq 0) { return "completed" }
  $combined = ("{0}`n{1}" -f $Run.stdout, $Run.stderr)
  if ($combined -match '(?i)(memoryerror|out of memory|cannot allocate|bad_alloc|oom)') {
    return "oom"
  }
  return "failed"
}

function Get-CommandHardware {
  $cpu = $null
  $gpu = $null
  $sys = $null
  try {
    $cpu = Get-CimInstance Win32_Processor | Select-Object -First 1 Name, NumberOfCores, NumberOfLogicalProcessors
  } catch {
  }
  try {
    $gpu = Get-CimInstance Win32_VideoController | Sort-Object -Property AdapterRAM -Descending | Select-Object -First 1 Name, AdapterRAM
  } catch {
  }
  try {
    $sys = Get-CimInstance Win32_ComputerSystem | Select-Object -First 1 TotalPhysicalMemory
  } catch {
  }
  return [ordered]@{
    cpu = [ordered]@{
      name = if ($null -ne $cpu) { $cpu.Name } else { "unknown" }
      cores = if ($null -ne $cpu) { $cpu.NumberOfCores } else { $null }
      logical_processors = if ($null -ne $cpu) { $cpu.NumberOfLogicalProcessors } else { $null }
    }
    gpu = [ordered]@{
      name = if ($null -ne $gpu) { $gpu.Name } else { "unknown" }
      adapter_ram_bytes = if ($null -ne $gpu) { $gpu.AdapterRAM } else { $null }
    }
    ram_bytes = if ($null -ne $sys) { $sys.TotalPhysicalMemory } else { $null }
  }
}

function Ensure-FrozenDataset {
  param(
    [string]$CsvPath,
    [string]$MetaPath,
    [int]$SynthLen,
    [hashtable]$Python
  )

  if ((Test-Path $CsvPath) -and (Test-Path $MetaPath)) { return }

  $scriptPath = Join-Path $repoRoot "scripts/generate_double_ma_faceoff_dataset.py"
  $cmdArgs = @()
  $cmdArgs += $Python.base_args
  $cmdArgs += @($scriptPath, "--synth-len", "$SynthLen", "--output-csv", $CsvPath, "--output-json", $MetaPath)
  $null = Invoke-LoggedCommand -Exe $Python.exe -CmdArgs $cmdArgs -StdoutPath (Join-Path $outPath "logs/dataset.stdout.txt") -StderrPath (Join-Path $outPath "logs/dataset.stderr.txt")
}

function Build-AppBinary {
  param([string]$Mode)
  $binName = if ($Mode -eq "app-path") { "ta_app_path_faceoff" } else { "ta_cli" }
  $featureArg = if ($Mode -eq "app-path") { "cuda,cuda-backtest-kernel" } else { "cli,cuda,cuda-backtest-kernel" }
  $cmdArgs = @("build", "-p", "ta_desktop_demo_app", "--release", "--bin", $binName, "--features", $featureArg)
  $null = Invoke-LoggedCommand -Exe "cargo" -CmdArgs $cmdArgs -StdoutPath (Join-Path $outPath "logs/build.stdout.txt") -StderrPath (Join-Path $outPath "logs/build.stderr.txt")
}

function New-ToolDirs {
  param(
    [string]$Tool,
    [string]$Backend
  )
  $base = Join-Path $outPath (Join-Path $Tool $Backend)
  $trialsDir = Join-Path $base "trials"
  $logsDir = Join-Path $base "logs"
  New-Item -ItemType Directory -Force -Path $trialsDir | Out-Null
  New-Item -ItemType Directory -Force -Path $logsDir | Out-Null
  return [ordered]@{ base = $base; trials = $trialsDir; logs = $logsDir }
}

function Sort-VectorGridBackends {
  param([string[]]$Backends)
  $preferred = @("gpu-kernel", "cpu")
  $ordered = New-Object System.Collections.Generic.List[string]
  foreach ($backend in $preferred) {
    if ($Backends -contains $backend -and -not $ordered.Contains($backend)) { $ordered.Add($backend) }
  }
  foreach ($backend in $Backends) {
    if (-not $ordered.Contains($backend)) { $ordered.Add($backend) }
  }
  return @($ordered)
}

function Invoke-VectorGridCliTrial {
  param([string]$BackendName, [int]$TrialIndex, [bool]$IsWarmup, [string]$DatasetCsv)
  $dirs = New-ToolDirs -Tool "vectorgrid" -Backend $BackendName
  $phase = if ($IsWarmup) { "warmup" } else { "measured" }
  $stem = "{0:D2}_{1}" -f $TrialIndex, $phase
  $stdoutPath = Join-Path $dirs.logs "$stem.stdout.txt"
  $stderrPath = Join-Path $dirs.logs "$stem.stderr.txt"
  $resultPath = Join-Path $dirs.trials "$stem.json"
  $cmdArgs = @("--backend", $BackendName, "--fast-ma", "alma", "--slow-ma", "alma", "--csv", $DatasetCsv, "--fast-period", $fastRange.spec, "--slow-period", $slowRange.spec, "--alma-offset", "$almaOffset", "--alma-sigma", "$almaSigma", "--top-k", "$topK", "--heatmap-bins", "$heatmapBins")
  $run = Invoke-LoggedCommand -Exe $taCliPath -CmdArgs $cmdArgs -StdoutPath $stdoutPath -StderrPath $stderrPath
  $parsed = $run.stdout | ConvertFrom-Json
  $boundary = Get-VectorGridBoundaryMetadata -Mode "cli-cold" -ParsedBatch $null
  $artifact = [ordered]@{
    tool = "vectorgrid"; backend = $BackendName; phase = $phase; status = "completed"; failure_reason = $null; metric_mode = $expectedVectorGridMetrics.label; metric_count = $expectedVectorGridMetrics.count; trial_index = $TrialIndex; runtime_ms_wall = $run.elapsed_ms
    timing_boundary = $boundary.timing_boundary; timing_boundary_note = $boundary.timing_boundary_note; build_profile = $boundary.build_profile
    dataset_csv = $DatasetCsv; pair_count = $parsed.num_combos; bars = $parsed.num_candles; pair_bars = [int64]$parsed.num_combos * [int64]$parsed.num_candles
    backend_used = $parsed.backend_used; backend_route = $parsed.backend_route
    best = [ordered]@{ fast = $parsed.best_params.fast_len; slow = $parsed.best_params.slow_len; sharpe = $parsed.best_metrics.sharpe; pnl = $parsed.best_metrics.pnl }
    peak_working_set_bytes = $null; peak_private_bytes = $null
    result = $parsed; stdout_path = $stdoutPath; stderr_path = $stderrPath; generated_at = (Get-Date).ToString("o")
  }
  $artifact | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 $resultPath
  return $artifact
}

function Invoke-VectorGridAppPathBatch {
  param([string]$BackendName, [string]$DatasetCsv)
  $dirs = New-ToolDirs -Tool "vectorgrid" -Backend $BackendName
  $stdoutPath = Join-Path $dirs.logs "batch.stdout.txt"
  $stderrPath = Join-Path $dirs.logs "batch.stderr.txt"
  $resultPath = Join-Path $dirs.base "batch.json"
  $cmdArgs = @("--backend", $BackendName, "--csv", $DatasetCsv, "--output-json", $resultPath, "--fast-ma", "alma", "--slow-ma", "alma", "--ma-source", "close", "--fast-period", $fastRange.spec, "--slow-period", $slowRange.spec, "--alma-offset", "$almaOffset", "--alma-sigma", "$almaSigma", "--top-k", "$topK", "--heatmap-bins", "$heatmapBins", "--warmup-trials", "$WarmupTrials", "--measured-trials", "$Trials")
  $null = Invoke-LoggedCommand -Exe $taAppPathExe -CmdArgs $cmdArgs -StdoutPath $stdoutPath -StderrPath $stderrPath
  $parsed = Read-JsonFile -Path $resultPath
  $boundary = Get-VectorGridBoundaryMetadata -Mode "app-path" -ParsedBatch $parsed
  $routeSummary = Get-JsonPropertyOrDefault -Object $parsed -Name "route_summary" -Default (Get-JsonPropertyOrDefault -Object $parsed -Name "measured_route_summary" -Default $null)
  $convertTrials = {
    param([object[]]$TrialsInput, [string]$Phase)
    $items = New-Object System.Collections.Generic.List[object]
    foreach ($trial in @($TrialsInput)) {
      $items.Add([ordered]@{
        tool = "vectorgrid"; backend = $BackendName; phase = $Phase; status = "completed"; failure_reason = $null; metric_mode = $expectedVectorGridMetrics.label; metric_count = $expectedVectorGridMetrics.count; trial_index = [int]$trial.trial_index; runtime_ms_wall = [double]$trial.runtime_ms_wall
        timing_boundary = $boundary.timing_boundary; timing_boundary_note = $boundary.timing_boundary_note; build_profile = $boundary.build_profile
        dataset_csv = $DatasetCsv; pair_count = $trial.result.num_combos; bars = $trial.result.num_candles; pair_bars = [int64]$trial.result.num_combos * [int64]$trial.result.num_candles
        backend_used = $trial.result.backend_used; backend_route = $trial.result.backend_route
        app_path_timing = (Get-JsonPropertyOrDefault -Object $trial -Name "timing" -Default $null)
        app_path_timing_summary = (Get-JsonPropertyOrDefault -Object (Get-JsonPropertyOrDefault -Object $parsed -Name "timing_summary" -Default $null) -Name $Phase -Default $null)
        route_summary = $routeSummary
        best = [ordered]@{ fast = $trial.result.best_params.fast_len; slow = $trial.result.best_params.slow_len; sharpe = $trial.result.best_metrics.sharpe; pnl = $trial.result.best_metrics.pnl }
        peak_working_set_bytes = $null; peak_private_bytes = $null
        result = $trial.result; stdout_path = $stdoutPath; stderr_path = $stderrPath; generated_at = (Get-Date).ToString("o")
      })
    }
    return $items
  }
  return [ordered]@{ warmups = & $convertTrials $parsed.warmup_trials "warmup"; measured = & $convertTrials $parsed.measured_trials "measured" }
}

function Invoke-VectorBtTrial {
  param([int]$TrialIndex, [bool]$IsWarmup, [string]$DatasetCsv, [hashtable]$Python)
  $dirs = New-ToolDirs -Tool "vectorbt" -Backend "cpu"
  $phase = if ($IsWarmup) { "warmup" } else { "measured" }
  $stem = "{0:D2}_{1}" -f $TrialIndex, $phase
  $stdoutPath = Join-Path $dirs.logs "$stem.stdout.txt"
  $stderrPath = Join-Path $dirs.logs "$stem.stderr.txt"
  $resultPath = Join-Path $dirs.trials "$stem.json"
  $toolJsonPath = Join-Path $dirs.trials "$stem.tool.json"
  $scriptPath = Join-Path $repoRoot "scripts/vectorbt_double_ma_faceoff.py"
  $cmdArgs = @()
  $cmdArgs += $Python.base_args
  $cmdArgs += @($scriptPath, "--csv", $DatasetCsv, "--benchmark-mode", $benchmarkMode, "--fast-range", $fastRange.spec, "--slow-range", $slowRange.spec, "--alma-offset", "$almaOffset", "--alma-sigma", "$almaSigma", "--fees", "$fees", "--hurdle-tau", "$hurdleTau", "--chunk-size", "$vectorBtChunkSize", "--cache-max-entries", "$vectorBtCacheMaxEntries", "--long-only", ($(if ($longOnly) { "true" } else { "false" })), "--allow-flip", ($(if ($allowFlip) { "true" } else { "false" })), "--trade-on-next-bar", ($(if ($tradeOnNextBar) { "true" } else { "false" })), "--output-json", $toolJsonPath)
  $run = Invoke-ObservedCommand -Exe $Python.exe -CmdArgs $cmdArgs -StdoutPath $stdoutPath -StderrPath $stderrPath -TimeoutSec $vectorBtTimeoutSec
  $status = Classify-ObservedStatus -Run $run
  $failureReason = if ($status -eq "completed") {
    $null
  } elseif ($run.timed_out) {
    "timeout_sec=$vectorBtTimeoutSec"
  } elseif (-not [string]::IsNullOrWhiteSpace($run.stderr)) {
    ($run.stderr -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1)
  } elseif (-not [string]::IsNullOrWhiteSpace($run.stdout)) {
    ($run.stdout -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1)
  } else {
    "exit_code=$($run.exit_code)"
  }
  $parsed = $null
  if ((Test-Path $toolJsonPath) -and $status -eq "completed") { $parsed = Read-JsonFile -Path $toolJsonPath }
  $boundary = Get-VectorBtBoundaryMetadata
  $artifact = [ordered]@{
    tool = "vectorbt"; backend = "cpu"; phase = $phase; status = $status; failure_reason = $failureReason; metric_mode = $benchmarkModeInfo.label; metric_count = $benchmarkModeInfo.metric_count; trial_index = $TrialIndex; runtime_ms_wall = $run.elapsed_ms
    timing_boundary = $boundary.timing_boundary; timing_boundary_note = $boundary.timing_boundary_note; build_profile = $boundary.build_profile
    exit_code = $run.exit_code; timed_out = $run.timed_out; peak_working_set_bytes = $run.peak_working_set_bytes; peak_private_bytes = $run.peak_private_bytes
    dataset_csv = $DatasetCsv; pair_count = if ($null -ne $parsed) { $parsed.pair_count } else { $intendedPairCount }; bars = if ($null -ne $parsed) { $parsed.dataset.bars } else { $datasetInfo.bars }
    pair_bars = if ($null -ne $parsed) { $parsed.pair_bars } else { [int64]$intendedPairCount * [int64]$datasetInfo.bars }
    best = if ($null -ne $parsed) { [ordered]@{ fast = $parsed.best.fast; slow = $parsed.best.slow; sharpe = $parsed.best.sharpe; pnl = $parsed.best.pnl } } else { $null }
    result = $parsed; stdout_path = $stdoutPath; stderr_path = $stderrPath; generated_at = (Get-Date).ToString("o")
  }
  $artifact | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 $resultPath
  return $artifact
}

function Get-TrialStats {
  param([object[]]$Trials)
  $completed = @($Trials | Where-Object { $_.status -eq "completed" })
  $times = @($completed | ForEach-Object { [double]$_.runtime_ms_wall } | Sort-Object)
  if ($times.Count -eq 0) { return [ordered]@{ count = 0; min_ms = $null; median_ms = $null; max_ms = $null } }
  $median = if ($times.Count % 2 -eq 1) { $times[[int]($times.Count / 2)] } else { ($times[($times.Count / 2) - 1] + $times[$times.Count / 2]) / 2.0 }
  return [ordered]@{ count = $times.Count; min_ms = [math]::Round($times[0], 3); median_ms = [math]::Round($median, 3); max_ms = [math]::Round($times[-1], 3) }
}

function Get-CanonicalTrial {
  param([object[]]$Trials)

  $completed = @($Trials | Where-Object { $_.status -eq "completed" })
  if ($completed.Count -eq 0) {
    if ($Trials.Count -gt 0) {
      return $Trials[0]
    }
    return $null
  }

  $stats = Get-TrialStats -Trials $completed
  $targetMedian = [double]$stats.median_ms
  return @(
    $completed |
      Sort-Object `
        @{ Expression = { [math]::Abs([double]$_.runtime_ms_wall - $targetMedian) } }, `
        @{ Expression = { [double]$_.runtime_ms_wall } }, `
        @{ Expression = { [int]$_.trial_index } }
  )[0]
}

function Compare-Parity {
  param([object]$Reference, [object]$Candidate, [string]$CandidateLabel)
  if ($null -eq $Reference -or $null -eq $Candidate) { return $null }
  if ($Reference.status -ne "completed" -or $Candidate.status -ne "completed") { return $null }
  $sameParams = ($Reference.best.fast -eq $Candidate.best.fast) -and ($Reference.best.slow -eq $Candidate.best.slow)
  $sharpeDelta = [math]::Abs([double]$Reference.best.sharpe - [double]$Candidate.best.sharpe)
  $status = if ($sameParams -and $sharpeDelta -le 1e-9) { "exact" } elseif ($sameParams) { "params_match_metric_diff" } else { "best_params_differ" }
  return [ordered]@{ reference = "vectorgrid_cpu"; candidate = $CandidateLabel; status = $status; same_best_params = $sameParams; sharpe_delta = $sharpeDelta }
}

function Format-TableLine {
  param([string[]]$Cells)
  return "| " + ($Cells -join " | ") + " |"
}

function New-FaceoffSummary {
  $summaryArtifacts = New-Object System.Collections.Generic.List[object]
  $seen = @{}
  foreach ($artifact in @($measuredArtifacts.ToArray())) {
    $summaryArtifacts.Add($artifact)
    $seen["$($artifact.tool)|$($artifact.backend)"] = $true
  }
  foreach ($artifact in @($warmupArtifacts.ToArray() | Where-Object { $_.status -ne "completed" })) {
    $key = "$($artifact.tool)|$($artifact.backend)"
    if (-not $seen.ContainsKey($key)) {
      $summaryArtifacts.Add($artifact)
      $seen[$key] = $true
    }
  }

  $grouped = $summaryArtifacts | Group-Object { "$($_.tool)|$($_.backend)" }
  $summaryRows = New-Object System.Collections.Generic.List[object]
  foreach ($group in $grouped) {
    $items = @($group.Group)
    $completed = @($items | Where-Object { $_.status -eq "completed" })
    $failed = @($items | Where-Object { $_.status -ne "completed" })
    $stats = Get-TrialStats -Trials $items
    $canonical = Get-CanonicalTrial -Trials $items
    $rowStatus = if ($completed.Count -eq $items.Count) { "completed" } elseif ($completed.Count -gt 0) { "partial" } else { $canonical.status }
    $metricCount = if (
      $canonical.result -and
      $canonical.result.PSObject.Properties.Name -contains "metric_payload" -and
      $canonical.result.metric_payload
    ) {
      $canonical.result.metric_payload.count
    } elseif (
      $canonical.result -and
      $canonical.result.PSObject.Properties.Name -contains "request" -and
      $canonical.result.request -and
      $canonical.result.request.PSObject.Properties.Name -contains "metric_contract" -and
      $canonical.result.request.metric_contract
    ) {
      $canonical.result.request.metric_contract.expected_metrics_count
    } elseif ($null -ne $canonical.metric_count) {
      $canonical.metric_count
    } else {
      $null
    }
    $summaryRows.Add([ordered]@{
      tool = $canonical.tool; backend = $canonical.backend; status = $rowStatus; metric_mode = $canonical.metric_mode; failure_reason = if ($failed.Count -gt 0) { $failed[0].failure_reason } else { $canonical.failure_reason }; min_ms = $stats.min_ms; median_ms = $stats.median_ms
      canonical_trial_index = $canonical.trial_index; timing_boundary = $canonical.timing_boundary; timing_boundary_note = $canonical.timing_boundary_note; build_profile = $canonical.build_profile
      pair_count = $canonical.pair_count; bars = $canonical.bars
      metric_count = $metricCount
      backend_route = if ($canonical.tool -eq "vectorgrid") { $canonical.backend_route } else { "cpu_python_workflow" }
      app_path_timing_summary = $(if ($canonical.PSObject.Properties.Name -contains "app_path_timing_summary") { $canonical.app_path_timing_summary } else { $null })
      app_path_timing = $(if ($canonical.PSObject.Properties.Name -contains "app_path_timing") { $canonical.app_path_timing } else { $null })
      route_summary = $(if ($canonical.PSObject.Properties.Name -contains "route_summary") { $canonical.route_summary } else { $null })
      best = $canonical.best; peak_working_set_bytes = $canonical.peak_working_set_bytes; peak_private_bytes = $canonical.peak_private_bytes
    })
  }
  $vectorGridCpu = Get-CanonicalTrial -Trials @($measuredArtifacts | Where-Object { $_.tool -eq "vectorgrid" -and $_.backend -eq "cpu" })
  $vectorGridGpu = Get-CanonicalTrial -Trials @($measuredArtifacts | Where-Object { $_.tool -eq "vectorgrid" -and $_.backend -eq "gpu-kernel" })
  $vectorBtCpu = Get-CanonicalTrial -Trials @($measuredArtifacts | Where-Object { $_.tool -eq "vectorbt" })
  $parity = @()
  $p = Compare-Parity -Reference $vectorGridCpu -Candidate $vectorBtCpu -CandidateLabel "vectorbt_cpu"
  if ($null -ne $p) { $parity += $p }
  $p = Compare-Parity -Reference $vectorGridCpu -Candidate $vectorGridGpu -CandidateLabel "vectorgrid_gpu_kernel"
  if ($null -ne $p) { $parity += $p }
  $vectorBtEnv = if ($null -ne $vectorBtCpu -and $null -ne $vectorBtCpu.result) { $vectorBtCpu.result.environment } else { $null }
  return [ordered]@{
    generated_at = (Get-Date).ToString("o")
    scenario_path = $scenarioFullPath.Path
    out_dir = $outPath
    dataset = $datasetInfo
    environment = [ordered]@{
      hardware = Get-CommandHardware
      python = [ordered]@{
        command = $python.display
        executable = if ($null -ne $vectorBtEnv -and $vectorBtEnv.PSObject.Properties.Name -contains "executable") { $vectorBtEnv.executable } else { $resolvedPythonExe }
        version = if ($null -ne $vectorBtEnv) { $vectorBtEnv.python } else { "unknown" }
        platform = if ($null -ne $vectorBtEnv) { $vectorBtEnv.platform } else { "unknown" }
        base_args = @($python.base_args)
        preferred_venv_python = $resolvedPreferredVenvPython
        auto_selected_preferred_venv = $usedPreferredVenv
        using_preferred_venv = $pythonUsesPreferredVenv
        requirements_lock = $pythonProvenance.lockfile
        pip = $pythonProvenance.pip
      }
      vectorbt = [ordered]@{
        version = if ($null -ne $vectorBtEnv) { $vectorBtEnv.vectorbt } else { "unknown" }
        numpy = if ($null -ne $vectorBtEnv) { $vectorBtEnv.numpy } else { "unknown" }
        pandas = if ($null -ne $vectorBtEnv) { $vectorBtEnv.pandas } else { "unknown" }
        numba = if ($null -ne $vectorBtEnv) { $vectorBtEnv.numba } else { "unknown" }
      }
    }
    scenario = [ordered]@{
      scenario_id = $scenario.scenario_id; preset_id = $presetId; benchmark_mode = $benchmarkMode; objective = $scenario.objective
      intended_metric_contract = $intendedMetricContract; intended_pair_count = $intendedPairCount; vectorgrid_mode = $VectorGridMode
      execution_order = @("vectorgrid_gpu_kernel", "vectorgrid_cpu", "vectorbt_cpu"); fast_range = $fastRange.spec; slow_range = $slowRange.spec
      warmup_trials = $WarmupTrials; measured_trials = $Trials
    }
    warmup_artifacts = $warmupArtifacts
    measured_artifacts = $measuredArtifacts
    summary_rows = $summaryRows
    parity = $parity
  }
}

function Write-MarkdownSummary {
  param([string]$Path, [object]$Summary)
  $lines = New-Object System.Collections.Generic.List[string]
  $lines.Add("# Double-MA Faceoff Summary")
  $lines.Add("")
  $lines.Add(("Scenario `{0}` (`{1}`), preset `{2}`, objective `{3}`, intended pairs `{4}`." -f $Summary.scenario.scenario_id, $Summary.scenario.benchmark_mode, $Summary.scenario.preset_id, $Summary.scenario.objective, $Summary.scenario.intended_pair_count))
  $lines.Add("")
  $lines.Add((Format-TableLine -Cells @("Tool", "Backend", "Boundary", "Build", "Status", "Metric mode", "Median ms", "Pairs", "Metrics", "Best fast", "Best slow", "Best Sharpe", "Route")))
  $lines.Add((Format-TableLine -Cells @("---", "---", "---", "---", "---", "---", "---", "---", "---", "---", "---", "---", "---")))
  foreach ($row in $Summary.summary_rows) {
    $bestSharpe = ""
    $bestFast = ""
    $bestSlow = ""
    if ($null -ne $row.best) {
      $bestSharpe = "{0:N12}" -f [double]$row.best.sharpe
      $bestFast = [string]$row.best.fast
      $bestSlow = [string]$row.best.slow
    }
    $lines.Add((Format-TableLine -Cells @($row.tool, $row.backend, [string]$row.timing_boundary, [string]$row.build_profile, $row.status, [string]$row.metric_mode, [string]$row.median_ms, [string]$row.pair_count, [string]$row.metric_count, $bestFast, $bestSlow, $bestSharpe, [string]$row.backend_route)))
    if ($row.failure_reason) {
      $lines.Add(("- {0}/{1} failure: {2}" -f $row.tool, $row.backend, $row.failure_reason))
    }
    if ($row.app_path_timing_summary) {
      $stateLookupMedian = Get-JsonPropertyOrDefault -Object $row.app_path_timing_summary -Name "state_lookup_ms_median" -Default (Get-JsonPropertyOrDefault -Object $row.app_path_timing -Name "state_lookup_ms" -Default $null)
      $engineWallMedian = Get-JsonPropertyOrDefault -Object $row.app_path_timing_summary -Name "engine_wall_ms_median" -Default (Get-JsonPropertyOrDefault -Object $row.app_path_timing -Name "engine_wall_ms" -Default $null)
      $lines.Add(("- {0}/{1} app-path measured medians: harness_wall_ms={2}, engine_runtime_ms={3}, state_lookup_ms={4}, engine_wall_ms={5}" -f $row.tool, $row.backend, [string]$row.app_path_timing_summary.harness_wall_ms_median, [string]$row.app_path_timing_summary.engine_runtime_ms_median, [string]$stateLookupMedian, [string]$engineWallMedian))
    }
    if ($row.route_summary) {
      $lines.Add(("- {0}/{1} route summary: {2}" -f $row.tool, $row.backend, (($row.route_summary | ConvertTo-Json -Compress -Depth 8))))
    }
  }
  Set-Content -Path $Path -Value ($lines -join "`n") -Encoding UTF8
}

function Flush-FaceoffSummary {
  $summary = New-FaceoffSummary
  $summary | ConvertTo-Json -Depth 14 | Set-Content -Encoding UTF8 $summaryPath
  Write-MarkdownSummary -Path $markdownPath -Summary $summary
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$preferredVenvPython = Join-Path $repoRoot ".venv-vectorbt-faceoff\Scripts\python.exe"
$usedPreferredVenv = $false
if ($PythonExe -in @("py", "py.exe") -and (Test-Path $preferredVenvPython)) {
  $PythonExe = $preferredVenvPython
  $PythonArgs = @()
  $usedPreferredVenv = $true
}

$scenarioFullPath = Resolve-Path $ScenarioPath
$scenario = Read-JsonFile -Path $scenarioFullPath
$benchmarkMode = if (-not [string]::IsNullOrWhiteSpace($VectorBtBenchmarkMode)) {
  $VectorBtBenchmarkMode
} elseif ($scenario.PSObject.Properties.Name -contains "benchmark_mode") {
  [string]$scenario.benchmark_mode
} elseif ($scenario.vectorbt -and $scenario.vectorbt.PSObject.Properties.Name -contains "benchmark_mode") {
  [string]$scenario.vectorbt.benchmark_mode
} else {
  "search_only_legacy"
}
$benchmarkModeInfo = Get-BenchmarkModeContractInfo -BenchmarkMode $benchmarkMode
$presetId = if ($scenario.PSObject.Properties.Name -contains "preset_id") { [string]$scenario.preset_id } else { [string]$scenario.scenario_id }
$intendedMetricContract = if ($scenario.PSObject.Properties.Name -contains "intended_metric_contract") { [string]$scenario.intended_metric_contract } else { "auto" }
$fastRange = Parse-RangeSpec -Spec ([string]$scenario.ranges.fast)
$slowRange = Parse-RangeSpec -Spec ([string]$scenario.ranges.slow)
$intendedPairCount = if ($scenario.PSObject.Properties.Name -contains "intended_pair_count") { [int64]$scenario.intended_pair_count } else { Get-ValidPairCount -FastRange $fastRange -SlowRange $slowRange }
$almaOffset = [double]$scenario.alma.offset
$almaSigma = [double]$scenario.alma.sigma
$topK = [int]$scenario.execution.top_k
$heatmapBins = [int]$scenario.execution.heatmap_bins
$expectedVectorGridMetrics = Get-ExpectedVectorGridMetricMetadata -Objective ([string]$scenario.objective)
$vectorBtChunkSize = [int]$scenario.vectorbt.chunk_size
$vectorBtCacheMaxEntries = if ($VectorBtCacheMaxEntries -ge 0) { $VectorBtCacheMaxEntries } elseif ($scenario.vectorbt.PSObject.Properties.Name -contains "cache_max_entries") { [int]$scenario.vectorbt.cache_max_entries } else { 1024 }
$vectorBtTimeoutSec = if ($VectorBtTimeoutSec -ge 0) { $VectorBtTimeoutSec } elseif ($scenario.vectorbt.PSObject.Properties.Name -contains "timeout_sec") { [int]$scenario.vectorbt.timeout_sec } else { 0 }
$fees = if ($scenario.PSObject.Properties.Name -contains "strategy_config" -and $scenario.strategy_config.PSObject.Properties.Name -contains "fees") { [double]$scenario.strategy_config.fees } else { 0.0 }
$hurdleTau = if ($scenario.PSObject.Properties.Name -contains "strategy_config" -and $scenario.strategy_config.PSObject.Properties.Name -contains "hurdle_tau") { [double]$scenario.strategy_config.hurdle_tau } else { 0.0 }
$longOnly = if ($scenario.PSObject.Properties.Name -contains "strategy_config" -and $scenario.strategy_config.PSObject.Properties.Name -contains "long_only") { [bool]$scenario.strategy_config.long_only } else { $true }
$allowFlip = if ($scenario.PSObject.Properties.Name -contains "strategy_config" -and $scenario.strategy_config.PSObject.Properties.Name -contains "allow_flip") { [bool]$scenario.strategy_config.allow_flip } else { $true }
$tradeOnNextBar = if ($scenario.PSObject.Properties.Name -contains "strategy_config" -and $scenario.strategy_config.PSObject.Properties.Name -contains "trade_on_next_bar") { [bool]$scenario.strategy_config.trade_on_next_bar } else { $true }

if ($WarmupTrials -lt 0) { $WarmupTrials = [int]$scenario.trials.warmup }
if ($Trials -lt 0) { $Trials = [int]$scenario.trials.measured }
if ($VectorGridBackends.Count -eq 0) { $VectorGridBackends = @($scenario.vectorgrid.backends) }
$VectorGridBackends = Sort-VectorGridBackends -Backends $VectorGridBackends

$safePreset = ($presetId -replace '[^A-Za-z0-9._-]+', '_')
$outPath = Join-Path $repoRoot (Join-Path $OutDir "$safePreset-$benchmarkMode")
New-Item -ItemType Directory -Force -Path $outPath | Out-Null
$summaryPath = Join-Path $outPath "summary.json"
$markdownPath = Join-Path $outPath "summary.md"
$python = Get-PythonCommand -Exe $PythonExe -BaseArgs $PythonArgs
$resolvedPreferredVenvPython = Resolve-ExistingPathOrValue -Path $preferredVenvPython
$resolvedPythonExe = if ($python.exe -and $python.exe -notin @("py", "py.exe")) { Resolve-ExistingPathOrValue -Path $python.exe } else { $python.exe }
$pythonUsesPreferredVenv = $false
if ($null -ne $resolvedPreferredVenvPython -and $null -ne $resolvedPythonExe) {
  $pythonUsesPreferredVenv = [string]::Equals($resolvedPreferredVenvPython, $resolvedPythonExe, [System.StringComparison]::OrdinalIgnoreCase)
}
$pythonProvenance = Capture-PythonProvenance -Python $python -OutDir $outPath -RepoRoot $repoRoot

$datasetCsv = $Csv
if ([string]::IsNullOrWhiteSpace($datasetCsv)) { $datasetCsv = Join-Path $outPath "faceoff_dataset_synth_200k.csv" }
$datasetMeta = Join-Path $outPath "faceoff_dataset_synth_200k.meta.json"
Ensure-FrozenDataset -CsvPath $datasetCsv -MetaPath $datasetMeta -SynthLen ([int]$scenario.dataset.synth_len) -Python $python
$datasetInfo = Read-JsonFile -Path $datasetMeta

if (-not $SkipBuild) { Build-AppBinary -Mode $VectorGridMode }

$taCliPath = Join-Path $repoRoot "target/release/ta_cli.exe"
$taAppPathExe = Join-Path $repoRoot "target/release/ta_app_path_faceoff.exe"
$vectorGridExe = if ($VectorGridMode -eq "app-path") { $taAppPathExe } else { $taCliPath }
if (-not (Test-Path $vectorGridExe)) { throw "Missing built app binary: $vectorGridExe" }

$warmupArtifacts = New-Object System.Collections.Generic.List[object]
$measuredArtifacts = New-Object System.Collections.Generic.List[object]

foreach ($backend in $VectorGridBackends) {
  if ($VectorGridMode -eq "app-path") {
    $batch = Invoke-VectorGridAppPathBatch -BackendName $backend -DatasetCsv $datasetCsv
    foreach ($artifact in @($batch.warmups)) { $warmupArtifacts.Add($artifact) }
    foreach ($artifact in @($batch.measured)) { $measuredArtifacts.Add($artifact) }
  } else {
    for ($i = 1; $i -le $WarmupTrials; $i += 1) {
      $warmupArtifacts.Add((Invoke-VectorGridCliTrial -BackendName $backend -TrialIndex $i -IsWarmup $true -DatasetCsv $datasetCsv))
    }
    for ($i = 1; $i -le $Trials; $i += 1) {
      $measuredArtifacts.Add((Invoke-VectorGridCliTrial -BackendName $backend -TrialIndex $i -IsWarmup $false -DatasetCsv $datasetCsv))
    }
  }
  Flush-FaceoffSummary
}

$vectorBtContinue = $true
for ($i = 1; $i -le $WarmupTrials -and $vectorBtContinue; $i += 1) {
  $artifact = Invoke-VectorBtTrial -TrialIndex $i -IsWarmup $true -DatasetCsv $datasetCsv -Python $python
  $warmupArtifacts.Add($artifact)
  if ($artifact.status -ne "completed") { $vectorBtContinue = $false }
}
for ($i = 1; $i -le $Trials -and $vectorBtContinue; $i += 1) {
  $artifact = Invoke-VectorBtTrial -TrialIndex $i -IsWarmup $false -DatasetCsv $datasetCsv -Python $python
  $measuredArtifacts.Add($artifact)
  if ($artifact.status -ne "completed") { $vectorBtContinue = $false }
}

Flush-FaceoffSummary

Write-Host "Faceoff summary written to $summaryPath"
Write-Host "Markdown summary written to $markdownPath"
