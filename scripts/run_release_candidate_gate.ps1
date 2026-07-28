param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [switch]$IncludePerf,
  [switch]$IncludeGpuValidationPerf,
  [switch]$IncludeOptionsGpuPromotion,
  [string]$OptionsGpuArtifactPath = "",
  [string]$OptionsGpuDatasetId = "",
  [string]$OptionsGpuImportCsv = "",
  [string]$OptionsGpuImportRootSymbol = "",
  [string]$OptionsGpuAppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OptionsGpuBaselinePath = "scripts/baselines/options_expression_perf_baseline.json",
  [switch]$SkipBrowserQa,
  [switch]$IncludeExpandedBrowserQa,
  [switch]$IncludeBaselineUi,
  [switch]$IncludeDesktopSmoke,
  [ValidateSet("smoke", "smoke-expanded", "workflow", "clean-start", "route-matrix", "release-proof")]
  [string]$DesktopSuite = "smoke",
  [string]$OutDir = "",
  [switch]$SkipPackagedSmoke,
  [switch]$SkipWorkspaceTests,
  [switch]$SkipWorkflowGate,
  [switch]$IncludeExecutionSafetyGate,
  [switch]$IncludeLiveSafetyHarness,
  [switch]$IncludeOutputParity = $true,
  [switch]$UnsignedBeta,
  [string]$PostBuildSignScript = ""
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 16
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
    throw "Release output must be a new or empty ordinary directory: $Path"
  }
  if ($null -ne (Get-ChildItem -LiteralPath $Path -Force | Select-Object -First 1)) {
    throw "Release output directory is not empty. Use a fresh path so stale evidence or installers cannot survive into this candidate: $Path"
  }
}

function Read-JsonFile {
  param([string]$Path)
  if (-not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path $Path)) {
    return Get-Content -Raw $Path | ConvertFrom-Json
  }
  return $null
}

function Resolve-RepoAnchoredPath {
  param([Parameter(Mandatory = $true)][string]$Path)
  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function Resolve-ReleaseBinaryPath {
  $candidate = Join-Path $targetDirectory "release\\ta_desktop_demo_app.exe"
  if (Test-Path -LiteralPath $candidate -PathType Leaf) {
    return (Resolve-Path -LiteralPath $candidate).Path
  }
  return $null
}

function Resolve-ReleaseBundleRoot {
  $candidate = Join-Path $targetDirectory "release\\bundle"
  if (Test-Path -LiteralPath $candidate -PathType Container) {
    return (Resolve-Path -LiteralPath $candidate).Path
  }
  return $null
}

function Resolve-CargoTargetDirectory {
  $manifestPath = Join-Path $repoRoot "Cargo.toml"
  $metadataText = (& cargo metadata --manifest-path $manifestPath --no-deps --format-version 1 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed while resolving the canonical target directory."
  }
  $metadata = $metadataText | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace([string]$metadata.target_directory)) {
    throw "cargo metadata did not report target_directory."
  }
  return [System.IO.Path]::GetFullPath([string]$metadata.target_directory)
}

function Resolve-ProjectTauriCliPath {
  $candidate = Join-Path $repoRoot "web\\node_modules\\.bin\\tauri.cmd"
  if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
    throw "The repository-pinned Tauri CLI is missing at '$candidate'. Run npm ci in web before the release gate."
  }
  return (Resolve-Path -LiteralPath $candidate).Path
}

function Clear-ReleasePackagingOutputs {
  $releaseDir = [System.IO.Path]::GetFullPath((Join-Path $targetDirectory "release"))
  $targetRoot = [System.IO.Path]::GetFullPath($targetDirectory)
  if (-not $releaseDir.StartsWith($targetRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean release outputs outside the Cargo target directory: $releaseDir"
  }

  $binaryPath = Join-Path $releaseDir "ta_desktop_demo_app.exe"
  if (Test-Path -LiteralPath $binaryPath -PathType Leaf) {
    Remove-Item -LiteralPath $binaryPath -Force
  }

  $bundleRoot = Join-Path $releaseDir "bundle"
  if (Test-Path -LiteralPath $bundleRoot -PathType Container) {
    $resolvedBundleRoot = [System.IO.Path]::GetFullPath($bundleRoot)
    foreach ($artifact in Get-ChildItem -LiteralPath $resolvedBundleRoot -Recurse -File) {
      if (@(".exe", ".msi", ".zip", ".sig") -notcontains $artifact.Extension.ToLowerInvariant()) {
        continue
      }
      $resolvedArtifact = [System.IO.Path]::GetFullPath($artifact.FullName)
      if (-not $resolvedArtifact.StartsWith($resolvedBundleRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a bundle artifact outside the release bundle root: $resolvedArtifact"
      }
      Remove-Item -LiteralPath $resolvedArtifact -Force
    }
  }
}

function Read-LatestMainBinaryFingerprint {
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
    last_write_time_utc = $fingerprint.LastWriteTimeUtc
    features = $features
  }
}

function Assert-FreshReleaseBuild {
  param(
    [Parameter(Mandatory = $true)]
    [datetime]$StartedAtUtc
  )

  $binaryPath = Resolve-ReleaseBinaryPath
  if ($null -eq $binaryPath) {
    throw "Tauri build did not recreate the canonical release executable under '$targetDirectory\\release'."
  }
  $binary = Get-Item -LiteralPath $binaryPath
  if ($binary.LastWriteTimeUtc -lt $StartedAtUtc.AddSeconds(-2)) {
    throw "Release executable predates this build and may be stale: $binaryPath"
  }

  $fingerprint = Read-LatestMainBinaryFingerprint
  if ($null -eq $fingerprint -or $fingerprint.last_write_time_utc -lt $StartedAtUtc.AddSeconds(-2)) {
    throw "No fresh Cargo fingerprint was found for ta_desktop_demo_app."
  }
  $expectedFeatures = @("desktop-ui")
  if ($Variant -eq "gpu-ma") {
    $expectedFeatures += "cuda"
  } elseif ($Variant -eq "gpu-kernel") {
    $expectedFeatures += @("cuda", "cuda-backtest-kernel")
  }
  foreach ($feature in $expectedFeatures) {
    if ($fingerprint.features -notcontains $feature) {
      throw "The fresh desktop executable fingerprint is missing required feature '$feature': $($fingerprint.path)"
    }
  }
  if ($Variant -eq "cpu" -and ($fingerprint.features -contains "cuda" -or $fingerprint.features -contains "cuda-backtest-kernel")) {
    throw "The CPU release executable fingerprint unexpectedly contains CUDA features."
  }
  if ($Variant -eq "gpu-ma" -and $fingerprint.features -contains "cuda-backtest-kernel") {
    throw "The GPU-MA release executable fingerprint unexpectedly contains cuda-backtest-kernel."
  }

  $bundleRoot = Resolve-ReleaseBundleRoot
  if ($null -eq $bundleRoot) {
    throw "Tauri build did not recreate a release bundle."
  }
  $bundleArtifacts = @(Get-ChildItem -LiteralPath $bundleRoot -Recurse -File | Where-Object {
    @(".exe", ".msi", ".zip") -contains $_.Extension.ToLowerInvariant()
  })
  if ($bundleArtifacts.Count -ne 1) {
    throw "Expected exactly one fresh release bundle artifact, found $($bundleArtifacts.Count) under '$bundleRoot'."
  }
  if ($bundleArtifacts[0].LastWriteTimeUtc -lt $StartedAtUtc.AddSeconds(-2)) {
    throw "Release bundle artifact predates this build and may be stale: $($bundleArtifacts[0].FullName)"
  }

  $installerScript = Get-ChildItem -LiteralPath (Join-Path $targetDirectory "release\\nsis") -Filter "installer.nsi" -File -Recurse -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
  if ($null -eq $installerScript) {
    throw "Generated NSIS installer script was not found."
  }
  $unexpectedExecutables = @(Select-String -LiteralPath $installerScript.FullName -Pattern '^\s*File\s+/a\s+".*oname=.*\.exe"' -CaseSensitive:$false)
  if ($unexpectedExecutables.Count -gt 0) {
    throw "NSIS payload contains unexpected support executables: $($unexpectedExecutables.Line -join '; ')"
  }

  $script:ReleaseBinaryEvidence = [ordered]@{
    path = $binary.FullName
    size_bytes = $binary.Length
    sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary.FullName).Hash.ToLowerInvariant()
    last_write_time_utc = $binary.LastWriteTimeUtc.ToString("o")
    fingerprint_path = $fingerprint.path
    fingerprint_features = $fingerprint.features
    bundle_artifact = $bundleArtifacts[0].FullName
  }
}

function Assert-RequiredScript {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Purpose
  )

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "$Purpose requires script '$Path', but it is not present in this checkout."
  }
}

function Invoke-ReleaseStep {
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
    $step = [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $true
    }
    $script:StepResults += $step
    Write-Host "[PASS] $Name"
  } catch {
    $step = [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $false
      error = $_.Exception.Message
    }
    $script:StepResults += $step
    throw
  }
}

function Write-ReleaseSummary {
  param(
    [string]$Status,
    [string]$ErrorMessage = ""
  )

  $manifest = Read-JsonFile -Path (Join-Path $resolvedOutDir "manifest.json")
  $packagedSmokeSummary = Read-JsonFile -Path (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = $Variant
    status = $Status
    out_dir = $resolvedOutDir
    include_perf = [bool]$IncludePerf
    include_gpu_validation_perf = [bool]$IncludeGpuValidationPerf
    include_options_gpu_promotion = [bool]$IncludeOptionsGpuPromotion
    skip_browser_qa = [bool]$SkipBrowserQa
    include_expanded_browser_qa = [bool]$IncludeExpandedBrowserQa
    include_baseline_ui = [bool]$IncludeBaselineUi
    include_desktop_smoke = [bool]$IncludeDesktopSmoke
    desktop_suite = $DesktopSuite
    include_execution_safety_gate = [bool]$IncludeExecutionSafetyGate
    include_live_safety_harness = [bool]$IncludeLiveSafetyHarness
    include_output_parity = [bool]$IncludeOutputParity
    skip_packaged_smoke = [bool]$SkipPackagedSmoke
    skip_workspace_tests = [bool]$SkipWorkspaceTests
    skip_workflow_gate = [bool]$SkipWorkflowGate
    workflow_authority_complete = -not [bool]$SkipWorkflowGate
    workflow_skip_reason = $script:WorkflowSkipReason
    unsigned_beta = [bool]$UnsignedBeta
    post_build_sign_script = $script:EffectivePostBuildSignScript
    post_build_sign_script_source = $script:EffectivePostBuildSignScriptSource
    error = if ([string]::IsNullOrWhiteSpace($ErrorMessage)) { $null } else { $ErrorMessage }
    artifacts = [ordered]@{
      readiness = (Join-Path $resolvedOutDir "readiness.json")
      provenance = (Join-Path $resolvedOutDir "provenance.json")
      manifest = (Join-Path $resolvedOutDir "manifest.json")
      hashes = (Join-Path $resolvedOutDir "SHA256SUMS.txt")
      packaged_smoke = (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
      live_safety = (Join-Path $resolvedOutDir "live-safety-summary.json")
      options_gpu_promotion = (Join-Path $resolvedOutDir "options-gpu-promotion-summary.json")
      desktop_smoke_dir = (Join-Path $resolvedOutDir "desktop-smoke")
    }
    release_artifact = [ordered]@{
      source_kind = if ($null -ne $manifest) { [string]$manifest.artifact_source_kind } else { $null }
      release_binary_path = if ($null -ne $manifest) { [string]$manifest.release_binary_path } else { $null }
      bundle_root = if ($null -ne $manifest) { [string]$manifest.bundle_root } else { $null }
    }
    packaged_smoke = [ordered]@{
      status = if ($null -ne $packagedSmokeSummary) { [string]$packagedSmokeSummary.status } else { $null }
      smoke_mode = if ($null -ne $packagedSmokeSummary) { [string]$packagedSmokeSummary.smoke_mode } else { $null }
      desktop_suite = if ($null -ne $packagedSmokeSummary) { [string]$packagedSmokeSummary.desktop_suite } else { $null }
      exact_package_release_proof = if ($null -ne $packagedSmokeSummary) { [bool]$packagedSmokeSummary.exact_package_release_proof } else { $false }
      diagnosis = if ($null -ne $packagedSmokeSummary) { [string]$packagedSmokeSummary.diagnosis } else { $null }
    }
    steps = $script:StepResults
  }
  Write-JsonUtf8 -Path $summaryJsonPath -Value $summary

  $lines = @(
    "# Release Candidate Gate",
    "",
    "- Status: $Status",
    "- Variant: $Variant",
    "- Output: $resolvedOutDir",
    "- Include perf: $([bool]$IncludePerf)",
    "- Include GPU validation perf: $([bool]$IncludeGpuValidationPerf)",
    "- Include options GPU promotion: $([bool]$IncludeOptionsGpuPromotion)",
    "- Skip browser QA: $([bool]$SkipBrowserQa)",
    "- Include expanded browser QA: $([bool]$IncludeExpandedBrowserQa)",
    "- Include baseline UI: $([bool]$IncludeBaselineUi)",
    "- Include desktop smoke: $([bool]$IncludeDesktopSmoke)",
    "- Desktop suite: $DesktopSuite",
    "- Include execution safety gate: $([bool]$IncludeExecutionSafetyGate)",
    "- Include live safety harness: $([bool]$IncludeLiveSafetyHarness)",
    "- Include output parity: $([bool]$IncludeOutputParity)",
    "- Skip packaged smoke: $([bool]$SkipPackagedSmoke)",
    "- Skip workspace tests: $([bool]$SkipWorkspaceTests)",
    "- Skip workflow gate: $([bool]$SkipWorkflowGate)",
    "- Workflow authority complete: $(-not [bool]$SkipWorkflowGate)",
    "- Unsigned beta artifact labeling: $([bool]$UnsignedBeta)",
    "- Post-build signing hook: $(if ($null -eq $script:EffectivePostBuildSignScript) { 'none' } else { $script:EffectivePostBuildSignScript })",
    "- Post-build signing hook source: $($script:EffectivePostBuildSignScriptSource)"
  )
  if ($null -ne $manifest -and -not [string]::IsNullOrWhiteSpace([string]$manifest.artifact_source_kind)) {
    $lines += "- Release artifact source: $([string]$manifest.artifact_source_kind)"
  }
  if ($null -ne $packagedSmokeSummary -and -not [string]::IsNullOrWhiteSpace([string]$packagedSmokeSummary.smoke_mode)) {
    $lines += "- Packaged smoke mode: $([string]$packagedSmokeSummary.smoke_mode)"
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
    $statusLabel = if ([bool]$step.skipped) { "SKIP" } elseif ($step.passed) { "PASS" } else { "FAIL" }
    $lines += "- [$statusLabel] $($step.name)"
    $lines += "  command: $($step.command)"
    if ($step.skip_reason) {
      $lines += "  reason: $($step.skip_reason)"
    }
    if ($step.error) {
      $lines += "  error: $($step.error)"
    }
  }
  if ($null -ne $packagedSmokeSummary -and -not [string]::IsNullOrWhiteSpace([string]$packagedSmokeSummary.diagnosis)) {
    $lines += ""
    $lines += "## Packaged Smoke Note"
    $lines += ""
    $lines += "- $([string]$packagedSmokeSummary.diagnosis)"
  }
  Set-Content -Path $summaryMdPath -Encoding UTF8 -Value $lines
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
Assert-FreshOutputDirectory -Path $resolvedOutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null
$targetDirectory = Resolve-CargoTargetDirectory
$tauriCliPath = Resolve-ProjectTauriCliPath
$summaryJsonPath = Join-Path $resolvedOutDir "summary.json"
$summaryMdPath = Join-Path $resolvedOutDir "summary.md"
$script:StepResults = @()
$script:WorkflowSkipReason = if ($SkipWorkflowGate) {
  "Performance/workflow authority was deliberately deferred; this output is topology/static-package rehearsal evidence only."
} else {
  $null
}
$script:EffectivePostBuildSignScriptSource = if (-not [string]::IsNullOrWhiteSpace($PostBuildSignScript)) {
  "parameter"
} elseif (-not [string]::IsNullOrWhiteSpace([string]$env:VECTORGRID_POST_BUILD_SIGN_SCRIPT)) {
  "environment"
} else {
  "none"
}
$effectivePostBuildSignScriptValue = if ($script:EffectivePostBuildSignScriptSource -eq "parameter") {
  $PostBuildSignScript
} elseif ($script:EffectivePostBuildSignScriptSource -eq "environment") {
  [string]$env:VECTORGRID_POST_BUILD_SIGN_SCRIPT
} else {
  $null
}
$script:EffectivePostBuildSignScript = if ([string]::IsNullOrWhiteSpace([string]$effectivePostBuildSignScriptValue)) {
  $null
} else {
  [System.IO.Path]::GetFullPath([string]$effectivePostBuildSignScriptValue)
}

try {
  if ($IncludeGpuValidationPerf -and $Variant -ne "gpu-kernel") {
    throw "-IncludeGpuValidationPerf requires -Variant gpu-kernel because the validation perf gate exercises cuda-backtest-kernel."
  }
  if ($IncludePerf -and $Variant -eq "gpu-ma") {
    $gpuMaPerfBaselinePath = Join-Path $repoRoot "scripts\\baselines\\gpu_ma_perf_baseline.json"
    if (-not (Test-Path -LiteralPath $gpuMaPerfBaselinePath -PathType Leaf)) {
      throw "The gpu-ma release performance baseline is missing: $gpuMaPerfBaselinePath. Establish it explicitly with scripts\\run_gpu_ma_perf_gate.ps1 -Mode baseline before release rehearsal."
    }
  }
  if ($IncludeOptionsGpuPromotion -and $Variant -ne "gpu-kernel") {
    throw "-IncludeOptionsGpuPromotion requires -Variant gpu-kernel because the options scorer uses cuda-backtest-kernel."
  }
  if ($IncludeOptionsGpuPromotion) {
    if ([string]::IsNullOrWhiteSpace($OptionsGpuArtifactPath)) {
      throw "-IncludeOptionsGpuPromotion requires an explicit -OptionsGpuArtifactPath; latest-artifact discovery is not release authority."
    }
    $hasOptionsDatasetId = -not [string]::IsNullOrWhiteSpace($OptionsGpuDatasetId)
    $hasOptionsImportCsv = -not [string]::IsNullOrWhiteSpace($OptionsGpuImportCsv)
    if ($hasOptionsDatasetId -eq $hasOptionsImportCsv) {
      throw "-IncludeOptionsGpuPromotion requires exactly one of -OptionsGpuDatasetId or -OptionsGpuImportCsv."
    }
    if ($hasOptionsDatasetId -and -not [string]::IsNullOrWhiteSpace($OptionsGpuImportRootSymbol)) {
      throw "-OptionsGpuImportRootSymbol is only valid with -OptionsGpuImportCsv."
    }
    if ($hasOptionsImportCsv -and [string]::IsNullOrWhiteSpace($OptionsGpuImportRootSymbol)) {
      throw "-OptionsGpuImportCsv requires -OptionsGpuImportRootSymbol."
    }

    $script:ResolvedOptionsGpuArtifactPath = Resolve-RepoAnchoredPath -Path $OptionsGpuArtifactPath
    if (-not (Test-Path -LiteralPath $script:ResolvedOptionsGpuArtifactPath -PathType Leaf)) {
      throw "Options GPU promotion artifact does not exist: $($script:ResolvedOptionsGpuArtifactPath)"
    }
    if ($hasOptionsImportCsv) {
      $script:ResolvedOptionsGpuImportCsv = Resolve-RepoAnchoredPath -Path $OptionsGpuImportCsv
      if (-not (Test-Path -LiteralPath $script:ResolvedOptionsGpuImportCsv -PathType Leaf)) {
        throw "Options GPU promotion import CSV does not exist: $($script:ResolvedOptionsGpuImportCsv)"
      }
    }
    $script:ResolvedOptionsGpuBaselinePath = Resolve-RepoAnchoredPath -Path $OptionsGpuBaselinePath
    if (-not (Test-Path -LiteralPath $script:ResolvedOptionsGpuBaselinePath -PathType Leaf)) {
      throw "Options GPU promotion baseline does not exist: $($script:ResolvedOptionsGpuBaselinePath)"
    }
  }
  if ($PSBoundParameters.ContainsKey("DesktopSuite") -and (-not $IncludeDesktopSmoke -or $SkipPackagedSmoke)) {
    throw "-DesktopSuite requires -IncludeDesktopSmoke and cannot be combined with -SkipPackagedSmoke."
  }
  if ($SkipWorkflowGate -and -not $UnsignedBeta) {
    throw "-SkipWorkflowGate is rehearsal-only and requires -UnsignedBeta."
  }
  if ($SkipWorkflowGate -and ($IncludePerf -or $IncludeGpuValidationPerf)) {
    throw "-SkipWorkflowGate cannot be combined with -IncludePerf or -IncludeGpuValidationPerf."
  }
  if ($SkipWorkflowGate -and $IncludeOptionsGpuPromotion) {
    throw "-SkipWorkflowGate cannot be combined with -IncludeOptionsGpuPromotion."
  }

  $doctorArgs = @("-File", (Join-Path $repoRoot "scripts\\run_release_doctor.ps1"), "-Variant", $Variant, "-OutDir", $resolvedOutDir)
  if ($IncludeDesktopSmoke -and -not $SkipPackagedSmoke) {
    $doctorArgs += "-RequirePackagedSmoke"
  }
  Invoke-ReleaseStep -Name "Release doctor" -CommandText ("powershell " + ($doctorArgs -join " ")) -Action {
    powershell @doctorArgs
    if ($LASTEXITCODE -ne 0) {
      throw "release doctor failed with exit code $LASTEXITCODE"
    }
  }

  $qualityArgs = @("-File", (Join-Path $repoRoot "scripts\\run_repo_quality_gate.ps1"))
  if (-not $SkipWorkspaceTests) {
    $qualityArgs += "-IncludeWorkspaceTests"
  }
  Invoke-ReleaseStep -Name "Repo quality gate" -CommandText ("powershell " + ($qualityArgs -join " ")) -Action {
    powershell @qualityArgs
    if ($LASTEXITCODE -ne 0) {
      throw "repo quality gate failed with exit code $LASTEXITCODE"
    }
  }

  Invoke-ReleaseStep -Name "Web parity" -CommandText "npm --prefix web run parity" -Action {
    & npm.cmd --prefix web run parity
    if ($LASTEXITCODE -ne 0) {
      throw "web parity failed with exit code $LASTEXITCODE"
    }
  }

  if (-not $SkipBrowserQa) {
    $browserQaArgs = @("-File", (Join-Path $repoRoot "scripts\\run_frontend_e2e_gate.ps1"), "-IncludeA11ySmoke")
    if ($IncludeExpandedBrowserQa) {
      $browserQaArgs += @("-IncludeResponsive", "-IncludeRegressions")
    }
    if ($IncludeBaselineUi) {
      $browserQaArgs += "-IncludeBaselineUi"
    }
    Invoke-ReleaseStep -Name "Frontend browser QA" -CommandText ("powershell " + ($browserQaArgs -join " ")) -Action {
      powershell @browserQaArgs
      if ($LASTEXITCODE -ne 0) {
        throw "frontend browser QA failed with exit code $LASTEXITCODE"
      }
    }
  }

  if ($IncludeOutputParity) {
    Assert-RequiredScript `
      -Path (Join-Path $repoRoot "scripts\\run_output_parity_check.mjs") `
      -Purpose "Output parity"
    Invoke-ReleaseStep -Name "Output parity" -CommandText "node scripts/run_output_parity_check.mjs" -Action {
      node "$repoRoot\\scripts\\run_output_parity_check.mjs"
      if ($LASTEXITCODE -ne 0) {
        throw "output parity failed with exit code $LASTEXITCODE"
      }
    }
  } else {
    Write-Host "[SKIP] Output parity was not requested for this release-candidate rehearsal."
  }

  if ($SkipWorkflowGate) {
    Write-Host "[SKIP] Workflow gate was explicitly deferred for this unsigned rehearsal."
    $skippedAt = (Get-Date).ToUniversalTime()
    $script:StepResults += [ordered]@{
      name = "Workflow gate"
      command = "powershell scripts\\run_workflow_gate.ps1 -Mode candidate"
      started_at_utc = $skippedAt.ToString("o")
      finished_at_utc = $skippedAt.ToString("o")
      passed = $true
      skipped = $true
      skip_reason = $script:WorkflowSkipReason
      error = $null
    }
  } else {
    $workflowArgs = @("-File", (Join-Path $repoRoot "scripts\\run_workflow_gate.ps1"), "-Mode", "candidate")
    Invoke-ReleaseStep -Name "Workflow gate" -CommandText ("powershell " + ($workflowArgs -join " ")) -Action {
      powershell @workflowArgs
      if ($LASTEXITCODE -ne 0) {
        throw "workflow gate failed with exit code $LASTEXITCODE"
      }
    }
  }

  if ($IncludePerf) {
    $cpuArgs = @("-File", (Join-Path $repoRoot "scripts\\run_cpu_perf_gate.ps1"), "-Mode", "candidate")
    Invoke-ReleaseStep -Name "CPU perf gate" -CommandText ("powershell " + ($cpuArgs -join " ")) -Action {
      powershell @cpuArgs
      if ($LASTEXITCODE -ne 0) {
        throw "CPU perf gate failed with exit code $LASTEXITCODE"
      }
    }

    if ($Variant -eq "gpu-ma") {
      $gpuArgs = @("-File", (Join-Path $repoRoot "scripts\\run_gpu_ma_perf_gate.ps1"), "-Mode", "candidate")
      Invoke-ReleaseStep -Name "GPU MA-sweep perf gate" -CommandText ("powershell " + ($gpuArgs -join " ")) -Action {
        powershell @gpuArgs
        if ($LASTEXITCODE -ne 0) {
          throw "GPU MA-sweep perf gate failed with exit code $LASTEXITCODE"
        }
      }
    } elseif ($Variant -eq "gpu-kernel") {
      $gpuArgs = @("-File", (Join-Path $repoRoot "scripts\\run_gpu_perf_gate.ps1"), "-Mode", "candidate")
      Invoke-ReleaseStep -Name "GPU kernel perf gate" -CommandText ("powershell " + ($gpuArgs -join " ")) -Action {
        powershell @gpuArgs
        if ($LASTEXITCODE -ne 0) {
          throw "GPU kernel perf gate failed with exit code $LASTEXITCODE"
        }
      }
    }
  }

  if ($IncludeGpuValidationPerf) {
    $gpuValidationArgs = @(
      "-File", (Join-Path $repoRoot "scripts\\run_gpu_validation_perf_gate.ps1"),
      "-Mode", "candidate",
      "-WorkloadProfile", "release-authority"
    )
    Invoke-ReleaseStep -Name "GPU validation perf gate" -CommandText ("powershell " + ($gpuValidationArgs -join " ")) -Action {
      powershell @gpuValidationArgs
      if ($LASTEXITCODE -ne 0) {
        throw "GPU validation perf gate failed with exit code $LASTEXITCODE"
      }
    }
  }

  if ($IncludeOptionsGpuPromotion) {
    $optionsGpuOutDir = Join-Path $resolvedOutDir "options-gpu-promotion"
    $optionsGpuSummaryPath = Join-Path $resolvedOutDir "options-gpu-promotion-summary.json"
    $optionsGpuArgs = @(
      "-File", (Join-Path $repoRoot "scripts\\run_options_expression_gpu_promotion_gate.ps1"),
      "-Mode", "candidate",
      "-ArtifactPath", $script:ResolvedOptionsGpuArtifactPath,
      "-OutDir", $optionsGpuOutDir,
      "-BaselinePath", $script:ResolvedOptionsGpuBaselinePath,
      "-SummaryOutPath", $optionsGpuSummaryPath
    )
    if (-not [string]::IsNullOrWhiteSpace($OptionsGpuDatasetId)) {
      $optionsGpuArgs += @("-DatasetId", $OptionsGpuDatasetId)
      if (-not [string]::IsNullOrWhiteSpace($OptionsGpuAppLocalDataDir)) {
        $optionsGpuArgs += @("-AppLocalDataDir", (Resolve-RepoAnchoredPath -Path $OptionsGpuAppLocalDataDir))
      }
    } else {
      $optionsGpuArgs += @(
        "-ImportCsv", $script:ResolvedOptionsGpuImportCsv,
        "-ImportRootSymbol", $OptionsGpuImportRootSymbol
      )
    }
    Invoke-ReleaseStep -Name "Options GPU promotion authority" -CommandText ("powershell " + ($optionsGpuArgs -join " ")) -Action {
      powershell @optionsGpuArgs
      if ($LASTEXITCODE -ne 0) {
        throw "Options GPU promotion authority failed with exit code $LASTEXITCODE"
      }
    }
  }

  $tauriBuildFeatures = @("desktop-ui")
  if ($Variant -eq "gpu-ma") {
    $tauriBuildFeatures += "cuda"
  } elseif ($Variant -eq "gpu-kernel") {
    $tauriBuildFeatures += "cuda-backtest-kernel"
  }
  $tauriBuildArgs = @(
    "build",
    "--ci",
    "--bundles", "nsis",
    "--features", ($tauriBuildFeatures -join ",")
  )
  $buildCommand = "Push-Location src-tauri; "
  if ($Variant -eq "gpu-kernel") {
    $buildCommand += '$env:VECTORBT_USE_PREBUILT_PTX = "1"; '
  }
  $buildCommand += "& `"$tauriCliPath`" " + ($tauriBuildArgs -join " ") + "; Pop-Location"
  Invoke-ReleaseStep -Name "Tauri release build" -CommandText $buildCommand -Action {
    $prevPrebuilt = $env:VECTORBT_USE_PREBUILT_PTX
    $pushedLocation = $false
    try {
      Clear-ReleasePackagingOutputs
      $script:TauriBuildStartedAtUtc = (Get-Date).ToUniversalTime()
      Push-Location (Join-Path $repoRoot "src-tauri")
      $pushedLocation = $true
      if ($Variant -eq "gpu-kernel") {
        $env:VECTORBT_USE_PREBUILT_PTX = "1"
      }
      & $tauriCliPath @tauriBuildArgs
      if ($LASTEXITCODE -ne 0) {
        throw "tauri build failed with exit code $LASTEXITCODE"
      }
    } finally {
      if ($pushedLocation) {
        Pop-Location
      }
      if ($null -ne $prevPrebuilt) {
        $env:VECTORBT_USE_PREBUILT_PTX = $prevPrebuilt
      } else {
        Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
      }
    }
  }

  Assert-FreshReleaseBuild -StartedAtUtc $script:TauriBuildStartedAtUtc
  $releaseBinaryPath = Resolve-ReleaseBinaryPath
  $releaseBundleRoot = Resolve-ReleaseBundleRoot

  $effectivePostBuildSignScript = $script:EffectivePostBuildSignScript
  if (-not [string]::IsNullOrWhiteSpace($effectivePostBuildSignScript)) {
    $resolvedSignScript = $effectivePostBuildSignScript
    if (-not (Test-Path -LiteralPath $resolvedSignScript -PathType Leaf)) {
      throw "Post-build signing script does not exist: $resolvedSignScript"
    }
    $signArgs = @(
      "-ExecutionPolicy", "Bypass",
      "-File", $resolvedSignScript,
      "-Variant", $Variant,
      "-OutDir", $resolvedOutDir,
      "-ReleaseBinaryPath", $releaseBinaryPath
    )
    if ($null -ne $releaseBundleRoot) {
      $signArgs += @("-BundleRoot", $releaseBundleRoot)
    }
    Invoke-ReleaseStep -Name "Post-build signing hook" -CommandText ("powershell " + ($signArgs -join " ")) -Action {
      powershell @signArgs
      if ($LASTEXITCODE -ne 0) {
        throw "post-build signing hook failed with exit code $LASTEXITCODE"
      }
    }
  }

  $provenanceArgs = @(
    "-File", (Join-Path $repoRoot "scripts\\capture_release_provenance.ps1"),
    "-Variant", $Variant,
    "-OutDir", $resolvedOutDir,
    "-BinaryPath", $releaseBinaryPath
  )
  Invoke-ReleaseStep -Name "Release provenance" -CommandText ("powershell " + ($provenanceArgs -join " ")) -Action {
    $prevPrebuilt = $env:VECTORBT_USE_PREBUILT_PTX
    try {
      if ($Variant -eq "gpu-kernel") {
        $env:VECTORBT_USE_PREBUILT_PTX = "1"
      }
      powershell @provenanceArgs
      if ($LASTEXITCODE -ne 0) {
        throw "release provenance failed with exit code $LASTEXITCODE"
      }
    } finally {
      if ($null -ne $prevPrebuilt) {
        $env:VECTORBT_USE_PREBUILT_PTX = $prevPrebuilt
      } else {
        Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
      }
    }
  }

  $manifestArgs = @(
    "-File", (Join-Path $repoRoot "scripts\\generate_release_manifest.ps1"),
    "-Variant", $Variant,
    "-OutDir", $resolvedOutDir,
    "-ReleaseBinaryPath", $releaseBinaryPath,
    "-ProvenancePath", (Join-Path $resolvedOutDir "provenance.json")
  )
  if ($UnsignedBeta) {
    $manifestArgs += "-UnsignedBeta"
  }
  Invoke-ReleaseStep -Name "Release manifest" -CommandText ("powershell " + ($manifestArgs -join " ")) -Action {
    powershell @manifestArgs
    if ($LASTEXITCODE -ne 0) {
      throw "release manifest generation failed with exit code $LASTEXITCODE"
    }
    $manifest = Read-JsonFile -Path (Join-Path $resolvedOutDir "manifest.json")
    if ($null -eq $manifest) {
      throw "release manifest generation did not produce manifest.json"
    }
    if ($UnsignedBeta) {
      if (-not [bool]$manifest.unsigned_beta -or [bool]$manifest.publishable -or [bool]$manifest.installer_signature_eligible) {
        throw "unsigned beta manifest must be explicitly labeled and ineligible for publication"
      }
    } elseif (
      [bool]$manifest.unsigned_beta -or
      [bool]$manifest.publishable -or
      -not [bool]$manifest.installer_signature_eligible
    ) {
      throw "release candidates require a validly signed installer before final embedded-app smoke; configure bundle-time signing or explicitly use -UnsignedBeta"
    }
  }

  if ($IncludeDesktopSmoke -and -not $SkipPackagedSmoke) {
    Assert-RequiredScript `
      -Path (Join-Path $repoRoot "scripts\\run_packaged_smoke.ps1") `
      -Purpose "Packaged desktop smoke"
    $smokeArgs = @(
      "-File", (Join-Path $repoRoot "scripts\\run_packaged_smoke.ps1"),
      "-OutDir", $resolvedOutDir,
      "-DesktopSuite", $DesktopSuite
    )
    Invoke-ReleaseStep -Name "Packaged desktop smoke" -CommandText ("powershell " + ($smokeArgs -join " ")) -Action {
      powershell @smokeArgs
      if ($LASTEXITCODE -ne 0) {
        throw "packaged desktop smoke failed with exit code $LASTEXITCODE"
      }
    }
  }

  if ($IncludeExecutionSafetyGate) {
    $executionSafetyArgs = @(
      "-NoProfile",
      "-ExecutionPolicy", "Bypass",
      "-File", (Join-Path $repoRoot "scripts\\run_execution_safety_gate.ps1")
    )
    Invoke-ReleaseStep -Name "Execution safety gate" -CommandText ("powershell " + ($executionSafetyArgs -join " ")) -Action {
      powershell @executionSafetyArgs
      if ($LASTEXITCODE -ne 0) {
        throw "execution safety gate failed with exit code $LASTEXITCODE"
      }
    }
  }

  if ($IncludeLiveSafetyHarness) {
    $liveArgs = @(
      "-NoProfile",
      "-ExecutionPolicy", "Bypass",
      "-File", (Join-Path $repoRoot "scripts\\run_live_execution_safety_harness.ps1"),
      "-OutDir", $resolvedOutDir
    )
    Invoke-ReleaseStep -Name "Live execution safety harness" -CommandText ("powershell " + ($liveArgs -join " ")) -Action {
      powershell @liveArgs
      if ($LASTEXITCODE -ne 0) {
        throw "live execution safety harness failed with exit code $LASTEXITCODE"
      }
    }
  }

  Write-ReleaseSummary -Status "passed"
  Write-Host "Release candidate gate completed successfully."
  Write-Host "Summary written to $summaryJsonPath"
} catch {
  Write-ReleaseSummary -Status "failed" -ErrorMessage $_.Exception.Message
  throw
}
