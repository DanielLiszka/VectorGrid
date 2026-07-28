param(
  [string]$ArtifactPath,
  [string]$DatasetId,
  [string]$ImportCsv,
  [string]$ImportRootSymbol,
  [string]$AppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OutputPrefix = "output/profiling/nsight/options_expression_compute_sanitizer",
  [ValidateSet("auto", "cpu_reference", "gpu_compiled_tensor")]
  [string]$BackendPreference = "gpu_compiled_tensor",
  [ValidateSet("event_chronological", "session_chronological")]
  [string]$SplitAssignmentMode = "event_chronological",
  [int]$TopK = 25,
  [int]$SealedFinalistCount = 5,
  [int]$WarmRepeats = 0,
  [int]$BenchmarkIterations = 1,
  [string[]]$RequestArgs = @(),
  [switch]$PrewarmGpuRuntime,
  [switch]$RunFull,
  [switch]$InspectTop,
  [ValidateSet("memcheck", "racecheck", "initcheck", "synccheck")]
  [string]$Tool = "memcheck",
  [string]$ComputeSanitizerExePath = ""
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

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value,
    [int]$Depth = 12
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Test-IsAdministrator {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
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

function Get-CargoRuntimeDllPaths {
  param([Parameter(Mandatory = $true)][string]$RepoRoot)

  $paths = New-Object System.Collections.Generic.List[string]
  $releaseDeps = Join-Path $RepoRoot "target\release\deps"
  if (Test-Path $releaseDeps) {
    $paths.Add((Resolve-Path $releaseDeps).Path)
  }

  $buildRoot = Join-Path $RepoRoot "target\release\build"
  if (Test-Path $buildRoot) {
    $outCandidates = Get-ChildItem -Path $buildRoot -Directory | ForEach-Object {
      @(
        (Join-Path $_.FullName "out"),
        (Join-Path $_.FullName "out\x64")
      )
    }
    foreach ($candidate in $outCandidates) {
      if (Test-Path $candidate) {
        $paths.Add((Resolve-Path $candidate).Path)
      }
    }
  }

  return $paths | Select-Object -Unique
}

function Resolve-ComputeSanitizerExe {
  param([string]$PreferredPath)

  if (-not [string]::IsNullOrWhiteSpace($PreferredPath)) {
    if (Test-Path $PreferredPath) {
      return $PreferredPath
    }
    throw "Requested compute-sanitizer executable does not exist: $PreferredPath"
  }

  $candidates = @(
    "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2\compute-sanitizer\compute-sanitizer.exe",
    "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1\compute-sanitizer\compute-sanitizer.exe",
    "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.8\compute-sanitizer\compute-sanitizer.exe"
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  $cmd = Get-Command compute-sanitizer.exe -ErrorAction SilentlyContinue
  if ($null -ne $cmd) {
    return $cmd.Source
  }

  throw "Compute Sanitizer CLI not found (expected compute-sanitizer.exe)."
}

$repoRoot = Resolve-RepoRoot
$resolvedAppLocalDataDir = Join-IfRelative -Path $AppLocalDataDir -Root $repoRoot
$runsDir = Join-Path $resolvedAppLocalDataDir "runs"
$resolvedOutputPrefix = Join-IfRelative -Path $OutputPrefix -Root $repoRoot
$outputDir = Split-Path -Parent $resolvedOutputPrefix
if ($outputDir) {
  New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
}

if (!$ArtifactPath) {
  $ArtifactPath = Get-DefaultArtifactPath -RunsDir $runsDir
}

$resolvedArtifactPath = Join-IfRelative -Path $ArtifactPath -Root $repoRoot
$summaryJson = "$resolvedOutputPrefix.summary.json"
$summaryMarkdown = "$resolvedOutputPrefix.summary.md"
$sanitizerLog = "$resolvedOutputPrefix.$Tool.log"
$sanitizerExe = Resolve-ComputeSanitizerExe -PreferredPath $ComputeSanitizerExePath
Write-Host "Using Compute Sanitizer CLI: $sanitizerExe"

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousPath = $env:PATH

$benchmarkLock = Enter-BenchmarkLock -Context "options Compute Sanitizer run"
try {
  Push-Location $repoRoot
  try {
    if (-not (Test-IsAdministrator)) {
      Write-Warning "Compute Sanitizer may fail to attach from a non-elevated PowerShell session. Rerun as Administrator if profiling reports are missing or attach permissions fail."
    }

  Wait-NoOptionsPerfProcesses -TimeoutSeconds 120 -Context "before options Compute Sanitizer run"

  $env:RUSTC_WRAPPER = ""
  Write-Host "Building ta_options_expression_feasibility with CUDA..."
  & cargo build -p ta_desktop_demo_app --release --bin ta_options_expression_feasibility --features "maintainer-tools,cuda,cuda-backtest-kernel"
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  $binaryPath = Join-Path $repoRoot "target\release\ta_options_expression_feasibility.exe"
  if (!(Test-Path $binaryPath)) {
    throw "Could not find built executable: $binaryPath"
  }

  $runtimeDllPaths = Get-CargoRuntimeDllPaths -RepoRoot $repoRoot
  if ($runtimeDllPaths.Count -gt 0) {
    $env:PATH = (($runtimeDllPaths -join ";") + ";" + $previousPath)
  }

  $binaryArgs = @(
    "--app-local-data-dir", $resolvedAppLocalDataDir,
    "--artifact", $resolvedArtifactPath,
    "--backend-preference", $BackendPreference,
    "--split-assignment-mode", $SplitAssignmentMode,
    "--top-k", [string][Math]::Max(1, $TopK),
    "--sealed-finalist-count", [string][Math]::Max(1, [Math]::Min($SealedFinalistCount, [Math]::Max(1, $TopK))),
    "--warm-repeats", [string][Math]::Max(0, $WarmRepeats),
    "--benchmark-iterations", [string][Math]::Max(1, $BenchmarkIterations),
    "--out-json", $summaryJson,
    "--out-markdown", $summaryMarkdown
  )

  if ($DatasetId) {
    $binaryArgs += @("--dataset-id", $DatasetId)
  }
  if ($ImportCsv) {
    $binaryArgs += @("--import-csv", (Join-IfRelative -Path $ImportCsv -Root $repoRoot))
  }
  if ($ImportRootSymbol) {
    $binaryArgs += @("--import-root-symbol", $ImportRootSymbol)
  }
  if ($RequestArgs -and $RequestArgs.Count -gt 0) {
    $binaryArgs += $RequestArgs
  }
  if ($PrewarmGpuRuntime) {
    $binaryArgs += "--prewarm-gpu-runtime"
  }
  if ($RunFull) {
    $binaryArgs += "--run-full"
  }
  if ($InspectTop) {
    $binaryArgs += "--inspect-top"
  }

  & $sanitizerExe --tool $Tool --log-file $sanitizerLog $binaryPath @binaryArgs
  $exitCode = $LASTEXITCODE

  $provenancePath = "$resolvedOutputPrefix.provenance.json"
  $provenance = [ordered]@{
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    profiler = "compute_sanitizer"
    profiler_exe = $sanitizerExe
    tool = $Tool
    binary_path = $binaryPath
    binary_args = $binaryArgs
    sanitizer_log = $sanitizerLog
    summary_json = $summaryJson
    summary_markdown = $summaryMarkdown
  }
  Write-JsonUtf8 -Path $provenancePath -Value $provenance

  Write-Host "Compute Sanitizer log written to $sanitizerLog"
  Write-Host "Summary JSON written to $summaryJson"
  Write-Host "Summary Markdown written to $summaryMarkdown"
  Write-Host "Provenance written to $provenancePath"

    if ($exitCode -ne 0) {
      throw "Compute Sanitizer exited with code $exitCode."
    }
  } finally {
    $env:RUSTC_WRAPPER = $previousRustcWrapper
    $env:PATH = $previousPath
    Pop-Location
  }
} finally {
  Exit-BenchmarkLock -Handle $benchmarkLock
}
