param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [switch]$IncludePerf,
  [switch]$IncludeGpuValidationPerf,
  [switch]$IncludeOptionsGpuPromotion,
  [string]$OptionsGpuArtifactPath = "",
  [string]$OptionsGpuDatasetId = "",
  [string]$OptionsGpuImportCsv = "",
  [string]$OptionsGpuImportRootSymbol = "",
  [string]$OptionsGpuAppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OptionsGpuBaselinePath = "scripts/baselines/options_expression_perf_baseline.json",
  [switch]$IncludeExpandedBrowserQa,
  [switch]$IncludeBaselineUi,
  [switch]$SkipWorkspaceTests,
  [switch]$AllowDirtyWorktree,
  [switch]$AllowUnsignedBeta,
  [string]$PostBuildSignScript = "",
  [string]$ExpectedSignerThumbprint = ""
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 18
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
  return Join-Path (Join-Path $repoRoot "output\\public_release_candidates") "${stamp}-${Variant}"
}

function Assert-FreshOutputDirectory {
  param([Parameter(Mandatory = $true)][string]$Path)

  if (-not (Test-Path -LiteralPath $Path)) {
    return
  }
  $item = Get-Item -LiteralPath $Path -Force
  if (
    -not $item.PSIsContainer -or
    ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  ) {
    throw "Public release output must be a new or empty ordinary directory: $Path"
  }
  if ($null -ne (Get-ChildItem -LiteralPath $Path -Force | Select-Object -First 1)) {
    throw "Public release output directory is not empty. Use a fresh path so stale evidence or installers cannot survive into this release: $Path"
  }
}

function Read-JsonFile {
  param([string]$Path)
  if (-not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path $Path)) {
    return Get-Content -Raw $Path | ConvertFrom-Json
  }
  return $null
}

function Normalize-Thumbprint {
  param([string]$Value)
  if ([string]::IsNullOrWhiteSpace($Value)) {
    return ""
  }
  return ($Value -replace '\s', '').ToUpperInvariant()
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
    $resolvedRoot += [System.IO.Path]::DirectorySeparatorChar
  }
  if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "$Purpose resolved outside the release output directory: $resolvedPath"
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
  return [string]::Equals(
    [System.IO.Path]::GetFullPath($Left),
    [System.IO.Path]::GetFullPath($Right),
    [System.StringComparison]::OrdinalIgnoreCase
  )
}

function Get-CodeSignatureInfo {
  param([Parameter(Mandatory = $true)][string]$Path)

  $signature = Get-AuthenticodeSignature -LiteralPath $Path
  return [ordered]@{
    checked = $true
    status = [string]$signature.Status
    signer_subject = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Subject } else { $null }
    thumbprint = if ($null -ne $signature.SignerCertificate) { [string]$signature.SignerCertificate.Thumbprint } else { $null }
    timestamp_subject = if ($null -ne $signature.TimeStamperCertificate) { [string]$signature.TimeStamperCertificate.Subject } else { $null }
    timestamp_thumbprint = if ($null -ne $signature.TimeStamperCertificate) { [string]$signature.TimeStamperCertificate.Thumbprint } else { $null }
  }
}

function Invoke-PublicReleaseStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [string]$CommandText,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Action
  )

  $startedAt = (Get-Date).ToUniversalTime()
  Write-Host "==> $Name"
  try {
    & $Action
    $script:StepResults += [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $true
    }
    Write-Host "[PASS] $Name"
  } catch {
    $script:StepResults += [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $false
      error = $_.Exception.Message
    }
    throw
  }
}

function Assert-CleanWorktree {
  if ($AllowDirtyWorktree) {
    Write-Warning "Dirty worktree allowed for this run. Do not publish this output as a final public release."
    return
  }
  $statusText = (& git -C $repoRoot status --porcelain 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) {
    throw "Could not verify the release worktree state: $($statusText.Trim())"
  }
  $status = $statusText.Trim()
  if (-not [string]::IsNullOrWhiteSpace($status)) {
    throw "Public release requires a clean git worktree. Commit or stash changes, or pass -AllowDirtyWorktree only for a non-publishable rehearsal."
  }
}

function Assert-PublicReleaseEvidence {
  $candidateSummary = Read-JsonFile -Path (Join-Path $resolvedOutDir "summary.json")
  $manifest = Read-JsonFile -Path (Join-Path $resolvedOutDir "manifest.json")
  $smoke = Read-JsonFile -Path (Join-Path $resolvedOutDir "packaged-smoke-summary.json")

  if ($null -eq $candidateSummary -or [string]$candidateSummary.status -ne "passed") {
    throw "Release candidate gate did not pass."
  }
  $workflowAuthorityProperty = $candidateSummary.PSObject.Properties["workflow_authority_complete"]
  if ($null -eq $workflowAuthorityProperty -or -not [bool]$workflowAuthorityProperty.Value) {
    throw "Public release requires complete workflow authority; a candidate created with -SkipWorkflowGate is rehearsal-only."
  }
  $liveSafetyContractScript = Join-Path $repoRoot "scripts\\public_live_safety_evidence_contract.mjs"
  $liveSafetyContractArgs = @(
    $liveSafetyContractScript,
    "--out-dir", $resolvedOutDir,
    "--candidate-summary-path", (Join-Path $resolvedOutDir "summary.json"),
    "--variant", $Variant
  )
  $liveSafetyContractOutput = (& node @liveSafetyContractArgs 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) {
    throw "Public live-execution safety evidence revalidation failed: $($liveSafetyContractOutput.Trim())"
  }
  $script:LiveSafetyEvidenceVerification = $liveSafetyContractOutput | ConvertFrom-Json
  if (
    [int]$script:LiveSafetyEvidenceVerification.schema_version -ne 1 -or
    [string]$script:LiveSafetyEvidenceVerification.contract -ne "vectorgrid-public-live-safety-evidence-v1" -or
    [string]$script:LiveSafetyEvidenceVerification.status -ne "passed" -or
    [string]$script:LiveSafetyEvidenceVerification.variant -ne $Variant -or
    [string]$script:LiveSafetyEvidenceVerification.authority.execution_safety_gate -ne "passed" -or
    [string]$script:LiveSafetyEvidenceVerification.authority.live_safety_harness -ne "passed" -or
    -not [bool]$script:LiveSafetyEvidenceVerification.authority.authority_eligible -or
    [int]$script:LiveSafetyEvidenceVerification.authority.required_test_count -ne 128
  ) {
    throw "Public live-execution safety evidence revalidation returned incomplete or mismatched authority."
  }
  if ($IncludeOptionsGpuPromotion) {
    $optionsPromotionProperty = $candidateSummary.PSObject.Properties["include_options_gpu_promotion"]
    if ($null -eq $optionsPromotionProperty -or -not [bool]$optionsPromotionProperty.Value) {
      throw "Public options GPU evidence requires a candidate created with -IncludeOptionsGpuPromotion."
    }
    $optionsPromotionStep = @($candidateSummary.steps | Where-Object {
      [string]$_.name -eq "Options GPU promotion authority"
    })
    if ($optionsPromotionStep.Count -ne 1 -or -not [bool]$optionsPromotionStep[0].passed) {
      throw "Public options GPU evidence requires one passing Options GPU promotion authority candidate step."
    }
    $optionsPromotionSummaryPath = Join-Path $resolvedOutDir "options-gpu-promotion-summary.json"
    $optionsPromotionSummary = Read-JsonFile -Path $optionsPromotionSummaryPath
    if ($null -eq $optionsPromotionSummary -or [string]$optionsPromotionSummary.mode -ne "candidate") {
      throw "Public options GPU evidence requires a candidate-mode options promotion summary."
    }
    $correctnessEvidence = @($optionsPromotionSummary.correctness_evidence)
    if (
      $correctnessEvidence.Count -lt 3 -or
      @($correctnessEvidence | Where-Object {
        -not [bool]$_.passed -or [int]$_.passed_test_count -lt 1
      }).Count -gt 0
    ) {
      throw "Public options GPU evidence requires passing CPU options tests plus GPU aggregate and ranking parity tests."
    }
    $requiredCorrectnessNames = @(
      "Options expression CPU test suite",
      "Options expression GPU aggregate parity",
      "Options expression CPU/GPU ranking parity"
    )
    foreach ($requiredCorrectnessName in $requiredCorrectnessNames) {
      if (@($correctnessEvidence | Where-Object { [string]$_.name -eq $requiredCorrectnessName }).Count -ne 1) {
        throw "Public options GPU evidence is missing required correctness evidence '$requiredCorrectnessName'."
      }
    }
    $promotionChecks = @($optionsPromotionSummary.checks)
    $baselineChecks = @($optionsPromotionSummary.baseline_comparison)
    if (
      $promotionChecks.Count -eq 0 -or
      @($promotionChecks | Where-Object { -not [bool]$_.Passed }).Count -gt 0 -or
      $baselineChecks.Count -eq 0 -or
      @($baselineChecks | Where-Object { -not [bool]$_.Passed }).Count -gt 0
    ) {
      throw "Public options GPU evidence requires passing promotion and baseline-regression checks."
    }
  }
  if ($null -eq $manifest) {
    throw "Release manifest is missing."
  }
  if (
    [string]$candidateSummary.variant -ne $Variant -or
    [string]$manifest.variant -ne $Variant
  ) {
    throw "Public release evidence belongs to a different release variant."
  }
  if (-not $AllowUnsignedBeta -and ([bool]$manifest.unsigned_beta -or -not [bool]$manifest.installer_signature_eligible)) {
    throw "Public release requires installer-signature-eligible staging evidence. Re-run without unsigned beta labeling after bundle-time signing."
  }
  if ($AllowUnsignedBeta -and -not [bool]$manifest.unsigned_beta) {
    throw "Unsigned beta rehearsal requires manifest.unsigned_beta=true and beta-labeled artifacts."
  }
  if ([string]$manifest.artifact_source_kind -ne "bundle") {
    throw "Public release requires staged bundle artifacts, not only a raw release executable."
  }
  if ($null -eq $smoke -or [string]$smoke.status -ne "passed" -or [string]$smoke.smoke_mode -ne "bundle") {
    throw "Public release requires a passing bundle-mode packaged smoke."
  }
  if (
    [string]$candidateSummary.desktop_suite -ne "release-proof" -or
    [string]$smoke.desktop_suite -ne "release-proof" -or
    -not [bool]$smoke.exact_package_release_proof -or
    [string]$smoke.release_variant -ne $Variant
  ) {
    throw "Public release requires the exact-package release-proof desktop suite for the current release variant."
  }
  $profileIsolation = $smoke.profile_isolation
  $appAttestation = if ($null -ne $profileIsolation) {
    $profileIsolation.app_attestation
  } else {
    $null
  }
  $expectedIsolatedAppData = if (
    -not [string]::IsNullOrWhiteSpace([string]$smoke.isolated_profile_root)
  ) {
    [System.IO.Path]::GetFullPath(
      (Join-Path ([string]$smoke.isolated_profile_root) "LocalAppData\\dev.vectoralpha.vectorgrid")
    )
  } else {
    ""
  }
  if (
    $null -eq $profileIsolation -or
    [int]$profileIsolation.schema_version -ne 2 -or
    [string]$profileIsolation.status -ne "passed" -or
    -not [bool]$profileIsolation.real_profile_unchanged -or
    -not [bool]$profileIsolation.effective_isolation_proven -or
    [string]$profileIsolation.teardown -ne "completed" -or
    $null -eq $appAttestation -or
    [int]$appAttestation.schema_version -ne 2 -or
    [string]$appAttestation.kind -ne "vectorgrid-qa-profile-attestation" -or
    [string]$appAttestation.run_id -ne [string]$smoke.run_id -or
    [string]$appAttestation.original_identifier -ne "dev.vectoralpha.vectorgrid" -or
    -not (Test-PathEquals -Left ([string]$appAttestation.isolation_root) -Right ([string]$smoke.isolated_profile_root)) -or
    -not (Test-PathEquals -Left ([string]$appAttestation.effective_identifier) -Right $expectedIsolatedAppData) -or
    -not (Test-PathEquals -Left ([string]$appAttestation.app_data_dir) -Right $expectedIsolatedAppData)
  ) {
    throw "Public release requires schema-2 isolated-profile evidence with an exact matching application attestation, completed teardown, and the real profile unchanged."
  }
  if (-not [bool]$smoke.runtime_executed -or [string]$smoke.binary_origin -ne "extracted-bundle") {
    throw "Public release packaged smoke must execute the product binary extracted from the staged installer."
  }
  $releaseProofEvidence = $smoke.release_proof_evidence
  if (
    $null -eq $releaseProofEvidence -or
    [int]$releaseProofEvidence.schema_version -ne 1 -or
    [string]$releaseProofEvidence.contract -ne "vectorgrid-release-proof-evidence-manifest-v1" -or
    [string]$releaseProofEvidence.status -ne "passed" -or
    [string]$releaseProofEvidence.run_id -ne [string]$smoke.run_id -or
    [string]$releaseProofEvidence.release_variant -ne $Variant -or
    [int]$releaseProofEvidence.required_journey_count -le 0 -or
    @($releaseProofEvidence.journey_ids).Count -ne [int]$releaseProofEvidence.required_journey_count
  ) {
    throw "Public release requires one passing, current-run release-proof manifest covering every allowlisted native journey."
  }
  $releaseProofEvidenceRoot = [string]$releaseProofEvidence.evidence_root
  $releaseProofEvidenceManifestPath = [string]$releaseProofEvidence.manifest_path
  if (
    [string]::IsNullOrWhiteSpace($releaseProofEvidenceRoot) -or
    [string]::IsNullOrWhiteSpace($releaseProofEvidenceManifestPath) -or
    -not (Test-Path -LiteralPath $releaseProofEvidenceRoot -PathType Container) -or
    -not (Test-Path -LiteralPath $releaseProofEvidenceManifestPath -PathType Leaf)
  ) {
    throw "Public release-proof journey evidence files are missing."
  }
  Assert-PathUnderRoot -Path $releaseProofEvidenceRoot -Root $resolvedOutDir -Purpose "Release-proof evidence root"
  Assert-PathUnderRoot -Path $releaseProofEvidenceManifestPath -Root $resolvedOutDir -Purpose "Release-proof evidence manifest"
  if (-not (Test-PathEquals -Left (Split-Path -Parent $releaseProofEvidenceManifestPath) -Right $releaseProofEvidenceRoot)) {
    throw "Release-proof evidence manifest must live directly under its recorded evidence root."
  }
  $releaseProofEvidenceRootItem = Get-Item -LiteralPath $releaseProofEvidenceRoot -Force
  $releaseProofEvidenceManifestItem = Get-Item -LiteralPath $releaseProofEvidenceManifestPath -Force
  if (
    ($releaseProofEvidenceRootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
    ($releaseProofEvidenceManifestItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
    [int64]$releaseProofEvidenceManifestItem.Length -ne [int64]$releaseProofEvidence.manifest_size_bytes -or
    (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseProofEvidenceManifestPath).Hash.ToLowerInvariant() -ne [string]$releaseProofEvidence.manifest_sha256
  ) {
    throw "Release-proof evidence manifest identity changed after packaged smoke."
  }
  $releaseProofExtractionRoot = [string]$smoke.bundle_extraction_root
  $releaseProofBinaryPath = [string]$smoke.binary_path
  if (
    [string]::IsNullOrWhiteSpace($releaseProofExtractionRoot) -or
    -not (Test-Path -LiteralPath $releaseProofExtractionRoot -PathType Container) -or
    [string]::IsNullOrWhiteSpace($releaseProofBinaryPath) -or
    -not (Test-Path -LiteralPath $releaseProofBinaryPath -PathType Leaf)
  ) {
    throw "Release-proof packaged-binary evidence is missing."
  }
  Assert-PathUnderRoot -Path $releaseProofExtractionRoot -Root $resolvedOutDir -Purpose "Release-proof package extraction root"
  Assert-PathUnderRoot -Path $releaseProofBinaryPath -Root $releaseProofExtractionRoot -Purpose "Release-proof packaged binary"
  $releaseProofExtractionRootItem = Get-Item -LiteralPath $releaseProofExtractionRoot -Force
  $releaseProofBinaryItem = Get-Item -LiteralPath $releaseProofBinaryPath -Force
  if (
    ($releaseProofExtractionRootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
    ($releaseProofBinaryItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  ) {
    throw "Release-proof packaged-binary evidence may not be read through a symlink or junction."
  }
  $releaseProofContractScript = Join-Path $repoRoot "scripts\\release_proof_evidence_contract.mjs"
  $releaseProofContractArgs = @(
    $releaseProofContractScript,
    "--mode", "verify",
    "--evidence-root", $releaseProofEvidenceRoot,
    "--run-id", ([string]$smoke.run_id),
    "--variant", $Variant,
    "--binary-path", ([string]$smoke.binary_path),
    "--binary-size-bytes", ([string]$smoke.binary_size_bytes),
    "--binary-sha256", ([string]$smoke.binary_sha256),
    "--manifest-path", $releaseProofEvidenceManifestPath
  )
  $releaseProofContractOutput = (& node @releaseProofContractArgs 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) {
    throw "Public release-proof journey evidence revalidation failed: $($releaseProofContractOutput.Trim())"
  }
  $script:ReleaseProofEvidenceVerification = $releaseProofContractOutput | ConvertFrom-Json
  if (
    [string]$script:ReleaseProofEvidenceVerification.manifest.status -ne "passed" -or
    [int]$script:ReleaseProofEvidenceVerification.manifest.required_journey_count -ne [int]$releaseProofEvidence.required_journey_count -or
    [string]$script:ReleaseProofEvidenceVerification.manifest_sha256 -ne [string]$releaseProofEvidence.manifest_sha256
  ) {
    throw "Public release-proof journey evidence revalidation returned incomplete or mismatched authority."
  }

  $artifacts = @($manifest.staged_artifacts)
  $stagedArtifactsDir = [string]$manifest.staged_artifacts_dir
  if (
    [string]::IsNullOrWhiteSpace($stagedArtifactsDir) -or
    -not (Test-Path -LiteralPath $stagedArtifactsDir -PathType Container)
  ) {
    throw "Release manifest staged-artifacts directory is missing."
  }
  Assert-PathUnderRoot -Path $stagedArtifactsDir -Root $resolvedOutDir -Purpose "Staged release artifacts directory"
  $stagingDirItem = Get-Item -LiteralPath $stagedArtifactsDir -Force
  if (($stagingDirItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Staged release artifacts directory must not be a symlink or junction."
  }
  $actualArtifactEntries = @(Get-ChildItem -LiteralPath $stagedArtifactsDir -Force)
  if (@($actualArtifactEntries | Where-Object {
    $_.PSIsContainer -or
    ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  }).Count -gt 0) {
    throw "Staged release artifacts directory may contain only ordinary files."
  }
  $expectedArtifactNames = @($artifacts | ForEach-Object { [string]$_.staged_name } | Sort-Object -Unique)
  $actualArtifactNames = @($actualArtifactEntries | ForEach-Object { [string]$_.Name } | Sort-Object -Unique)
  if (
    $expectedArtifactNames.Count -ne $artifacts.Count -or
    $actualArtifactNames.Count -ne $expectedArtifactNames.Count -or
    @(Compare-Object -ReferenceObject $expectedArtifactNames -DifferenceObject $actualArtifactNames).Count -gt 0
  ) {
    throw "Staged release artifacts directory contains files not declared exactly once by manifest.staged_artifacts."
  }
  $setupArtifacts = @($artifacts | Where-Object {
    [string]$_.source_kind -eq "bundle" -and @(".exe", ".msi") -contains [string]$_.extension
  })
  if ($setupArtifacts.Count -eq 0) {
    throw "Public release manifest contains no staged installer artifact."
  }
  $smokedBundleSha256 = [string]$smoke.bundle_artifact_sha256
  $matchingSmokedBundle = @($setupArtifacts | Where-Object {
    -not [string]::IsNullOrWhiteSpace($smokedBundleSha256) -and
    [string]$_.sha256 -eq $smokedBundleSha256
  })
  if ($matchingSmokedBundle.Count -ne 1) {
    throw "Packaged smoke did not execute content extracted from exactly one staged installer in the release manifest."
  }

  $hashFilePath = [string]$manifest.hash_file
  if ([string]::IsNullOrWhiteSpace($hashFilePath) -or -not (Test-Path -LiteralPath $hashFilePath -PathType Leaf)) {
    throw "Release checksum file is missing."
  }
  Assert-PathUnderRoot -Path $hashFilePath -Root $resolvedOutDir -Purpose "Release checksum file"
  $checksumEntries = @{}
  foreach ($line in @(Get-Content -LiteralPath $hashFilePath)) {
    if ([string]::IsNullOrWhiteSpace($line)) {
      continue
    }
    if ($line -notmatch '^([A-Fa-f0-9]{64}) \*(.+)$') {
      throw "Release checksum file contains a malformed entry: $line"
    }
    $relativePath = $Matches[2].Replace([char]92, [char]47).ToLowerInvariant()
    if ($checksumEntries.ContainsKey($relativePath)) {
      throw "Release checksum file contains a duplicate entry: $relativePath"
    }
    $checksumEntries[$relativePath] = $Matches[1].ToLowerInvariant()
  }
  if ($checksumEntries.Count -ne $artifacts.Count) {
    throw "Release checksum entry count does not match the staged artifact count."
  }

  $finalArtifactHashes = @()
  foreach ($artifact in $artifacts) {
    $artifactPath = [string]$artifact.staged_path
    if ([string]::IsNullOrWhiteSpace($artifactPath) -or -not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
      throw "Staged release artifact is missing: $artifactPath"
    }
    Assert-PathUnderRoot -Path $artifactPath -Root $resolvedOutDir -Purpose "Staged release artifact"
    if (-not (Test-PathEquals -Left (Split-Path -Parent $artifactPath) -Right $stagedArtifactsDir)) {
      throw "Staged release artifact is not an immediate child of manifest.staged_artifacts_dir: $artifactPath"
    }
    $actualSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifactPath).Hash.ToLowerInvariant()
    $manifestSha256 = ([string]$artifact.sha256).ToLowerInvariant()
    $checksumKey = ("artifacts/" + [string]$artifact.staged_name).ToLowerInvariant()
    if ($actualSha256 -ne $manifestSha256) {
      throw "Staged artifact hash changed after manifest generation: $artifactPath"
    }
    if (-not $checksumEntries.ContainsKey($checksumKey) -or $checksumEntries[$checksumKey] -ne $actualSha256) {
      throw "Staged artifact hash does not match SHA256SUMS.txt: $artifactPath"
    }
    $finalArtifactHashes += [ordered]@{
      staged_path = $artifactPath
      staged_name = [string]$artifact.staged_name
      extension = [string]$artifact.extension
      sha256 = $actualSha256
      size_bytes = [int64](Get-Item -LiteralPath $artifactPath).Length
      code_signature = if (@(".exe", ".msi") -contains [string]$artifact.extension) {
        Get-CodeSignatureInfo -Path $artifactPath
      } else {
        $null
      }
    }
  }
  $script:FinalArtifactHashes = $finalArtifactHashes

  $extractionRoot = [string]$smoke.bundle_extraction_root
  $embeddedApplicationPath = [string]$smoke.binary_path
  if (
    [string]::IsNullOrWhiteSpace($extractionRoot) -or
    -not (Test-Path -LiteralPath $extractionRoot -PathType Container) -or
    [string]::IsNullOrWhiteSpace($embeddedApplicationPath) -or
    -not (Test-Path -LiteralPath $embeddedApplicationPath -PathType Leaf)
  ) {
    throw "Packaged smoke extracted-application evidence is missing."
  }
  Assert-PathUnderRoot -Path $extractionRoot -Root $resolvedOutDir -Purpose "Packaged smoke extraction root"
  Assert-PathUnderRoot -Path $embeddedApplicationPath -Root $extractionRoot -Purpose "Packaged smoke embedded application"
  $finalEmbeddedSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $embeddedApplicationPath).Hash.ToLowerInvariant()
  if ($finalEmbeddedSha256 -ne ([string]$smoke.binary_sha256).ToLowerInvariant()) {
    throw "Embedded application hash changed after packaged smoke."
  }
  $finalEmbeddedSignature = Get-CodeSignatureInfo -Path $embeddedApplicationPath
  $script:FinalEmbeddedApplication = [ordered]@{
    path = $embeddedApplicationPath
    sha256 = $finalEmbeddedSha256
    size_bytes = [int64](Get-Item -LiteralPath $embeddedApplicationPath).Length
    code_signature = $finalEmbeddedSignature
  }

  if (-not $AllowUnsignedBeta) {
    $unsignedOrInvalid = @($setupArtifacts | Where-Object {
      $null -eq $_.code_signature -or [string]$_.code_signature.status -ne "Valid"
    })
    if ($unsignedOrInvalid.Count -gt 0) {
      $names = ($unsignedOrInvalid | ForEach-Object { [string]$_.staged_name }) -join ", "
      throw "Public release requires valid Authenticode signatures for installer artifacts. Invalid or unsigned: $names"
    }
    if ($null -eq $smoke.binary_code_signature -or [string]$smoke.binary_code_signature.status -ne "Valid") {
      throw "Public release requires a valid Authenticode signature on the application executable embedded in the installer."
    }
    if ([string]$finalEmbeddedSignature.status -ne "Valid") {
      throw "Final Authenticode recheck failed for the embedded application."
    }
    $innerThumbprint = Normalize-Thumbprint -Value ([string]$finalEmbeddedSignature.thumbprint)
    if ($innerThumbprint -ne $resolvedExpectedSignerThumbprint) {
      throw "Embedded application signer does not match the expected publisher thumbprint."
    }
    if ([string]::IsNullOrWhiteSpace([string]$finalEmbeddedSignature.timestamp_thumbprint)) {
      throw "Public release requires a trusted timestamp on the embedded application signature."
    }
    $finalSetupArtifacts = @($finalArtifactHashes | Where-Object {
      @(".exe", ".msi") -contains [string]$_.extension
    })
    foreach ($artifact in $finalSetupArtifacts) {
      if ($null -eq $artifact.code_signature -or [string]$artifact.code_signature.status -ne "Valid") {
        throw "Final Authenticode recheck failed for installer '$($artifact.staged_name)'."
      }
      $outerThumbprint = Normalize-Thumbprint -Value ([string]$artifact.code_signature.thumbprint)
      if ($outerThumbprint -ne $resolvedExpectedSignerThumbprint -or $outerThumbprint -ne $innerThumbprint) {
        throw "Installer and embedded application signatures must match the expected publisher."
      }
      if ([string]::IsNullOrWhiteSpace([string]$artifact.code_signature.timestamp_thumbprint)) {
        throw "Public release requires a trusted timestamp on every installer signature."
      }
    }
  } else {
    Write-Warning "Unsigned beta mode enabled. Output may be useful for tester rehearsal but is not public-release complete."
  }
}

function Write-PublicReleaseSummary {
  param(
    [string]$Status,
    [string]$ErrorMessage = ""
  )

  $manifest = Read-JsonFile -Path (Join-Path $resolvedOutDir "manifest.json")
  $smoke = Read-JsonFile -Path (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = $Variant
    status = $Status
    out_dir = $resolvedOutDir
    allow_dirty_worktree = [bool]$AllowDirtyWorktree
    allow_unsigned_beta = [bool]$AllowUnsignedBeta
    include_options_gpu_promotion = [bool]$IncludeOptionsGpuPromotion
    public_release_eligible = ($Status -eq "passed" -and -not [bool]$AllowDirtyWorktree -and -not [bool]$AllowUnsignedBeta)
    expected_signer_thumbprint = if ([string]::IsNullOrWhiteSpace($resolvedExpectedSignerThumbprint)) { $null } else { $resolvedExpectedSignerThumbprint }
    post_build_sign_script = if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { $null } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) }
    error = if ([string]::IsNullOrWhiteSpace($ErrorMessage)) { $null } else { $ErrorMessage }
    candidate_summary = (Join-Path $resolvedOutDir "summary.json")
    manifest = (Join-Path $resolvedOutDir "manifest.json")
    hashes = (Join-Path $resolvedOutDir "SHA256SUMS.txt")
    packaged_smoke = (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
    live_safety = (Join-Path $resolvedOutDir "live-safety-summary.json")
    options_gpu_promotion = if ($IncludeOptionsGpuPromotion) { (Join-Path $resolvedOutDir "options-gpu-promotion-summary.json") } else { $null }
    artifact_source_kind = if ($null -ne $manifest) { [string]$manifest.artifact_source_kind } else { $null }
    manifest_publishable = if ($null -ne $manifest) { [bool]$manifest.publishable } else { $null }
    manifest_installer_signature_eligible = if ($null -ne $manifest) { [bool]$manifest.installer_signature_eligible } else { $null }
    manifest_unsigned_beta = if ($null -ne $manifest) { [bool]$manifest.unsigned_beta } else { $null }
    smoke_mode = if ($null -ne $smoke) { [string]$smoke.smoke_mode } else { $null }
    desktop_suite = if ($null -ne $smoke) { [string]$smoke.desktop_suite } else { $null }
    exact_package_release_proof = if ($null -ne $smoke) { [bool]$smoke.exact_package_release_proof } else { $false }
    profile_isolation_status = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [string]$smoke.profile_isolation.status } else { $null }
    profile_isolation_schema_version = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [int]$smoke.profile_isolation.schema_version } else { $null }
    real_profile_unchanged = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [bool]$smoke.profile_isolation.real_profile_unchanged } else { $false }
    effective_profile_isolation_proven = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [bool]$smoke.profile_isolation.effective_isolation_proven } else { $false }
    profile_app_attestation = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { $smoke.profile_isolation.app_attestation } else { $null }
    profile_isolation_teardown = if ($null -ne $smoke -and $null -ne $smoke.profile_isolation) { [string]$smoke.profile_isolation.teardown } else { $null }
    release_proof_evidence = if ($null -ne $smoke) { $smoke.release_proof_evidence } else { $null }
    release_proof_evidence_revalidated = if ($null -ne $script:ReleaseProofEvidenceVerification) {
      [ordered]@{
        status = [string]$script:ReleaseProofEvidenceVerification.manifest.status
        required_journey_count = [int]$script:ReleaseProofEvidenceVerification.manifest.required_journey_count
        manifest_path = [string]$script:ReleaseProofEvidenceVerification.manifest_path
        manifest_sha256 = [string]$script:ReleaseProofEvidenceVerification.manifest_sha256
      }
    } else {
      $null
    }
    live_safety_evidence_revalidated = $script:LiveSafetyEvidenceVerification
    final_artifact_hashes = @($script:FinalArtifactHashes)
    final_embedded_application = $script:FinalEmbeddedApplication
    staged_artifacts = if ($null -ne $manifest) { @($manifest.staged_artifacts | ForEach-Object { [string]$_.staged_path }) } else { @() }
    steps = $script:StepResults
  }
  Write-JsonUtf8 -Path $summaryJsonPath -Value $summary

  $lines = @(
    "# Public Release Gate",
    "",
    "- Status: $Status",
    "- Variant: $Variant",
    "- Output: $resolvedOutDir",
    "- Dirty worktree allowed: $([bool]$AllowDirtyWorktree)",
    "- Unsigned beta allowed: $([bool]$AllowUnsignedBeta)",
    "- Include options GPU promotion: $([bool]$IncludeOptionsGpuPromotion)",
    "- Public release eligible: $($Status -eq 'passed' -and -not [bool]$AllowDirtyWorktree -and -not [bool]$AllowUnsignedBeta)",
    "- Expected signer thumbprint: $(if ([string]::IsNullOrWhiteSpace($resolvedExpectedSignerThumbprint)) { 'not configured' } else { $resolvedExpectedSignerThumbprint })",
    "- Post-build signing hook: $(if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { 'env/default or none' } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) })"
  )
  if ($null -ne $manifest) {
    $lines += "- Manifest publishable: $([bool]$manifest.publishable)"
    $lines += "- Installer signature eligible: $([bool]$manifest.installer_signature_eligible)"
    $lines += "- Manifest unsigned beta: $([bool]$manifest.unsigned_beta)"
  }
  if ($null -ne $smoke) {
    $lines += "- Desktop suite: $([string]$smoke.desktop_suite)"
    $lines += "- Exact-package release proof: $([bool]$smoke.exact_package_release_proof)"
    $lines += "- Profile isolation: $(if ($null -ne $smoke.profile_isolation) { [string]$smoke.profile_isolation.status } else { 'missing' })"
  }
  if ($null -ne $script:LiveSafetyEvidenceVerification) {
    $lines += "- Execution safety authority: $([string]$script:LiveSafetyEvidenceVerification.authority.execution_safety_gate)"
    $lines += "- Live safety harness authority: $([string]$script:LiveSafetyEvidenceVerification.authority.live_safety_harness)"
    $lines += "- Live safety required tests: $([int]$script:LiveSafetyEvidenceVerification.authority.required_test_count)"
  }
  if (-not [string]::IsNullOrWhiteSpace($ErrorMessage)) {
    $lines += ""
    $lines += "## Failure"
    $lines += ""
    $lines += "- $ErrorMessage"
  }
  $lines += ""
  $lines += "## Steps"
  $lines += ""
  foreach ($step in $script:StepResults) {
    $statusLabel = if ($step.passed) { "PASS" } else { "FAIL" }
    $lines += "- [$statusLabel] $($step.name)"
    $lines += "  command: $($step.command)"
    if ($step.error) {
      $lines += "  error: $($step.error)"
    }
  }
  Set-Content -Path $summaryMdPath -Encoding UTF8 -Value $lines
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
Assert-FreshOutputDirectory -Path $resolvedOutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null
$summaryJsonPath = Join-Path $resolvedOutDir "public-release-summary.json"
$summaryMdPath = Join-Path $resolvedOutDir "public-release-summary.md"
$script:StepResults = @()
$script:FinalArtifactHashes = @()
$script:FinalEmbeddedApplication = $null
$script:ReleaseProofEvidenceVerification = $null
$script:LiveSafetyEvidenceVerification = $null
$resolvedExpectedSignerThumbprint = Normalize-Thumbprint -Value $(if (-not [string]::IsNullOrWhiteSpace($ExpectedSignerThumbprint)) {
  $ExpectedSignerThumbprint
} else {
  [string]$env:VECTORGRID_EXPECTED_SIGNER_THUMBPRINT
})

try {
  if (@("gpu-ma", "gpu-kernel") -contains $Variant -and -not $IncludePerf) {
    throw "Public GPU releases require -IncludePerf for the variant-specific optimization performance authority."
  }
  if ($Variant -eq "gpu-kernel" -and -not $IncludeGpuValidationPerf) {
    throw "Public gpu-kernel releases require -IncludeGpuValidationPerf for fixed-candidate GPU validation authority."
  }
  if ($IncludeGpuValidationPerf -and $Variant -ne "gpu-kernel") {
    throw "-IncludeGpuValidationPerf requires -Variant gpu-kernel."
  }
  if ($IncludeOptionsGpuPromotion -and $Variant -ne "gpu-kernel") {
    throw "-IncludeOptionsGpuPromotion requires -Variant gpu-kernel."
  }
  if ($AllowDirtyWorktree -and -not $AllowUnsignedBeta) {
    throw "-AllowDirtyWorktree is rehearsal-only and requires -AllowUnsignedBeta."
  }
  if (-not $AllowUnsignedBeta) {
    if ($resolvedExpectedSignerThumbprint -notmatch '^[A-F0-9]{40}$') {
      throw "Final public release requires -ExpectedSignerThumbprint (or VECTORGRID_EXPECTED_SIGNER_THUMBPRINT) as a 40-character certificate thumbprint."
    }
  }

  Invoke-PublicReleaseStep -Name "Clean worktree check" -CommandText "git -C `"$repoRoot`" status --porcelain" -Action {
    Assert-CleanWorktree
  }

  $candidateArgs = @(
    "-File", (Join-Path $repoRoot "scripts\\run_release_candidate_gate.ps1"),
    "-Variant", $Variant,
    "-OutDir", $resolvedOutDir,
    "-IncludeDesktopSmoke",
    "-DesktopSuite", "release-proof"
  )
  $candidateArgs += "-IncludeOutputParity"
  $candidateArgs += @("-IncludeExecutionSafetyGate", "-IncludeLiveSafetyHarness")
  if ($IncludePerf) {
    $candidateArgs += "-IncludePerf"
  }
  if ($IncludeGpuValidationPerf) {
    $candidateArgs += "-IncludeGpuValidationPerf"
  }
  if ($IncludeOptionsGpuPromotion) {
    $candidateArgs += @(
      "-IncludeOptionsGpuPromotion",
      "-OptionsGpuArtifactPath", $OptionsGpuArtifactPath,
      "-OptionsGpuBaselinePath", $OptionsGpuBaselinePath
    )
    if (-not [string]::IsNullOrWhiteSpace($OptionsGpuDatasetId)) {
      $candidateArgs += @(
        "-OptionsGpuDatasetId", $OptionsGpuDatasetId,
        "-OptionsGpuAppLocalDataDir", $OptionsGpuAppLocalDataDir
      )
    }
    if (-not [string]::IsNullOrWhiteSpace($OptionsGpuImportCsv)) {
      $candidateArgs += @(
        "-OptionsGpuImportCsv", $OptionsGpuImportCsv,
        "-OptionsGpuImportRootSymbol", $OptionsGpuImportRootSymbol
      )
    }
  }
  if ($IncludeExpandedBrowserQa) {
    $candidateArgs += "-IncludeExpandedBrowserQa"
  }
  if ($IncludeBaselineUi) {
    $candidateArgs += "-IncludeBaselineUi"
  }
  if ($SkipWorkspaceTests) {
    $candidateArgs += "-SkipWorkspaceTests"
  }
  if (-not [string]::IsNullOrWhiteSpace($PostBuildSignScript)) {
    $candidateArgs += @("-PostBuildSignScript", [System.IO.Path]::GetFullPath($PostBuildSignScript))
  }
  if ($AllowUnsignedBeta) {
    $candidateArgs += "-UnsignedBeta"
  }

  Invoke-PublicReleaseStep -Name "Release candidate gate" -CommandText ("powershell " + ($candidateArgs -join " ")) -Action {
    powershell @candidateArgs
    if ($LASTEXITCODE -ne 0) {
      throw "release candidate gate failed with exit code $LASTEXITCODE"
    }
  }

  Invoke-PublicReleaseStep -Name "Public release evidence check" -CommandText "manifest/signature/smoke/live-safety assertions" -Action {
    Assert-PublicReleaseEvidence
  }

  $completedStatus = if ($AllowUnsignedBeta) { "rehearsal-passed" } else { "passed" }
  Write-PublicReleaseSummary -Status $completedStatus
  Write-Host "Public release gate completed with status '$completedStatus'."
  Write-Host "Summary written to $summaryJsonPath"
} catch {
  Write-PublicReleaseSummary -Status "failed" -ErrorMessage $_.Exception.Message
  throw
}
