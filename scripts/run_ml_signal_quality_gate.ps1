


param(
  [switch]$SkipWeb,
  [switch]$SkipDesktopCommands,
  [switch]$SkipTrainingScriptCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-NativeChecked {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,
    [string[]]$ArgumentList = @()
  )

  & $FilePath @ArgumentList
  if ($LASTEXITCODE -ne 0) {
    throw "$FilePath exited with code $LASTEXITCODE"
  }
}

function Resolve-NpmCommand {
  $npmCmd = Get-Command npm.cmd -ErrorAction SilentlyContinue
  if ($npmCmd) {
    return $npmCmd.Source
  }
  $npm = Get-Command npm -ErrorAction Stop
  return $npm.Source
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $repoRoot
try {
  Write-Host '== app-core MlSignal tests =='
  Invoke-NativeChecked -FilePath 'cargo' -ArgumentList @('test', '-p', 'ta_app_core', '--lib', 'ml_signal')

  if (-not $SkipDesktopCommands) {
    Write-Host '== Tauri MlSignal command tests =='
    Invoke-NativeChecked -FilePath 'cargo' -ArgumentList @('test', '-p', 'ta_desktop_demo_app', '--lib', 'commands_ml_signal')

    Write-Host '== Tauri MlSignal artifact tests =='
    Invoke-NativeChecked -FilePath 'cargo' -ArgumentList @('test', '-p', 'ta_desktop_demo_app', '--lib', 'run_artifact_summary_surfaces_ml_signal_provenance_fields_when_present')
    Invoke-NativeChecked -FilePath 'cargo' -ArgumentList @('test', '-p', 'ta_desktop_demo_app', '--lib', 'run_artifact_detail_response_formats_ml_summaries')
    Invoke-NativeChecked -FilePath 'cargo' -ArgumentList @('test', '-p', 'ta_desktop_demo_app', '--lib', 'workflow_hydration_payload_preserves_report_tags_and_ml_signal_without_validation')
  }

  if (-not $SkipWeb) {
    Write-Host '== web MlSignal tests =='
    $npmCommand = Resolve-NpmCommand
    Invoke-NativeChecked -FilePath $npmCommand -ArgumentList @('--prefix', 'web', 'run', 'test', '--', 'src/services/mlSignalService.test.ts', 'src/components/MlSignalWorkspacePanel.test.ts')
  }

  if (-not $SkipTrainingScriptCheck) {
    Write-Host '== training script syntax/config checks =='
    Invoke-NativeChecked -FilePath 'python' -ArgumentList @('-m', 'py_compile', 'scripts/train_ml_signal_catboost.py')
    Invoke-NativeChecked -FilePath 'python' -ArgumentList @('-c', "import json, pathlib; json.load(open(pathlib.Path('configs/ml_signal_catboost_default.json'), encoding='utf-8'))")
  }
} finally {
  Pop-Location
}
