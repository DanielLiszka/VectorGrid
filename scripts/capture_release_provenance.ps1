param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [string]$BinaryPath = ""
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
  return Join-Path (Join-Path $PSScriptRoot "..\\output\\release_candidates") "${stamp}-${Variant}"
}

function Get-CommandPathOrNull {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [string[]]$FallbackPaths = @()
  )

  foreach ($candidate in $FallbackPaths) {
    if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path $candidate)) {
      return (Resolve-Path $candidate).Path
    }
  }

  $cmd = Get-Command $Name -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($null -ne $cmd) {
    return $cmd.Source
  }

  return $null
}

function Get-ToolInfo {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [string[]]$Args = @("--version"),
    [string[]]$FallbackPaths = @()
  )

  $path = Get-CommandPathOrNull -Name $Name -FallbackPaths $FallbackPaths
  if ($null -eq $path) {
    return [ordered]@{
      available = $false
      path = $null
      version = $null
    }
  }

  try {
    $output = & $path @Args 2>&1 | Out-String
    $version = $output.Trim()
  } catch {
    $version = $null
  }

  return [ordered]@{
    available = $true
    path = $path
    version = if ([string]::IsNullOrWhiteSpace($version)) { $null } else { $version }
  }
}

function Get-CargoTauriInfo {
  $cargo = Get-CommandPathOrNull -Name "cargo"
  if ($null -eq $cargo) {
    return [ordered]@{
      available = $false
      path = $null
      version = $null
    }
  }

  try {
    $output = & $cargo "tauri" "--version" 2>&1 | Out-String
    $version = $output.Trim()
    if ([string]::IsNullOrWhiteSpace($version)) {
      $version = $null
    }
    return [ordered]@{
      available = $LASTEXITCODE -eq 0
      path = $cargo
      version = $version
    }
  } catch {
    return [ordered]@{
      available = $false
      path = $cargo
      version = $null
    }
  }
}

function Get-GitCommit {
  try {
    return (& git rev-parse HEAD 2>$null).Trim()
  } catch {
    return $null
  }
}

function Get-GitDirty {
  try {
    $status = & git status --porcelain 2>$null
    return -not [string]::IsNullOrWhiteSpace(($status | Out-String).Trim())
  } catch {
    return $null
  }
}

function Format-OneLine {
  param([string]$Value)
  if ([string]::IsNullOrWhiteSpace($Value)) {
    return ""
  }
  return (($Value -replace "\s+", " ").Trim())
}

function Resolve-BinaryPath {
  param([string]$Value)
  if (-not [string]::IsNullOrWhiteSpace($Value)) {
    return [System.IO.Path]::GetFullPath($Value)
  }

  $candidates = @(
    (Join-Path $repoRoot "src-tauri\\target\\release\\ta_desktop_demo_app.exe"),
    (Join-Path $repoRoot "target\\release\\ta_desktop_demo_app.exe")
  )
  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return (Resolve-Path $candidate).Path
    }
  }
  return $null
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
$provenanceJsonPath = Join-Path $resolvedOutDir "provenance.json"
$provenanceTxtPath = Join-Path $resolvedOutDir "provenance.txt"

New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$cargoInfo = Get-ToolInfo -Name "cargo" -Args @("--version", "--verbose")
$rustcInfo = Get-ToolInfo -Name "rustc" -Args @("--version", "--verbose")
$nodeInfo = Get-ToolInfo -Name "node"
$npmInfo = Get-ToolInfo -Name "npm.cmd" -FallbackPaths @(
  (Join-Path ${env:ProgramFiles} "nodejs\\npm.cmd")
)
$tauriDriverInfo = Get-ToolInfo -Name "tauri-driver" -Args @("--help")
$edgeDriverInfo = Get-ToolInfo -Name "msedgedriver" -FallbackPaths @(
  (Join-Path $repoRoot "msedgedriver.exe")
)
$cargoTauriInfo = Get-CargoTauriInfo
$nvidiaSmiInfo = Get-ToolInfo -Name "nvidia-smi" -Args @("--query-gpu=name,driver_version", "--format=csv,noheader")

$resolvedBinaryPath = Resolve-BinaryPath -Value $BinaryPath

$provenance = [ordered]@{
  schema_version = 1
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  variant = $Variant
  repo_root = $repoRoot
  git = [ordered]@{
    commit = Get-GitCommit
    dirty = Get-GitDirty
  }
  toolchain = [ordered]@{
    rust_toolchain_toml_present = Test-Path (Join-Path $repoRoot "rust-toolchain.toml")
    cargo = $cargoInfo
    rustc = $rustcInfo
    cargo_tauri = $cargoTauriInfo
    node = $nodeInfo
    npm = $npmInfo
  }
  desktop_tooling = [ordered]@{
    tauri_driver = $tauriDriverInfo
    msedgedriver = $edgeDriverInfo
  }
  gpu = [ordered]@{
    nvidia_smi = $nvidiaSmiInfo
  }
  env = [ordered]@{
    VECTORBT_USE_PREBUILT_PTX = $env:VECTORBT_USE_PREBUILT_PTX
    VECTORBT_REQUIRE_NVCC = $env:VECTORBT_REQUIRE_NVCC
  }
  binary = [ordered]@{
    path = $resolvedBinaryPath
    exists = ($null -ne $resolvedBinaryPath) -and (Test-Path $resolvedBinaryPath)
  }
}

Write-JsonUtf8 -Path $provenanceJsonPath -Value $provenance

$summaryLines = @(
  "Release provenance",
  "generated_at_utc: $($provenance.generated_at_utc)",
  "variant: $Variant",
  "repo_root: $repoRoot",
  "git_commit: $($provenance.git.commit)",
  "git_dirty: $($provenance.git.dirty)",
  "cargo: $(Format-OneLine $cargoInfo.version)",
  "rustc: $(Format-OneLine $rustcInfo.version)",
  "cargo_tauri: $(Format-OneLine $cargoTauriInfo.version)",
  "node: $(Format-OneLine $nodeInfo.version)",
  "npm: $(Format-OneLine $npmInfo.version)",
  "tauri_driver: $(Format-OneLine $tauriDriverInfo.version)",
  "msedgedriver: $(Format-OneLine $edgeDriverInfo.version)",
  "nvidia_smi: $(Format-OneLine $nvidiaSmiInfo.version)",
  "VECTORBT_USE_PREBUILT_PTX: $($env:VECTORBT_USE_PREBUILT_PTX)",
  "VECTORBT_REQUIRE_NVCC: $($env:VECTORBT_REQUIRE_NVCC)",
  "binary: $resolvedBinaryPath"
)
Set-Content -Path $provenanceTxtPath -Encoding UTF8 -Value $summaryLines

Write-Host "Release provenance written to $provenanceJsonPath"
