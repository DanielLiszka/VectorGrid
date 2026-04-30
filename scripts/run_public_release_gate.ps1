param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [switch]$IncludePerf,
  [switch]$IncludeGpuValidationPerf,
  [switch]$IncludeExpandedBrowserQa,
  [switch]$IncludeBaselineUi,
  [switch]$SkipWorkspaceTests,
  [switch]$AllowDirtyWorktree,
  [switch]$AllowUnsignedBeta,
  [string]$PostBuildSignScript = ""
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

function Read-JsonFile {
  param([string]$Path)
  if (-not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path $Path)) {
    return Get-Content -Raw $Path | ConvertFrom-Json
  }
  return $null
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
  $status = (& git status --porcelain 2>$null | Out-String).Trim()
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
  if ($null -eq $manifest) {
    throw "Release manifest is missing."
  }
  if (-not $AllowUnsignedBeta -and ([bool]$manifest.unsigned_beta -or [bool]$manifest.publishable -eq $false)) {
    throw "Public release requires a publishable manifest. Re-run without unsigned beta labeling."
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

  $artifacts = @($manifest.staged_artifacts)
  $setupArtifacts = @($artifacts | Where-Object {
    [string]$_.source_kind -eq "bundle" -and @(".exe", ".msi") -contains [string]$_.extension
  })
  if ($setupArtifacts.Count -eq 0) {
    throw "Public release manifest contains no staged installer artifact."
  }

  if (-not $AllowUnsignedBeta) {
    $unsignedOrInvalid = @($setupArtifacts | Where-Object {
      $null -eq $_.code_signature -or [string]$_.code_signature.status -ne "Valid"
    })
    if ($unsignedOrInvalid.Count -gt 0) {
      $names = ($unsignedOrInvalid | ForEach-Object { [string]$_.staged_name }) -join ", "
      throw "Public release requires valid Authenticode signatures for installer artifacts. Invalid or unsigned: $names"
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
    post_build_sign_script = if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { $null } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) }
    error = if ([string]::IsNullOrWhiteSpace($ErrorMessage)) { $null } else { $ErrorMessage }
    candidate_summary = (Join-Path $resolvedOutDir "summary.json")
    manifest = (Join-Path $resolvedOutDir "manifest.json")
    hashes = (Join-Path $resolvedOutDir "SHA256SUMS.txt")
    packaged_smoke = (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
    artifact_source_kind = if ($null -ne $manifest) { [string]$manifest.artifact_source_kind } else { $null }
    manifest_publishable = if ($null -ne $manifest) { [bool]$manifest.publishable } else { $null }
    manifest_unsigned_beta = if ($null -ne $manifest) { [bool]$manifest.unsigned_beta } else { $null }
    smoke_mode = if ($null -ne $smoke) { [string]$smoke.smoke_mode } else { $null }
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
    "- Post-build signing hook: $(if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { 'env/default or none' } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) })"
  )
  if ($null -ne $manifest) {
    $lines += "- Manifest publishable: $([bool]$manifest.publishable)"
    $lines += "- Manifest unsigned beta: $([bool]$manifest.unsigned_beta)"
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
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null
$summaryJsonPath = Join-Path $resolvedOutDir "public-release-summary.json"
$summaryMdPath = Join-Path $resolvedOutDir "public-release-summary.md"
$script:StepResults = @()

try {
  if ($IncludeGpuValidationPerf -and $Variant -ne "gpu-kernel") {
    throw "-IncludeGpuValidationPerf requires -Variant gpu-kernel."
  }

  Invoke-PublicReleaseStep -Name "Clean worktree check" -CommandText "git status --porcelain" -Action {
    Assert-CleanWorktree
  }

  $candidateArgs = @(
    "-File", (Join-Path $repoRoot "scripts\\run_release_candidate_gate.ps1"),
    "-Variant", $Variant,
    "-OutDir", $resolvedOutDir,
    "-IncludeDesktopSmoke"
  )
  $candidateArgs += "-IncludeOutputParity"
  if ($IncludePerf) {
    $candidateArgs += "-IncludePerf"
  }
  if ($IncludeGpuValidationPerf) {
    $candidateArgs += "-IncludeGpuValidationPerf"
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

  Invoke-PublicReleaseStep -Name "Public release evidence check" -CommandText "manifest/signature/smoke assertions" -Action {
    Assert-PublicReleaseEvidence
  }

  Write-PublicReleaseSummary -Status "passed"
  Write-Host "Public release gate completed successfully."
  Write-Host "Summary written to $summaryJsonPath"
} catch {
  Write-PublicReleaseSummary -Status "failed" -ErrorMessage $_.Exception.Message
  throw
}
