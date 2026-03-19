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
  [ValidateSet("off", "on")]
  [string]$DynamicStrategyMode = "off"
)

$ErrorActionPreference = "Stop"

$nsysCandidates = @(
  "C:\Program Files\NVIDIA Corporation\Nsight Systems 2023.3.3\target-windows-x64\nsys.exe",
  "C:\Program Files\NVIDIA Corporation\Nsight Systems 2025.3.2\target-windows-x64\nsys.exe",
  "C:\Program Files\NVIDIA Corporation\Nsight Systems 2022.4.2\target-windows-x64\nsys.exe",
  "C:\Program Files\NVIDIA Corporation\Nsight Compute 2025.3.1\host\target-windows-x64\nsys.exe"
)
$nsysExe = $nsysCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $nsysExe) {
  $cmd = Get-Command nsys.exe -ErrorAction SilentlyContinue
  if ($null -ne $cmd) {
    $nsysExe = $cmd.Source
  }
}
if (-not $nsysExe -or !(Test-Path $nsysExe)) {
  throw "Nsight Systems CLI not found (expected nsys.exe)."
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

Write-Host "Profiling with Nsight Systems (dynamic_strategy=$DynamicStrategyMode)..."
$nsysArgs = @(
  "profile",
  "--trace=cuda",
  "--sample=none",
  "--cpuctxsw=none",
  "--wait=primary",
  "--stop-on-exit=true",
  "--stats=false",
  "--force-overwrite=true",
  "--output=$OutputPrefix"
)
if ($Target -eq "runner") {
  $nsysArgs += @(
    $targetExe.FullName,
    "--strategy", $RunnerStrategy,
    "--adx-mode", $RunnerAdxMode,
    "--workload", $RunnerWorkload,
    "--iterations", "$RunnerIterations",
    "--dynamic-mode", $DynamicStrategyMode
  )
} else {
  $nsysArgs += @(
    $targetExe.FullName,
    $BenchFilter,
    "--sample-size", "$BenchSampleSize",
    "--measurement-time", "$BenchMeasurementTimeSec",
    "--warm-up-time", "$BenchWarmupTimeSec"
  )
}

& $nsysExe @nsysArgs
$exitCode = $LASTEXITCODE

[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", $prevFastPath, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", $prevDynamic, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $prevBenchOnly, "Process")

$reportPath = "$OutputPrefix.nsys-rep"
if (!(Test-Path $reportPath)) {
  $altReport = "$OutputPrefix.qdrep"
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
    throw "Nsight Systems report not generated at '$reportPath' or '$altReport'."
  }
}

if ($exitCode -ne 0) {
  Write-Warning "Nsight Systems exited with code $exitCode but produced '$reportPath'; treating the capture as usable."
}

Write-Host "Nsight Systems report written to $reportPath"
