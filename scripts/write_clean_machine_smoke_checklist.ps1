param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [string]$EvidenceDir = "",
  [string]$Tester = "",
  [string]$Machine = "",
  [string]$Notes = ""
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 10
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Resolve-OutputDir {
  if (-not [string]::IsNullOrWhiteSpace($OutDir)) {
    return [System.IO.Path]::GetFullPath($OutDir)
  }
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
  return Join-Path (Join-Path $repoRoot "output\clean_machine_smoke_checklists") "${stamp}-${Variant}"
}

function Test-File {
  param([string]$Path)
  return (-not [string]::IsNullOrWhiteSpace($Path)) -and (Test-Path -LiteralPath $Path -PathType Leaf)
}

function Test-Dir {
  param([string]$Path)
  return (-not [string]::IsNullOrWhiteSpace($Path)) -and (Test-Path -LiteralPath $Path -PathType Container)
}

function New-Precheck {
  param(
    [string]$Id,
    [string]$Title,
    [bool]$Passed,
    [string]$Detail
  )
  [ordered]@{
    id = $Id
    title = $Title
    status = $(if ($Passed) { "pass" } else { "manual_or_missing" })
    detail = $Detail
  }
}

function New-ManualStep {
  param(
    [string]$Id,
    [string]$Title,
    [string]$Acceptance,
    [string]$EvidenceHint
  )
  [ordered]@{
    id = $Id
    title = $Title
    status = "manual_pending"
    acceptance = $Acceptance
    evidence_hint = $EvidenceHint
  }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutputDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$resolvedEvidenceDir = ""
if (-not [string]::IsNullOrWhiteSpace($EvidenceDir)) {
  $resolvedEvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)
}

$manifestPath = if ($resolvedEvidenceDir) { Join-Path $resolvedEvidenceDir "manifest.json" } else { "" }
$publicSummaryPath = if ($resolvedEvidenceDir) { Join-Path $resolvedEvidenceDir "public-release-summary.json" } else { "" }
$candidateSummaryPath = if ($resolvedEvidenceDir) { Join-Path $resolvedEvidenceDir "summary.json" } else { "" }
$shaPath = if ($resolvedEvidenceDir) { Join-Path $resolvedEvidenceDir "SHA256SUMS.txt" } else { "" }
$artifactsDir = if ($resolvedEvidenceDir) { Join-Path $resolvedEvidenceDir "artifacts" } else { "" }

$manifest = $null
if (Test-File $manifestPath) {
  $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
}

$artifactCount = 0
if (Test-Dir $artifactsDir) {
  $artifactCount = @(
    Get-ChildItem -LiteralPath $artifactsDir -Recurse -File |
      Where-Object { @(".exe", ".msi", ".zip", ".sig") -contains $_.Extension.ToLowerInvariant() }
  ).Count
}

$sampleResourceTracked = $false
if ($manifest -and $manifest.required_resources) {
  $sampleResourceTracked = [bool]$manifest.required_resources.synthetic_sample_csv_exists
}

$prechecks = @(
  New-Precheck "evidence_dir" "Release evidence directory is available" (Test-Dir $resolvedEvidenceDir) $resolvedEvidenceDir
  New-Precheck "manifest" "Release manifest is available" (Test-File $manifestPath) $manifestPath
  New-Precheck "hashes" "SHA256SUMS.txt is available" (Test-File $shaPath) $shaPath
  New-Precheck "public_summary" "Public release summary is available" (Test-File $publicSummaryPath) $publicSummaryPath
  New-Precheck "candidate_summary" "Release candidate summary is available" (Test-File $candidateSummaryPath) $candidateSummaryPath
  New-Precheck "staged_artifacts" "Staged installer/update artifacts are available" ($artifactCount -gt 0) "$artifactCount artifact file(s) found under $artifactsDir"
  New-Precheck "synthetic_sample_manifest" "Manifest tracks bundled synthetic sample CSV" $sampleResourceTracked "manifest.required_resources.synthetic_sample_csv_exists=$sampleResourceTracked"
)

$manualSteps = @(
  New-ManualStep "download_public_artifact" "Download artifact from intended public channel" "Artifact downloads without browser or proxy warnings that differ from the expected unsigned/beta posture." "Record URL, filename, size, and timestamp."
  New-ManualStep "verify_hash" "Verify SHA-256" "Downloaded artifact hash matches SHA256SUMS.txt." "Attach command output or screenshot."
  New-ManualStep "inspect_signature" "Inspect Authenticode publisher and timestamp" "Publisher, certificate chain, and timestamp match release expectations; unsigned beta is explicitly labeled if used." "Attach signature details."
  New-ManualStep "install_clean_profile" "Install on clean Windows profile" "Installer completes without developer tools, repo files, or local build outputs present." "Record Windows version, install path, and any prompts."
  New-ManualStep "launch_shortcuts" "Launch from Start menu and desktop shortcut" "Both launch paths work when shortcuts are created by the installer." "Record launch result and shortcut names."
  New-ManualStep "advanced_workbench_entry" "Confirm first-run Advanced workbench" "App opens into the supported Advanced workbench with Research and Data Source immediately reachable." "Screenshot first viewport and record the initial workspace and workflow step."
  New-ManualStep "load_bundled_sample" "Load bundled synthetic sample" "Synthetic sample appears with metadata and loads without requiring repo data files." "Record sample label and loaded dataset summary."
  New-ManualStep "run_cpu_optimization" "Run a CPU-safe conventional optimization" "Optimization completes and the result identifies the dataset, strategy, candidate, metrics, and CPU route." "Attach the ranked-result and run-context screenshots."
  New-ManualStep "run_validation" "Run bounded conventional validation" "Validation completes or presents a clear evidence/review state without live-trading claims." "Attach the validation result and gate-summary screenshots."
  New-ManualStep "reload_saved_validation" "Reload the saved validation artifact" "The Library can find and load the saved validation artifact, and the matching validation result is rehydrated." "Record the artifact path and attach the rehydrated result screenshot."
  New-ManualStep "advanced_exports" "Export reports from the Advanced workbench" "JSON/CSV/XLSX export paths work and each generated file is readable." "Record exported artifact paths and file sizes."
  New-ManualStep "local_data_policy" "Verify local data directories and privacy posture" "App data is under expected app identifier; no credentials are bundled and public logs avoid private local paths." "Record app-local data path and inspected log names."
  New-ManualStep "uninstall" "Uninstall cleanly" "Application is removed cleanly; expected user data retention/removal behavior is understood and recorded." "Record uninstall result."
)

if ($Variant -ne "cpu") {
  $manualSteps += @(
    New-ManualStep "gpu_device_evidence" "Capture GPU route evidence" "Supported NVIDIA GPU/driver is detected and the variant-specific GPU route evidence matches release notes." "Attach GPU diagnostics and perf-gate references."
  )
}

$report = [ordered]@{
  schema_version = 1
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  variant = $Variant
  repo_root = $repoRoot
  evidence_dir = $resolvedEvidenceDir
  tester = $Tester
  machine = $Machine
  notes = $Notes
  prechecks = $prechecks
  manual_steps = $manualSteps
}

$jsonPath = Join-Path $resolvedOutDir "clean-machine-smoke-checklist.json"
$mdPath = Join-Path $resolvedOutDir "clean-machine-smoke-checklist.md"
Write-JsonUtf8 -Path $jsonPath -Value $report -Depth 12

$md = New-Object System.Collections.Generic.List[string]
$md.Add("# VectorGrid Clean-Machine Smoke Checklist")
$md.Add("")
$md.Add("- Generated UTC: $($report.generated_at_utc)")
$md.Add("- Variant: $Variant")
$md.Add("- Evidence dir: $(if ($resolvedEvidenceDir) { $resolvedEvidenceDir } else { 'not provided' })")
$md.Add("- Tester: $(if ($Tester) { $Tester } else { 'manual entry required' })")
$md.Add("- Machine: $(if ($Machine) { $Machine } else { 'manual entry required' })")
if ($Notes) {
  $md.Add("- Notes: $Notes")
}
$md.Add("")
$md.Add("## Automated Prechecks")
foreach ($item in $prechecks) {
  $md.Add("- [$($item.status)] $($item.title): $($item.detail)")
}
$md.Add("")
$md.Add("## Manual Clean-Machine Steps")
foreach ($item in $manualSteps) {
  $md.Add("- [ ] $($item.title)")
  $md.Add("  - Status: $($item.status)")
  $md.Add("  - Acceptance: $($item.acceptance)")
  $md.Add("  - Evidence: $($item.evidence_hint)")
}
$md.Add("")
$md.Add("## Result")
$md.Add("")
$md.Add("- Overall status: manual_pending")
$md.Add("- Reviewer sign-off:")
$md.Add("- Date:")
$md.Add("")
$md -join "`n" | Set-Content -Encoding UTF8 $mdPath

Write-Host "Clean-machine smoke checklist written:"
Write-Host "  $jsonPath"
Write-Host "  $mdPath"
