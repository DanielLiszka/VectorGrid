param(
  [string]$ScenarioPath = "scripts/vectorbt_double_ma_faceoff_scenario.json",
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
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

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
    return [ordered]@{
      exe = $Exe
      base_args = @()
      display = $Exe
    }
  }

  return [ordered]@{
    exe = $Exe
    base_args = $BaseArgs
    display = "$Exe $($BaseArgs -join ' ')"
  }
}

function Invoke-LoggedCommand {
  param(
    [string]$Exe,
    [string[]]$CmdArgs,
    [string]$StdoutPath,
    [string]$StderrPath
  )

  $stdoutDir = Split-Path -Parent $StdoutPath
  if ($stdoutDir -and -not (Test-Path $stdoutDir)) {
    New-Item -ItemType Directory -Force -Path $stdoutDir | Out-Null
  }
  $stderrDir = Split-Path -Parent $StderrPath
  if ($stderrDir -and -not (Test-Path $stderrDir)) {
    New-Item -ItemType Directory -Force -Path $stderrDir | Out-Null
  }

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

  $proc = [System.Diagnostics.Process]::new()
  $proc.StartInfo = $psi
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
  }
}

function Get-CommandHardware {
  $cpu = Get-CimInstance Win32_Processor | Select-Object -First 1 Name, NumberOfCores, NumberOfLogicalProcessors
  $gpu = Get-CimInstance Win32_VideoController |
    Sort-Object -Property AdapterRAM -Descending |
    Select-Object -First 1 Name, AdapterRAM
  $sys = Get-CimInstance Win32_ComputerSystem | Select-Object -First 1 TotalPhysicalMemory

  return [ordered]@{
    cpu = [ordered]@{
      name = $cpu.Name
      cores = $cpu.NumberOfCores
      logical_processors = $cpu.NumberOfLogicalProcessors
    }
    gpu = [ordered]@{
      name = $gpu.Name
      adapter_ram_bytes = $gpu.AdapterRAM
    }
    ram_bytes = $sys.TotalPhysicalMemory
  }
}

function Ensure-FrozenDataset {
  param(
    [string]$CsvPath,
    [string]$MetaPath,
    [int]$SynthLen,
    [hashtable]$Python
  )

  if ((Test-Path $CsvPath) -and (Test-Path $MetaPath)) {
    return
  }

  $scriptPath = Join-Path $repoRoot "scripts/generate_double_ma_faceoff_dataset.py"
  $cmdArgs = @()
  $cmdArgs += $Python.base_args
  $cmdArgs += @(
    $scriptPath,
    "--synth-len", "$SynthLen",
    "--output-csv", $CsvPath,
    "--output-json", $MetaPath
  )
  $null = Invoke-LoggedCommand `
    -Exe $Python.exe `
    -CmdArgs $cmdArgs `
    -StdoutPath (Join-Path $outPath "logs/dataset.stdout.txt") `
    -StderrPath (Join-Path $outPath "logs/dataset.stderr.txt")
}

function Build-AppBinary {
  param([string]$Mode)

  $binName = if ($Mode -eq "app-path") { "ta_app_path_faceoff" } else { "ta_cli" }
  $featureArg = if ($Mode -eq "app-path") { "cuda,cuda-backtest-kernel" } else { "cli,cuda,cuda-backtest-kernel" }
  $cmdArgs = @(
    "build", "-p", "ta_desktop_demo_app",
    "--release",
    "--bin", $binName,
    "--features", $featureArg
  )
  $null = Invoke-LoggedCommand `
    -Exe "cargo" `
    -CmdArgs $cmdArgs `
    -StdoutPath (Join-Path $outPath "logs/build.stdout.txt") `
    -StderrPath (Join-Path $outPath "logs/build.stderr.txt")
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
  return [ordered]@{
    base = $base
    trials = $trialsDir
    logs = $logsDir
  }
}

function Invoke-VectorGridCliTrial {
  param(
    [string]$BackendName,
    [int]$TrialIndex,
    [bool]$IsWarmup,
    [string]$DatasetCsv
  )

  $dirs = New-ToolDirs -Tool "vectorgrid" -Backend $BackendName
  $phase = if ($IsWarmup) { "warmup" } else { "measured" }
  $stem = "{0:D2}_{1}" -f $TrialIndex, $phase
  $stdoutPath = Join-Path $dirs.logs "$stem.stdout.txt"
  $stderrPath = Join-Path $dirs.logs "$stem.stderr.txt"
  $resultPath = Join-Path $dirs.trials "$stem.json"

  $cmdArgs = @(
    "--backend", $BackendName,
    "--fast-ma", "alma",
    "--slow-ma", "alma",
    "--csv", $DatasetCsv,
    "--fast-period", $fastRange.spec,
    "--slow-period", $slowRange.spec,
    "--alma-offset", "$almaOffset",
    "--alma-sigma", "$almaSigma",
    "--top-k", "$topK",
    "--heatmap-bins", "$heatmapBins"
  )

  $run = Invoke-LoggedCommand -Exe $taCliPath -CmdArgs $cmdArgs -StdoutPath $stdoutPath -StderrPath $stderrPath
  $parsed = $run.stdout | ConvertFrom-Json
  $artifact = [ordered]@{
    tool = "vectorgrid"
    backend = $BackendName
    phase = $phase
    trial_index = $TrialIndex
    runtime_ms_wall = $run.elapsed_ms
    dataset_csv = $DatasetCsv
    pair_count = $parsed.num_combos
    bars = $parsed.num_candles
    pair_bars = [int64]$parsed.num_combos * [int64]$parsed.num_candles
    backend_used = $parsed.backend_used
    backend_route = $parsed.backend_route
    best = [ordered]@{
      fast = $parsed.best_params.fast_len
      slow = $parsed.best_params.slow_len
      sharpe = $parsed.best_metrics.sharpe
      pnl = $parsed.best_metrics.pnl
    }
    result = $parsed
    stdout_path = $stdoutPath
    stderr_path = $stderrPath
    generated_at = (Get-Date).ToString("o")
  }
  $artifact | ConvertTo-Json -Depth 10 | Set-Content -Encoding UTF8 $resultPath
  return $artifact
}

function Invoke-VectorGridAppPathBatch {
  param(
    [string]$BackendName,
    [string]$DatasetCsv
  )

  $dirs = New-ToolDirs -Tool "vectorgrid" -Backend $BackendName
  $stdoutPath = Join-Path $dirs.logs "batch.stdout.txt"
  $stderrPath = Join-Path $dirs.logs "batch.stderr.txt"
  $resultPath = Join-Path $dirs.base "batch.json"

  $cmdArgs = @(
    "--backend", $BackendName,
    "--csv", $DatasetCsv,
    "--output-json", $resultPath,
    "--fast-ma", "alma",
    "--slow-ma", "alma",
    "--ma-source", "close",
    "--fast-period", $fastRange.spec,
    "--slow-period", $slowRange.spec,
    "--alma-offset", "$almaOffset",
    "--alma-sigma", "$almaSigma",
    "--top-k", "$topK",
    "--heatmap-bins", "$heatmapBins",
    "--warmup-trials", "$WarmupTrials",
    "--measured-trials", "$Trials"
  )

  $null = Invoke-LoggedCommand -Exe $taAppPathExe -CmdArgs $cmdArgs -StdoutPath $stdoutPath -StderrPath $stderrPath
  $parsed = Read-JsonFile -Path $resultPath

  $convertTrials = {
    param([object[]]$TrialsInput, [string]$Phase)
    $items = New-Object System.Collections.Generic.List[object]
    foreach ($trial in @($TrialsInput)) {
      $items.Add([ordered]@{
        tool = "vectorgrid"
        backend = $BackendName
        phase = $Phase
        trial_index = [int]$trial.trial_index
        runtime_ms_wall = [double]$trial.runtime_ms_wall
        dataset_csv = $DatasetCsv
        pair_count = $trial.result.num_combos
        bars = $trial.result.num_candles
        pair_bars = [int64]$trial.result.num_combos * [int64]$trial.result.num_candles
        backend_used = $trial.result.backend_used
        backend_route = $trial.result.backend_route
        best = [ordered]@{
          fast = $trial.result.best_params.fast_len
          slow = $trial.result.best_params.slow_len
          sharpe = $trial.result.best_metrics.sharpe
          pnl = $trial.result.best_metrics.pnl
        }
        result = $trial.result
        stdout_path = $stdoutPath
        stderr_path = $stderrPath
        vectorgrid_mode = "app-path"
        dataset_load_ms = $parsed.dataset.load_ms
        dataset_resample_ms = $parsed.dataset.resample_ms
        generated_at = (Get-Date).ToString("o")
      })
    }
    return $items
  }

  return [ordered]@{
    warmups = & $convertTrials $parsed.warmup_trials "warmup"
    measured = & $convertTrials $parsed.measured_trials "measured"
  }
}

function Invoke-VectorBtTrial {
  param(
    [int]$TrialIndex,
    [bool]$IsWarmup,
    [string]$DatasetCsv,
    [hashtable]$Python
  )

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
  $cmdArgs += @(
    $scriptPath,
    "--csv", $DatasetCsv,
    "--fast-range", $fastRange.spec,
    "--slow-range", $slowRange.spec,
    "--alma-offset", "$almaOffset",
    "--alma-sigma", "$almaSigma",
    "--chunk-size", "$vectorBtChunkSize",
    "--output-json", $toolJsonPath
  )

  $run = Invoke-LoggedCommand -Exe $Python.exe -CmdArgs $cmdArgs -StdoutPath $stdoutPath -StderrPath $stderrPath
  $parsed = Read-JsonFile -Path $toolJsonPath
  $artifact = [ordered]@{
    tool = "vectorbt"
    backend = "cpu"
    phase = $phase
    trial_index = $TrialIndex
    runtime_ms_wall = $run.elapsed_ms
    dataset_csv = $DatasetCsv
    pair_count = $parsed.pair_count
    bars = $parsed.dataset.bars
    pair_bars = $parsed.pair_bars
    best = [ordered]@{
      fast = $parsed.best.fast
      slow = $parsed.best.slow
      sharpe = $parsed.best.sharpe
      pnl = $parsed.best.pnl
    }
    result = $parsed
    stdout_path = $stdoutPath
    stderr_path = $stderrPath
    generated_at = (Get-Date).ToString("o")
  }
  $artifact | ConvertTo-Json -Depth 10 | Set-Content -Encoding UTF8 $resultPath
  return $artifact
}

function Get-TrialStats {
  param([object[]]$Trials)

  $times = @($Trials | ForEach-Object { [double]$_.runtime_ms_wall } | Sort-Object)
  if ($times.Count -eq 0) {
    return [ordered]@{
      count = 0
      min_ms = $null
      median_ms = $null
      max_ms = $null
    }
  }

  $median = if ($times.Count % 2 -eq 1) {
    $times[[int]($times.Count / 2)]
  } else {
    ($times[($times.Count / 2) - 1] + $times[$times.Count / 2]) / 2.0
  }

  return [ordered]@{
    count = $times.Count
    min_ms = [math]::Round($times[0], 3)
    median_ms = [math]::Round($median, 3)
    max_ms = [math]::Round($times[-1], 3)
  }
}

function Compare-Parity {
  param(
    [object]$Reference,
    [object]$Candidate,
    [string]$CandidateLabel,
    [double]$SharpeTolerance = 1e-9
  )

  $sameParams = ($Reference.best.fast -eq $Candidate.best.fast) -and ($Reference.best.slow -eq $Candidate.best.slow)
  $sharpeDelta = [math]::Abs([double]$Reference.best.sharpe - [double]$Candidate.best.sharpe)
  $status = if ($sameParams -and $sharpeDelta -le $SharpeTolerance) {
    "exact"
  } elseif ($sameParams) {
    "params_match_metric_diff"
  } else {
    "best_params_differ"
  }

  return [ordered]@{
    reference = "vectorgrid_cpu"
    candidate = $CandidateLabel
    status = $status
    same_best_params = $sameParams
    sharpe_delta = $sharpeDelta
    reference_best = $Reference.best
    candidate_best = $Candidate.best
  }
}

function Format-TableLine {
  param([string[]]$Cells)
  return "| " + ($Cells -join " | ") + " |"
}

function Write-MarkdownSummary {
  param(
    [string]$Path,
    [object]$Summary
  )

  $lines = New-Object System.Collections.Generic.List[string]
  $lines.Add("# Double-MA Faceoff Summary")
  $lines.Add("")
  $lines.Add('Scenario: `alma_alma`, `200,000` bars, `58,300` valid `fast < slow` pairs, objective `Sharpe`.')
  $lines.Add("")
  $lines.Add("## Environment")
  $lines.Add("")
  $lines.Add("- CPU: $($Summary.environment.hardware.cpu.name)")
  $lines.Add("- GPU: $($Summary.environment.hardware.gpu.name)")
  $lines.Add("- RAM bytes: $($Summary.environment.hardware.ram_bytes)")
  $lines.Add("- Python: $($Summary.environment.python.command)")
  $lines.Add("- VectorBT: $($Summary.environment.vectorbt.version)")
  $lines.Add("")
  $lines.Add("## Timings")
  $lines.Add("")
  $headerLine = Format-TableLine -Cells @("Tool", "Backend", "Median ms", "Min ms", "Pairs", "Bars", "Pair-bars", "Best fast", "Best slow", "Best Sharpe", "Route")
  $dividerLine = Format-TableLine -Cells @("---", "---", "---", "---", "---", "---", "---", "---", "---", "---", "---")
  $lines.Add($headerLine)
  $lines.Add($dividerLine)
  foreach ($row in $Summary.summary_rows) {
    $rowLine = Format-TableLine -Cells @(
      $row.tool,
      $row.backend,
      [string]$row.median_ms,
      [string]$row.min_ms,
      [string]$row.pair_count,
      [string]$row.bars,
      [string]$row.pair_bars,
      [string]$row.best.fast,
      [string]$row.best.slow,
      ("{0:N12}" -f [double]$row.best.sharpe),
      [string]$row.backend_route
    )
    $lines.Add($rowLine)
  }
  $lines.Add("")
  $lines.Add("## Parity")
  $lines.Add("")
  foreach ($parity in $Summary.parity) {
    $parityLine = "- {0}: status={1} same_best_params={2} sharpe_delta={3}" -f `
      $parity.candidate,
      $parity.status,
      $parity.same_best_params,
      ([math]::Round([double]$parity.sharpe_delta, 12))
    $lines.Add($parityLine)
  }
  $lines.Add("")
  $lines.Add("## Capture Notes")
  $lines.Add("")
  $lines.Add("- Use the shared dataset at $($Summary.dataset.csv_path) in both tools.")
  $lines.Add("- Keep timer visible through the full capture.")
  $lines.Add("- If playback is accelerated after the app finishes, keep an explicit on-screen speed label.")
  $lines.Add("- End frame should show the final elapsed times plus hardware from this summary.")

  Set-Content -Path $Path -Value ($lines -join "`n") -Encoding UTF8
}

function Get-FirstOrNull {
  param([object[]]$Items)
  if ($null -eq $Items -or $Items.Count -eq 0) {
    return $null
  }
  return $Items[0]
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$preferredVenvPython = Join-Path $repoRoot ".venv-vectorbt-faceoff\Scripts\python.exe"
if ($PythonExe -in @("py", "py.exe") -and (Test-Path $preferredVenvPython)) {
  $PythonExe = $preferredVenvPython
  $PythonArgs = @()
}

$scenarioFullPath = Resolve-Path $ScenarioPath
$scenario = Read-JsonFile -Path $scenarioFullPath
$fastRange = Parse-RangeSpec -Spec ([string]$scenario.ranges.fast)
$slowRange = Parse-RangeSpec -Spec ([string]$scenario.ranges.slow)
$almaOffset = [double]$scenario.alma.offset
$almaSigma = [double]$scenario.alma.sigma
$topK = [int]$scenario.execution.top_k
$heatmapBins = [int]$scenario.execution.heatmap_bins
$vectorBtChunkSize = [int]$scenario.vectorbt.chunk_size

if ($WarmupTrials -lt 0) {
  $WarmupTrials = [int]$scenario.trials.warmup
}
if ($Trials -lt 0) {
  $Trials = [int]$scenario.trials.measured
}

if ($VectorGridBackends.Count -eq 0) {
  $VectorGridBackends = @($scenario.vectorgrid.backends)
}

$outPath = Join-Path $repoRoot $OutDir
New-Item -ItemType Directory -Force -Path $outPath | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $outPath "logs") | Out-Null

$python = Get-PythonCommand -Exe $PythonExe -BaseArgs $PythonArgs

$datasetCsv = $Csv
if ([string]::IsNullOrWhiteSpace($datasetCsv)) {
  $datasetCsv = Join-Path $outPath "faceoff_dataset_synth_200k.csv"
}
$datasetMeta = Join-Path $outPath "faceoff_dataset_synth_200k.meta.json"
Ensure-FrozenDataset -CsvPath $datasetCsv -MetaPath $datasetMeta -SynthLen ([int]$scenario.dataset.synth_len) -Python $python
$datasetInfo = Read-JsonFile -Path $datasetMeta

if (-not $SkipBuild) {
  Build-AppBinary -Mode $VectorGridMode
}

$taCliPath = Join-Path $repoRoot "target/release/ta_cli.exe"
$taAppPathExe = Join-Path $repoRoot "target/release/ta_app_path_faceoff.exe"
$vectorGridExe = if ($VectorGridMode -eq "app-path") { $taAppPathExe } else { $taCliPath }
if (-not (Test-Path $vectorGridExe)) {
  throw "Missing built app binary: $vectorGridExe"
}

$measuredArtifacts = New-Object System.Collections.Generic.List[object]
$warmupArtifacts = New-Object System.Collections.Generic.List[object]

foreach ($backend in $VectorGridBackends) {
  if ($VectorGridMode -eq "app-path") {
    $batch = Invoke-VectorGridAppPathBatch -BackendName $backend -DatasetCsv $datasetCsv
    foreach ($artifact in @($batch.warmups)) {
      $warmupArtifacts.Add($artifact)
    }
    foreach ($artifact in @($batch.measured)) {
      $measuredArtifacts.Add($artifact)
    }
  } else {
    for ($i = 1; $i -le $WarmupTrials; $i += 1) {
      $warmupArtifacts.Add((Invoke-VectorGridCliTrial -BackendName $backend -TrialIndex $i -IsWarmup $true -DatasetCsv $datasetCsv))
    }
    for ($i = 1; $i -le $Trials; $i += 1) {
      $measuredArtifacts.Add((Invoke-VectorGridCliTrial -BackendName $backend -TrialIndex $i -IsWarmup $false -DatasetCsv $datasetCsv))
    }
  }
}

for ($i = 1; $i -le $WarmupTrials; $i += 1) {
  $warmupArtifacts.Add((Invoke-VectorBtTrial -TrialIndex $i -IsWarmup $true -DatasetCsv $datasetCsv -Python $python))
}
for ($i = 1; $i -le $Trials; $i += 1) {
  $measuredArtifacts.Add((Invoke-VectorBtTrial -TrialIndex $i -IsWarmup $false -DatasetCsv $datasetCsv -Python $python))
}

$grouped = $measuredArtifacts | Group-Object { "$($_.tool)|$($_.backend)" }
$summaryRows = New-Object System.Collections.Generic.List[object]
foreach ($group in $grouped) {
  $items = @($group.Group)
  $stats = Get-TrialStats -Trials $items
  $summaryRows.Add([ordered]@{
    tool = $items[0].tool
    backend = $items[0].backend
    min_ms = $stats.min_ms
    median_ms = $stats.median_ms
    max_ms = $stats.max_ms
    pair_count = $items[0].pair_count
    bars = $items[0].bars
    pair_bars = $items[0].pair_bars
    backend_route = if ($items[0].tool -eq "vectorgrid") { $items[0].backend_route } else { "cpu_python_workflow" }
    best = $items[0].best
    trial_paths = @($items | ForEach-Object { $_.stdout_path })
  })
}

$vectorGridCpu = Get-FirstOrNull -Items @($measuredArtifacts | Where-Object { $_.tool -eq "vectorgrid" -and $_.backend -eq "cpu" })
$vectorGridGpu = Get-FirstOrNull -Items @($measuredArtifacts | Where-Object { $_.tool -eq "vectorgrid" -and $_.backend -eq "gpu-kernel" })
$vectorBtCpu = Get-FirstOrNull -Items @($measuredArtifacts | Where-Object { $_.tool -eq "vectorbt" })

$parity = @()
if ($null -ne $vectorGridCpu -and $null -ne $vectorBtCpu) {
  $parity += Compare-Parity -Reference $vectorGridCpu -Candidate $vectorBtCpu -CandidateLabel "vectorbt_cpu"
}
if ($null -ne $vectorGridCpu -and $null -ne $vectorGridGpu) {
  $parity += Compare-Parity -Reference $vectorGridCpu -Candidate $vectorGridGpu -CandidateLabel "vectorgrid_gpu_kernel"
}

$vectorBtEnv = if ($null -ne $vectorBtCpu) { $vectorBtCpu.result.environment } else { $null }

$summary = [ordered]@{
  generated_at = (Get-Date).ToString("o")
  scenario_path = $scenarioFullPath.Path
  out_dir = $outPath
  dataset = $datasetInfo
  environment = [ordered]@{
    hardware = Get-CommandHardware
    python = [ordered]@{
      command = $python.display
    }
    vectorbt = [ordered]@{
      version = if ($null -ne $vectorBtEnv) { $vectorBtEnv.vectorbt } else { "unknown" }
      numpy = if ($null -ne $vectorBtEnv) { $vectorBtEnv.numpy } else { "unknown" }
      pandas = if ($null -ne $vectorBtEnv) { $vectorBtEnv.pandas } else { "unknown" }
      numba = if ($null -ne $vectorBtEnv) { $vectorBtEnv.numba } else { "unknown" }
    }
  }
  scenario = [ordered]@{
    strategy = $scenario.strategy
    ma_case = $scenario.ma_case
    objective = $scenario.objective
    vectorgrid_mode = $VectorGridMode
    fast_range = $fastRange.spec
    slow_range = $slowRange.spec
    alma_offset = $almaOffset
    alma_sigma = $almaSigma
    top_k = $topK
    heatmap_bins = $heatmapBins
    warmup_trials = $WarmupTrials
    measured_trials = $Trials
  }
  warmup_artifacts = $warmupArtifacts
  measured_artifacts = $measuredArtifacts
  summary_rows = $summaryRows
  parity = $parity
}

$summaryPath = Join-Path $outPath "summary.json"
$summary | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 $summaryPath
$markdownPath = Join-Path $outPath "summary.md"
Write-MarkdownSummary -Path $markdownPath -Summary $summary

Write-Host "Faceoff summary written to $summaryPath"
Write-Host "Markdown summary written to $markdownPath"
