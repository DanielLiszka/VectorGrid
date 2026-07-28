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

function Resolve-CargoTargetDirectory {
  $manifestPath = Join-Path $repoRoot "Cargo.toml"
  Push-Location $repoRoot
  try {
    $metadataText = (& cargo metadata --manifest-path $manifestPath --no-deps --format-version 1 2>&1 | Out-String)
  } finally {
    Pop-Location
  }
  if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed while resolving the canonical target directory."
  }
  $metadata = $metadataText | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace([string]$metadata.target_directory)) {
    throw "cargo metadata did not report target_directory."
  }
  return [System.IO.Path]::GetFullPath([string]$metadata.target_directory)
}

function Get-LatestMainBinaryFingerprint {
  $fingerprintRoot = Join-Path $targetDirectory "release\\.fingerprint"
  if (-not (Test-Path -LiteralPath $fingerprintRoot -PathType Container)) {
    return $null
  }
  $fingerprint = Get-ChildItem -LiteralPath $fingerprintRoot -Directory -Filter "ta_desktop_demo_app-*" |
    ForEach-Object {
      Get-ChildItem -LiteralPath $_.FullName -File -Filter "bin-ta_desktop_demo_app.json" -ErrorAction SilentlyContinue
    } |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
  if ($null -eq $fingerprint) {
    return $null
  }
  $metadata = Get-Content -Raw -LiteralPath $fingerprint.FullName | ConvertFrom-Json
  $features = @()
  if (-not [string]::IsNullOrWhiteSpace([string]$metadata.features)) {
    $features = @(([string]$metadata.features | ConvertFrom-Json))
  }
  return [ordered]@{
    path = $fingerprint.FullName
    last_write_time_utc = $fingerprint.LastWriteTimeUtc.ToString("o")
    features = $features
  }
}

function Get-GitCommit {
  try {
    return (& git -C $repoRoot rev-parse HEAD 2>$null).Trim()
  } catch {
    return $null
  }
}

function Get-GitDirty {
  try {
    $status = & git -C $repoRoot status --porcelain 2>$null
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

  $candidate = Join-Path $targetDirectory "release\\ta_desktop_demo_app.exe"
  if (Test-Path -LiteralPath $candidate -PathType Leaf) {
    return (Resolve-Path -LiteralPath $candidate).Path
  }
  return $null
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$targetDirectory = Resolve-CargoTargetDirectory
$resolvedOutDir = Resolve-OutDir -Value $OutDir
$provenanceJsonPath = Join-Path $resolvedOutDir "provenance.json"
$provenanceTxtPath = Join-Path $resolvedOutDir "provenance.txt"

New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

Push-Location $repoRoot
try {
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
  $projectTauriInfo = Get-ToolInfo -Name "tauri.cmd" -FallbackPaths @(
    (Join-Path $repoRoot "web\\node_modules\\.bin\\tauri.cmd")
  )
  $nvidiaSmiInfo = Get-ToolInfo -Name "nvidia-smi" -Args @("--query-gpu=name,driver_version", "--format=csv,noheader")
} finally {
  Pop-Location
}
$cudaPath = if ([string]::IsNullOrWhiteSpace([string]$env:CUDA_PATH)) {
  $null
} else {
  [System.IO.Path]::GetFullPath([string]$env:CUDA_PATH)
}
$cudaNvccFallbacks = if ($null -eq $cudaPath) { @() } else { @((Join-Path $cudaPath "bin\\nvcc.exe")) }
$cudaPtxasFallbacks = if ($null -eq $cudaPath) { @() } else { @((Join-Path $cudaPath "bin\\ptxas.exe")) }
$selectedNvccInfo = Get-ToolInfo -Name "nvcc" -Args @("--version") -FallbackPaths $cudaNvccFallbacks
$selectedPtxasInfo = Get-ToolInfo -Name "ptxas" -Args @("--version") -FallbackPaths $cudaPtxasFallbacks
$pathNvccInfo = Get-ToolInfo -Name "nvcc" -Args @("--version")
$pathPtxasInfo = Get-ToolInfo -Name "ptxas" -Args @("--version")

$resolvedBinaryPath = Resolve-BinaryPath -Value $BinaryPath
$binaryExists = ($null -ne $resolvedBinaryPath) -and (Test-Path -LiteralPath $resolvedBinaryPath -PathType Leaf)
$binaryItem = if ($binaryExists) { Get-Item -LiteralPath $resolvedBinaryPath } else { $null }
$binaryFingerprint = Get-LatestMainBinaryFingerprint

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
    project_tauri_cli = $projectTauriInfo
    node = $nodeInfo
    npm = $npmInfo
    cuda = [ordered]@{
      cuda_path = $cudaPath
      selected_nvcc = $selectedNvccInfo
      selected_ptxas = $selectedPtxasInfo
      path_nvcc = $pathNvccInfo
      path_ptxas = $pathPtxasInfo
    }
  }
  desktop_tooling = [ordered]@{
    tauri_driver = $tauriDriverInfo
    msedgedriver = $edgeDriverInfo
  }
  gpu = [ordered]@{
    nvidia_smi = $nvidiaSmiInfo
  }
  env = [ordered]@{
    CUDA_PATH = $env:CUDA_PATH
    CUDA_HOME = $env:CUDA_HOME
    VECTORBT_USE_PREBUILT_PTX = $env:VECTORBT_USE_PREBUILT_PTX
    VECTORBT_REQUIRE_NVCC = $env:VECTORBT_REQUIRE_NVCC
  }
  binary = [ordered]@{
    path = $resolvedBinaryPath
    exists = $binaryExists
    size_bytes = if ($binaryExists) { $binaryItem.Length } else { $null }
    sha256 = if ($binaryExists) { (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedBinaryPath).Hash.ToLowerInvariant() } else { $null }
    last_write_time_utc = if ($binaryExists) { $binaryItem.LastWriteTimeUtc.ToString("o") } else { $null }
    cargo_target_directory = $targetDirectory
    cargo_fingerprint = $binaryFingerprint
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
  "project_tauri_cli: $(Format-OneLine $projectTauriInfo.version)",
  "node: $(Format-OneLine $nodeInfo.version)",
  "npm: $(Format-OneLine $npmInfo.version)",
  "CUDA_PATH: $cudaPath",
  "selected_nvcc: $(Format-OneLine $selectedNvccInfo.version)",
  "selected_nvcc_path: $($selectedNvccInfo.path)",
  "selected_ptxas: $(Format-OneLine $selectedPtxasInfo.version)",
  "selected_ptxas_path: $($selectedPtxasInfo.path)",
  "PATH_nvcc: $(Format-OneLine $pathNvccInfo.version)",
  "PATH_nvcc_path: $($pathNvccInfo.path)",
  "PATH_ptxas: $(Format-OneLine $pathPtxasInfo.version)",
  "PATH_ptxas_path: $($pathPtxasInfo.path)",
  "tauri_driver: $(Format-OneLine $tauriDriverInfo.version)",
  "msedgedriver: $(Format-OneLine $edgeDriverInfo.version)",
  "nvidia_smi: $(Format-OneLine $nvidiaSmiInfo.version)",
  "VECTORBT_USE_PREBUILT_PTX: $($env:VECTORBT_USE_PREBUILT_PTX)",
  "VECTORBT_REQUIRE_NVCC: $($env:VECTORBT_REQUIRE_NVCC)",
  "binary: $resolvedBinaryPath",
  "binary_size_bytes: $($provenance.binary.size_bytes)",
  "binary_sha256: $($provenance.binary.sha256)",
  "binary_last_write_time_utc: $($provenance.binary.last_write_time_utc)",
  "binary_fingerprint_features: $($binaryFingerprint.features -join ',')"
)
Set-Content -Path $provenanceTxtPath -Encoding UTF8 -Value $summaryLines

Write-Host "Release provenance written to $provenanceJsonPath"
