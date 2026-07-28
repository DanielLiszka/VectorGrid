param(
  [string]$ArtifactPath,
  [string]$DatasetId,
  [string]$ImportCsv,
  [string]$ImportRootSymbol,
  [string]$AppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OutDir = "output/options_expression_phase05",
  [ValidateSet("auto", "cpu_reference", "gpu_compiled_tensor")]
  [string]$BackendPreference = "auto",
  [ValidateSet("event_chronological", "session_chronological")]
  [string]$SplitAssignmentMode = "event_chronological",
  [int]$TopK = 25,
  [int]$SealedFinalistCount = 5,
  [int]$WarmRepeats = 1,
  [int]$BenchmarkIterations = 3,
  [string[]]$RequestArgs = @(),
  [switch]$PrewarmGpuRuntime,
  [switch]$Cuda,
  [switch]$CompareBackends,
  [switch]$RunFull,
  [switch]$InspectTop,
  [switch]$ListDatasets
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

function Get-DefaultArtifactPath {
  param([Parameter(Mandatory = $true)][string]$RunsDir)

  if (!(Test-Path $RunsDir)) {
    throw "Runs directory does not exist: $RunsDir"
  }

  $candidate = Get-ChildItem $RunsDir -Filter "vectorgrid_run_*.json" -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

  if ($null -eq $candidate) {
    throw "No saved vectorgrid run artifacts were found under $RunsDir"
  }

  return $candidate.FullName
}

function Write-Section([string]$Label) {
  Write-Host ""
  Write-Host "== $Label =="
}

function Get-OptionsPerfProcesses() {
  @(Get-Process | Where-Object {
      $_.ProcessName -match '^(cargo|criterion|nsys|ncu|compute-sanitizer)$' -or
      $_.ProcessName -like 'ta_options_expression*' -or
      $_.ProcessName -like 'options_expression*'
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

$repoRoot = Resolve-RepoRoot
$resolvedAppLocalDataDir = Join-IfRelative -Path $AppLocalDataDir -Root $repoRoot
$runsDir = Join-Path $resolvedAppLocalDataDir "runs"
$datasetsDir = Join-Path (Join-Path $resolvedAppLocalDataDir "options_research") "datasets"
$resolvedOutDir = Join-IfRelative -Path $OutDir -Root $repoRoot
$timestamp = Get-Date -Format "yyyy-MM-ddTHH-mm-ss"
$runOutDir = Join-Path $resolvedOutDir $timestamp
$jsonOut = Join-Path $runOutDir "phase05_summary.json"
$markdownOut = Join-Path $runOutDir "phase05_summary.md"

if (!$ArtifactPath) {
  $ArtifactPath = Get-DefaultArtifactPath -RunsDir $runsDir
}

if (!$ImportCsv -and !$DatasetId -and !(Test-Path $datasetsDir)) {
  Write-Section "Missing Options Dataset"
  Write-Host "No imported options datasets were found under:"
  Write-Host "  $datasetsDir"
  Write-Host ""
  Write-Host "Import a Polygon/Massive normalized options CSV first, then rerun this script. Example:"
  Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/run_options_expression_phase05.ps1 -ImportCsv `"<path-to-options-csv>`" -ImportRootSymbol SPY -CompareBackends -InspectTop"
  exit 1
}

$cargoArgs = @(
  "run",
  "-q",
  "-p",
  "ta_desktop_demo_app"
)

$cargoFeatures = @("maintainer-tools")
if ($Cuda) {
  $cargoFeatures += @("cuda", "cuda-backtest-kernel")
} elseif ($CompareBackends) {
  Write-Warning "CompareBackends is enabled without -Cuda. The accelerated lane will fall back to the CPU-only build and the GPU comparison will not be meaningful."
}
$cargoArgs += @("--features", ($cargoFeatures -join ","))

$cargoArgs += @(
  "--bin",
  "ta_options_expression_feasibility",
  "--",
  "--app-local-data-dir",
  $resolvedAppLocalDataDir,
  "--artifact",
  $ArtifactPath,
  "--backend-preference",
  $BackendPreference,
  "--split-assignment-mode",
  $SplitAssignmentMode,
  "--top-k",
  [string][Math]::Max(1, $TopK),
  "--sealed-finalist-count",
  [string][Math]::Max(1, [Math]::Min($SealedFinalistCount, [Math]::Max(1, $TopK))),
  "--warm-repeats",
  [string][Math]::Max(0, $WarmRepeats),
  "--benchmark-iterations",
  [string][Math]::Max(1, $BenchmarkIterations),
  "--out-json",
  $jsonOut,
  "--out-markdown",
  $markdownOut
)

if ($DatasetId) {
  $cargoArgs += @("--dataset-id", $DatasetId)
}
if ($ImportCsv) {
  $cargoArgs += @("--import-csv", (Join-IfRelative -Path $ImportCsv -Root $repoRoot))
}
if ($ImportRootSymbol) {
  $cargoArgs += @("--import-root-symbol", $ImportRootSymbol)
}
if ($RequestArgs -and $RequestArgs.Count -gt 0) {
  $cargoArgs += $RequestArgs
}
if ($PrewarmGpuRuntime) {
  $cargoArgs += "--prewarm-gpu-runtime"
}
if ($CompareBackends) {
  $cargoArgs += "--compare-backends"
}
if ($RunFull) {
  $cargoArgs += "--run-full"
}
if ($InspectTop) {
  $cargoArgs += "--inspect-top"
}
if ($ListDatasets) {
  $cargoArgs += "--list-datasets"
}

Write-Section "Phase 0.5 Command"
Write-Host ("cargo " + ($cargoArgs -join " "))

$benchmarkLock = Enter-BenchmarkLock -Context "options Phase 0.5 run"
$phase05ExitCode = 0
try {
  Wait-NoOptionsPerfProcesses -TimeoutSeconds 120 -Context "before options Phase 0.5 run"

  Push-Location $repoRoot
  $previousRustcWrapper = $env:RUSTC_WRAPPER
  $env:RUSTC_WRAPPER = ""
  try {
    & cargo @cargoArgs
    $phase05ExitCode = $LASTEXITCODE
  } finally {
    $env:RUSTC_WRAPPER = $previousRustcWrapper
    Pop-Location
  }

  Wait-NoOptionsPerfProcesses -TimeoutSeconds 300 -Context "after options Phase 0.5 run"
} finally {
  Exit-BenchmarkLock -Handle $benchmarkLock
}
if ($phase05ExitCode -ne 0) {
  exit $phase05ExitCode
}

if ($ListDatasets) {
  return
}

if (!(Test-Path $jsonOut)) {
  throw "Expected summary JSON was not written: $jsonOut"
}

$summary = Get-Content -Raw $jsonOut | ConvertFrom-Json
$gates = $summary.phase05_gates
$report = $summary.response.report

Write-Section "Phase 0.5 Summary"
Write-Host ("artifact:      {0}" -f $summary.artifact_path)
Write-Host ("dataset:       {0}" -f $summary.options_dataset_id)
if ($summary.request_split_assignment_mode) {
  Write-Host ("split mode:    {0}" -f $summary.request_split_assignment_mode)
}
Write-Host ("density:       {0:P1}" -f [double]$report.candidate_density)
Write-Host ("invalid rate:  {0:P1}" -f [double]$report.invalid_slot_rate)

if ($null -ne $gates.structural_cache_reuse_observed) {
  Write-Host ("struct reuse:  {0:P1}" -f [double]$gates.structural_cache_reuse_observed)
}
if ($null -ne $gates.summary_cache_reuse_observed) {
  Write-Host ("summary reuse: {0:P1}" -f [double]$gates.summary_cache_reuse_observed)
}
if ($null -ne $gates.gpu_speedup_observed) {
  Write-Host ("gpu speedup:   {0:N2}x" -f [double]$gates.gpu_speedup_observed)
} else {
  Write-Host "gpu speedup:   pending"
}
if ($null -ne $summary.backend_comparison) {
  $previewWallSpeedup = $summary.backend_comparison.estimated_backend_preview_wall_speedup_vs_cpu
  if ($null -eq $previewWallSpeedup) {
    $previewWallSpeedup = $summary.backend_comparison.preview_wall_speedup_vs_cpu
  }
  if ($null -ne $previewWallSpeedup) {
    Write-Host ("preview wall:  {0:N2}x" -f [double]$previewWallSpeedup)
  }
}

if ($summary.recommendations) {
  Write-Host ""
  Write-Host "recommend:"
  foreach ($recommendation in $summary.recommendations) {
    Write-Host ("  - {0}" -f $recommendation)
  }
}

Write-Host ("json:          {0}" -f $jsonOut)
Write-Host ("markdown:      {0}" -f $markdownOut)
