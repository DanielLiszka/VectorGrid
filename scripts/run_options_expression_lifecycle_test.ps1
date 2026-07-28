param(
  [string]$AppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$SeedArtifact,
  [string]$OutDir = "output/qa/options_expression_lifecycle"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Resolve-RepoRoot {
  return (Split-Path -Parent $PSScriptRoot)
}

function Resolve-RepoPath {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$RepoRoot
  )

  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $Path))
}

function Read-JsonFile {
  param([Parameter(Mandatory = $true)][string]$Path)
  return (Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json)
}

function Write-JsonFile {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value
  )

  $json = $Value | ConvertTo-Json -Depth 100
  [System.IO.File]::WriteAllText($Path, $json, [System.Text.UTF8Encoding]::new($false))
}

function Find-UsableSeedArtifact {
  param([Parameter(Mandatory = $true)][string]$SourceRoot)

  $runsDir = Join-Path $SourceRoot "runs"
  if (!(Test-Path -LiteralPath $runsDir -PathType Container)) {
    throw "Options-expression lifecycle source has no runs directory: $runsDir"
  }

  $candidates = Get-ChildItem -LiteralPath $runsDir -Filter "vectorgrid_options_expression_*.json" -File |
    Sort-Object LastWriteTimeUtc -Descending
  foreach ($candidate in $candidates) {
    try {
      $artifact = Read-JsonFile -Path $candidate.FullName
      $parentPath = [string]$artifact.request.parent_artifact_path
      $datasetId = [string]$artifact.request.options_dataset_id
      $datasetDir = Join-Path (Join-Path $SourceRoot "options_research\datasets") $datasetId
      if (
        $parentPath -and
        $datasetId -and
        (Test-Path -LiteralPath $parentPath -PathType Leaf) -and
        (Test-Path -LiteralPath (Join-Path $datasetDir "manifest.json") -PathType Leaf) -and
        (Test-Path -LiteralPath (Join-Path $datasetDir "rows.csv") -PathType Leaf)
      ) {
        $parent = Read-JsonFile -Path $parentPath
        $sourceCsv = [string]$parent.data.csv_path
        if ($sourceCsv -and (Test-Path -LiteralPath $sourceCsv -PathType Leaf)) {
          return $candidate.FullName
        }
      }
    } catch {
      continue
    }
  }
  throw "No saved options-expression artifact has a readable parent artifact, parent CSV, and imported dataset under $SourceRoot"
}

function Build-LifecycleHarness {
  param(
    [Parameter(Mandatory = $true)][string]$RepoRoot
  )

  Push-Location $RepoRoot
  $previousRustcWrapper = $env:RUSTC_WRAPPER
  $previousCargoIncremental = $env:CARGO_INCREMENTAL
  $env:RUSTC_WRAPPER = ""
  $env:CARGO_INCREMENTAL = "0"
  try {
    & cargo build -q -p ta_desktop_demo_app --no-default-features --features maintainer-tools --bin ta_options_expression_lifecycle
    if ($LASTEXITCODE -ne 0) {
      throw "Failed to build the options-expression lifecycle harness (exit code $LASTEXITCODE)"
    }
    $metadataText = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) {
      throw "Failed to resolve the Cargo target directory (exit code $LASTEXITCODE)"
    }
  } finally {
    $env:RUSTC_WRAPPER = $previousRustcWrapper
    $env:CARGO_INCREMENTAL = $previousCargoIncremental
    Pop-Location
  }

  $metadata = $metadataText | ConvertFrom-Json
  $exeName = if ($env:OS -eq "Windows_NT") {
    "ta_options_expression_lifecycle.exe"
  } else {
    "ta_options_expression_lifecycle"
  }
  $binaryPath = Join-Path (Join-Path ([string]$metadata.target_directory) "debug") $exeName
  if (!(Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
    throw "Built lifecycle harness was not found: $binaryPath"
  }
  return $binaryPath
}

function Invoke-LifecyclePhase {
  param(
    [Parameter(Mandatory = $true)][string]$BinaryPath,
    [Parameter(Mandatory = $true)][string]$Phase,
    [Parameter(Mandatory = $true)][string]$WorkingRoot,
    [Parameter(Mandatory = $true)][string]$Artifact,
    [Parameter(Mandatory = $true)][string]$ExportCsv,
    [Parameter(Mandatory = $true)][string]$SummaryJson
  )

  $args = @(
    "--app-local-data-dir",
    $WorkingRoot,
    "--artifact",
    $Artifact,
    "--phase",
    $Phase,
    "--export-csv",
    $ExportCsv,
    "--out-json",
    $SummaryJson
  )

  & $BinaryPath @args
  if ($LASTEXITCODE -ne 0) {
    throw "Options-expression lifecycle '$Phase' phase failed with exit code $LASTEXITCODE"
  }
}

$repoRoot = Resolve-RepoRoot
$sourceRoot = Resolve-RepoPath -Path $AppLocalDataDir -RepoRoot $repoRoot
$resolvedOutDir = Resolve-RepoPath -Path $OutDir -RepoRoot $repoRoot
if (!(Test-Path -LiteralPath $sourceRoot -PathType Container)) {
  throw "Options-expression lifecycle source root does not exist: $sourceRoot"
}

if ([string]::IsNullOrWhiteSpace($SeedArtifact)) {
  $SeedArtifact = Find-UsableSeedArtifact -SourceRoot $sourceRoot
} else {
  $SeedArtifact = Resolve-RepoPath -Path $SeedArtifact -RepoRoot $repoRoot
}
if (!(Test-Path -LiteralPath $SeedArtifact -PathType Leaf)) {
  throw "Options-expression seed artifact does not exist: $SeedArtifact"
}

$runId = "{0}_{1}" -f (Get-Date -Format "yyyy-MM-ddTHH-mm-ss"), ([guid]::NewGuid().ToString("N").Substring(0, 8))
$runDir = Join-Path $resolvedOutDir $runId
$workingRoot = Join-Path $runDir "isolated_app_local_data"
$workingRunsDir = Join-Path $workingRoot "runs"
$workingDatasetsRoot = Join-Path $workingRoot "options_research\datasets"
$workingFixtureDir = Join-Path $workingRoot "lifecycle_fixture"
[System.IO.Directory]::CreateDirectory($workingRunsDir) | Out-Null
[System.IO.Directory]::CreateDirectory($workingDatasetsRoot) | Out-Null
[System.IO.Directory]::CreateDirectory($workingFixtureDir) | Out-Null

$seed = Read-JsonFile -Path $SeedArtifact
$sourceParentPath = [string]$seed.request.parent_artifact_path
if (!(Test-Path -LiteralPath $sourceParentPath -PathType Leaf)) {
  throw "Seed parent artifact does not exist: $sourceParentPath"
}
$sourceParent = Read-JsonFile -Path $sourceParentPath
$sourceCsvPath = [string]$sourceParent.data.csv_path
if (!(Test-Path -LiteralPath $sourceCsvPath -PathType Leaf)) {
  throw "Seed parent CSV does not exist: $sourceCsvPath"
}
$datasetId = [string]$seed.request.options_dataset_id
$sourceDatasetDir = Join-Path (Join-Path $sourceRoot "options_research\datasets") $datasetId
if (!(Test-Path -LiteralPath $sourceDatasetDir -PathType Container)) {
  throw "Seed options dataset does not exist: $sourceDatasetDir"
}

$workingParentPath = Join-Path $workingRunsDir ([System.IO.Path]::GetFileName($sourceParentPath))
$workingSeedPath = Join-Path $workingRunsDir ([System.IO.Path]::GetFileName($SeedArtifact))
$workingCsvPath = Join-Path $workingFixtureDir ([System.IO.Path]::GetFileName($sourceCsvPath))
$workingDatasetDir = Join-Path $workingDatasetsRoot $datasetId

Copy-Item -LiteralPath $sourceCsvPath -Destination $workingCsvPath
Copy-Item -LiteralPath $sourceDatasetDir -Destination $workingDatasetDir -Recurse

$sourceParent.data.csv_path = $workingCsvPath
if ($null -ne $sourceParent.dataset_meta -and $null -ne $sourceParent.dataset_meta.csv_path) {
  $sourceParent.dataset_meta.csv_path = $workingCsvPath
}
Write-JsonFile -Path $workingParentPath -Value $sourceParent

$seed.request.parent_artifact_path = $workingParentPath
$seed.artifact_path = $workingSeedPath
Write-JsonFile -Path $workingSeedPath -Value $seed

$lifecycleBinary = Build-LifecycleHarness -RepoRoot $repoRoot
$coldJson = Join-Path $runDir "cold-run-summary.json"
$coldCsv = Join-Path $runDir "cold-run-export.csv"
Invoke-LifecyclePhase `
  -BinaryPath $lifecycleBinary `
  -Phase "run" `
  -WorkingRoot $workingRoot `
  -Artifact $workingSeedPath `
  -ExportCsv $coldCsv `
  -SummaryJson $coldJson

$cold = Read-JsonFile -Path $coldJson
if ([string]$cold.status -ne "passed") {
  throw "Cold options-expression lifecycle phase did not pass"
}
if (
  [bool]$cold.replay_build_timing.parent_signal_process_cache_hit -or
  [bool]$cold.replay_build_timing.persisted_signal_manifest_cache_hit -or
  [bool]$cold.replay_build_timing.options_dataset_process_cache_hit -or
  [bool]$cold.replay_build_timing.options_dataset_persisted_cache_hit
) {
  throw "Cold lifecycle phase unexpectedly reused a replay cache inside the isolated fixture"
}

$recoveryJson = Join-Path $runDir "cold-recovery-summary.json"
$recoveryCsv = Join-Path $runDir "cold-recovery-export.csv"
Invoke-LifecyclePhase `
  -BinaryPath $lifecycleBinary `
  -Phase "recover" `
  -WorkingRoot $workingRoot `
  -Artifact ([string]$cold.active_artifact_path) `
  -ExportCsv $recoveryCsv `
  -SummaryJson $recoveryJson

$recovery = Read-JsonFile -Path $recoveryJson
if ([string]$recovery.status -ne "passed") {
  throw "Cold-recovery options-expression lifecycle phase did not pass"
}
if ([bool]$recovery.replay_build_timing.parent_signal_process_cache_hit) {
  throw "Recovery phase unexpectedly inherited an in-process parent-signal cache across processes"
}
if ([bool]$recovery.replay_build_timing.options_dataset_process_cache_hit) {
  throw "Recovery phase unexpectedly inherited an in-process options-dataset cache across processes"
}
if (![bool]$recovery.replay_build_timing.persisted_signal_manifest_cache_hit) {
  throw "Recovery phase did not reconstruct the parent signal manifest from the persisted cache"
}
if (![bool]$recovery.replay_build_timing.options_dataset_persisted_cache_hit) {
  throw "Recovery phase did not reconstruct the imported options dataset from the persisted cache"
}

$combined = [ordered]@{
  status = "passed"
  test = "options-expression-headless-lifecycle"
  terminal_only = $true
  computer_use_used = $false
  performance_evidence = $false
  source_app_local_data_dir = $sourceRoot
  source_seed_artifact = $SeedArtifact
  isolated_app_local_data_dir = $workingRoot
  cold_run = $cold
  cold_recovery = $recovery
}
$combinedJson = Join-Path $runDir "lifecycle-summary.json"
Write-JsonFile -Path $combinedJson -Value $combined

Write-Host ""
Write-Host "Options-expression headless lifecycle: PASS"
Write-Host "  cold run:       $coldJson"
Write-Host "  cold recovery:  $recoveryJson"
Write-Host "  combined:       $combinedJson"
