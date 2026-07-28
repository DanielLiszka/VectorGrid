param(
  [string]$VenvPath = "",
  [switch]$Recreate
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$requirementsPath = Join-Path $PSScriptRoot "requirements-ml-signal-training.lock.txt"
if (-not (Test-Path -LiteralPath $requirementsPath -PathType Leaf)) {
  throw "MlSignal training dependency lock is missing: $requirementsPath"
}

if ([string]::IsNullOrWhiteSpace($VenvPath)) {
  $VenvPath = Join-Path $repoRoot "output\\tools\\ml-signal-training-venv"
}
$resolvedVenv = [System.IO.Path]::GetFullPath($VenvPath)
$outputRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "output"))
$outputPrefix = $outputRoot.TrimEnd([char]92, [char]47) + [System.IO.Path]::DirectorySeparatorChar
if (-not $resolvedVenv.StartsWith($outputPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "Training virtual environments must remain under the repository output directory: $outputRoot"
}

if ($Recreate -and (Test-Path -LiteralPath $resolvedVenv)) {
  $resolvedExisting = (Resolve-Path -LiteralPath $resolvedVenv).Path
  if (-not $resolvedExisting.StartsWith($outputPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove virtual environment outside the repository output directory."
  }
  Remove-Item -LiteralPath $resolvedExisting -Recurse -Force
}

$venvPython = Join-Path $resolvedVenv "Scripts\python.exe"
if (-not (Test-Path -LiteralPath $venvPython -PathType Leaf)) {
  python -m venv $resolvedVenv
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to create MlSignal training virtual environment."
  }
}

& $venvPython -m pip install --disable-pip-version-check -r $requirementsPath
if ($LASTEXITCODE -ne 0) {
  throw "Failed to install the locked MlSignal training environment."
}

$verification = & $venvPython -c @'
import json
import catboost
import numpy
import pandas
import sklearn

print(json.dumps(dict(
    catboost=catboost.__version__,
    numpy=numpy.__version__,
    pandas=pandas.__version__,
    scikit_learn=sklearn.__version__,
), sort_keys=True))
'@
if ($LASTEXITCODE -ne 0) {
  throw "MlSignal training environment import verification failed."
}

[ordered]@{
  status = "ready"
  venv_path = $resolvedVenv
  python_path = $venvPython
  requirements_path = $requirementsPath
  versions = ($verification | ConvertFrom-Json)
} | ConvertTo-Json -Depth 5
