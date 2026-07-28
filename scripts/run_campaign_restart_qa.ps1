param(
  [string]$OutputRoot = "output/qa/campaign-restart",
  [string]$SourceDataset = "src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv",
  [switch]$KeepIsolatedAppData
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Resolve-RepoPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )
  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function Invoke-Harness {
  param(
    [Parameter(Mandatory = $true)]
    [string[]]$Arguments
  )
  & $script:HarnessBinary @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "ta_campaign_restart_qa failed with exit code $LASTEXITCODE"
  }
}

$outputRootPath = Resolve-RepoPath $OutputRoot
$sourceDatasetPath = Resolve-RepoPath $SourceDataset
if (-not (Test-Path -LiteralPath $sourceDatasetPath -PathType Leaf)) {
  throw "Campaign restart QA source dataset does not exist: $sourceDatasetPath"
}

$timestamp = [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssZ")
$nonce = [Guid]::NewGuid().ToString("N")
$runId = "qa$($nonce.Substring(0, 20))"
$runDir = Join-Path $outputRootPath "$timestamp-$runId"
[System.IO.Directory]::CreateDirectory($runDir) | Out-Null
$script:HarnessBinary = $null

Push-Location $repoRoot
try {
  & cargo build -p ta_desktop_demo_app --bin ta_campaign_restart_qa --features maintainer-tools -j 1
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to build ta_campaign_restart_qa (exit $LASTEXITCODE)"
  }

  $metadata = (& cargo metadata --no-deps --format-version 1 | ConvertFrom-Json)
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to resolve the Cargo target directory"
  }
  $script:HarnessBinary = Join-Path ([string]$metadata.target_directory) "debug/ta_campaign_restart_qa.exe"
  if (-not (Test-Path -LiteralPath $script:HarnessBinary -PathType Leaf)) {
    throw "Campaign restart QA binary was not produced: $script:HarnessBinary"
  }

  $prepareJson = Join-Path $runDir "prepare.json"
  Invoke-Harness -Arguments @(
    "--run-root", $runDir,
    "--run-id", $runId,
    "prepare",
    "--source-dataset", $sourceDatasetPath,
    "--output-json", $prepareJson
  )
  $prepare = Get-Content -LiteralPath $prepareJson -Raw | ConvertFrom-Json
  $datasets = @($prepare.datasets | ForEach-Object { [string]$_.path })
  if ($datasets.Count -ne 3) {
    throw "Prepare phase produced $($datasets.Count) datasets; expected 3"
  }
  $partialFailureDatasets = @($prepare.partial_failure_datasets | ForEach-Object { [string]$_.path })
  if ($partialFailureDatasets.Count -ne 3) {
    throw "Prepare phase produced $($partialFailureDatasets.Count) partial-failure datasets; expected 3"
  }
  $largeCatalogDatasets = @($prepare.large_catalog_datasets | ForEach-Object { [string]$_.path })
  if ($largeCatalogDatasets.Count -ne 12) {
    throw "Prepare phase produced $($largeCatalogDatasets.Count) large-catalog datasets; expected 12"
  }

  $phaseFiles = [ordered]@{}
  foreach ($phase in @("interrupted", "resume", "noop")) {
    $phaseJson = Join-Path $runDir "phase-$phase.json"
    $arguments = @(
      "--run-root", $runDir,
      "--run-id", $runId,
      "run",
      "--phase", $phase,
      "--output-json", $phaseJson
    )
    foreach ($dataset in $datasets) {
      $arguments += @("--dataset", $dataset)
    }
    Invoke-Harness -Arguments $arguments
    $phaseFiles[$phase] = $phaseJson
  }

  $probeJson = Join-Path $runDir "corruption-probe.json"
  Invoke-Harness -Arguments @(
    "--run-root", $runDir,
    "--run-id", $runId,
    "probe-corrupt",
    "--phase-json", $phaseFiles["noop"],
    "--output-json", $probeJson
  )

  $mutationJson = Join-Path $runDir "mutation.json"
  Invoke-Harness -Arguments @(
    "--run-root", $runDir,
    "--run-id", $runId,
    "mutate",
    "--dataset", $datasets[0],
    "--output-json", $mutationJson
  )

  foreach ($phase in @("stale_resume", "stale_noop")) {
    $phaseJson = Join-Path $runDir "phase-$phase.json"
    $arguments = @(
      "--run-root", $runDir,
      "--run-id", $runId,
      "run",
      "--phase", $phase,
      "--output-json", $phaseJson
    )
    foreach ($dataset in $datasets) {
      $arguments += @("--dataset", $dataset)
    }
    Invoke-Harness -Arguments $arguments
    $phaseFiles[$phase] = $phaseJson
  }

  $partialFailureJson = Join-Path $runDir "phase-partial_failure.json"
  $partialFailureArguments = @(
    "--run-root", $runDir,
    "--run-id", $runId,
    "run",
    "--phase", "partial_failure",
    "--output-json", $partialFailureJson
  )
  foreach ($dataset in $partialFailureDatasets) {
    $partialFailureArguments += @("--dataset", $dataset)
  }
  Invoke-Harness -Arguments $partialFailureArguments
  $phaseFiles["partial_failure"] = $partialFailureJson

  $repairJson = Join-Path $runDir "repair.json"
  Invoke-Harness -Arguments @(
    "--run-root", $runDir,
    "--run-id", $runId,
    "repair",
    "--dataset", $partialFailureDatasets[1],
    "--source-dataset", $datasets[0],
    "--output-json", $repairJson
  )

  $partialRetryJson = Join-Path $runDir "phase-partial_retry.json"
  $partialRetryArguments = @(
    "--run-root", $runDir,
    "--run-id", $runId,
    "run",
    "--phase", "partial_retry",
    "--output-json", $partialRetryJson
  )
  foreach ($dataset in $partialFailureDatasets) {
    $partialRetryArguments += @("--dataset", $dataset)
  }
  Invoke-Harness -Arguments $partialRetryArguments
  $phaseFiles["partial_retry"] = $partialRetryJson

  foreach ($phase in @("large_catalog", "large_noop")) {
    $phaseJson = Join-Path $runDir "phase-$phase.json"
    $arguments = @(
      "--run-root", $runDir,
      "--run-id", $runId,
      "run",
      "--phase", $phase,
      "--output-json", $phaseJson
    )
    foreach ($dataset in $largeCatalogDatasets) {
      $arguments += @("--dataset", $dataset)
    }
    Invoke-Harness -Arguments $arguments
    $phaseFiles[$phase] = $phaseJson
  }

  $verificationJson = Join-Path $runDir "verification-summary.json"
  $verifyArguments = @(
    "--run-root", $runDir,
    "--run-id", $runId,
    "verify",
    "--probe-json", $probeJson,
    "--mutation-json", $mutationJson,
    "--repair-json", $repairJson,
    "--output-json", $verificationJson
  )
  foreach ($phaseJson in $phaseFiles.Values) {
    $verifyArguments += @("--phase-json", [string]$phaseJson)
  }
  Invoke-Harness -Arguments $verifyArguments

  $verification = Get-Content -LiteralPath $verificationJson -Raw | ConvertFrom-Json
  if ($verification.passed -ne $true) {
    throw "Campaign restart QA verification did not report passed=true"
  }

  Write-Host "Campaign restart QA passed."
  Write-Host "Evidence: $verificationJson"
}
finally {
  if (-not $KeepIsolatedAppData -and $null -ne $script:HarnessBinary -and (Test-Path -LiteralPath $script:HarnessBinary -PathType Leaf)) {
    $cleanupJson = Join-Path $runDir "cleanup.json"
    try {
      Invoke-Harness -Arguments @(
        "--run-root", $runDir,
        "--run-id", $runId,
        "cleanup",
        "--output-json", $cleanupJson
      )
      $cleanup = Get-Content -LiteralPath $cleanupJson -Raw | ConvertFrom-Json
      if ($cleanup.passed -ne $true) {
        throw "Campaign restart QA cleanup did not report passed=true"
      }
    }
    catch {
      Write-Error "Campaign restart QA cleanup failed: $($_.Exception.Message)"
    }
  }
  Pop-Location
}
