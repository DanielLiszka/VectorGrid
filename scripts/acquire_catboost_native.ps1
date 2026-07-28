param(
  [string]$ManifestPath = '',
  [string]$DestinationPath = '',
  [string]$EvidenceDirectory = '',
  [switch]$ReplaceUnreviewed,
  [switch]$VerifyOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if ([string]::IsNullOrWhiteSpace($ManifestPath)) {
  $ManifestPath = Join-Path $repoRoot 'scripts\baselines\catboost-native-v1.2.10.json'
}
if ([string]::IsNullOrWhiteSpace($DestinationPath)) {
  $DestinationPath = Join-Path $repoRoot 'src-tauri\resources\native\catboostmodel.dll'
}
if ([string]::IsNullOrWhiteSpace($EvidenceDirectory)) {
  $EvidenceDirectory = Join-Path $repoRoot 'output\qa\catboost-provenance'
}

function Resolve-ContainedPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Root,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  $resolvedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd('\')
  $resolvedPath = [System.IO.Path]::GetFullPath($Path)
  if (
    $resolvedPath -ne $resolvedRoot -and
    -not $resolvedPath.StartsWith("$resolvedRoot\", [System.StringComparison]::OrdinalIgnoreCase)
  ) {
    throw "$Label '$resolvedPath' resolves outside '$resolvedRoot'."
  }
  return $resolvedPath
}

function Get-FileIdentity {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $item = Get-Item -LiteralPath $Path
  [pscustomobject]@{
    path = $item.FullName
    size_bytes = [int64]$item.Length
    sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToUpperInvariant()
    authenticode_status = (Get-AuthenticodeSignature -LiteralPath $item.FullName).Status.ToString()
  }
}

function Test-ExpectedIdentity {
  param(
    [Parameter(Mandatory = $true)]
    [object]$Identity,
    [Parameter(Mandatory = $true)]
    [object]$Manifest
  )

  return (
    [int64]$Identity.size_bytes -eq [int64]$Manifest.asset_size_bytes -and
    [string]$Identity.sha256 -eq ([string]$Manifest.asset_sha256).ToUpperInvariant()
  )
}

$ManifestPath = Resolve-ContainedPath -Path $ManifestPath -Root $repoRoot -Label 'ManifestPath'
$DestinationPath = Resolve-ContainedPath `
  -Path $DestinationPath `
  -Root (Join-Path $repoRoot 'src-tauri\resources\native') `
  -Label 'DestinationPath'
$EvidenceDirectory = Resolve-ContainedPath `
  -Path $EvidenceDirectory `
  -Root (Join-Path $repoRoot 'output') `
  -Label 'EvidenceDirectory'

if (!(Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
  throw "CatBoost native manifest is missing: $ManifestPath"
}
$manifest = Get-Content -Raw -LiteralPath $ManifestPath | ConvertFrom-Json
foreach ($required in @(
  'version',
  'tag',
  'tag_commit',
  'asset_name',
  'asset_url',
  'asset_size_bytes',
  'asset_sha256',
  'license',
  'license_url',
  'license_sha256',
  'bundle_target'
)) {
  if (
    -not ($manifest.PSObject.Properties.Name -contains $required) -or
    [string]::IsNullOrWhiteSpace([string]$manifest.$required)
  ) {
    throw "CatBoost native manifest is missing required field '$required'."
  }
}
if ([string]$manifest.license -ne 'Apache-2.0') {
  throw "CatBoost native manifest license must be Apache-2.0."
}
if ([string]$manifest.bundle_target -ne 'native/catboostmodel.dll') {
  throw "CatBoost native manifest bundle_target must be native/catboostmodel.dll."
}

$existingIdentity = $null
if (Test-Path -LiteralPath $DestinationPath -PathType Leaf) {
  $existingIdentity = Get-FileIdentity -Path $DestinationPath
  if (Test-ExpectedIdentity -Identity $existingIdentity -Manifest $manifest) {
    [pscustomobject]@{
      status = 'verified'
      version = [string]$manifest.version
      destination = $DestinationPath
      size_bytes = $existingIdentity.size_bytes
      sha256 = $existingIdentity.sha256
      authenticode_status = $existingIdentity.authenticode_status
    } | ConvertTo-Json -Depth 4
    exit 0
  }
}

if ($VerifyOnly) {
  $actual = if ($null -eq $existingIdentity) {
    'missing'
  } else {
    "size=$($existingIdentity.size_bytes), sha256=$($existingIdentity.sha256)"
  }
  throw "CatBoost native artifact verification failed at '$DestinationPath': $actual. Expected size=$($manifest.asset_size_bytes), sha256=$($manifest.asset_sha256). Run this script without -VerifyOnly to acquire the reviewed upstream artifact."
}
if ($null -ne $existingIdentity -and -not $ReplaceUnreviewed) {
  throw "Refusing to replace unreviewed CatBoost DLL at '$DestinationPath' (size=$($existingIdentity.size_bytes), sha256=$($existingIdentity.sha256)). Re-run with -ReplaceUnreviewed to quarantine it and install the reviewed upstream artifact."
}

$destinationDirectory = Split-Path -Parent $DestinationPath
New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null

$nonce = [Guid]::NewGuid().ToString('N')
$stagedPath = Join-Path $destinationDirectory ".catboostmodel.$nonce.download"
try {
  Invoke-WebRequest `
    -Uri ([string]$manifest.asset_url) `
    -OutFile $stagedPath `
    -Headers @{ 'User-Agent' = 'VectorGrid-CatBoost-Acquisition/1' }

  $stagedIdentity = Get-FileIdentity -Path $stagedPath
  if (-not (Test-ExpectedIdentity -Identity $stagedIdentity -Manifest $manifest)) {
    throw "Downloaded CatBoost artifact failed integrity verification: size=$($stagedIdentity.size_bytes), sha256=$($stagedIdentity.sha256); expected size=$($manifest.asset_size_bytes), sha256=$($manifest.asset_sha256)."
  }

  $quarantinedPath = $null
  if ($null -ne $existingIdentity) {
    $quarantineDirectory = Join-Path $EvidenceDirectory 'quarantine'
    New-Item -ItemType Directory -Force -Path $quarantineDirectory | Out-Null
    $quarantinedPath = Join-Path $quarantineDirectory (
      "catboostmodel-unreviewed-$($existingIdentity.sha256)-$nonce.dll"
    )
    Move-Item -LiteralPath $DestinationPath -Destination $quarantinedPath
  }

  Move-Item -LiteralPath $stagedPath -Destination $DestinationPath
  $installedIdentity = Get-FileIdentity -Path $DestinationPath
  if (-not (Test-ExpectedIdentity -Identity $installedIdentity -Manifest $manifest)) {
    throw 'Installed CatBoost artifact identity changed after publication.'
  }

  $summary = [ordered]@{
    schema_version = 1
    status = 'acquired_and_verified'
    version = [string]$manifest.version
    tag = [string]$manifest.tag
    tag_commit = [string]$manifest.tag_commit
    source_url = [string]$manifest.asset_url
    license = [string]$manifest.license
    destination = $DestinationPath
    size_bytes = $installedIdentity.size_bytes
    sha256 = $installedIdentity.sha256
    authenticode_status = $installedIdentity.authenticode_status
    quarantined_previous_path = $quarantinedPath
    acquired_at_utc = [DateTimeOffset]::UtcNow.ToString('o')
  }
  $summaryPath = Join-Path $EvidenceDirectory 'acquisition-summary.json'
  $summary | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $summaryPath -Encoding utf8
  $summary | ConvertTo-Json -Depth 5
} finally {
  if (Test-Path -LiteralPath $stagedPath -PathType Leaf) {
    Remove-Item -LiteralPath $stagedPath -Force
  }
}
