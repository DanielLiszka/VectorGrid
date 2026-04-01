param(
  [ValidateSet("runner", "bench")]
  [string]$Target = "runner",
  [string]$BenchName = "double_ma_opt",
  [string]$BenchFilter = "double_ma/gpu_kernel/sma_sma/200k/small",
  [string]$BenchStrategyOnly = "",
  [int]$BenchSampleSize = 10,
  [int]$BenchMeasurementTimeSec = 3,
  [int]$BenchWarmupTimeSec = 1,
  [ValidateSet("double_ma", "trend_quality", "regime_hybrid", "breakout_momentum")]
  [string]$RunnerStrategy = "breakout_momentum",
  [ValidateSet("off", "on")]
  [string]$RunnerAdxMode = "off",
  [ValidateSet("small", "medium", "large", "xlarge")]
  [string]$RunnerWorkload = "xlarge",
  [ValidateSet("auto", "full17", "trade22")]
  [string]$RunnerMetricContract = "auto",
  [ValidateSet("inherit", "off", "on")]
  [string]$RunnerIncludeTradeMetrics = "inherit",
  [int]$RunnerIterations = 1,
  [int]$RunnerWarmupIterations = 0,
  [int]$RunnerHeatmapBins = 64,
  [int]$RunnerDeviceId = 0,
  [ValidateSet("off", "on")]
  [string]$NvtxMode = "off",
  [ValidateSet("none", "nvtx")]
  [string]$CaptureMode = "none",
  [string]$NvtxCapture = "double_ma_opt_total",
  [string]$NvtxDomains = "default,engine,kernel",
  [string]$OutputPrefix = "target/nsight/gpu_kernel_small",
  [ValidateSet("off", "on")]
  [string]$DynamicStrategyMode = "off",
  [ValidateSet("inherit", "off", "on")]
  [string]$CudaGraphsMode = "off",
  [string]$NsysExePath = ""
)

$ErrorActionPreference = "Stop"

if (-not ("NsysShortPath" -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class NsysShortPath {
  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern uint GetShortPathName(string lpszLongPath, StringBuilder lpszShortPath, uint cchBuffer);
}
"@
}

function Get-NativeSafePath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [switch]$AllowMissingLeaf
  )

  $resolved = $Path
  $leaf = $null
  if ($AllowMissingLeaf -and -not (Test-Path $resolved)) {
    $leaf = Split-Path -Leaf $resolved
    $resolved = Split-Path -Parent $resolved
  }

  $buffer = New-Object System.Text.StringBuilder 512
  $len = [NsysShortPath]::GetShortPathName($resolved, $buffer, $buffer.Capacity)
  if ($len -gt 0) {
    $short = $buffer.ToString()
    if ($leaf) {
      return Join-Path $short $leaf
    }
    return $short
  }

  if ($leaf) {
    return Join-Path $resolved $leaf
  }
  return $resolved
}

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 12
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

if (-not [string]::IsNullOrWhiteSpace($NsysExePath)) {
  $nsysExe = $NsysExePath
} else {
  $nsysCandidates = @(
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2023.3.3\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2026.2.1\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2025.6.3\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2025.3.2\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2023.1.2\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Systems 2022.4.2\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\host\target-windows-x64\nsys.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\host\target-windows-x64\nsys.exe"
  )
  $nsysExe = $nsysCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (-not $nsysExe) {
    $cmd = Get-Command nsys.exe -ErrorAction SilentlyContinue
    if ($null -ne $cmd) {
      $nsysExe = $cmd.Source
    }
  }
}
if (-not $nsysExe -or !(Test-Path $nsysExe)) {
  throw "Nsight Systems CLI not found (expected nsys.exe)."
}
Write-Host "Using Nsight Systems CLI: $nsysExe"

$nsightTempDir = Join-Path $env:LOCALAPPDATA "Temp\nvidia\nsight_systems"
New-Item -ItemType Directory -Force -Path $nsightTempDir | Out-Null
$nsysInstallRoot = Split-Path (Split-Path $nsysExe -Parent) -Parent

$isElevated = $false
try {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  $isElevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
} catch {
  $isElevated = $false
}
if ($nsysInstallRoot -match "Nsight Systems 20(25|26)" -and -not $isElevated) {
  Write-Warning "Local Nsight Systems 2025.x/2026.x on this machine has failed unelevated with 'Access is denied' while opening the create-agent memory-mapped file. If capture startup times out or no report is produced, rerun from an elevated shell or repair/reinstall Nsight Systems."
}

if ($Target -eq "runner") {
  Write-Host "Building profile runner executable..."
  cargo build -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
  $targetExePath = "target/release/profile_gpu_dynamic_runner.exe"
  if (!(Test-Path $targetExePath)) {
    throw "Could not find built runner executable: $targetExePath"
  }
  $targetExe = Get-Item $targetExePath
} else {
  Write-Host "Building benchmark executable..."
  cargo bench -p ta_app_core --bench $BenchName --features cuda,cuda-backtest-kernel --no-run
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  $targetExe = Get-ChildItem -Path "target/release/deps" -File -Filter "$BenchName-*.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if ($null -eq $targetExe) {
    throw "Could not find built bench executable: target/release/deps/$BenchName-*.exe"
  }
}

$targetExePathForNsys = Get-NativeSafePath -Path $targetExe.FullName

$outputPrefixPath = if ([System.IO.Path]::IsPathRooted($OutputPrefix)) {
  $OutputPrefix
} else {
  Join-Path (Get-Location) $OutputPrefix
}
$outputPrefixPathForNsys = Get-NativeSafePath -Path $outputPrefixPath -AllowMissingLeaf
$outDir = Split-Path -Parent $outputPrefixPath
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$prevFastPath = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "Process")
$prevGraphs = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "Process")
$prevDynamic = [System.Environment]::GetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "Process")
$prevBenchOnly = [System.Environment]::GetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", "Process")
$prevNvtx = [System.Environment]::GetEnvironmentVariable("VECTORBT_ENABLE_NVTX", "Process")
$prevNsysNvtxRegisterOnly = [System.Environment]::GetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", "Process")

function Invoke-NsysProfileCommand {
  param(
    [string]$ExePath,
    [string[]]$CommandArgs,
    [string]$WorkingDirectory
  )

  $argumentLine = ($CommandArgs | ForEach-Object {
    if ($_ -match '\s') {
      '"' + ($_ -replace '"', '\"') + '"'
    } else {
      $_
    }
  }) -join ' '

  $startParams = @{
    FilePath    = $ExePath
    ArgumentList = $argumentLine
    NoNewWindow = $true
    Wait        = $true
    PassThru    = $true
  }
  if ($WorkingDirectory) {
    $startParams.WorkingDirectory = $WorkingDirectory
  }
  $process = Start-Process @startParams
  return $process.ExitCode
}

$exitCode = $null
$reportPath = $null
$captureModeApplied = $CaptureMode
$fallbackUsed = $false
$requestedNsysArgs = $null
try {
  [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "1", "Process")
  switch ($CudaGraphsMode) {
    "on" { [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "1", "Process") }
    "off" { [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "0", "Process") }
    default { [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process") }
  }
  if ($DynamicStrategyMode -eq "on") {
    [System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "1", "Process")
  } else {
    [System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", $null, "Process")
  }
  if ([string]::IsNullOrWhiteSpace($BenchStrategyOnly)) {
    [System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $null, "Process")
  } else {
    [System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $BenchStrategyOnly, "Process")
  }
  if ($NvtxMode -eq "on") {
    [System.Environment]::SetEnvironmentVariable("VECTORBT_ENABLE_NVTX", "1", "Process")
  } else {
    [System.Environment]::SetEnvironmentVariable("VECTORBT_ENABLE_NVTX", $null, "Process")
  }
  if ($NvtxMode -eq "on" -and $CaptureMode -eq "nvtx") {
    # Nsight Systems only considers registered NVTX strings for capture triggering by
    # default. Our current Rust NVTX path uses push/pop ranges with plain strings,
    # so opt into matching non-registered strings when bounded capture is requested.
    [System.Environment]::SetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", "0", "Process")
  } else {
    [System.Environment]::SetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", $prevNsysNvtxRegisterOnly, "Process")
  }

  if ($Target -eq "runner" -and $CudaGraphsMode -eq "on" -and $RunnerHeatmapBins -gt 0) {
    Write-Warning "RunnerHeatmapBins=$RunnerHeatmapBins may keep the exact kernel path graph-ineligible; set -RunnerHeatmapBins 0 for a graph A/B."
  }

  Write-Host "Profiling with Nsight Systems (dynamic_strategy=$DynamicStrategyMode, cuda_graphs=$CudaGraphsMode)..."

  $requestedNsysArgs = @(
    "profile",
    "--sample=none",
    "--cpuctxsw=none",
    "--wait=primary",
    "--stop-on-exit=true",
    "--stats=false",
    "--force-overwrite=true",
    "--output=$outputPrefixPathForNsys"
  )
  if ($NvtxMode -eq "on") {
    $requestedNsysArgs += @(
      "--trace=cuda,nvtx",
      "--cuda-graph-trace=graph"
    )
    if ($CaptureMode -eq "nvtx") {
      $requestedNsysArgs += @(
        "--capture-range=nvtx",
        "--nvtx-capture=$NvtxCapture",
        "--nvtx-domain-include=$NvtxDomains"
      )
    }
  } else {
    $requestedNsysArgs += @("--trace=cuda")
  }
  if ($Target -eq "runner") {
    $requestedNsysArgs += @(
      $targetExePathForNsys,
      "--strategy", $RunnerStrategy,
      "--adx-mode", $RunnerAdxMode,
      "--workload", $RunnerWorkload,
      "--iterations", "$RunnerIterations",
      "--warmup-iterations", "$RunnerWarmupIterations",
      "--heatmap-bins", "$RunnerHeatmapBins",
      "--device-id", "$RunnerDeviceId",
      "--nvtx", $NvtxMode,
      "--dynamic-mode", $DynamicStrategyMode,
      "--cuda-graphs", $CudaGraphsMode
    )
    if ($RunnerMetricContract -ne "auto") {
      $requestedNsysArgs += @("--metric-contract", $RunnerMetricContract)
    }
    if ($RunnerIncludeTradeMetrics -ne "inherit") {
      $requestedNsysArgs += @("--include-trade-metrics", $RunnerIncludeTradeMetrics)
    }
  } else {
    $requestedNsysArgs += @(
      $targetExePathForNsys,
      $BenchFilter,
      "--sample-size", "$BenchSampleSize",
      "--measurement-time", "$BenchMeasurementTimeSec",
      "--warm-up-time", "$BenchWarmupTimeSec"
    )
  }

  $nsysWorkDir = $null
  $exitCode = Invoke-NsysProfileCommand -ExePath $nsysExe -CommandArgs $requestedNsysArgs -WorkingDirectory $nsysWorkDir

  $reportPath = "$outputPrefixPath.nsys-rep"
  $nvLogPaths = @()
  if ($outDir) {
    $nvLogPaths += Join-Path $outDir "nsys-ui.log"
  }
  $nvLogPaths += Join-Path (Get-Location) "nsys-ui.log"
  $nvLogPaths = $nvLogPaths | Select-Object -Unique
  if (!(Test-Path $reportPath)) {
    $altReport = "$outputPrefixPath.qdrep"
    $deadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $deadline) {
      if ((Test-Path $reportPath) -or (Test-Path $altReport)) {
        break
      }
      Start-Sleep -Milliseconds 500
    }
    foreach ($nvLogPath in $nvLogPaths) {
      if ((!(Test-Path $reportPath)) -and (!(Test-Path $altReport)) -and (Test-Path $nvLogPath)) {
        $nvlogTail = Get-Content $nvLogPath -Tail 200 -ErrorAction SilentlyContinue
        if ($nvlogTail -match "Failed to open memory mapped file" -and $nvlogTail -match "Access is denied") {
          throw "Nsight Systems failed before capture startup. Local profiler logs show 'Access is denied' while opening the create-agent memory-mapped file. On this machine that points to a local 2025.x/2026.x Nsight Systems privilege/tooling problem. Rerun from an elevated shell or repair/reinstall the selected Nsight Systems build."
        }
      }
    }
    if (Test-Path $altReport) {
      $reportPath = $altReport
    } elseif (!(Test-Path $reportPath) -and $CaptureMode -eq "nvtx") {
      Write-Warning "Nsight Systems completed without producing a bounded NVTX report. Rerunning as whole-process capture because local --capture-range=nvtx is still returning an empty result on this machine."
      $fallbackUsed = $true
      $captureModeApplied = "none"
      $fallbackOutputPrefixPath = "${outputPrefixPath}_whole_fallback"
      $fallbackOutputPrefix = if ([System.IO.Path]::IsPathRooted($OutputPrefix)) {
        "${OutputPrefix}_whole_fallback"
      } else {
        "$OutputPrefix`_whole_fallback"
      }
      $rerunArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-Target", $Target,
        "-NvtxMode", $NvtxMode,
        "-CaptureMode", "none",
        "-NvtxCapture", $NvtxCapture,
        "-NvtxDomains", $NvtxDomains,
        "-OutputPrefix", $fallbackOutputPrefix,
        "-DynamicStrategyMode", $DynamicStrategyMode,
        "-CudaGraphsMode", $CudaGraphsMode,
        "-NsysExePath", $nsysExe
      )
      if ($Target -eq "runner") {
        $rerunArgs += @(
          "-RunnerStrategy", $RunnerStrategy,
          "-RunnerAdxMode", $RunnerAdxMode,
          "-RunnerWorkload", $RunnerWorkload,
          "-RunnerIterations", "$RunnerIterations",
          "-RunnerWarmupIterations", "$RunnerWarmupIterations",
          "-RunnerHeatmapBins", "$RunnerHeatmapBins",
          "-RunnerDeviceId", "$RunnerDeviceId"
        )
      } else {
        $rerunArgs += @(
          "-BenchName", $BenchName,
          "-BenchFilter", $BenchFilter,
          "-BenchSampleSize", "$BenchSampleSize",
          "-BenchMeasurementTimeSec", "$BenchMeasurementTimeSec",
          "-BenchWarmupTimeSec", "$BenchWarmupTimeSec"
        )
        if (-not [string]::IsNullOrWhiteSpace($BenchStrategyOnly)) {
          $rerunArgs += @("-BenchStrategyOnly", $BenchStrategyOnly)
        }
      }

      & powershell @rerunArgs
      $exitCode = $LASTEXITCODE

      $reportPath = "$fallbackOutputPrefixPath.nsys-rep"
      $altReport = "$fallbackOutputPrefixPath.qdrep"
      $deadline = (Get-Date).AddSeconds(30)
      while ((Get-Date) -lt $deadline) {
        if ((Test-Path $reportPath) -or (Test-Path $altReport)) {
          break
        }
        Start-Sleep -Milliseconds 500
      }

      if (Test-Path $altReport) {
        $reportPath = $altReport
      } elseif (!(Test-Path $reportPath)) {
        throw "Nsight Systems bounded capture failed and the whole-process fallback also did not generate a report at '$reportPath' or '$altReport'."
      }
    } elseif (!(Test-Path $reportPath)) {
      throw "Nsight Systems report not generated at '$reportPath' or '$altReport'."
    }
  }
} finally {
  [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", $prevFastPath, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", $prevDynamic, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $prevBenchOnly, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_ENABLE_NVTX", $prevNvtx, "Process")
  [System.Environment]::SetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", $prevNsysNvtxRegisterOnly, "Process")
}

if ($exitCode -ne 0) {
  Write-Warning "Nsight Systems exited with code $exitCode but produced '$reportPath'; treating the capture as usable."
}

$reportPrefixPath = if ($reportPath.EndsWith(".nsys-rep")) {
  $reportPath.Substring(0, $reportPath.Length - ".nsys-rep".Length)
} elseif ($reportPath.EndsWith(".qdrep")) {
  $reportPath.Substring(0, $reportPath.Length - ".qdrep".Length)
} else {
  [System.IO.Path]::ChangeExtension($reportPath, $null)
}
$provenancePath = "${reportPrefixPath}.provenance.json"
$provenance = [ordered]@{
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  profiler = "nsight_systems"
  profiler_exe = $nsysExe
  target = $Target
  build_profile = $(if ($Target -eq "runner") { "release" } else { "bench" })
  requested_output_prefix = $outputPrefixPath
  report_path = $reportPath
  fallback_used = $fallbackUsed
  capture_mode_requested = $CaptureMode
  capture_mode_applied = $captureModeApplied
  nvtx_mode = $NvtxMode
  dynamic_strategy_mode = $DynamicStrategyMode
  cuda_graphs_mode = $CudaGraphsMode
  target_exe = $targetExe.FullName
  requested_command_args = $requestedNsysArgs
  runner = if ($Target -eq "runner") {
    [ordered]@{
      strategy = $RunnerStrategy
      adx_mode = $RunnerAdxMode
      workload = $RunnerWorkload
      iterations = $RunnerIterations
      warmup_iterations = $RunnerWarmupIterations
      heatmap_bins = $RunnerHeatmapBins
      device_id = $RunnerDeviceId
      metric_contract = $RunnerMetricContract
      include_trade_metrics = $RunnerIncludeTradeMetrics
    }
  } else {
    $null
  }
  bench = if ($Target -eq "bench") {
    [ordered]@{
      bench_name = $BenchName
      bench_filter = $BenchFilter
      bench_strategy_only = $BenchStrategyOnly
      sample_size = $BenchSampleSize
      measurement_time_sec = $BenchMeasurementTimeSec
      warmup_time_sec = $BenchWarmupTimeSec
    }
  } else {
    $null
  }
  environment = [ordered]@{
    VECTORBT_KERNEL_FAST_PATH = "1"
    VECTORBT_KERNEL_USE_CUDA_GRAPHS = $CudaGraphsMode
    VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL = $DynamicStrategyMode
    VECTORBT_GPU_NATIVE_BENCH_ONLY = $BenchStrategyOnly
    VECTORBT_ENABLE_NVTX = $NvtxMode
    NSYS_NVTX_PROFILER_REGISTER_ONLY = if ($NvtxMode -eq "on" -and $CaptureMode -eq "nvtx") { "0" } else { $prevNsysNvtxRegisterOnly }
  }
}
Write-JsonUtf8 -Path $provenancePath -Value $provenance

Write-Host "Nsight Systems report written to $reportPath"
Write-Host "Nsight Systems provenance written to $provenancePath"
