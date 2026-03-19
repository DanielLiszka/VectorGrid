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
  [int]$RunnerIterations = 1,
  [string]$OutputPrefix = "target/nsight/gpu_kernel_small",
  [int]$LaunchCount = 200,
  [string]$KernelName = "regex:^double_cross_backtest_tm_flex_f32",
  [bool]$UseKernelNameFilter = $false,
  [ValidateSet("off", "on")]
  [string]$DynamicStrategyMode = "off"
)

$ErrorActionPreference = "Stop"

$ncuExe = "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\ncu.bat"
if (!(Test-Path $ncuExe)) {
  $cmd = Get-Command ncu.bat -ErrorAction SilentlyContinue
  if ($null -ne $cmd) {
    $ncuExe = $cmd.Source
  }
}
if (!(Test-Path $ncuExe)) {
  throw "Nsight Compute CLI not found (expected ncu.bat)."
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

$outDir = Split-Path -Parent $OutputPrefix
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$prevFastPath = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "Process")
$prevGraphs = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "Process")
$prevDynamic = [System.Environment]::GetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "Process")
$prevBenchOnly = [System.Environment]::GetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", "Process")

[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "1", "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "0", "Process")
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

Write-Host "Profiling with Nsight Compute (dynamic_strategy=$DynamicStrategyMode)..."
$ncuArgs = @(
  "--target-processes", "all",
  "--set", "full",
  "--kernel-name-base", "demangled"
)
if ($UseKernelNameFilter -and ![string]::IsNullOrWhiteSpace($KernelName)) {
  $ncuArgs += @("--kernel-name", $KernelName)
}
$ncuArgs += @(
  "--launch-count", "$LaunchCount",
  "--force-overwrite",
  "--export", $OutputPrefix
)
if ($Target -eq "runner") {
  $ncuArgs += @(
    $targetExe.FullName,
    "--strategy", $RunnerStrategy,
    "--adx-mode", $RunnerAdxMode,
    "--workload", $RunnerWorkload,
    "--iterations", "$RunnerIterations",
    "--dynamic-mode", $DynamicStrategyMode
  )
} else {
  $ncuArgs += @(
    $targetExe.FullName,
    $BenchFilter,
    "--sample-size", "$BenchSampleSize",
    "--measurement-time", "$BenchMeasurementTimeSec",
    "--warm-up-time", "$BenchWarmupTimeSec"
  )
}

& $ncuExe @ncuArgs
$exitCode = $LASTEXITCODE

[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", $prevFastPath, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", $prevDynamic, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $prevBenchOnly, "Process")

if ($exitCode -ne 0) {
  exit $exitCode
}

$reportPath = "$OutputPrefix.ncu-rep"
if (!(Test-Path $reportPath)) {
  throw "Nsight report not generated at '$reportPath' (likely no kernels were profiled)."
}

Write-Host "Nsight report written to $reportPath"
