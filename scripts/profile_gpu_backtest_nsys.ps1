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

$outputPrefixPath = if ([System.IO.Path]::IsPathRooted($OutputPrefix)) {
  $OutputPrefix
} else {
  Join-Path (Get-Location) $OutputPrefix
}
$outDir = Split-Path -Parent $outputPrefixPath
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$prevFastPath = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "Process")
$prevGraphs = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "Process")
$prevDynamic = [System.Environment]::GetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "Process")
$prevBenchOnly = [System.Environment]::GetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", "Process")
$prevNvtx = [System.Environment]::GetEnvironmentVariable("VECTORBT_ENABLE_NVTX", "Process")

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

if ($Target -eq "runner" -and $CudaGraphsMode -eq "on" -and $RunnerHeatmapBins -gt 0) {
  Write-Warning "RunnerHeatmapBins=$RunnerHeatmapBins may keep the exact kernel path graph-ineligible; set -RunnerHeatmapBins 0 for a graph A/B."
}

Write-Host "Profiling with Nsight Systems (dynamic_strategy=$DynamicStrategyMode, cuda_graphs=$CudaGraphsMode)..."
$nsysArgs = @(
  "profile",
  "--sample=none",
  "--cpuctxsw=none",
  "--wait=primary",
  "--stop-on-exit=true",
  "--stats=false",
  "--force-overwrite=true",
  "--output=$outputPrefixPath"
)
if ($NvtxMode -eq "on") {
  $nsysArgs += @(
    "--trace=cuda,nvtx",
    "--cuda-graph-trace=graph"
  )
  if ($CaptureMode -eq "nvtx") {
    $nsysArgs += @(
      "--capture-range=nvtx",
      "--nvtx-capture=$NvtxCapture",
      "--nvtx-domain-include=$NvtxDomains"
    )
  }
} else {
  $nsysArgs += @("--trace=cuda")
}
if ($Target -eq "runner") {
  $nsysArgs += @(
    $targetExe.FullName,
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
} else {
  $nsysArgs += @(
    $targetExe.FullName,
    $BenchFilter,
    "--sample-size", "$BenchSampleSize",
    "--measurement-time", "$BenchMeasurementTimeSec",
    "--warm-up-time", "$BenchWarmupTimeSec"
  )
}

$nsysWorkDir = if ($outDir -and (Test-Path (Join-Path $outDir "nvlog.config"))) {
  $outDir
} else {
  $null
}
if ($null -ne $nsysWorkDir) {
  Push-Location $nsysWorkDir
  try {
    & $nsysExe @nsysArgs
    $exitCode = $LASTEXITCODE
  } finally {
    Pop-Location
  }
} else {
  & $nsysExe @nsysArgs
  $exitCode = $LASTEXITCODE
}

[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", $prevFastPath, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", $prevDynamic, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $prevBenchOnly, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_ENABLE_NVTX", $prevNvtx, "Process")

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
  } elseif (!(Test-Path $reportPath)) {
    throw "Nsight Systems report not generated at '$reportPath' or '$altReport'."
  }
}

if ($exitCode -ne 0) {
  Write-Warning "Nsight Systems exited with code $exitCode but produced '$reportPath'; treating the capture as usable."
}

Write-Host "Nsight Systems report written to $reportPath"
