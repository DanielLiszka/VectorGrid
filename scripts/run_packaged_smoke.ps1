param(
  [string]$BinaryPath = "",
  [string]$OutDir = "",
  [string]$RunId = "",
  [ValidateSet("smoke", "smoke-expanded", "workflow", "clean-start", "route-matrix", "release-proof")]
  [string]$DesktopSuite = "smoke",
  [switch]$StaticInspectionOnly
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
  return Join-Path (Join-Path $repoRoot "output\\release_candidates") "${stamp}-packaged-smoke"
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

function Resolve-BinaryPath {
  param(
    [string]$Candidate,
    [string]$ManifestPath
  )

  if (-not [string]::IsNullOrWhiteSpace($Candidate)) {
    if ([System.IO.Path]::IsPathRooted($Candidate)) {
      return (Resolve-Path $Candidate).Path
    }
    return (Resolve-Path (Join-Path $repoRoot $Candidate)).Path
  }

  if (-not [string]::IsNullOrWhiteSpace($ManifestPath) -and (Test-Path $ManifestPath)) {
    $manifest = Get-Content -Raw $ManifestPath | ConvertFrom-Json
    $manifestBinary = [string]$manifest.release_binary_path
    if (-not [string]::IsNullOrWhiteSpace($manifestBinary) -and (Test-Path $manifestBinary)) {
      return (Resolve-Path $manifestBinary).Path
    }
  }

  return $null
}

function Assert-SafeRunIdSegment {
  param([string]$Value)
  if ([string]::IsNullOrWhiteSpace($Value)) {
    throw "Packaged smoke run id must not be empty."
  }
  if ($Value -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$') {
    throw "Packaged smoke run id must be a single filename-safe segment using letters, numbers, '.', '_', or '-'."
  }
}

function Assert-PathUnderRoot {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string]$Purpose
  )

  $resolvedPath = [System.IO.Path]::GetFullPath($Path)
  $resolvedRoot = [System.IO.Path]::GetFullPath($Root)
  if (-not $resolvedRoot.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
    $resolvedRoot = $resolvedRoot + [System.IO.Path]::DirectorySeparatorChar
  }
  if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "$Purpose resolved outside the expected root: $resolvedPath"
  }
}

function Test-PathEquals {
  param(
    [string]$Left,
    [string]$Right
  )

  if (
    [string]::IsNullOrWhiteSpace($Left) -or
    [string]::IsNullOrWhiteSpace($Right)
  ) {
    return $false
  }
  $resolvedLeft = [System.IO.Path]::GetFullPath($Left)
  $resolvedRight = [System.IO.Path]::GetFullPath($Right)
  return [string]::Equals(
    $resolvedLeft,
    $resolvedRight,
    [System.StringComparison]::OrdinalIgnoreCase
  )
}

function Get-ManifestArtifactSourceKind {
  param([string]$ManifestPath)
  if (-not [string]::IsNullOrWhiteSpace($ManifestPath) -and (Test-Path $ManifestPath)) {
    $manifest = Get-Content -Raw $ManifestPath | ConvertFrom-Json
    return [string]$manifest.artifact_source_kind
  }
  return ""
}

function Resolve-SevenZipPath {
  $commands = @(
    (Get-Command "7z" -ErrorAction SilentlyContinue | Select-Object -First 1),
    (Get-Command "7za" -ErrorAction SilentlyContinue | Select-Object -First 1)
  )
  foreach ($command in $commands) {
    if ($null -ne $command -and -not [string]::IsNullOrWhiteSpace([string]$command.Source)) {
      return [string]$command.Source
    }
  }

  $candidates = @(
    (Join-Path ${env:ProgramFiles} "7-Zip\\7z.exe"),
    (if (${env:ProgramFiles(x86)}) { Join-Path ${env:ProgramFiles(x86)} "7-Zip\\7z.exe" } else { $null })
  )
  foreach ($candidate in $candidates) {
    if (-not [string]::IsNullOrWhiteSpace([string]$candidate) -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
      return (Resolve-Path -LiteralPath $candidate).Path
    }
  }
  return $null
}

function Get-CodeSignatureInfo {
  param([Parameter(Mandatory = $true)][string]$Path)

  try {
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    return [ordered]@{
      checked = $true
      status = [string]$signature.Status
      signer_subject = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Subject } else { $null }
      thumbprint = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Thumbprint } else { $null }
      timestamp_subject = if ($null -ne $signature.TimeStamperCertificate) { [string]$signature.TimeStamperCertificate.Subject } else { $null }
      timestamp_thumbprint = if ($null -ne $signature.TimeStamperCertificate) { [string]$signature.TimeStamperCertificate.Thumbprint } else { $null }
    }
  } catch {
    return [ordered]@{
      checked = $true
      status = "SignatureCheckFailed"
      signer_subject = $null
      thumbprint = $null
      timestamp_subject = $null
      timestamp_thumbprint = $null
      error = $_.Exception.Message
    }
  }
}

function Resolve-ManifestBundleArtifact {
  param([Parameter(Mandatory = $true)]$Manifest)

  $artifacts = @($Manifest.staged_artifacts | Where-Object {
    [string]$_.source_kind -eq "bundle" -and @(".exe", ".msi") -contains [string]$_.extension
  })
  if ($artifacts.Count -ne 1) {
    throw "Bundle smoke requires exactly one staged installer artifact in the manifest; found $($artifacts.Count)."
  }
  $artifact = $artifacts[0]
  $path = [string]$artifact.staged_path
  if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Manifest staged installer does not exist: $path"
  }
  if ([System.IO.Path]::GetExtension($path).ToLowerInvariant() -ne ".exe") {
    throw "Extracted bundle smoke currently supports the configured NSIS .exe artifact only: $path"
  }
  $resolvedPath = (Resolve-Path -LiteralPath $path).Path
  Assert-PathUnderRoot -Path $resolvedPath -Root $resolvedOutDir -Purpose "Staged installer artifact"
  $actualSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedPath).Hash.ToLowerInvariant()
  $expectedSha256 = [string]$artifact.sha256
  if ([string]::IsNullOrWhiteSpace($expectedSha256) -or $actualSha256 -ne $expectedSha256.ToLowerInvariant()) {
    throw "Staged installer hash no longer matches manifest evidence: $resolvedPath"
  }
  return [ordered]@{
    path = $resolvedPath
    sha256 = $actualSha256
    manifest_sha256 = $expectedSha256.ToLowerInvariant()
  }
}

function Expand-NsisBundleProduct {
  param(
    [Parameter(Mandatory = $true)][string]$BundlePath,
    [Parameter(Mandatory = $true)][string]$Destination,
    [Parameter(Mandatory = $true)]$Manifest
  )

  $sevenZip = Resolve-SevenZipPath
  if ($null -eq $sevenZip) {
    throw "Bundle smoke requires 7z/7za to extract the NSIS artifact without installing it."
  }
  Assert-PathUnderRoot -Path $Destination -Root $resolvedOutDir -Purpose "Packaged smoke extraction directory"
  if (Test-Path -LiteralPath $Destination) {
    $existing = Get-Item -LiteralPath $Destination -Force
    if (($existing.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Packaged smoke extraction directory must not be a symlink or junction: $Destination"
    }
    Remove-Item -LiteralPath $Destination -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path $Destination | Out-Null
  & $sevenZip "x" "-y" "-o$Destination" $BundlePath | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "NSIS extraction failed with exit code $LASTEXITCODE"
  }

  $reparseItems = @(Get-ChildItem -LiteralPath $Destination -Recurse -Force | Where-Object {
    ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  })
  if ($reparseItems.Count -gt 0) {
    $paths = ($reparseItems | ForEach-Object { $_.FullName }) -join ", "
    throw "Extracted NSIS payload must not contain symlinks or junctions: $paths"
  }

  $allowedFiles = @(
    "ta_desktop_demo_app.exe",
    '$PLUGINSDIR/modern-wizard.bmp',
    '$PLUGINSDIR/nsDialogs.dll',
    '$PLUGINSDIR/nsis_tauri_utils.dll',
    '$PLUGINSDIR/NSISdl.dll',
    '$PLUGINSDIR/StartMenu.dll',
    '$PLUGINSDIR/System.dll',
    "licenses/CatBoost-LICENSE.txt",
    "native/catboostmodel.dll",
    "sample-data/vectorgrid_synthetic_ohlcv.csv"
  )
  $requiredFiles = @(
    "ta_desktop_demo_app.exe",
    "licenses/CatBoost-LICENSE.txt",
    "native/catboostmodel.dll",
    "sample-data/vectorgrid_synthetic_ohlcv.csv"
  )
  $destinationRoot = [System.IO.Path]::GetFullPath($Destination)
  if (-not $destinationRoot.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
    $destinationRoot += [System.IO.Path]::DirectorySeparatorChar
  }
  $payloadFiles = @(Get-ChildItem -LiteralPath $Destination -Recurse -File -Force | ForEach-Object {
    $fullPath = [System.IO.Path]::GetFullPath($_.FullName)
    if (-not $fullPath.StartsWith($destinationRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
      throw "Extracted payload file resolved outside the extraction root: $fullPath"
    }
    $fullPath.Substring($destinationRoot.Length).Replace([char]92, [char]47)
  } | Sort-Object)

  $unexpectedFiles = @($payloadFiles | Where-Object { $allowedFiles -notcontains $_ })
  if ($unexpectedFiles.Count -gt 0) {
    throw "Extracted NSIS payload contains unexpected files: $($unexpectedFiles -join ', ')"
  }
  $missingFiles = @($requiredFiles | Where-Object { $payloadFiles -notcontains $_ })
  if ($missingFiles.Count -gt 0) {
    throw "Extracted NSIS payload is missing required files: $($missingFiles -join ', ')"
  }

  $resourceExpectations = @(
    [ordered]@{
      relative_path = "native/catboostmodel.dll"
      manifest_sha256 = [string]$Manifest.required_resources.catboost_bundle_resource_sha256
      reviewed_sha256 = [string]$Manifest.required_resources.catboost_bundle_resource_expected_sha256
    },
    [ordered]@{
      relative_path = "licenses/CatBoost-LICENSE.txt"
      manifest_sha256 = [string]$Manifest.required_resources.catboost_license_sha256
      reviewed_sha256 = [string]$Manifest.required_resources.catboost_license_expected_sha256
    },
    [ordered]@{
      relative_path = "sample-data/vectorgrid_synthetic_ohlcv.csv"
      manifest_sha256 = [string]$Manifest.required_resources.synthetic_sample_csv_sha256
      reviewed_sha256 = ""
    }
  )
  $resourceEvidence = @()
  foreach ($expectation in $resourceExpectations) {
    if ([string]::IsNullOrWhiteSpace($expectation.manifest_sha256)) {
      throw "Manifest is missing the expected hash for extracted resource '$($expectation.relative_path)'."
    }
    $resourcePath = Join-Path $Destination $expectation.relative_path
    $actualSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resourcePath).Hash.ToLowerInvariant()
    $expectedSha256 = $expectation.manifest_sha256.ToLowerInvariant()
    if ($actualSha256 -ne $expectedSha256) {
      throw "Extracted resource hash does not match the manifest for '$($expectation.relative_path)'."
    }
    $reviewedSha256 = [string]$expectation.reviewed_sha256
    if (-not [string]::IsNullOrWhiteSpace($reviewedSha256) -and $actualSha256 -ne $reviewedSha256.ToLowerInvariant()) {
      throw "Extracted resource hash does not match the reviewed upstream baseline for '$($expectation.relative_path)'."
    }
    $resourceEvidence += [ordered]@{
      relative_path = $expectation.relative_path
      size_bytes = [int64](Get-Item -LiteralPath $resourcePath).Length
      sha256 = $actualSha256
      manifest_sha256 = $expectedSha256
      reviewed_sha256 = if ([string]::IsNullOrWhiteSpace($reviewedSha256)) { $null } else { $reviewedSha256.ToLowerInvariant() }
    }
  }

  $products = @(Get-ChildItem -LiteralPath $Destination -Recurse -File -Filter "ta_desktop_demo_app.exe")
  if ($products.Count -ne 1) {
    throw "Expected exactly one product executable in the extracted NSIS payload; found $($products.Count)."
  }
  $productPath = $products[0].FullName
  Assert-PathUnderRoot -Path $productPath -Root $Destination -Purpose "Extracted product executable"
  return [ordered]@{
    product_path = $productPath
    payload_files = $payloadFiles
    verified_resources = $resourceEvidence
  }
}

function Resolve-AppLogPath {
  $configPath = Join-Path $repoRoot "src-tauri\\tauri.conf.json"
  if (-not (Test-Path $configPath)) {
    return $null
  }
  $config = Get-Content -Raw $configPath | ConvertFrom-Json
  $identifier = [string]$config.identifier
  $productName = [string]$config.productName
  if ([string]::IsNullOrWhiteSpace($identifier) -or [string]::IsNullOrWhiteSpace($productName)) {
    return $null
  }
  $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
  $candidate = Join-Path (Join-Path $localAppData $identifier) "logs\\$productName.log"
  if (Test-Path $candidate) {
    return (Resolve-Path $candidate).Path
  }
  return $candidate
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$manifestPath = Join-Path $resolvedOutDir "manifest.json"
$manifest = if (Test-Path -LiteralPath $manifestPath -PathType Leaf) {
  Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
} else {
  $null
}
$manifestArtifactSourceKind = Get-ManifestArtifactSourceKind -ManifestPath $manifestPath
$manifestArtifactSourceKind = if ([string]::IsNullOrWhiteSpace($manifestArtifactSourceKind)) {
  "release_binary"
} else {
  $manifestArtifactSourceKind
}
$isReleaseProof = $DesktopSuite -eq "release-proof"
$requiresIsolatedProfile = @("clean-start", "route-matrix", "release-proof") -contains $DesktopSuite
$releaseVariant = if ($null -ne $manifest) {
  [string]$manifest.variant
} else {
  ""
}
if ($isReleaseProof -and $StaticInspectionOnly) {
  throw "The release-proof desktop suite requires runtime execution and cannot use -StaticInspectionOnly."
}
if ($isReleaseProof -and $manifestArtifactSourceKind -ne "bundle") {
  throw "The release-proof desktop suite requires an exact staged bundle manifest."
}
if ($isReleaseProof -and @("cpu", "gpu-ma", "gpu-kernel") -notcontains $releaseVariant) {
  throw "The release-proof desktop suite requires a staged manifest with an explicit cpu, gpu-ma, or gpu-kernel variant."
}
$bundleArtifactEvidence = $null
$bundleExtractionRoot = $null
$bundlePayloadEvidence = $null
$binaryOrigin = "raw-release-binary"
if ($manifestArtifactSourceKind -eq "bundle") {
  if ($null -eq $manifest) {
    throw "Bundle smoke requires a readable manifest."
  }
  if (-not [string]::IsNullOrWhiteSpace($BinaryPath)) {
    throw "-BinaryPath cannot override a bundle manifest; bundle smoke must run the executable extracted from the staged installer."
  }
  $bundleArtifactEvidence = Resolve-ManifestBundleArtifact -Manifest $manifest
  $bundleExtractionRoot = Join-Path $resolvedOutDir "packaged-smoke-extracted"
  $bundlePayloadEvidence = Expand-NsisBundleProduct `
    -BundlePath $bundleArtifactEvidence.path `
    -Destination $bundleExtractionRoot `
    -Manifest $manifest
  $resolvedBinaryPath = $bundlePayloadEvidence.product_path
  $binaryOrigin = "extracted-bundle"
} else {
  $resolvedBinaryPath = Resolve-BinaryPath -Candidate $BinaryPath -ManifestPath $manifestPath
}
if ($null -eq $resolvedBinaryPath) {
  throw "Could not resolve a release executable for packaged smoke. Provide -BinaryPath or run manifest generation first."
}
$resolvedBinaryPath = (Resolve-Path -LiteralPath $resolvedBinaryPath).Path
$resolvedBinaryItem = Get-Item -LiteralPath $resolvedBinaryPath
$resolvedBinarySha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedBinaryPath).Hash.ToLowerInvariant()
$resolvedBinarySignature = Get-CodeSignatureInfo -Path $resolvedBinaryPath
$appLogPath = if ($requiresIsolatedProfile) {
  # Isolated desktop suites must never inspect or copy the user's real app log.
  # Runtime diagnostics for those suites come only from the marker-owned WDIO
  # artifact tree and the application attestation captured before teardown.
  $null
} else {
  Resolve-AppLogPath
}
$smokeSummaryPath = Join-Path $resolvedOutDir "packaged-smoke-summary.json"

$resolvedRunId = if ([string]::IsNullOrWhiteSpace($RunId)) {
  "release-smoke-" + (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
} else {
  $RunId
}

Assert-SafeRunIdSegment -Value $resolvedRunId
$smokeArtifactRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "output\\wdio\\desktop-smoke"))
$sourceArtifactDir = [System.IO.Path]::GetFullPath((Join-Path $smokeArtifactRoot $resolvedRunId))
Assert-PathUnderRoot -Path $sourceArtifactDir -Root $smokeArtifactRoot -Purpose "Packaged smoke source artifact directory"
if ($isReleaseProof -and (Test-Path -LiteralPath $sourceArtifactDir)) {
  throw "The release-proof source artifact directory already exists. Use a fresh run id so stale journey evidence cannot be reused: $sourceArtifactDir"
}
$destArtifactDir = Join-Path $resolvedOutDir "desktop-smoke"
$isolatedProfileRoot = if ($requiresIsolatedProfile) {
  [System.IO.Path]::GetFullPath(
    (Join-Path $repoRoot "output\\wdio\\desktop-smoke-profiles\\$resolvedRunId")
  )
} else {
  $null
}

$prevSkipBuild = $env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD
$prevBinary = $env:VECTORGRID_DESKTOP_SMOKE_BINARY
$prevRunId = $env:VECTORGRID_DESKTOP_SMOKE_RUN_ID
$prevDisablePreview = $env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW
$prevExactPackage = $env:VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE
$prevIsolatedProfileRoot = $env:VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT
$prevReleaseVariant = $env:VECTORGRID_DESKTOP_SMOKE_VARIANT
$pushedLocation = $false
$status = "failed"
$errorMessage = $null
$appLogArtifactPath = $null
$profileIsolationEvidencePath = if ($requiresIsolatedProfile) {
  Join-Path $sourceArtifactDir "logs\\profile-isolation.json"
} else {
  $null
}
$profileIsolationEvidence = $null
$releaseProofEvidenceRoot = if ($isReleaseProof) {
  Join-Path $sourceArtifactDir "logs\\release-proof"
} else {
  $null
}
$releaseProofEvidenceManifestPath = if ($isReleaseProof) {
  Join-Path $releaseProofEvidenceRoot "release-proof-evidence-manifest.json"
} else {
  $null
}
$releaseProofEvidence = $null

try {
  if ($manifestArtifactSourceKind -eq "release_binary") {
    Write-Warning "Manifest resolved only a raw release executable, not a bundle artifact. Packaged smoke will verify whether that release binary can load its own frontend/resources without preview bootstrap."
  }
  if ($StaticInspectionOnly) {
    $status = "inspected"
    Write-Host "Static package inspection completed without launching the desktop app."
  } else {
    $env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD = "1"
    $env:VECTORGRID_DESKTOP_SMOKE_BINARY = $resolvedBinaryPath
    $env:VECTORGRID_DESKTOP_SMOKE_RUN_ID = $resolvedRunId
    $env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW = "1"
    if ($isReleaseProof) {
      $env:VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE = "1"
      $env:VECTORGRID_DESKTOP_SMOKE_VARIANT = $releaseVariant
    } else {
      Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE -ErrorAction SilentlyContinue
      Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_VARIANT -ErrorAction SilentlyContinue
    }
    if ($requiresIsolatedProfile) {
      $env:VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT = $isolatedProfileRoot
    } else {
      Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT -ErrorAction SilentlyContinue
    }

    Push-Location $repoRoot
    $pushedLocation = $true
    $desktopScript = switch ($DesktopSuite) {
      "smoke-expanded" { "test:desktop:smoke:expanded" }
      "workflow" { "test:desktop:workflow" }
      "clean-start" { "test:desktop:clean-start" }
      "route-matrix" { "test:desktop:route-matrix" }
      "release-proof" { "test:desktop:release-proof" }
      default { "test:desktop:smoke" }
    }
    & npm.cmd --prefix web run $desktopScript
    $desktopExitCode = $LASTEXITCODE
    if (
      $requiresIsolatedProfile -and
      -not [string]::IsNullOrWhiteSpace($profileIsolationEvidencePath) -and
      (Test-Path -LiteralPath $profileIsolationEvidencePath -PathType Leaf)
    ) {
      $profileEvidenceItem = Get-Item -LiteralPath $profileIsolationEvidencePath -Force
      if (($profileEvidenceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Profile-isolation evidence may not be read through a symlink or junction."
      }
      $profileIsolationEvidence = Get-Content -Raw -LiteralPath $profileIsolationEvidencePath | ConvertFrom-Json
    }
    if ($desktopExitCode -ne 0) {
      throw "Desktop suite '$DesktopSuite' failed with exit code $desktopExitCode"
    }
    if ($requiresIsolatedProfile) {
      if ($null -eq $profileIsolationEvidence) {
        throw "Desktop suite '$DesktopSuite' did not produce profile-isolation evidence."
      }
      $appAttestation = $profileIsolationEvidence.appAttestation
      $attestedPaths = if ($null -ne $appAttestation) {
        $appAttestation.resolvedPaths
      } else {
        $null
      }
      $expectedIsolatedAppData = [System.IO.Path]::GetFullPath(
        (Join-Path $isolatedProfileRoot "LocalAppData\\dev.vectoralpha.vectorgrid")
      )
      if (
        [int]$profileIsolationEvidence.schemaVersion -ne 2 -or
        [string]$profileIsolationEvidence.runId -ne $resolvedRunId -or
        [string]$profileIsolationEvidence.status -ne "passed" -or
        -not [bool]$profileIsolationEvidence.realProfileUnchanged -or
        -not [bool]$profileIsolationEvidence.effectiveIsolationProven -or
        [string]$profileIsolationEvidence.teardown -ne "completed" -or
        $null -eq $appAttestation -or
        [int]$appAttestation.schemaVersion -ne 2 -or
        [string]$appAttestation.kind -ne "vectorgrid-qa-profile-attestation" -or
        [string]$appAttestation.runId -ne $resolvedRunId -or
        [string]$appAttestation.markerCreatedAt -ne [string]$profileIsolationEvidence.markerCreatedAt -or
        [string]$appAttestation.originalIdentifier -ne "dev.vectoralpha.vectorgrid" -or
        -not (Test-PathEquals -Left ([string]$appAttestation.isolationRoot) -Right $isolatedProfileRoot) -or
        -not (Test-PathEquals -Left ([string]$appAttestation.effectiveIdentifier) -Right $expectedIsolatedAppData) -or
        -not (Test-PathEquals -Left ([string]$appAttestation.appDataDir) -Right $expectedIsolatedAppData) -or
        $null -eq $attestedPaths -or
        -not (Test-PathEquals -Left ([string]$attestedPaths.appConfigDir) -Right $expectedIsolatedAppData) -or
        -not (Test-PathEquals -Left ([string]$attestedPaths.appDataDir) -Right $expectedIsolatedAppData) -or
        -not (Test-PathEquals -Left ([string]$attestedPaths.appLocalDataDir) -Right $expectedIsolatedAppData) -or
        -not (Test-PathEquals -Left ([string]$attestedPaths.appCacheDir) -Right $expectedIsolatedAppData) -or
        -not (Test-PathEquals -Left ([string]$attestedPaths.appLogDir) -Right (Join-Path $expectedIsolatedAppData "logs")) -or
        -not (Test-PathEquals -Left ([string]$attestedPaths.webviewDataDir) -Right (Join-Path $expectedIsolatedAppData "webview2"))
      ) {
        throw "Desktop suite '$DesktopSuite' did not prove a schema-2 isolated-profile lifecycle with an exact matching application attestation, completed teardown, and the real profile unchanged."
      }
    }
    if ($isReleaseProof) {
      $contractScript = Join-Path $repoRoot "scripts\\release_proof_evidence_contract.mjs"
      $contractArgs = @(
        $contractScript,
        "--mode", "write",
        "--evidence-root", $releaseProofEvidenceRoot,
        "--run-id", $resolvedRunId,
        "--variant", $releaseVariant,
        "--binary-path", $resolvedBinaryPath,
        "--binary-size-bytes", [string]$resolvedBinaryItem.Length,
        "--binary-sha256", $resolvedBinarySha256,
        "--manifest-path", $releaseProofEvidenceManifestPath
      )
      $contractOutput = (& node @contractArgs 2>&1 | Out-String)
      if ($LASTEXITCODE -ne 0) {
        throw "Release-proof journey evidence validation failed: $($contractOutput.Trim())"
      }
      if (-not (Test-Path -LiteralPath $releaseProofEvidenceManifestPath -PathType Leaf)) {
        throw "Release-proof journey evidence validation did not produce its required manifest."
      }
      $releaseProofEvidence = Get-Content -Raw -LiteralPath $releaseProofEvidenceManifestPath | ConvertFrom-Json
      if (
        [int]$releaseProofEvidence.schema_version -ne 1 -or
        [string]$releaseProofEvidence.contract -ne "vectorgrid-release-proof-evidence-manifest-v1" -or
        [string]$releaseProofEvidence.status -ne "passed" -or
        [string]$releaseProofEvidence.run_id -ne $resolvedRunId -or
        [string]$releaseProofEvidence.release_variant -ne $releaseVariant -or
        [int]$releaseProofEvidence.required_journey_count -le 0 -or
        @($releaseProofEvidence.journeys).Count -ne [int]$releaseProofEvidence.required_journey_count
      ) {
        throw "Release-proof journey evidence manifest failed its wrapper-level schema and completeness check."
      }
    }

    $status = "passed"
    Write-Host "Packaged smoke completed against $resolvedBinaryPath"
  }
} catch {
  $errorMessage = $_.Exception.Message
  throw
} finally {
  if ($pushedLocation) {
    Pop-Location
  }
  if (Test-Path $sourceArtifactDir) {
    $sourceItem = Get-Item -LiteralPath $sourceArtifactDir -Force
    if (($sourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Packaged smoke source artifact directory must not be a symlink or junction: $sourceArtifactDir"
    }
    if (Test-Path $destArtifactDir) {
      Remove-Item -Recurse -Force $destArtifactDir
    }
    New-Item -ItemType Directory -Force -Path $destArtifactDir | Out-Null
    Get-ChildItem -LiteralPath $sourceArtifactDir -Force | Copy-Item -Destination $destArtifactDir -Recurse -Force
  }
  if (
    -not $requiresIsolatedProfile -and
    -not $StaticInspectionOnly -and
    -not [string]::IsNullOrWhiteSpace($appLogPath) -and
    (Test-Path $appLogPath)
  ) {
    $appLogDir = Join-Path $destArtifactDir "logs\\app"
    New-Item -ItemType Directory -Force -Path $appLogDir | Out-Null
    $appLogArtifactPath = Join-Path $appLogDir ([System.IO.Path]::GetFileName($appLogPath))
    Copy-Item -LiteralPath $appLogPath -Destination $appLogArtifactPath -Force
  }
  if ($null -ne $prevSkipBuild) {
    $env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD = $prevSkipBuild
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevBinary) {
    $env:VECTORGRID_DESKTOP_SMOKE_BINARY = $prevBinary
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_BINARY -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevRunId) {
    $env:VECTORGRID_DESKTOP_SMOKE_RUN_ID = $prevRunId
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_RUN_ID -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevDisablePreview) {
    $env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW = $prevDisablePreview
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevExactPackage) {
    $env:VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE = $prevExactPackage
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevIsolatedProfileRoot) {
    $env:VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT = $prevIsolatedProfileRoot
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevReleaseVariant) {
    $env:VECTORGRID_DESKTOP_SMOKE_VARIANT = $prevReleaseVariant
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_VARIANT -ErrorAction SilentlyContinue
  }

  $diagnosis = if ($StaticInspectionOnly) {
    "Static extraction and integrity inspection only; the desktop runtime was not launched and this is not packaged-smoke runtime evidence."
  } elseif ($manifestArtifactSourceKind -eq "release_binary") {
    if ($status -eq "passed") {
      "Smoke target was a raw release executable rather than a bundle artifact. This proves shell boot on the built binary, but it is not a bundle-level packaging proof."
    } else {
      "Smoke target was a raw release executable rather than a bundle artifact. Blank-shell failures here often reflect WebDriver URL semantics or missing packaged resource context rather than a broken shell."
    }
  } else {
    $null
  }
  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    status = $status
    error = $errorMessage
    run_id = $resolvedRunId
    desktop_suite = $DesktopSuite
    exact_package_release_proof = $isReleaseProof
    release_variant = if ([string]::IsNullOrWhiteSpace($releaseVariant)) { $null } else { $releaseVariant }
    isolated_profile_root = $isolatedProfileRoot
    profile_isolation = if ($null -ne $profileIsolationEvidence) {
      [ordered]@{
        schema_version = [int]$profileIsolationEvidence.schemaVersion
        status = [string]$profileIsolationEvidence.status
        run_id = [string]$profileIsolationEvidence.runId
        real_profile_unchanged = if ($null -ne $profileIsolationEvidence.PSObject.Properties["realProfileUnchanged"]) {
          [bool]$profileIsolationEvidence.realProfileUnchanged
        } else {
          $false
        }
        effective_isolation_proven = if ($null -ne $profileIsolationEvidence.PSObject.Properties["effectiveIsolationProven"]) {
          [bool]$profileIsolationEvidence.effectiveIsolationProven
        } else {
          $false
        }
        teardown = if ($null -ne $profileIsolationEvidence.PSObject.Properties["teardown"]) {
          [string]$profileIsolationEvidence.teardown
        } else {
          $null
        }
        app_attestation = if ($null -ne $profileIsolationEvidence.appAttestation) {
          [ordered]@{
            schema_version = [int]$profileIsolationEvidence.appAttestation.schemaVersion
            kind = [string]$profileIsolationEvidence.appAttestation.kind
            run_id = [string]$profileIsolationEvidence.appAttestation.runId
            original_identifier = [string]$profileIsolationEvidence.appAttestation.originalIdentifier
            effective_identifier = [string]$profileIsolationEvidence.appAttestation.effectiveIdentifier
            isolation_root = [string]$profileIsolationEvidence.appAttestation.isolationRoot
            app_data_dir = [string]$profileIsolationEvidence.appAttestation.appDataDir
          }
        } else {
          $null
        }
        evidence_path = $profileIsolationEvidencePath
      }
    } else {
      $null
    }
    manifest_path = if (Test-Path $manifestPath) { $manifestPath } else { $null }
    artifact_source_kind = $manifestArtifactSourceKind
    smoke_mode = if ($StaticInspectionOnly) {
      if ($manifestArtifactSourceKind -eq "bundle") { "bundle-static-inspection" } else { "raw-release-binary-static-inspection" }
    } elseif ($manifestArtifactSourceKind -eq "bundle") {
      "bundle"
    } else {
      "raw-release-binary"
    }
    runtime_executed = -not [bool]$StaticInspectionOnly
    binary_origin = $binaryOrigin
    binary_path = $resolvedBinaryPath
    binary_size_bytes = $resolvedBinaryItem.Length
    binary_sha256 = $resolvedBinarySha256
    binary_code_signature = $resolvedBinarySignature
    bundle_artifact_path = if ($null -ne $bundleArtifactEvidence) { $bundleArtifactEvidence.path } else { $null }
    bundle_artifact_sha256 = if ($null -ne $bundleArtifactEvidence) { $bundleArtifactEvidence.sha256 } else { $null }
    bundle_manifest_sha256 = if ($null -ne $bundleArtifactEvidence) { $bundleArtifactEvidence.manifest_sha256 } else { $null }
    bundle_extraction_root = $bundleExtractionRoot
    bundle_payload_files = if ($null -ne $bundlePayloadEvidence) { @($bundlePayloadEvidence.payload_files) } else { @() }
    bundle_verified_resources = if ($null -ne $bundlePayloadEvidence) { @($bundlePayloadEvidence.verified_resources) } else { @() }
    release_proof_evidence = if ($null -ne $releaseProofEvidence) {
      $copiedEvidenceRoot = Join-Path $destArtifactDir "logs\\release-proof"
      $copiedEvidenceManifestPath = Join-Path $copiedEvidenceRoot "release-proof-evidence-manifest.json"
      [ordered]@{
        schema_version = [int]$releaseProofEvidence.schema_version
        contract = [string]$releaseProofEvidence.contract
        status = [string]$releaseProofEvidence.status
        run_id = [string]$releaseProofEvidence.run_id
        release_variant = [string]$releaseProofEvidence.release_variant
        required_journey_count = [int]$releaseProofEvidence.required_journey_count
        journey_ids = @($releaseProofEvidence.journeys | ForEach-Object { [string]$_.id })
        evidence_root = $copiedEvidenceRoot
        manifest_path = $copiedEvidenceManifestPath
        manifest_size_bytes = if (Test-Path -LiteralPath $copiedEvidenceManifestPath -PathType Leaf) {
          [int64](Get-Item -LiteralPath $copiedEvidenceManifestPath).Length
        } else {
          $null
        }
        manifest_sha256 = if (Test-Path -LiteralPath $copiedEvidenceManifestPath -PathType Leaf) {
          (Get-FileHash -Algorithm SHA256 -LiteralPath $copiedEvidenceManifestPath).Hash.ToLowerInvariant()
        } else {
          $null
        }
      }
    } else {
      $null
    }
    preview_disabled = $true
    source_artifact_dir = $sourceArtifactDir
    copied_artifact_dir = if (Test-Path $destArtifactDir) { $destArtifactDir } else { $null }
    app_log_path = $appLogPath
    copied_app_log_path = $appLogArtifactPath
    diagnosis = $diagnosis
  }
  Write-JsonUtf8 -Path $smokeSummaryPath -Value $summary
}

if (Test-Path $destArtifactDir) {
  Write-Host "Smoke artifacts copied to $destArtifactDir"
}
Write-Host "Packaged smoke summary written to $smokeSummaryPath"
