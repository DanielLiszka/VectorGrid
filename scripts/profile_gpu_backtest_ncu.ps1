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
  [string]$RunnerFastMa = "sma",
  [string]$RunnerSlowMa = "sma",
  [ValidateSet("off", "on")]
  [string]$RunnerAdxMode = "off",
  [ValidateSet("small", "medium", "large", "xlarge")]
  [string]$RunnerWorkload = "xlarge",
  [ValidateSet("auto", "full17", "trade22")]
  [string]$RunnerMetricContract = "auto",
  [ValidateSet("none", "halves", "quartiles")]
  [string]$RunnerShortlistSliceMode = "none",
  [ValidateSet("inherit", "off", "on")]
  [string]$RunnerIncludeTradeMetrics = "inherit",
  [ValidateSet("auto", "synth", "trending", "choppy", "breakout")]
  [string]$RunnerCandleShape = "auto",
  [int]$RunnerIterations = 1,
  [int]$RunnerWarmupIterations = 0,
  [int]$RunnerHeatmapBins = 64,
  [int]$RunnerDeviceId = 0,
  [string]$OutputPrefix = "output/profiling/nsight/gpu_kernel_small",
  [int]$LaunchCount = 200,
  [string]$KernelName = "regex:^double_cross_backtest_tm_flex_f32",
  [bool]$UseKernelNameFilter = $false,
  [ValidateSet("off", "on")]
  [string]$DynamicStrategyMode = "off",
  [ValidateSet("inherit", "off", "on")]
  [string]$CudaGraphsMode = "off",
  [string]$NcuExePath = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")

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

$ncuExe = $NcuExePath
if ([string]::IsNullOrWhiteSpace($ncuExe)) {
  $ncuCandidates = @(
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\ncu.bat",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\target\windows-desktop-win7-x64\ncu.exe",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\ncu.bat",
    "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\host\target-windows-x64\ncu.exe"
  )
  $ncuExe = $ncuCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (-not $ncuExe) {
    $cmd = Get-Command ncu.bat -ErrorAction SilentlyContinue
    if ($null -eq $cmd) {
      $cmd = Get-Command ncu.exe -ErrorAction SilentlyContinue
    }
    if ($null -ne $cmd) {
      $ncuExe = $cmd.Source
    }
  }
}
if (!(Test-Path $ncuExe)) {
  throw "Nsight Compute CLI not found (expected ncu.bat or ncu.exe)."
}

$benchmarkLock = Enter-BenchmarkLock -Context "GPU Nsight Compute profile"
try {
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

$outDir = Split-Path -Parent $OutputPrefix
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$prevFastPath = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "Process")
$prevGraphs = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "Process")
$prevDynamic = [System.Environment]::GetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "Process")
$prevBenchOnly = [System.Environment]::GetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", "Process")
$exitCode = $null
$requestedNcuArgs = $null
$reportPath = $null
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

  if ($Target -eq "runner" -and $CudaGraphsMode -eq "on" -and $RunnerHeatmapBins -gt 0) {
    Write-Warning "RunnerHeatmapBins=$RunnerHeatmapBins may keep the exact kernel path graph-ineligible; set -RunnerHeatmapBins 0 for a graph A/B."
  }

  Write-Host "Profiling with Nsight Compute (dynamic_strategy=$DynamicStrategyMode, cuda_graphs=$CudaGraphsMode)..."
  $requestedNcuArgs = @(
    "--target-processes", "all",
    "--set", "full",
    "--kernel-name-base", "demangled"
  )
  if ($UseKernelNameFilter -and ![string]::IsNullOrWhiteSpace($KernelName)) {
    $requestedNcuArgs += @("--kernel-name", $KernelName)
  }
  $requestedNcuArgs += @(
    "--launch-count", "$LaunchCount",
    "--force-overwrite",
    "--export", $OutputPrefix
  )
  if ($Target -eq "runner") {
    $requestedNcuArgs += @(
      $targetExe.FullName,
      "--strategy", $RunnerStrategy,
      "--fast-ma", $RunnerFastMa,
      "--slow-ma", $RunnerSlowMa,
      "--adx-mode", $RunnerAdxMode,
      "--workload", $RunnerWorkload,
      "--candle-shape", $RunnerCandleShape,
      "--iterations", "$RunnerIterations",
      "--warmup-iterations", "$RunnerWarmupIterations",
      "--heatmap-bins", "$RunnerHeatmapBins",
      "--device-id", "$RunnerDeviceId",
      "--dynamic-mode", $DynamicStrategyMode,
      "--cuda-graphs", $CudaGraphsMode
    )
    if ($RunnerMetricContract -ne "auto") {
      $requestedNcuArgs += @("--metric-contract", $RunnerMetricContract)
    }
    if ($RunnerShortlistSliceMode -ne "none") {
      $requestedNcuArgs += @("--shortlist-slice-mode", $RunnerShortlistSliceMode)
    }
    if ($RunnerIncludeTradeMetrics -ne "inherit") {
      $requestedNcuArgs += @("--include-trade-metrics", $RunnerIncludeTradeMetrics)
    }
  } else {
    $requestedNcuArgs += @(
      $targetExe.FullName,
      $BenchFilter,
      "--sample-size", "$BenchSampleSize",
      "--measurement-time", "$BenchMeasurementTimeSec",
      "--warm-up-time", "$BenchWarmupTimeSec"
    )
  }

  & $ncuExe @requestedNcuArgs
  $exitCode = $LASTEXITCODE
  $reportPath = "$OutputPrefix.ncu-rep"
  if ($exitCode -eq 0 -and !(Test-Path $reportPath)) {
    throw "Nsight report not generated at '$reportPath' (likely no kernels were profiled)."
  }
} finally {
  [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", $prevFastPath, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", $prevDynamic, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $prevBenchOnly, "Process")
}

$reportPath = if ([string]::IsNullOrWhiteSpace($reportPath)) { "$OutputPrefix.ncu-rep" } else { $reportPath }
$provenancePath = "$OutputPrefix.provenance.json"
$provenance = [ordered]@{
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  profiler = "nsight_compute"
  profiler_exe = $ncuExe
  target = $Target
  build_profile = $(if ($Target -eq "runner") { "release" } else { "bench" })
  report_path = $reportPath
  requested_output_prefix = $OutputPrefix
  requested_command_args = $requestedNcuArgs
  runner = if ($Target -eq "runner") {
    [ordered]@{
      strategy = $RunnerStrategy
      fast_ma = $RunnerFastMa
      slow_ma = $RunnerSlowMa
      adx_mode = $RunnerAdxMode
      workload = $RunnerWorkload
      candle_shape = $RunnerCandleShape
      shortlist_slice_mode = $RunnerShortlistSliceMode
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
  }
}
Write-JsonUtf8 -Path $provenancePath -Value $provenance

if ($exitCode -ne 0) {
  throw "Nsight Compute exited with code $exitCode."
}

Write-Host "Nsight report written to $reportPath"
Write-Host "Nsight Compute provenance written to $provenancePath"
} finally {
  Exit-BenchmarkLock -Handle $benchmarkLock
}
