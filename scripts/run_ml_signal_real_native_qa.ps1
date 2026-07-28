param(
  [string]$SourceCsv = '',
  [string]$OutputDirectory = '',
  [string]$VenvPath = '',
  [ValidateRange(3200, 10000)]
  [int]$MaxRows = 4000,
  [ValidateRange(20, 200)]
  [int]$Iterations = 60,
  [ValidateRange(1, 8192)]
  [int]$PredictionBatchSize = 512,
  [string]$DataId = 'ml_signal_real_native_qa',
  [switch]$SkipEnvironmentSetup,
  [switch]$ReplaceOwnedOutput,
  [switch]$KeepFailureFixtures,
  [switch]$ContractOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:ContractVersion = 1
$script:OwnerKind = 'vectorgrid_ml_signal_real_native_qa'
$script:FailureOwnerKind = 'vectorgrid_ml_signal_real_native_qa_failure_fixtures'
$script:RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$script:OutputRoot = [System.IO.Path]::GetFullPath((Join-Path $script:RepoRoot 'output'))
$script:ResolvedSourceCsv = $null

function ConvertTo-CanonicalFullPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$BasePath
  )

  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $BasePath $Path))
}

function Test-PathContainedBy {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Root
  )

  $resolvedPath = [System.IO.Path]::GetFullPath($Path)
  $resolvedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
  $prefix = $resolvedRoot + [System.IO.Path]::DirectorySeparatorChar
  return (
    $resolvedPath.Equals($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
    $resolvedPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)
  )
}

function Assert-ContainedOutputPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  $resolved = [System.IO.Path]::GetFullPath($Path)
  if (-not (Test-PathContainedBy -Path $resolved -Root $script:OutputRoot)) {
    throw "$Label must remain under the repository output directory."
  }
  if ($resolved.Equals($script:OutputRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "$Label cannot be the output directory itself."
  }
  return $resolved
}

function Get-StringSha256 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Value
  )

  $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
  } finally {
    $sha.Dispose()
  }
}

function Get-PathOwnerHash {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $canonical = [System.IO.Path]::GetFullPath($Path).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  ).ToLowerInvariant()
  return Get-StringSha256 -Value $canonical
}

function Write-JsonFile {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [object]$Value
  )

  $parent = Split-Path -Parent $Path
  if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  $json = $Value | ConvertTo-Json -Depth 30
  $tempPath = "$Path.tmp-$([Guid]::NewGuid().ToString('N'))"
  [System.IO.File]::WriteAllText(
    $tempPath,
    $json + [Environment]::NewLine,
    [System.Text.UTF8Encoding]::new($false)
  )
  Move-Item -LiteralPath $tempPath -Destination $Path -Force
}

function New-OwnedDirectory {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Kind
  )

  New-Item -ItemType Directory -Force -Path $Path | Out-Null
  Write-JsonFile -Path (Join-Path $Path '.qa-owner.json') -Value ([ordered]@{
    schema_version = 1
    kind = $Kind
    root_sha256 = Get-PathOwnerHash -Path $Path
  })
}

function Assert-OwnedDirectory {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Kind
  )

  $resolved = Assert-ContainedOutputPath -Path $Path -Label 'owned QA directory'
  $sentinelPath = Join-Path $resolved '.qa-owner.json'
  if (-not (Test-Path -LiteralPath $sentinelPath -PathType Leaf)) {
    throw "Refusing to operate on '$resolved': the QA ownership sentinel is missing."
  }
  $sentinel = Get-Content -Raw -LiteralPath $sentinelPath | ConvertFrom-Json
  if (
    [int]$sentinel.schema_version -ne 1 -or
    [string]$sentinel.kind -ne $Kind -or
    [string]$sentinel.root_sha256 -ne (Get-PathOwnerHash -Path $resolved)
  ) {
    throw "Refusing to operate on '$resolved': the QA ownership sentinel does not match."
  }
  return $resolved
}

function Remove-OwnedDirectory {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Kind
  )

  $resolved = Assert-OwnedDirectory -Path $Path -Kind $Kind
  Remove-Item -LiteralPath $resolved -Recurse -Force
}

function Get-RepoRelativePath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $resolved = [System.IO.Path]::GetFullPath($Path)
  if (-not (Test-PathContainedBy -Path $resolved -Root $script:RepoRoot)) {
    throw "Generated QA path is unexpectedly outside the repository."
  }
  $rootUri = [Uri]::new($script:RepoRoot.TrimEnd('\') + '\')
  $pathUri = [Uri]::new($resolved)
  return [Uri]::UnescapeDataString($rootUri.MakeRelativeUri($pathUri).ToString())
}

function Get-FileSha256Lower {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Protect-PrivatePaths {
  param(
    [AllowEmptyString()]
    [string]$Text
  )

  $protected = [string]$Text
  $replacements = [ordered]@{}
  if ($script:ResolvedSourceCsv) {
    $replacements[[string]$script:ResolvedSourceCsv] = '<source_csv>'
  }
  $replacements[[string]$script:RepoRoot] = '<repo>'
  if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
    $replacements[[string]$env:USERPROFILE] = '<user_profile>'
  }
  foreach ($entry in $replacements.GetEnumerator()) {
    $rawPath = [string]$entry.Key
    $variants = @(
      $rawPath.Replace('\', '\\'),
      $rawPath.Replace('\', '/'),
      $rawPath
    ) | Sort-Object Length -Descending -Unique
    foreach ($variant in $variants) {
      $protected = [Regex]::Replace(
        $protected,
        [Regex]::Escape($variant),
        [string]$entry.Value,
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
      )
    }
  }
  return $protected
}

function Test-ContainsPrivatePath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Text,
    [Parameter(Mandatory = $true)]
    [string]$PrivatePath
  )

  foreach ($variant in @(
    $PrivatePath,
    $PrivatePath.Replace('\', '/'),
    $PrivatePath.Replace('\', '\\')
  )) {
    if ($Text.IndexOf($variant, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
      return $true
    }
  }
  return $false
}

function Invoke-CapturedNative {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,
    [Parameter(Mandatory = $true)]
    [string[]]$ArgumentList,
    [Parameter(Mandatory = $true)]
    [string]$LogPath,
    [switch]$ExpectFailure
  )

  $previousErrorActionPreference = $ErrorActionPreference
  try {
    $ErrorActionPreference = 'Continue'
    $lines = @(& $FilePath @ArgumentList 2>&1 | ForEach-Object { $_.ToString() })
    $exitCode = $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previousErrorActionPreference
  }
  $rendered = Protect-PrivatePaths -Text ($lines -join [Environment]::NewLine)
  [System.IO.File]::WriteAllText(
    $LogPath,
    $rendered + [Environment]::NewLine,
    [System.Text.UTF8Encoding]::new($false)
  )
  if ($ExpectFailure) {
    if ($exitCode -eq 0) {
      throw "Expected '$FilePath' to reject the negative QA case, but it exited successfully."
    }
  } elseif ($exitCode -ne 0) {
    $tail = if ($rendered.Length -gt 1200) {
      $rendered.Substring($rendered.Length - 1200)
    } else {
      $rendered
    }
    throw "'$FilePath' exited with code $exitCode.`n$tail"
  }
  return [pscustomobject]@{
    exit_code = $exitCode
    output = $rendered
  }
}

function Copy-BoundedCsv {
  param(
    [Parameter(Mandatory = $true)]
    [string]$InputPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [Parameter(Mandatory = $true)]
    [int]$RowLimit
  )

  $reader = [System.IO.StreamReader]::new($InputPath, $true)
  $writer = [System.IO.StreamWriter]::new(
    $OutputPath,
    $false,
    [System.Text.UTF8Encoding]::new($false)
  )
  try {
    $header = $reader.ReadLine()
    if ([string]::IsNullOrWhiteSpace($header)) {
      throw 'SourceCsv does not contain a non-empty CSV header.'
    }
    $writer.WriteLine($header)
    $rows = 0
    while ($rows -lt $RowLimit -and -not $reader.EndOfStream) {
      $line = $reader.ReadLine()
      if ($null -eq $line) {
        break
      }
      $writer.WriteLine($line)
      $rows++
    }
  } finally {
    $writer.Dispose()
    $reader.Dispose()
  }
  if ($rows -lt 3200) {
    throw "SourceCsv supplied only $rows data rows; at least 3200 are required for bounded walk-forward QA."
  }
  return $rows
}

function Get-ErrorExcerpt {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Text
  )

  $singleLine = (($Text -split '\r?\n') | Where-Object {
    -not [string]::IsNullOrWhiteSpace($_)
  } | Select-Object -Last 6) -join ' | '
  if ($singleLine.Length -gt 900) {
    return $singleLine.Substring($singleLine.Length - 900)
  }
  return $singleLine
}

function Get-Contract {
  return [ordered]@{
    contract_version = $script:ContractVersion
    kind = $script:OwnerKind
    opt_in = $true
    default_max_rows = 4000
    max_rows_upper_bound = 10000
    qa_artifact_scope = 'repository_output_only'
    toolchain_cache_scope = 'normal_repository_target_and_output_venv'
    source_mutation = 'forbidden'
    app_profile_access = 'none'
    backend = 'native_cpu'
    reviewed_catboost_baseline = 'scripts/baselines/catboost-native-v1.2.10.json'
    required_success_checks = @(
      'walk_forward_export',
      'deterministic_long_short_training',
      'native_cpu_inference',
      'release_strict_validation',
      'bounded_policy_sweep'
    )
    required_negative_checks = @(
      'missing_native_library',
      'corrupt_native_library',
      'missing_model',
      'corrupt_model'
    )
    evidence_path_policy = 'repo_relative_or_redacted'
    cleanup_policy = 'sentinel_owned_directories_only'
    approval_boundary = 'functional_inference_only_not_model_quality_or_trading_approval'
  }
}

if ($ContractOnly) {
  Get-Contract | ConvertTo-Json -Depth 10
  exit 0
}

if ([string]::IsNullOrWhiteSpace($SourceCsv)) {
  throw 'SourceCsv is required unless -ContractOnly is used.'
}
if ([string]::IsNullOrWhiteSpace($DataId) -or $DataId -match '[\\/:*?"<>|]') {
  throw 'DataId must be non-empty and cannot contain filesystem-reserved characters.'
}

$script:ResolvedSourceCsv = ConvertTo-CanonicalFullPath -Path $SourceCsv -BasePath (Get-Location).Path
if (-not (Test-Path -LiteralPath $script:ResolvedSourceCsv -PathType Leaf)) {
  throw 'SourceCsv does not exist or is not a file.'
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
  $runStamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
  $OutputDirectory = Join-Path $script:OutputRoot "qa\ml-signal-real-native\run-$runStamp-$PID"
} else {
  $OutputDirectory = ConvertTo-CanonicalFullPath -Path $OutputDirectory -BasePath $script:RepoRoot
}
$runRoot = Assert-ContainedOutputPath -Path $OutputDirectory -Label 'OutputDirectory'

if (Test-Path -LiteralPath $runRoot) {
  if (-not $ReplaceOwnedOutput) {
    throw "OutputDirectory already exists. Use a fresh directory or -ReplaceOwnedOutput for sentinel-owned QA output."
  }
  Remove-OwnedDirectory -Path $runRoot -Kind $script:OwnerKind
}
New-OwnedDirectory -Path $runRoot -Kind $script:OwnerKind

$logsRoot = Join-Path $runRoot 'logs'
$bundleRoot = Join-Path $runRoot 'bundle'
$failureRoot = Join-Path $runRoot 'failure-fixtures'
New-Item -ItemType Directory -Force -Path $logsRoot | Out-Null
New-OwnedDirectory -Path $failureRoot -Kind $script:FailureOwnerKind

$subsetCsv = Join-Path $runRoot 'input-subset.csv'
$datasetCsv = Join-Path $runRoot 'ml-signal-dataset.csv'
$splitPlanPath = Join-Path $runRoot 'split-plan.json'
$trainingConfigPath = Join-Path $runRoot 'training-config.json'
$scoreCachePath = Join-Path $runRoot 'score-cache.json'
$nativeSmokePath = Join-Path $runRoot 'native-smoke.json'
$summaryPath = Join-Path $runRoot 'summary.json'
$nativeLibraryPath = Join-Path $script:RepoRoot 'src-tauri\resources\native\catboostmodel.dll'
$nativeBaselinePath = Join-Path $script:RepoRoot 'scripts\baselines\catboost-native-v1.2.10.json'
$requirementsPath = Join-Path $script:RepoRoot 'scripts\requirements-ml-signal-training.lock.txt'

$resultStatus = 'failed'
Push-Location $script:RepoRoot
try {
  $sourceIdentityBefore = [ordered]@{
    size_bytes = [int64](Get-Item -LiteralPath $script:ResolvedSourceCsv).Length
    sha256 = Get-FileSha256Lower -Path $script:ResolvedSourceCsv
  }
  $subsetRows = Copy-BoundedCsv `
    -InputPath $script:ResolvedSourceCsv `
    -OutputPath $subsetCsv `
    -RowLimit $MaxRows
  $sourceIdentityAfter = [ordered]@{
    size_bytes = [int64](Get-Item -LiteralPath $script:ResolvedSourceCsv).Length
    sha256 = Get-FileSha256Lower -Path $script:ResolvedSourceCsv
  }
  if (
    $sourceIdentityBefore.size_bytes -ne $sourceIdentityAfter.size_bytes -or
    $sourceIdentityBefore.sha256 -ne $sourceIdentityAfter.sha256
  ) {
    throw 'SourceCsv changed while the QA subset was being created; refusing ambiguous evidence.'
  }

  Write-JsonFile -Path $splitPlanPath -Value ([ordered]@{
    version = 1
    train_bars = 1600
    calibration_bars = 500
    test_bars = 500
    gap_bars = 10
    step_bars = 500
    windows = @()
  })

  $defaultTrainingConfigPath = Join-Path $script:RepoRoot 'configs\ml_signal_catboost_default.json'
  $trainingConfig = Get-Content -Raw -LiteralPath $defaultTrainingConfigPath | ConvertFrom-Json
  $trainingConfig.seed = 20260424
  $trainingConfig.dataset.data_id = $DataId
  $trainingConfig.dataset.symbol_scope = 'qa_single_dataset'
  $trainingConfig.catboost.params.iterations = $Iterations
  $trainingConfig.catboost.params.thread_count = 1
  $trainingConfig.catboost.params.bootstrap_type = 'No'
  $trainingConfig.catboost.params.random_strength = 0.0
  $trainingConfig.catboost.params.allow_writing_files = $false
  $trainingConfig.catboost.early_stopping_rounds = [Math]::Min(
    20,
    [Math]::Max(5, [int][Math]::Floor($Iterations / 4))
  )
  $trainingConfig.metrics.thresholds = @(0.5, 0.6)
  Write-JsonFile -Path $trainingConfigPath -Value $trainingConfig

  $verifyNative = Invoke-CapturedNative `
    -FilePath 'powershell' `
    -ArgumentList @(
      '-NoProfile',
      '-File', 'scripts/acquire_catboost_native.ps1',
      '-VerifyOnly'
    ) `
    -LogPath (Join-Path $logsRoot 'catboost-native-verification.log')
  if (-not (Test-Path -LiteralPath $nativeLibraryPath -PathType Leaf)) {
    throw 'Reviewed CatBoost native library verification passed without the expected DLL.'
  }

  if ([string]::IsNullOrWhiteSpace($VenvPath)) {
    $VenvPath = Join-Path $script:OutputRoot 'tools\ml-signal-training-venv'
  } else {
    $VenvPath = ConvertTo-CanonicalFullPath -Path $VenvPath -BasePath $script:RepoRoot
  }
  $VenvPath = Assert-ContainedOutputPath -Path $VenvPath -Label 'VenvPath'
  $venvPython = Join-Path $VenvPath 'Scripts\python.exe'
  if (-not $SkipEnvironmentSetup) {
    Invoke-CapturedNative `
      -FilePath 'powershell' `
      -ArgumentList @(
        '-NoProfile',
        '-File', 'scripts/setup_ml_signal_training_env.ps1',
        '-VenvPath', $VenvPath
      ) `
      -LogPath (Join-Path $logsRoot 'training-environment.log') | Out-Null
  }
  if (-not (Test-Path -LiteralPath $venvPython -PathType Leaf)) {
    throw 'The pinned MlSignal training Python environment is unavailable.'
  }
  Invoke-CapturedNative `
    -FilePath $venvPython `
    -ArgumentList @(
      '-c',
      "import catboost, numpy, pandas, sklearn; assert catboost.__version__ == '1.2.10'"
    ) `
    -LogPath (Join-Path $logsRoot 'training-environment-imports.log') | Out-Null

  $subsetRelative = Get-RepoRelativePath -Path $subsetCsv
  $datasetRelative = Get-RepoRelativePath -Path $datasetCsv
  $splitPlanRelative = Get-RepoRelativePath -Path $splitPlanPath
  Invoke-CapturedNative `
    -FilePath 'cargo' `
    -ArgumentList @(
      'run',
      '-p', 'ta_app_core',
      '--features', 'cli',
      '--bin', 'ta_ml_signal_export_dataset',
      '--',
      '--csv-path', $subsetRelative,
      '--output-path', $datasetRelative,
      '--data-id', $DataId,
      '--horizon-bars', '10',
      '--round-trip-cost-bps', '12.5',
      '--split-plan-json', $splitPlanRelative
    ) `
    -LogPath (Join-Path $logsRoot 'dataset-export.log') | Out-Null

  $datasetStem = Join-Path (Split-Path -Parent $datasetCsv) (
    [System.IO.Path]::GetFileNameWithoutExtension($datasetCsv)
  )
  $featureSchemaPath = "$datasetStem.feature_schema.json"
  $splitManifestPath = "$datasetStem.split_manifest.json"
  $exportManifestPath = "$datasetStem.export_manifest.json"
  foreach ($requiredExport in @(
    $datasetCsv,
    $featureSchemaPath,
    $splitManifestPath,
    $exportManifestPath
  )) {
    if (-not (Test-Path -LiteralPath $requiredExport -PathType Leaf)) {
      throw "MlSignal export did not publish required artifact '$([IO.Path]::GetFileName($requiredExport))'."
    }
  }

  Invoke-CapturedNative `
    -FilePath $venvPython `
    -ArgumentList @(
      'scripts/train_ml_signal_catboost.py',
      '--dataset-csv', (Get-RepoRelativePath -Path $datasetCsv),
      '--feature-schema', (Get-RepoRelativePath -Path $featureSchemaPath),
      '--split-manifest', (Get-RepoRelativePath -Path $splitManifestPath),
      '--export-manifest', (Get-RepoRelativePath -Path $exportManifestPath),
      '--training-config', (Get-RepoRelativePath -Path $trainingConfigPath),
      '--bundle-dir', (Get-RepoRelativePath -Path $bundleRoot),
      '--bundle-id', 'qa-catboost-real-native-v1'
    ) `
    -LogPath (Join-Path $logsRoot 'training.log') | Out-Null

  $bundleManifestPath = Join-Path $bundleRoot 'manifest.json'
  $bundleMetricsPath = Join-Path $bundleRoot 'metrics.json'
  $bundleProvenancePath = Join-Path $bundleRoot 'provenance.json'
  foreach ($requiredBundleFile in @(
    $bundleManifestPath,
    $bundleMetricsPath,
    $bundleProvenancePath,
    (Join-Path $bundleRoot 'model_long.cbm'),
    (Join-Path $bundleRoot 'model_short.cbm')
  )) {
    if (-not (Test-Path -LiteralPath $requiredBundleFile -PathType Leaf)) {
      throw "Training did not publish required bundle artifact '$([IO.Path]::GetFileName($requiredBundleFile))'."
    }
  }

  $nativeSmokeBaseArguments = @(
    '-NoProfile',
    '-File', 'scripts/run_ml_signal_native_smoke.ps1',
    '-CsvPath', $subsetRelative,
    '-BundleManifest', (Get-RepoRelativePath -Path $bundleManifestPath),
    '-NativeLibraryPath', (Get-RepoRelativePath -Path $nativeLibraryPath),
    '-BackendPreference', 'native_cpu',
    '-PredictionBatchSize', $PredictionBatchSize.ToString(),
    '-DataId', $DataId,
    '-ScoreCacheOut', (Get-RepoRelativePath -Path $scoreCachePath),
    '-OutJson', (Get-RepoRelativePath -Path $nativeSmokePath),
    '-RunValidation',
    '-RunPolicySweep'
  )
  Invoke-CapturedNative `
    -FilePath 'powershell' `
    -ArgumentList $nativeSmokeBaseArguments `
    -LogPath (Join-Path $logsRoot 'native-smoke.log') | Out-Null

  $nativeSmoke = Get-Content -Raw -LiteralPath $nativeSmokePath | ConvertFrom-Json
  if ([string]$nativeSmoke.result.native_backend_used -ne 'native_cpu') {
    throw "Native smoke used '$($nativeSmoke.result.native_backend_used)' instead of native_cpu."
  }
  if ([string]$nativeSmoke.result.score_scope -ne 'walkforward_test') {
    throw "Native smoke produced '$($nativeSmoke.result.score_scope)' scores instead of walkforward_test."
  }
  if ([int]$nativeSmoke.result.bars_scored -le 0) {
    throw 'Native smoke did not score any walk-forward rows.'
  }
  if ($null -eq $nativeSmoke.validation -or $null -eq $nativeSmoke.policy_sweep) {
    throw 'Native smoke did not publish both validation and policy-sweep evidence.'
  }

  $failureCases = [System.Collections.Generic.List[object]]::new()
  $missingDllResult = Invoke-CapturedNative `
    -FilePath 'powershell' `
    -ArgumentList @(
      '-NoProfile',
      '-File', 'scripts/run_ml_signal_native_smoke.ps1',
      '-CsvPath', $subsetRelative,
      '-BundleManifest', (Get-RepoRelativePath -Path $bundleManifestPath),
      '-NativeLibraryPath', (Get-RepoRelativePath -Path (Join-Path $failureRoot 'missing-catboostmodel.dll')),
      '-BackendPreference', 'native_cpu'
    ) `
    -LogPath (Join-Path $failureRoot 'missing-native-library.log') `
    -ExpectFailure
  $failureCases.Add([ordered]@{
    case = 'missing_native_library'
    rejected = $true
    exit_code = $missingDllResult.exit_code
    error_excerpt = Get-ErrorExcerpt -Text $missingDllResult.output
  })

  $corruptDllPath = Join-Path $failureRoot 'corrupt-catboostmodel.dll'
  [System.IO.File]::WriteAllBytes(
    $corruptDllPath,
    [System.Text.Encoding]::ASCII.GetBytes('not-a-dynamic-library')
  )
  $corruptDllResult = Invoke-CapturedNative `
    -FilePath 'powershell' `
    -ArgumentList @(
      '-NoProfile',
      '-File', 'scripts/run_ml_signal_native_smoke.ps1',
      '-CsvPath', $subsetRelative,
      '-BundleManifest', (Get-RepoRelativePath -Path $bundleManifestPath),
      '-NativeLibraryPath', (Get-RepoRelativePath -Path $corruptDllPath),
      '-BackendPreference', 'native_cpu',
      '-ScoreCacheOut', (Get-RepoRelativePath -Path (Join-Path $failureRoot 'corrupt-dll-cache.json'))
    ) `
    -LogPath (Join-Path $failureRoot 'corrupt-native-library.log') `
    -ExpectFailure
  $failureCases.Add([ordered]@{
    case = 'corrupt_native_library'
    rejected = $true
    exit_code = $corruptDllResult.exit_code
    error_excerpt = Get-ErrorExcerpt -Text $corruptDllResult.output
  })

  foreach ($modelCase in @('missing_model', 'corrupt_model')) {
    $caseBundle = Join-Path $failureRoot "$modelCase-bundle"
    Copy-Item -LiteralPath $bundleRoot -Destination $caseBundle -Recurse
    $caseModelPath = Join-Path $caseBundle 'model_long.cbm'
    if ($modelCase -eq 'missing_model') {
      Remove-Item -LiteralPath $caseModelPath -Force
    } else {
      [System.IO.File]::WriteAllBytes(
        $caseModelPath,
        [System.Text.Encoding]::ASCII.GetBytes('not-a-catboost-model')
      )
    }
    $modelFailure = Invoke-CapturedNative `
      -FilePath 'powershell' `
      -ArgumentList @(
        '-NoProfile',
        '-File', 'scripts/run_ml_signal_native_smoke.ps1',
        '-CsvPath', $subsetRelative,
        '-BundleManifest', (Get-RepoRelativePath -Path (Join-Path $caseBundle 'manifest.json')),
        '-NativeLibraryPath', (Get-RepoRelativePath -Path $nativeLibraryPath),
        '-BackendPreference', 'native_cpu',
        '-ScoreCacheOut', (Get-RepoRelativePath -Path (Join-Path $failureRoot "$modelCase-cache.json"))
      ) `
      -LogPath (Join-Path $failureRoot "$modelCase.log") `
      -ExpectFailure
    $failureCases.Add([ordered]@{
      case = $modelCase
      rejected = $true
      exit_code = $modelFailure.exit_code
      error_excerpt = Get-ErrorExcerpt -Text $modelFailure.output
    })
  }

  $baseline = Get-Content -Raw -LiteralPath $nativeBaselinePath | ConvertFrom-Json
  $metrics = Get-Content -Raw -LiteralPath $bundleMetricsPath | ConvertFrom-Json
  $provenance = Get-Content -Raw -LiteralPath $bundleProvenancePath | ConvertFrom-Json
  $bundleManifest = Get-Content -Raw -LiteralPath $bundleManifestPath | ConvertFrom-Json
  $splitManifest = Get-Content -Raw -LiteralPath $splitManifestPath | ConvertFrom-Json

  $artifactHashes = [ordered]@{}
  foreach ($artifactPath in @(
    $subsetCsv,
    $datasetCsv,
    $featureSchemaPath,
    $splitManifestPath,
    $exportManifestPath,
    $trainingConfigPath,
    $bundleManifestPath,
    $bundleMetricsPath,
    $bundleProvenancePath,
    (Join-Path $bundleRoot 'model_long.cbm'),
    (Join-Path $bundleRoot 'model_short.cbm'),
    $scoreCachePath,
    $nativeSmokePath
  )) {
    $artifactHashes[(Get-RepoRelativePath -Path $artifactPath)] = Get-FileSha256Lower -Path $artifactPath
  }

  $summary = [ordered]@{
    schema_version = 1
    contract = Get-Contract
    status = 'passed'
    outcome = 'functional_native_inference_proven'
    model_quality_approval = $false
    trading_readiness_approval = $false
    interpretation = @(
      'This run proves the bounded exporter/trainer/native CPU inference/validation plumbing.',
      'Metrics are diagnostic evidence only and do not approve the trained model or an automated trading strategy.'
    )
    bounds = [ordered]@{
      requested_max_source_rows = $MaxRows
      copied_source_rows = $subsetRows
      catboost_iterations = $Iterations
      prediction_batch_size = $PredictionBatchSize
      policy_sweep_candidates = [int]$nativeSmoke.policy_sweep.sweep_size
    }
    source_integrity = [ordered]@{
      path = '<redacted>'
      unchanged = $true
      size_bytes = $sourceIdentityBefore.size_bytes
      sha256 = $sourceIdentityBefore.sha256
    }
    export = [ordered]@{
      data_id = $DataId
      exported_rows = [int]$nativeSmoke.result.raw_score_observations
      split_window_count = @($splitManifest.windows).Count
      walkforward_test_rows = [int]$nativeSmoke.result.effective_score_observations
    }
    training = [ordered]@{
      deterministic_controls = [ordered]@{
        seed = [int]$trainingConfig.seed
        thread_count = [int]$trainingConfig.catboost.params.thread_count
        bootstrap_type = [string]$trainingConfig.catboost.params.bootstrap_type
        random_strength = [double]$trainingConfig.catboost.params.random_strength
      }
      bundle_id = [string]$bundleManifest.bundle_id
      catboost_version = [string]$bundleManifest.catboost_version
      requirements_sha256 = Get-FileSha256Lower -Path $requirementsPath
      package_versions = $provenance.package_versions
      model_long_sha256 = [string]$bundleManifest.model_long_sha256
      model_short_sha256 = [string]$bundleManifest.model_short_sha256
      diagnostic_test_metrics = [ordered]@{
        long = $metrics.long.metrics.test.calibrated_probability
        short = $metrics.short.metrics.test.calibrated_probability
      }
    }
    native_runtime = [ordered]@{
      baseline_version = [string]$baseline.version
      baseline_tag = [string]$baseline.tag
      baseline_tag_commit = [string]$baseline.tag_commit
      expected_library_sha256 = ([string]$baseline.asset_sha256).ToLowerInvariant()
      observed_library_sha256 = Get-FileSha256Lower -Path $nativeLibraryPath
      scored_library_sha256 = ([string]$nativeSmoke.result.native_library_sha256).ToLowerInvariant()
      observed_library_size_bytes = [int64](Get-Item -LiteralPath $nativeLibraryPath).Length
      requested_backend = [string]$nativeSmoke.result.native_backend_requested
      used_backend = [string]$nativeSmoke.result.native_backend_used
      producer_kind = [string]$nativeSmoke.result.producer_kind
      scored_model_long_sha256 = [string]$nativeSmoke.result.model_long_sha256
      scored_model_short_sha256 = [string]$nativeSmoke.result.model_short_sha256
      bars_scored = [int]$nativeSmoke.result.bars_scored
      score_scope = [string]$nativeSmoke.result.score_scope
      score_space = [string]$nativeSmoke.result.score_space
      coverage_ratio = [double]$nativeSmoke.result.coverage_ratio
    }
    validation = [ordered]@{
      candidate_count = @($nativeSmoke.validation.candidates).Count
      run_verdict = $nativeSmoke.validation.run_verdict
      policy_sweep_size = [int]$nativeSmoke.policy_sweep.sweep_size
      policy_sweep_retained_candidates = @($nativeSmoke.policy_sweep.validation.candidates).Count
      policy_sweep_run_verdict = $nativeSmoke.policy_sweep.validation.run_verdict
    }
    negative_contracts = @($failureCases)
    artifacts = [ordered]@{
      root = Get-RepoRelativePath -Path $runRoot
      hashes = $artifactHashes
    }
  }
  if (
    $summary.native_runtime.expected_library_sha256 -ne
    $summary.native_runtime.observed_library_sha256 -or
    $summary.native_runtime.expected_library_sha256 -ne
    $summary.native_runtime.scored_library_sha256
  ) {
    throw 'Observed CatBoost library hash does not match the reviewed baseline.'
  }
  if (
    $summary.training.model_long_sha256 -ne
    $summary.native_runtime.scored_model_long_sha256 -or
    $summary.training.model_short_sha256 -ne
    $summary.native_runtime.scored_model_short_sha256
  ) {
    throw 'Native scoring model hashes do not match the generated bundle manifest.'
  }
  Write-JsonFile -Path $summaryPath -Value $summary

  $retainedTextFiles = Get-ChildItem -LiteralPath $runRoot -Recurse -File | Where-Object {
    $_.Extension -in @('.json', '.log', '.txt') -or $_.Name -eq '.qa-owner.json'
  }
  foreach ($file in $retainedTextFiles) {
    $content = Get-Content -Raw -LiteralPath $file.FullName
    foreach ($privateValue in @($script:RepoRoot, $script:ResolvedSourceCsv, $env:USERPROFILE)) {
      if (
        -not [string]::IsNullOrWhiteSpace($privateValue) -and
        (Test-ContainsPrivatePath -Text $content -PrivatePath $privateValue)
      ) {
        throw "Retained QA evidence contains an unredacted private path in '$($file.Name)'."
      }
    }
  }

  if (-not $KeepFailureFixtures) {
    Remove-OwnedDirectory -Path $failureRoot -Kind $script:FailureOwnerKind
  }
  $resultStatus = 'passed'
  [ordered]@{
    status = $resultStatus
    outcome = 'functional_native_inference_proven'
    model_quality_approval = $false
    summary = Get-RepoRelativePath -Path $summaryPath
    run_root = Get-RepoRelativePath -Path $runRoot
  } | ConvertTo-Json -Depth 5
} catch {
  $safeMessage = Protect-PrivatePaths -Text $_.Exception.Message
  Write-JsonFile -Path (Join-Path $runRoot 'failure.json') -Value ([ordered]@{
    schema_version = 1
    status = 'failed'
    model_quality_approval = $false
    error = $safeMessage
  })
  throw $safeMessage
} finally {
  Pop-Location -ErrorAction SilentlyContinue
}
