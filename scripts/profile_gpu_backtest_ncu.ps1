param(
  [string]$BenchFilter = "double_ma/gpu_kernel/sma_sma/200k/small",
  [string]$OutputPrefix = "target/nsight/gpu_kernel_small",
  [int]$LaunchCount = 200,
  [string]$KernelName = "double_cross_backtest_tm_flex_f32"
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

Write-Host "Building benchmark executable..."
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

$benchExe = Get-ChildItem -Path "target/release/deps" -File -Filter "double_ma_opt-*.exe" |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
if ($null -eq $benchExe) {
  throw "Could not find built bench executable: target/release/deps/double_ma_opt-*.exe"
}

$outDir = Split-Path -Parent $OutputPrefix
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$prevFastPath = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "Process")
$prevGraphs = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "Process")
$prevMetrics = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_METRICS_COUNT", "Process")
$prevSkip = [System.Environment]::GetEnvironmentVariable("VECTORBT_KERNEL_SKIP_RECOMPUTE", "Process")

[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", "1", "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "0", "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_METRICS_COUNT", "7", "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_SKIP_RECOMPUTE", $null, "Process")

Write-Host "Profiling with Nsight Compute..."
& $ncuExe `
  --target-processes all `
  --set full `
  --kernel-name-base demangled `
  --kernel-name $KernelName `
  --launch-count $LaunchCount `
  --force-overwrite `
  --export $OutputPrefix `
  $benchExe.FullName `
  $BenchFilter `
  --sample-size 10 `
  --measurement-time 3 `
  --warm-up-time 1
$exitCode = $LASTEXITCODE

[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", $prevFastPath, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_METRICS_COUNT", $prevMetrics, "Process")
[System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_SKIP_RECOMPUTE", $prevSkip, "Process")

if ($exitCode -ne 0) {
  exit $exitCode
}

Write-Host "Nsight report written to $OutputPrefix.ncu-rep"
