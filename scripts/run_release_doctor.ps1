param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [string]$AppLocalDataDir = "",
  [switch]$RequirePackagedSmoke,
  [switch]$RequirePublicSigning
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

function Read-JsonFile {
  param([string]$Path)
  if (-not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path $Path)) {
    return Get-Content -Raw $Path | ConvertFrom-Json
  }
  return $null
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
$nodeReproPath = Join-Path $resolvedOutDir "node-reproducibility.json"
$secretHygienePath = Join-Path $resolvedOutDir "release-secret-hygiene.json"
$authorityMatrixPath = Join-Path $resolvedOutDir "release-authority-matrix-contract.json"
$commandRiskPolicyPath = Join-Path $resolvedOutDir "tauri-command-risk-policy.json"
$coreReportPath = Join-Path $resolvedOutDir "readiness.core.json"
$readinessPath = Join-Path $resolvedOutDir "readiness.json"

$cargoFeatures = @("desktop-ui", "maintainer-tools")
switch ($Variant) {
  "gpu-ma" { $cargoFeatures += "cuda" }
  "gpu-kernel" { $cargoFeatures += "cuda-backtest-kernel" }
}
$cargoArgs = @(
  "run",
  "-p", "ta_desktop_demo_app",
  "--bin", "ta_release_readiness",
  "--features", ($cargoFeatures -join ",")
)
$cargoArgs += @("--", "--variant", $Variant, "--json", $coreReportPath)
if ($RequirePackagedSmoke) {
  $cargoArgs += "--require-packaged-smoke"
}
if ($RequirePublicSigning) {
  $cargoArgs += "--require-public-signing"
}
if (-not [string]::IsNullOrWhiteSpace($AppLocalDataDir)) {
  $cargoArgs += @("--app-local-data-dir", [System.IO.Path]::GetFullPath($AppLocalDataDir))
}
$readinessCommand = "cargo " + ($cargoArgs -join " ")
if ($Variant -eq "gpu-kernel") {
  $readinessCommand = '$env:VECTORBT_USE_PREBUILT_PTX="1"; ' + $readinessCommand
}

$steps = @()
try {
  $steps += Invoke-DoctorStep -Name "Tauri config sync" -CommandText $checkSyncCommand -Action {
    node "$repoRoot\\scripts\\check_tauri_config_sync.mjs"
  }

  $steps += Invoke-DoctorStep -Name "Node/npm reproducibility" -CommandText "node scripts/check_node_reproducibility.mjs --out $nodeReproPath" -Action {
    node "$repoRoot\\scripts\\check_node_reproducibility.mjs" --out $nodeReproPath
  }

  $steps += Invoke-DoctorStep -Name "Release secret hygiene" -CommandText "node scripts/check_release_secret_hygiene.mjs --out $secretHygienePath" -Action {
    node "$repoRoot\\scripts\\check_release_secret_hygiene.mjs" --out $secretHygienePath
  }

  $steps += Invoke-DoctorStep -Name "Release authority matrix contract" -CommandText "node scripts/check_release_authority_matrix.mjs --out $authorityMatrixPath" -Action {
    node "$repoRoot\\scripts\\check_release_authority_matrix.mjs" --out $authorityMatrixPath
  }

  $steps += Invoke-DoctorStep -Name "Tauri command risk policy" -CommandText "node web/scripts/check-tauri-command-risk-policy.mjs --out $commandRiskPolicyPath" -Action {
    node "$repoRoot\\web\\scripts\\check-tauri-command-risk-policy.mjs" --out $commandRiskPolicyPath
  }

  $steps += Invoke-DoctorStep -Name "Release readiness core" -CommandText $readinessCommand -Action {
    $previousPrebuiltPtx = $env:VECTORBT_USE_PREBUILT_PTX
    try {
      if ($Variant -eq "gpu-kernel") {
        $env:VECTORBT_USE_PREBUILT_PTX = "1"
      }
      & cargo @cargoArgs
    } finally {
      if ($null -eq $previousPrebuiltPtx) {
        Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
      } else {
        $env:VECTORBT_USE_PREBUILT_PTX = $previousPrebuiltPtx
      }
    }
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
    public_signing_required = [bool]$RequirePublicSigning
    supported = [bool]$core.supported
    steps = $steps
    node_reproducibility = Read-JsonFile -Path $nodeReproPath
    secret_hygiene = Read-JsonFile -Path $secretHygienePath
    authority_matrix = Read-JsonFile -Path $authorityMatrixPath
    command_risk_policy = Read-JsonFile -Path $commandRiskPolicyPath
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
    public_signing_required = [bool]$RequirePublicSigning
    supported = $false
    error = $_.Exception.Message
    steps = $steps
  }
  Write-JsonUtf8 -Path $readinessPath -Value $failure
  throw
}
