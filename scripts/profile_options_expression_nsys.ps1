param(
  [string]$ArtifactPath,
  [string]$DatasetId,
  [string]$ImportCsv,
  [string]$ImportRootSymbol,
  [string]$AppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OutputPrefix = "output/profiling/nsight/options_expression_nsys",
  [ValidateSet("auto", "cpu_reference", "gpu_compiled_tensor")]
  [string]$BackendPreference = "auto",
  [ValidateSet("event_chronological", "session_chronological")]
  [string]$SplitAssignmentMode = "event_chronological",
  [int]$TopK = 25,
  [int]$SealedFinalistCount = 5,
  [int]$WarmRepeats = 0,
  [int]$BenchmarkIterations = 1,
  [string[]]$RequestArgs = @(),
  [switch]$PrewarmGpuRuntime,
  [switch]$CompareBackends,
  [switch]$RunFull,
  [switch]$InspectTop,
  [ValidateSet("none", "nvtx")]
  [string]$CaptureMode = "nvtx",
  [string]$CaptureRange = "",
  [string]$NsysExePath = ""
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

function Resolve-NsysExe {
  param([string]$PreferredPath)

  if (-not [string]::IsNullOrWhiteSpace($PreferredPath)) {
    if (Test-Path $PreferredPath) {
      return $PreferredPath
    }
    throw "Requested Nsight Systems executable does not exist: $PreferredPath"
  }

  $candidates = @(
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2026.2.1\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2025.6.3\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2025.3.2\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2023.3.3\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\host\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\host\target-windows-x64\nsys.exe"
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  $cmd = Get-Command nsys.exe -ErrorAction SilentlyContinue
  if ($null -ne $cmd) {
    return $cmd.Source
  }

  throw "Nsight Systems CLI not found (expected nsys.exe)."
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
$captureRangeName = if ([string]::IsNullOrWhiteSpace($CaptureRange)) {
  if ($RunFull) { "options_expression_run_total" } else { "options_expression_feasibility_total" }
} else {
  $CaptureRange.Trim()
}

$nsysExe = Resolve-NsysExe -PreferredPath $NsysExePath
Write-Host "Using Nsight Systems CLI: $nsysExe"

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousPath = $env:PATH
$previousNvtx = [System.Environment]::GetEnvironmentVariable("VECTORBT_ENABLE_NVTX", "Process")
$previousRegisterOnly = [System.Environment]::GetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", "Process")
$reportPath = "$resolvedOutputPrefix.nsys-rep"
$fallbackUsed = $false
$captureModeApplied = $CaptureMode
$requestedNsysArgs = $null

$benchmarkLock = Enter-BenchmarkLock -Context "options Nsight Systems profile"
try {
  Push-Location $repoRoot
  try {
    if (-not (Test-IsAdministrator)) {
      Write-Warning "Nsight Systems may fail to attach from a non-elevated PowerShell session. Rerun as Administrator if profiling reports are missing or attach permissions fail."
    }

  Wait-NoOptionsPerfProcesses -TimeoutSeconds 120 -Context "before options Nsight Systems profile"

  $env:RUSTC_WRAPPER = ""
  Write-Host "Building ta_options_expression_feasibility with CUDA + NVTX profiling..."
  & cargo build -p ta_desktop_demo_app --release --bin ta_options_expression_feasibility --features "maintainer-tools,cuda,cuda-backtest-kernel,nvtx-profiling"
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
  if ($CompareBackends) {
    $binaryArgs += "--compare-backends"
  }
  if ($RunFull) {
    $binaryArgs += "--run-full"
  }
  if ($InspectTop) {
    $binaryArgs += "--inspect-top"
  }

  [System.Environment]::SetEnvironmentVariable("VECTORBT_ENABLE_NVTX", "1", "Process")
  if ($CaptureMode -eq "nvtx") {
    [System.Environment]::SetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", "0", "Process")
  } else {
    [System.Environment]::SetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", $previousRegisterOnly, "Process")
  }

  $requestedNsysArgs = @(
    "profile",
    "--sample=none",
    "--cpuctxsw=none",
    "--wait=primary",
    "--stop-on-exit=true",
    "--stats=false",
    "--force-overwrite=true",
    "--trace=cuda,nvtx",
    "--cuda-graph-trace=graph",
    "--output=$resolvedOutputPrefix"
  )
  if ($CaptureMode -eq "nvtx") {
    $requestedNsysArgs += @(
      "--capture-range=nvtx",
      "--nvtx-capture=$captureRangeName"
    )
  }
  $requestedNsysArgs += @($binaryPath) + $binaryArgs

  & $nsysExe @requestedNsysArgs
  $exitCode = $LASTEXITCODE

  if ($exitCode -eq 0 -and !(Test-Path $reportPath)) {
    $altReportPath = "$resolvedOutputPrefix.qdrep"
    if (Test-Path $altReportPath) {
      $reportPath = $altReportPath
    } elseif ($CaptureMode -eq "nvtx") {
      Write-Warning "Bounded NVTX capture did not emit a report. Falling back to whole-process capture."
      $fallbackUsed = $true
      $captureModeApplied = "none"
      $fallbackPrefix = "${resolvedOutputPrefix}_whole_fallback"
      $fallbackArgs = @(
        "profile",
        "--sample=none",
        "--cpuctxsw=none",
        "--wait=primary",
        "--stop-on-exit=true",
        "--stats=false",
        "--force-overwrite=true",
        "--trace=cuda,nvtx",
        "--cuda-graph-trace=graph",
        "--output=$fallbackPrefix",
        $binaryPath
      ) + $binaryArgs
      & $nsysExe @fallbackArgs
      $exitCode = $LASTEXITCODE
      $reportPath = "$fallbackPrefix.nsys-rep"
      if (!(Test-Path $reportPath)) {
        $fallbackAlt = "$fallbackPrefix.qdrep"
        if (Test-Path $fallbackAlt) {
          $reportPath = $fallbackAlt
        }
      }
    }
  }

  if (!(Test-Path $reportPath)) {
    throw "Nsight Systems did not produce a report at '$reportPath'."
  }

  $provenancePath = if ($reportPath.EndsWith(".qdrep")) {
    $reportPath.Substring(0, $reportPath.Length - ".qdrep".Length) + ".provenance.json"
  } else {
    $reportPath.Substring(0, $reportPath.Length - ".nsys-rep".Length) + ".provenance.json"
  }
  $provenance = [ordered]@{
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    profiler = "nsight_systems"
    profiler_exe = $nsysExe
    report_path = $reportPath
    requested_output_prefix = $resolvedOutputPrefix
    requested_capture_mode = $CaptureMode
    applied_capture_mode = $captureModeApplied
    capture_range = $captureRangeName
    fallback_used = $fallbackUsed
    profiler_exit_code = $exitCode
    binary_path = $binaryPath
    binary_args = $binaryArgs
    requested_command_args = $requestedNsysArgs
    summary_json = $summaryJson
    summary_markdown = $summaryMarkdown
    environment = [ordered]@{
      VECTORBT_ENABLE_NVTX = "1"
      NSYS_NVTX_PROFILER_REGISTER_ONLY = if ($CaptureMode -eq "nvtx") { "0" } else { $previousRegisterOnly }
    }
  }
  Write-JsonUtf8 -Path $provenancePath -Value $provenance

  Write-Host "Nsight Systems report written to $reportPath"
  Write-Host "Summary JSON written to $summaryJson"
  Write-Host "Summary Markdown written to $summaryMarkdown"
  Write-Host "Provenance written to $provenancePath"

    if ($exitCode -ne 0) {
      Write-Warning "Nsight Systems produced a report, but the profiled process exited with code $exitCode. Inspect the summary and report before trusting parity."
    }
  } finally {
    $env:RUSTC_WRAPPER = $previousRustcWrapper
    $env:PATH = $previousPath
    [System.Environment]::SetEnvironmentVariable("VECTORBT_ENABLE_NVTX", $previousNvtx, "Process")
    [System.Environment]::SetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", $previousRegisterOnly, "Process")
    Pop-Location
  }
} finally {
  Exit-BenchmarkLock -Handle $benchmarkLock
}
