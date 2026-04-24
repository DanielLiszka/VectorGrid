<#
.SYNOPSIS
Runs the headless MlSignal native score-cache smoke harness.

.DESCRIPTION
Defaults to the standard CPU-native runtime path. Leave -NativeLibraryPath blank
to rely on bundle-relative discovery, preferably native\catboostmodel.dll next
to the selected bundle manifest.

.EXAMPLE
powershell -File scripts/run_ml_signal_native_smoke.ps1 `
  -CsvPath C:\data\prices.csv `
  -BundleManifest C:\models\ml_signal\bundle-a\manifest.json `
  -BackendPreference native_cpu
#>

param(
  [Parameter(Mandatory = $true)]
  [string]$CsvPath,

  [Parameter(Mandatory = $true)]
  [string]$BundleManifest,

  [string]$ScoreCacheOut,
  [string]$OutJson,
  [string]$NativeLibraryPath,
  [string]$DataId,
  [ValidateSet('auto', 'native_cuda', 'native_cpu', 'score_cache_only')]
  [string]$BackendPreference = 'native_cpu',
  [int]$PredictionBatchSize = 0,
  [int]$GpuDeviceId = 0,
  [switch]$PassGpuDeviceId
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-ExistingFile {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "$Label '$Path' does not exist or is not a file."
  }
}

function Ensure-ParentDirectory {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $parent = Split-Path -Parent $Path
  if ([string]::IsNullOrWhiteSpace($parent)) {
    return
  }
  if (!(Test-Path -LiteralPath $parent -PathType Container)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
}

Assert-ExistingFile -Path $CsvPath -Label 'CsvPath'
Assert-ExistingFile -Path $BundleManifest -Label 'BundleManifest'
if ($NativeLibraryPath) {
  Assert-ExistingFile -Path $NativeLibraryPath -Label 'NativeLibraryPath'
}
if ($ScoreCacheOut) {
  Ensure-ParentDirectory -Path $ScoreCacheOut
}
if ($OutJson) {
  Ensure-ParentDirectory -Path $OutJson
}
if ($PassGpuDeviceId -and $BackendPreference -notin @('auto', 'native_cuda')) {
  Write-Warning "GpuDeviceId was requested while BackendPreference is '$BackendPreference'; the smoke binary may ignore it."
}
if ($BackendPreference -eq 'score_cache_only') {
  throw "BackendPreference 'score_cache_only' is incompatible with the native smoke harness. Use auto, native_cuda, or native_cpu."
}
if ($PredictionBatchSize -lt 0) {
  throw "PredictionBatchSize must be 0 (default) or a positive integer."
}

$args = @(
  'run',
  '-p', 'ta_app_core',
  '--features', 'cli',
  '--bin', 'ta_ml_signal_native_smoke',
  '--',
  '--csv-path', $CsvPath,
  '--bundle-manifest', $BundleManifest,
  '--backend-preference', $BackendPreference
)

if ($ScoreCacheOut) {
  $args += @('--score-cache-out', $ScoreCacheOut)
}
if ($OutJson) {
  $args += @('--out-json', $OutJson)
}
if ($NativeLibraryPath) {
  $args += @('--native-library-path', $NativeLibraryPath)
}
if ($DataId) {
  $args += @('--data-id', $DataId)
}
if ($PassGpuDeviceId) {
  $args += @('--gpu-device-id', $GpuDeviceId.ToString())
}
if ($PredictionBatchSize -gt 0) {
  $args += @('--prediction-batch-size', $PredictionBatchSize.ToString())
}

cargo @args
exit $LASTEXITCODE
