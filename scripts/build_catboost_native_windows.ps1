


param(
  [ValidateSet('cpu', 'cuda')]
  [string]$Mode = 'cpu',

  [string]$CatBoostTag = 'v1.2.10',
  [string]$SourceDir = 'C:\cbsrc\catboost',
  [string]$BuildRootDir = '',
  [string]$PythonVenv = '',
  [string]$CudaRootDir = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2',
  [string]$CudaArchitectures = '89',
  [string]$MsvsVersion = '2022',
  [string]$MsvcToolset = '14.44.35207',
  [int]$ParallelJobs = 8,
  [string]$LlvmRootDir = 'C:\Program Files\LLVM',
  [string]$LlvmDrive = 'L:'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if ([string]::IsNullOrWhiteSpace($PythonVenv)) {
  $PythonVenv = Join-Path $repoRoot '.catboost-native-build-venv'
}

function Get-NormalizedFullPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  return [System.IO.Path]::GetFullPath($Path)
}

function Assert-SafeBuildRootDir {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    throw 'BuildRootDir must not be empty.'
  }

  $fullPath = Get-NormalizedFullPath -Path $Path
  $root = [System.IO.Path]::GetPathRoot($fullPath)
  $relative = $fullPath.Substring($root.Length).Trim('\')
  if ([string]::IsNullOrWhiteSpace($relative) -or $relative -notmatch '[\\/]') {
    throw "Refusing to recursively delete build root '$fullPath'. Choose a nested directory under a dedicated build root."
  }
  $segments = @($relative -split '[\\/]' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
  $hasDedicatedMarker = $false
  foreach ($segment in $segments) {
    if ($segment -match '(?i)^(cbbuild|catboost|catboostmodel|catboost[-_].*)$' -or $segment -match '(?i)^.*[-_]catboost[-_].*$') {
      $hasDedicatedMarker = $true
      break
    }
  }
  if (-not $hasDedicatedMarker) {
    throw "Refusing to recursively delete build root '$fullPath'. Include a dedicated catboost/cbbuild path segment."
  }
  if (Test-Path -LiteralPath $fullPath -PathType Leaf) {
    throw "BuildRootDir '$fullPath' is a file, not a directory."
  }
  return $fullPath
}

function Get-GitText {
  param(
    [Parameter(Mandatory = $true)]
    [string[]]$Arguments
  )

  $output = & git @Arguments 2>$null
  if ($LASTEXITCODE -ne 0) {
    return $null
  }
  return ($output | Out-String).Trim()
}

function Assert-CatBoostSourceDirMatchesTag {
  param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDir,
    [Parameter(Mandatory = $true)]
    [string]$CatBoostTag
  )

  if (!(Test-Path -LiteralPath $SourceDir -PathType Container)) {
    return
  }
  if (!(Test-Path -LiteralPath (Join-Path $SourceDir '.git'))) {
    throw "Existing SourceDir '$SourceDir' is not a git checkout."
  }

  $head = Get-GitText -Arguments @('-C', $SourceDir, 'rev-parse', 'HEAD')
  $tagCommit = Get-GitText -Arguments @('-C', $SourceDir, 'rev-parse', "refs/tags/$CatBoostTag`^{commit}")
  if ([string]::IsNullOrWhiteSpace($head)) {
    throw "Failed to resolve HEAD for existing SourceDir '$SourceDir'."
  }
  if ([string]::IsNullOrWhiteSpace($tagCommit)) {
    throw "Existing SourceDir '$SourceDir' does not contain requested tag '$CatBoostTag'."
  }
  if ($head -ne $tagCommit) {
    throw "Existing SourceDir '$SourceDir' is checked out at $head, not requested tag '$CatBoostTag' ($tagCommit)."
  }
}

if ([string]::IsNullOrWhiteSpace($BuildRootDir)) {
  if ($Mode -eq 'cpu') {
    $BuildRootDir = 'C:\cbbuild\catboostmodel_cpu_llvm'
  } else {
    $BuildRootDir = 'C:\cbbuild\catboostmodel_cuda132_sm89'
  }
}

$BuildRootDir = Assert-SafeBuildRootDir -Path $BuildRootDir

if (!(Test-Path $PythonVenv)) {
  python -m venv $PythonVenv
}
& "$PythonVenv\Scripts\python.exe" -m pip install --upgrade pip conan

if (!(Test-Path $SourceDir)) {
  New-Item -ItemType Directory -Force -Path (Split-Path $SourceDir) | Out-Null
  git clone --depth 1 --branch $CatBoostTag https://github.com/catboost/catboost.git $SourceDir
}
Assert-CatBoostSourceDirMatchesTag -SourceDir $SourceDir -CatBoostTag $CatBoostTag

if (Test-Path $BuildRootDir) {
  Remove-Item -Recurse -Force -LiteralPath $BuildRootDir
}

$env:PATH = "$PythonVenv\Scripts;$env:PATH"
$python = "$PythonVenv\Scripts\python.exe"
$buildScript = Join-Path $SourceDir 'build\build_native.py'
$cmd = @(
  $python,
  $buildScript,
  '--build-root-dir', $BuildRootDir,
  '--targets', 'catboostmodel',
  '--msvs-version', $MsvsVersion,
  '--msvc-toolset', $MsvcToolset,
  '--parallel-build-jobs', $ParallelJobs.ToString()
)

try {
  if ($Mode -eq 'cpu') {
    if (!(Test-Path $LlvmRootDir)) {
      throw "LLVM root '$LlvmRootDir' does not exist."
    }
    $llvmDriveLetter = $LlvmDrive.TrimEnd(':')
    subst "$llvmDriveLetter`:" $LlvmRootDir
    $cmd += "--cmake-extra-args=-DCMAKE_CXX_COMPILER:FILEPATH=$LlvmDrive/bin/clang-cl.exe,-DCMAKE_C_COMPILER:FILEPATH=$LlvmDrive/bin/clang-cl.exe,-DCMAKE_RC_COMPILER:FILEPATH=$LlvmDrive/bin/llvm-rc.exe"
  } else {
    if (!(Test-Path $CudaRootDir)) {
      throw "CUDA root '$CudaRootDir' does not exist."
    }
    $cmd += @(
      '--have-cuda',
      '--cuda-root-dir', $CudaRootDir,
      "--cmake-extra-args=-DCMAKE_CUDA_ARCHITECTURES=$CudaArchitectures"
    )
  }

  & $cmd[0] $cmd[1..($cmd.Length - 1)]
  $exitCode = $LASTEXITCODE
  if ($exitCode -ne 0) {
    throw "CatBoost native build failed with exit code $exitCode"
  }
} finally {
  if ($Mode -eq 'cpu') {
    $llvmDriveLetter = $LlvmDrive.TrimEnd(':')
    subst "$llvmDriveLetter`:" /d | Out-Null
  }
}

$dllPath = Join-Path $BuildRootDir 'catboost\libs\model_interface\catboostmodel.dll'
if (Test-Path $dllPath) {
  Write-Output "Built: $dllPath"
} else {
  Write-Warning "Build finished without catboostmodel.dll at expected path: $dllPath"
}
