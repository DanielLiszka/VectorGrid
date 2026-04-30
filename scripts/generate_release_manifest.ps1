param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [string]$BundleRoot = "",
  [string]$ReleaseBinaryPath = "",
  [string]$ProvenancePath = "",
  [string]$Version = "",
  [switch]$AllowMissingCatboostNative,
  [switch]$UnsignedBeta
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 14
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

function Resolve-ExistingPath {
  param([string[]]$Candidates)
  foreach ($candidate in $Candidates) {
    if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path $candidate)) {
      return (Resolve-Path $candidate).Path
    }
  }
  return $null
}

function Get-ArtifactCandidates {
  param([Parameter(Mandatory = $true)][string]$Root)
  Get-ChildItem -Path $Root -Recurse -File | Where-Object {
    @(".msi", ".exe", ".zip", ".sig") -contains $_.Extension.ToLowerInvariant()
  } | Sort-Object FullName
}

function ConvertTo-SafeNamePart {
  param([Parameter(Mandatory = $true)][string]$Value)
  $part = ($Value.ToLowerInvariant() -replace "[^a-z0-9]+", "-").Trim("-")
  if ([string]::IsNullOrWhiteSpace($part)) {
    return "artifact"
  }
  return $part
}

function Get-CodeSignatureInfo {
  param([Parameter(Mandatory = $true)][string]$Path)

  $ext = [System.IO.Path]::GetExtension($Path).ToLowerInvariant()
  if (@(".exe", ".msi") -notcontains $ext) {
    return [ordered]@{
      checked = $false
      status = "not_applicable"
      signer_subject = $null
      thumbprint = $null
    }
  }

  try {
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    return [ordered]@{
      checked = $true
      status = [string]$signature.Status
      signer_subject = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Subject } else { $null }
      thumbprint = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Thumbprint } else { $null }
    }
  } catch {
    return [ordered]@{
      checked = $true
      status = "SignatureCheckFailed"
      signer_subject = $null
      thumbprint = $null
      error = $_.Exception.Message
    }
  }
}

function ConvertTo-RelativeSlashPath {
  param([string]$Path)
  if ([string]::IsNullOrWhiteSpace($Path)) {
    return $null
  }
  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $rootPath = [System.IO.Path]::GetFullPath($repoRoot)
  while ($rootPath.EndsWith("\") -or $rootPath.EndsWith("/")) {
    $rootPath = $rootPath.Substring(0, $rootPath.Length - 1)
  }
  $rootPath = $rootPath + [System.IO.Path]::DirectorySeparatorChar
  if ($fullPath.StartsWith($rootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    return $fullPath.Substring($rootPath.Length).Replace([char]92, [char]47)
  }
  return $fullPath.Replace([char]92, [char]47)
}

function Get-OptionalSha256 {
  param([string]$Path)
  if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    return $null
  }
  return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Test-WindowsBundleResourceTarget {
  param([string]$Target)
  if ($null -eq $windowsBundleResources) {
    return $false
  }
  $normalizedTarget = $Target.Replace([char]92, [char]47)
  foreach ($property in $windowsBundleResources.PSObject.Properties) {
    if ([string]$property.Value -eq $normalizedTarget) {
      return $true
    }
  }
  return $false
}

function New-RequiredResourceDetail {
  param(
    [string]$Id,
    [string]$Path,
    [string]$BundleTarget,
    [bool]$ExpectedInWindowsBundle
  )

  $exists = Test-Path -LiteralPath $Path -PathType Leaf
  [ordered]@{
    id = $Id
    path = $Path
    repo_relative_path = ConvertTo-RelativeSlashPath -Path $Path
    exists = $exists
    sha256 = Get-OptionalSha256 -Path $Path
    expected_in_windows_bundle = $ExpectedInWindowsBundle
    bundle_target = if ([string]::IsNullOrWhiteSpace($BundleTarget)) { $null } else { $BundleTarget }
    declared_in_windows_bundle_resources = if ([string]::IsNullOrWhiteSpace($BundleTarget)) { $false } else { Test-WindowsBundleResourceTarget -Target $BundleTarget }
  }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$resolvedBundleRoot = Resolve-ExistingPath -Candidates @(
  $BundleRoot,
  (Join-Path $repoRoot "src-tauri\\target\\release\\bundle"),
  (Join-Path $repoRoot "target\\release\\bundle")
)

$resolvedReleaseBinary = Resolve-ExistingPath -Candidates @(
  $ReleaseBinaryPath,
  (Join-Path $repoRoot "src-tauri\\target\\release\\ta_desktop_demo_app.exe"),
  (Join-Path $repoRoot "target\\release\\ta_desktop_demo_app.exe")
)
if ($null -eq $resolvedReleaseBinary) {
  throw "Could not resolve a release executable. Expected src-tauri\\target\\release\\ta_desktop_demo_app.exe or target\\release\\ta_desktop_demo_app.exe."
}

$tauriConfig = Get-Content -Raw (Join-Path $repoRoot "src-tauri\\tauri.conf.json") | ConvertFrom-Json
$windowsTauriConfig = Get-Content -Raw (Join-Path $repoRoot "src-tauri\\tauri.windows.conf.json") | ConvertFrom-Json
$windowsBundleResources = $windowsTauriConfig.bundle.resources
$productName = [string]$tauriConfig.productName
$resolvedVersion = if ([string]::IsNullOrWhiteSpace($Version)) { [string]$tauriConfig.version } else { $Version }
$identifier = [string]$tauriConfig.identifier

$bundleArtifacts = @()
if ($null -ne $resolvedBundleRoot) {
  $bundleArtifacts = @(Get-ArtifactCandidates -Root $resolvedBundleRoot)
}

$artifactsDir = Join-Path $resolvedOutDir "artifacts"
New-Item -ItemType Directory -Force -Path $artifactsDir | Out-Null

$stagedArtifacts = @()
$hashLines = @()
if ($bundleArtifacts.Count -gt 0) {
  foreach ($artifact in $bundleArtifacts) {
    $sourceNamePart = ConvertTo-SafeNamePart -Value $artifact.BaseName
    $betaPart = if ($UnsignedBeta) { "-unsigned-beta" } else { "" }
    $normalizedName = "vectorgrid-$resolvedVersion-windows-x64-$Variant$betaPart-$sourceNamePart$($artifact.Extension.ToLowerInvariant())"
    $stagedPath = Join-Path $artifactsDir $normalizedName
    Copy-Item -LiteralPath $artifact.FullName -Destination $stagedPath -Force
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $stagedPath
    $signature = Get-CodeSignatureInfo -Path $stagedPath
    $relativePath = "artifacts/$normalizedName"
    $hashLines += "$($hash.Hash.ToLowerInvariant()) *$relativePath"
    $stagedArtifacts += [ordered]@{
      source_kind = "bundle"
      source_path = $artifact.FullName
      staged_path = $stagedPath
      staged_name = $normalizedName
      extension = $artifact.Extension.ToLowerInvariant()
      size_bytes = [int64](Get-Item -LiteralPath $stagedPath).Length
      sha256 = $hash.Hash.ToLowerInvariant()
      code_signature = $signature
    }
  }
} else {
  $betaPart = if ($UnsignedBeta) { "-unsigned-beta" } else { "" }
  $normalizedName = "vectorgrid-$resolvedVersion-windows-x64-$Variant$betaPart.exe"
  $stagedPath = Join-Path $artifactsDir $normalizedName
  Copy-Item -LiteralPath $resolvedReleaseBinary -Destination $stagedPath -Force
  $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $stagedPath
  $signature = Get-CodeSignatureInfo -Path $stagedPath
  $relativePath = "artifacts/$normalizedName"
  $hashLines += "$($hash.Hash.ToLowerInvariant()) *$relativePath"
  $stagedArtifacts += [ordered]@{
    source_kind = "release_binary"
    source_path = $resolvedReleaseBinary
    staged_path = $stagedPath
    staged_name = $normalizedName
    extension = ".exe"
    size_bytes = [int64](Get-Item -LiteralPath $stagedPath).Length
    sha256 = $hash.Hash.ToLowerInvariant()
    code_signature = $signature
  }
}

$hashPath = Join-Path $resolvedOutDir "SHA256SUMS.txt"
Set-Content -Path $hashPath -Encoding UTF8 -Value $hashLines

$catboostPath = Join-Path $repoRoot "src-tauri\\resources\\native\\catboostmodel.dll"
$sampleCsvPath = Join-Path $repoRoot "src-tauri\\resources\\sample-data\\vectorgrid_synthetic_ohlcv.csv"
$kernelPtxPath = Join-Path $repoRoot "crates\\app_core\\src\\kernels\\double_crossover.prebuilt.ptx"
$optionsPtxPath = Join-Path $repoRoot "crates\\app_core\\src\\kernels\\options_expression.prebuilt.ptx"
$catboostExists = Test-Path -LiteralPath $catboostPath -PathType Leaf
if (-not $AllowMissingCatboostNative -and -not $catboostExists) {
  throw "Required packaged CatBoost native library is missing: $catboostPath. Use -AllowMissingCatboostNative only for non-shipping diagnostic manifests."
}

$requiredResourceDetails = @(
  (New-RequiredResourceDetail -Id "catboost_native_library" -Path $catboostPath -BundleTarget "native/catboostmodel.dll" -ExpectedInWindowsBundle $true),
  (New-RequiredResourceDetail -Id "synthetic_sample_csv" -Path $sampleCsvPath -BundleTarget "sample-data/vectorgrid_synthetic_ohlcv.csv" -ExpectedInWindowsBundle $true),
  (New-RequiredResourceDetail -Id "double_crossover_prebuilt_ptx" -Path $kernelPtxPath -BundleTarget "" -ExpectedInWindowsBundle $false),
  (New-RequiredResourceDetail -Id "options_expression_prebuilt_ptx" -Path $optionsPtxPath -BundleTarget "" -ExpectedInWindowsBundle $false)
)

$signedArtifacts = @($stagedArtifacts | Where-Object {
  $null -ne $_.code_signature -and [bool]$_.code_signature.checked -and [string]$_.code_signature.status -eq "Valid"
})
$signatureCheckedArtifacts = @($stagedArtifacts | Where-Object {
  $null -ne $_.code_signature -and [bool]$_.code_signature.checked
})
$unsignedOrInvalidArtifacts = @($signatureCheckedArtifacts | Where-Object {
  [string]$_.code_signature.status -ne "Valid"
})

$manifest = [ordered]@{
  schema_version = 1
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  variant = $Variant
  publishable = -not [bool]$UnsignedBeta
  unsigned_beta = [bool]$UnsignedBeta
  artifact_source_kind = if ($bundleArtifacts.Count -gt 0) { "bundle" } else { "release_binary" }
  product_name = $productName
  version = $resolvedVersion
  identifier = $identifier
  bundle_root = $resolvedBundleRoot
  release_binary_path = $resolvedReleaseBinary
  provenance_path = if ([string]::IsNullOrWhiteSpace($ProvenancePath)) { $null } else { [System.IO.Path]::GetFullPath($ProvenancePath) }
  staged_artifacts_dir = $artifactsDir
  hash_file = $hashPath
  required_resources = [ordered]@{
    catboost_bundle_resource_path = $catboostPath
    catboost_bundle_resource_exists = $catboostExists
    catboost_bundle_resource_sha256 = Get-OptionalSha256 -Path $catboostPath
    synthetic_sample_csv_path = $sampleCsvPath
    synthetic_sample_csv_exists = Test-Path -LiteralPath $sampleCsvPath -PathType Leaf
    synthetic_sample_csv_sha256 = Get-OptionalSha256 -Path $sampleCsvPath
    double_crossover_prebuilt_ptx_path = $kernelPtxPath
    double_crossover_prebuilt_ptx_exists = Test-Path $kernelPtxPath
    double_crossover_prebuilt_ptx_sha256 = Get-OptionalSha256 -Path $kernelPtxPath
    options_expression_prebuilt_ptx_path = $optionsPtxPath
    options_expression_prebuilt_ptx_exists = Test-Path $optionsPtxPath
    options_expression_prebuilt_ptx_sha256 = Get-OptionalSha256 -Path $optionsPtxPath
    details = $requiredResourceDetails
  }
  signing = [ordered]@{
    checked_artifact_count = $signatureCheckedArtifacts.Count
    valid_signed_artifact_count = $signedArtifacts.Count
    unsigned_or_invalid_artifact_count = $unsignedOrInvalidArtifacts.Count
  }
  staged_artifacts = $stagedArtifacts
}

$manifestPath = Join-Path $resolvedOutDir "manifest.json"
Write-JsonUtf8 -Path $manifestPath -Value $manifest

Write-Host "Release manifest written to $manifestPath"
Write-Host "SHA256 checksums written to $hashPath"
