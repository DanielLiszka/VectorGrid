param(
  [string]$ArtifactPath,
  [string]$DatasetId,
  [string]$ImportCsv,
  [string]$ImportRootSymbol,
  [string]$AppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OutputPrefix = "target/nsight/options_expression_ncu",
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
  [int]$LaunchCount = 20,
  [string]$KernelName = "regex:^options_expression_score_split_f64$",
  [bool]$UseKernelNameFilter = $true,
  [ValidateSet("application-only", "all")]
  [string]$TargetProcesses = "application-only",
  [string]$NcuExePath = ""
)

$ErrorActionPreference = "Stop"

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

function Resolve-NcuExe {
  param([string]$PreferredPath)

  if (-not [string]::IsNullOrWhiteSpace($PreferredPath)) {
    if (Test-Path $PreferredPath) {
      return $PreferredPath
    }
    throw "Requested Nsight Compute executable does not exist: $PreferredPath"
  }

  $candidates = @(
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\target\windows-desktop-win7-x64\ncu.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\ncu.bat",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\host\target-windows-x64\ncu.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\ncu.bat"
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  $cmd = Get-Command ncu.bat -ErrorAction SilentlyContinue
  if ($null -eq $cmd) {
    $cmd = Get-Command ncu.exe -ErrorAction SilentlyContinue
  }
  if ($null -ne $cmd) {
    return $cmd.Source
  }

  throw "Nsight Compute CLI not found (expected ncu.bat or ncu.exe)."
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
$ncuExe = Resolve-NcuExe -PreferredPath $NcuExePath
Write-Host "Using Nsight Compute CLI: $ncuExe"

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousPath = $env:PATH
$reportPath = "$resolvedOutputPrefix.ncu-rep"
$requestedNcuArgs = $null

Push-Location $repoRoot
try {
  $env:RUSTC_WRAPPER = ""
  Write-Host "Building ta_options_expression_feasibility with CUDA + NVTX profiling..."
  & cargo build -p ta_desktop_demo_app --release --bin ta_options_expression_feasibility --features "cuda,cuda-backtest-kernel,nvtx-profiling"
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

  $requestedNcuArgs = @(
    "--target-processes", $TargetProcesses,
    "--set", "full",
    "--kernel-name-base", "demangled"
  )
  if ($UseKernelNameFilter -and ![string]::IsNullOrWhiteSpace($KernelName)) {
    $requestedNcuArgs += @("--kernel-name", $KernelName)
  }
  $requestedNcuArgs += @(
    "--launch-count", "$LaunchCount",
    "--force-overwrite",
    "--export", $resolvedOutputPrefix,
    $binaryPath
  ) + $binaryArgs

  & $ncuExe @requestedNcuArgs
  $exitCode = $LASTEXITCODE
  $reportExists = Test-Path $reportPath

  $provenancePath = "$resolvedOutputPrefix.provenance.json"
  $provenance = [ordered]@{
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    profiler = "nsight_compute"
    profiler_exe = $ncuExe
    report_path = $reportPath
    report_exists = $reportExists
    requested_output_prefix = $resolvedOutputPrefix
    kernel_name = $KernelName
    use_kernel_name_filter = $UseKernelNameFilter
    launch_count = $LaunchCount
    target_processes = $TargetProcesses
    profiler_exit_code = $exitCode
    binary_path = $binaryPath
    binary_args = $binaryArgs
    requested_command_args = $requestedNcuArgs
    summary_json = $summaryJson
    summary_markdown = $summaryMarkdown
  }
  Write-JsonUtf8 -Path $provenancePath -Value $provenance

  if (!$reportExists) {
    throw "Nsight Compute did not produce a report at '$reportPath'."
  }

  Write-Host "Nsight Compute report written to $reportPath"
  Write-Host "Summary JSON written to $summaryJson"
  Write-Host "Summary Markdown written to $summaryMarkdown"
  Write-Host "Provenance written to $provenancePath"

  if ($exitCode -ne 0) {
    Write-Warning "Nsight Compute produced a report but exited with code $exitCode."
  }
} finally {
  $env:RUSTC_WRAPPER = $previousRustcWrapper
  $env:PATH = $previousPath
  Pop-Location
}
