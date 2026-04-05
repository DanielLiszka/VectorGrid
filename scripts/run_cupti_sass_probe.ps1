param(
    [ValidateSet("inventory", "capture")]
    [string]$Mode = "inventory",
    [ValidateSet("large", "xlarge")]
    [string]$Workload = "large",
    [ValidateSet("auto", "full17", "trade22")]
    [string]$MetricContract = "full17",
    [ValidateSet("none", "halves", "quartiles")]
    [string]$ShortlistSliceMode = "none",
    [string]$KernelFilter = "double_cross_backtest_tm_flex_f32_lockstep_time",
    [string]$Metric = "smsp__sass_inst_executed",
    [string]$OutputDir = "target/cupti_sass_probe",
    [int]$DeviceId = 0
)

$ErrorActionPreference = "Stop"

$blocked = @(
    "profile_gpu_dynamic_runner",
    "ncu",
    "nsys",
    "cupti_sass_probe"
)

$active = Get-Process -ErrorAction SilentlyContinue |
    Where-Object { $blocked -contains $_.ProcessName }

if ($active) {
    $names = ($active | Select-Object -ExpandProperty ProcessName | Sort-Object -Unique) -join ", "
    throw "Refusing to start CUPTI probe while these processes are active: $names"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$artifactDir = Join-Path $repoRoot $OutputDir
New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null

$args = @(
    "run",
    "-p",
    "ta_app_core",
    "--features",
    "cuda,cuda-backtest-kernel",
    "--bin",
    "cupti_sass_probe",
    "--",
    "--mode",
    $Mode,
    "--workload",
    $Workload,
    "--metric-contract",
    $MetricContract,
    "--shortlist-slice-mode",
    $ShortlistSliceMode,
    "--kernel-filter",
    $KernelFilter,
    "--metric",
    $Metric,
    "--device-id",
    "$DeviceId",
    "--output-dir",
    $artifactDir
)

$logPath = Join-Path $artifactDir "run_cupti_sass_probe_last_command.txt"
($args -join " ") | Set-Content -Path $logPath

Push-Location $repoRoot
try {
    cargo @args
}
finally {
    Pop-Location
}
