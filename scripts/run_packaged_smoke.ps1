param(
  [string]$BinaryPath = "",
  [string]$OutDir = "",
  [string]$RunId = ""
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 12
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
  return Join-Path (Join-Path $repoRoot "output\\release_candidates") "${stamp}-packaged-smoke"
}

function Resolve-ExistingPath {
  param([string[]]$Candidates)
  foreach ($candidate in $Candidates) {
    if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path $candidate)) {
      return (Resolve-Path $candidate).Path
    }
  }
  return $null
}

function Resolve-BinaryPath {
  param(
    [string]$Candidate,
    [string]$ManifestPath
  )

  if (-not [string]::IsNullOrWhiteSpace($Candidate)) {
    if ([System.IO.Path]::IsPathRooted($Candidate)) {
      return (Resolve-Path $Candidate).Path
    }
    return (Resolve-Path (Join-Path $repoRoot $Candidate)).Path
  }

  if (-not [string]::IsNullOrWhiteSpace($ManifestPath) -and (Test-Path $ManifestPath)) {
    $manifest = Get-Content -Raw $ManifestPath | ConvertFrom-Json
    $manifestBinary = [string]$manifest.release_binary_path
    if (-not [string]::IsNullOrWhiteSpace($manifestBinary) -and (Test-Path $manifestBinary)) {
      return (Resolve-Path $manifestBinary).Path
    }
  }

  return $null
}

function Get-ManifestArtifactSourceKind {
  param([string]$ManifestPath)
  if (-not [string]::IsNullOrWhiteSpace($ManifestPath) -and (Test-Path $ManifestPath)) {
    $manifest = Get-Content -Raw $ManifestPath | ConvertFrom-Json
    return [string]$manifest.artifact_source_kind
  }
  return ""
}

function Resolve-AppLogPath {
  $configPath = Join-Path $repoRoot "src-tauri\\tauri.conf.json"
  if (-not (Test-Path $configPath)) {
    return $null
  }
  $config = Get-Content -Raw $configPath | ConvertFrom-Json
  $identifier = [string]$config.identifier
  $productName = [string]$config.productName
  if ([string]::IsNullOrWhiteSpace($identifier) -or [string]::IsNullOrWhiteSpace($productName)) {
    return $null
  }
  $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
  $candidate = Join-Path (Join-Path $localAppData $identifier) "logs\\$productName.log"
  if (Test-Path $candidate) {
    return (Resolve-Path $candidate).Path
  }
  return $candidate
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$manifestPath = Join-Path $resolvedOutDir "manifest.json"
$manifestArtifactSourceKind = Get-ManifestArtifactSourceKind -ManifestPath $manifestPath
$manifestArtifactSourceKind = if ([string]::IsNullOrWhiteSpace($manifestArtifactSourceKind)) {
  "release_binary"
} else {
  $manifestArtifactSourceKind
}
$resolvedBinaryPath = Resolve-BinaryPath -Candidate $BinaryPath -ManifestPath $manifestPath
if ($null -eq $resolvedBinaryPath) {
  throw "Could not resolve a release executable for packaged smoke. Provide -BinaryPath or run manifest generation first."
}
$appLogPath = Resolve-AppLogPath
$smokeSummaryPath = Join-Path $resolvedOutDir "packaged-smoke-summary.json"

$resolvedRunId = if ([string]::IsNullOrWhiteSpace($RunId)) {
  "release-smoke-" + (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
} else {
  $RunId
}

$sourceArtifactDir = Join-Path $repoRoot "output\\wdio\\desktop-smoke\\$resolvedRunId"
$destArtifactDir = Join-Path $resolvedOutDir "desktop-smoke"

$prevSkipBuild = $env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD
$prevBinary = $env:VECTORGRID_DESKTOP_SMOKE_BINARY
$prevRunId = $env:VECTORGRID_DESKTOP_SMOKE_RUN_ID
$prevDisablePreview = $env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW
$pushedLocation = $false
$status = "failed"
$errorMessage = $null
$appLogArtifactPath = $null

try {
  if ($manifestArtifactSourceKind -eq "release_binary") {
    Write-Warning "Manifest resolved only a raw release executable, not a bundle artifact. Packaged smoke will verify whether that release binary can load its own frontend/resources without preview bootstrap."
  }
  $env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD = "1"
  $env:VECTORGRID_DESKTOP_SMOKE_BINARY = $resolvedBinaryPath
  $env:VECTORGRID_DESKTOP_SMOKE_RUN_ID = $resolvedRunId
  $env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW = "1"

  Push-Location $repoRoot
  $pushedLocation = $true
  & npm.cmd --prefix web run test:desktop:smoke
  if ($LASTEXITCODE -ne 0) {
    throw "Desktop smoke failed with exit code $LASTEXITCODE"
  }

  $status = "passed"
  Write-Host "Packaged smoke completed against $resolvedBinaryPath"
} catch {
  $errorMessage = $_.Exception.Message
  throw
} finally {
  if ($pushedLocation) {
    Pop-Location
  }
  if (Test-Path $sourceArtifactDir) {
    if (Test-Path $destArtifactDir) {
      Remove-Item -Recurse -Force $destArtifactDir
    }
    New-Item -ItemType Directory -Force -Path $destArtifactDir | Out-Null
    Copy-Item -Recurse -Force (Join-Path $sourceArtifactDir "*") $destArtifactDir
  }
  if (-not [string]::IsNullOrWhiteSpace($appLogPath) -and (Test-Path $appLogPath)) {
    $appLogDir = Join-Path $destArtifactDir "logs\\app"
    New-Item -ItemType Directory -Force -Path $appLogDir | Out-Null
    $appLogArtifactPath = Join-Path $appLogDir ([System.IO.Path]::GetFileName($appLogPath))
    Copy-Item -LiteralPath $appLogPath -Destination $appLogArtifactPath -Force
  }
  if ($null -ne $prevSkipBuild) {
    $env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD = $prevSkipBuild
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevBinary) {
    $env:VECTORGRID_DESKTOP_SMOKE_BINARY = $prevBinary
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_BINARY -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevRunId) {
    $env:VECTORGRID_DESKTOP_SMOKE_RUN_ID = $prevRunId
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_RUN_ID -ErrorAction SilentlyContinue
  }
  if ($null -ne $prevDisablePreview) {
    $env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW = $prevDisablePreview
  } else {
    Remove-Item Env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW -ErrorAction SilentlyContinue
  }

  $diagnosis = if ($manifestArtifactSourceKind -eq "release_binary") {
    if ($status -eq "passed") {
      "Smoke target was a raw release executable rather than a bundle artifact. This proves shell boot on the built binary, but it is not a bundle-level packaging proof."
    } else {
      "Smoke target was a raw release executable rather than a bundle artifact. Blank-shell failures here often reflect WebDriver URL semantics or missing packaged resource context rather than a broken shell."
    }
  } else {
    $null
  }
  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    status = $status
    error = $errorMessage
    run_id = $resolvedRunId
    manifest_path = if (Test-Path $manifestPath) { $manifestPath } else { $null }
    artifact_source_kind = $manifestArtifactSourceKind
    smoke_mode = if ($manifestArtifactSourceKind -eq "bundle") { "bundle" } else { "raw-release-binary" }
    binary_path = $resolvedBinaryPath
    preview_disabled = $true
    source_artifact_dir = $sourceArtifactDir
    copied_artifact_dir = if (Test-Path $destArtifactDir) { $destArtifactDir } else { $null }
    app_log_path = $appLogPath
    copied_app_log_path = $appLogArtifactPath
    diagnosis = $diagnosis
  }
  Write-JsonUtf8 -Path $smokeSummaryPath -Value $summary
}

if (Test-Path $destArtifactDir) {
  Write-Host "Smoke artifacts copied to $destArtifactDir"
}
Write-Host "Packaged smoke summary written to $smokeSummaryPath"
