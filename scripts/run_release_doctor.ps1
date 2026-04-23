param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [string]$AppLocalDataDir = "",
  [switch]$RequirePackagedSmoke
)

$ErrorActionPreference = "Stop"

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

function Resolve-OutDir {
  param([string]$Value)
  if (-not [string]::IsNullOrWhiteSpace($Value)) {
    return [System.IO.Path]::GetFullPath($Value)
  }
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
  return Join-Path (Join-Path $repoRoot "output\\release_candidates") "${stamp}-${Variant}"
}

function Invoke-DoctorStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [string]$CommandText,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Action
  )

  Write-Host "==> $Name"
  & $Action | Out-Host
  $step = [ordered]@{
    name = $Name
    command = $CommandText
    exit_code = $LASTEXITCODE
    passed = ($LASTEXITCODE -eq 0)
  }
  if (-not $step.passed) {
    throw "$Name failed with exit code $LASTEXITCODE"
  }
  Write-Host "[PASS] $Name"
  return $step
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$checkSyncCommand = "node scripts/check_tauri_config_sync.mjs"
$coreReportPath = Join-Path $resolvedOutDir "readiness.core.json"
$readinessPath = Join-Path $resolvedOutDir "readiness.json"

$cargoArgs = @("run", "-p", "ta_desktop_demo_app", "--bin", "ta_release_readiness")
switch ($Variant) {
  "gpu-ma" { $cargoArgs += @("--features", "cuda") }
  "gpu-kernel" { $cargoArgs += @("--features", "cuda-backtest-kernel") }
}
$cargoArgs += @("--", "--variant", $Variant, "--json", $coreReportPath)
if ($RequirePackagedSmoke) {
  $cargoArgs += "--require-packaged-smoke"
}
if (-not [string]::IsNullOrWhiteSpace($AppLocalDataDir)) {
  $cargoArgs += @("--app-local-data-dir", [System.IO.Path]::GetFullPath($AppLocalDataDir))
}

$steps = @()
try {
  $steps += Invoke-DoctorStep -Name "Tauri config sync" -CommandText $checkSyncCommand -Action {
    node "$repoRoot\\scripts\\check_tauri_config_sync.mjs"
  }

  $steps += Invoke-DoctorStep -Name "Release readiness core" -CommandText ("cargo " + ($cargoArgs -join " ")) -Action {
    & cargo @cargoArgs
  }

  if (!(Test-Path $coreReportPath)) {
    throw "Release readiness core report was not written to $coreReportPath"
  }
  $core = Get-Content -Raw $coreReportPath | ConvertFrom-Json

  $report = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = $Variant
    out_dir = $resolvedOutDir
    packaged_smoke_required = [bool]$RequirePackagedSmoke
    supported = [bool]$core.supported
    steps = $steps
    core = $core
  }

  Write-JsonUtf8 -Path $readinessPath -Value $report

  Write-Host ""
  Write-Host "Release doctor summary:"
  Write-Host "  variant: $Variant"
  Write-Host "  supported: $($core.supported)"
  Write-Host "  errors: $(@($core.errors).Count)"
  Write-Host "  warnings: $(@($core.warnings).Count)"
  Write-Host "  report: $readinessPath"

  if (-not $core.supported) {
    throw "Release doctor found blocking errors. See $readinessPath"
  }
} catch {
  $failure = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = $Variant
    out_dir = $resolvedOutDir
    packaged_smoke_required = [bool]$RequirePackagedSmoke
    supported = $false
    error = $_.Exception.Message
    steps = $steps
  }
  Write-JsonUtf8 -Path $readinessPath -Value $failure
  throw
}
