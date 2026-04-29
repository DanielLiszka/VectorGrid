param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [string]$OutDir = "",
  [Parameter(Mandatory = $true)]
  [string]$ReleaseBinaryPath,
  [string]$BundleRoot = "",
  [string]$SignToolPath = "",
  [string]$CertificateSha1 = "",
  [string]$CertificateSubject = "",
  [string]$TimestampUrl = "http://timestamp.digicert.com",
  [string]$FileDigestAlgorithm = "SHA256"
)

$ErrorActionPreference = "Stop"

function Resolve-SignTool {
  param([string]$Candidate)
  if (-not [string]::IsNullOrWhiteSpace($Candidate)) {
    if (-not (Test-Path -LiteralPath $Candidate -PathType Leaf)) {
      throw "SignToolPath does not exist: $Candidate"
    }
    return (Resolve-Path -LiteralPath $Candidate).Path
  }

  $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($null -ne $cmd) {
    return $cmd.Source
  }

  $programFilesX86 = [Environment]::GetFolderPath("ProgramFilesX86")
  if ([string]::IsNullOrWhiteSpace($programFilesX86)) {
    $programFilesX86 = ${env:ProgramFiles(x86)}
  }
  $kitsRoot = if ([string]::IsNullOrWhiteSpace($programFilesX86)) {
    $null
  } else {
    Join-Path $programFilesX86 "Windows Kits\\10\\bin"
  }
  if ($null -ne $kitsRoot -and (Test-Path $kitsRoot)) {
    $candidate = Get-ChildItem -Path $kitsRoot -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
      Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
      Sort-Object FullName -Descending |
      Select-Object -First 1
    if ($null -ne $candidate) {
      return $candidate.FullName
    }
  }

  throw "signtool.exe was not found. Install Windows SDK or pass -SignToolPath."
}

function Resolve-SignIdentityArgs {
  if (-not [string]::IsNullOrWhiteSpace($CertificateSha1)) {
    return @("/sha1", $CertificateSha1)
  }
  if (-not [string]::IsNullOrWhiteSpace($CertificateSubject)) {
    return @("/n", $CertificateSubject)
  }
  throw "Provide -CertificateSha1 or -CertificateSubject for SignTool certificate selection."
}

function Get-SignableArtifacts {
  $paths = New-Object System.Collections.Generic.List[string]
  if (-not [string]::IsNullOrWhiteSpace($ReleaseBinaryPath) -and (Test-Path -LiteralPath $ReleaseBinaryPath -PathType Leaf)) {
    $paths.Add((Resolve-Path -LiteralPath $ReleaseBinaryPath).Path)
  }
  if (-not [string]::IsNullOrWhiteSpace($BundleRoot) -and (Test-Path -LiteralPath $BundleRoot -PathType Container)) {
    Get-ChildItem -Path $BundleRoot -Recurse -File | Where-Object {
      @(".exe", ".msi") -contains $_.Extension.ToLowerInvariant()
    } | Sort-Object FullName | ForEach-Object {
      $resolved = (Resolve-Path -LiteralPath $_.FullName).Path
      if (-not $paths.Contains($resolved)) {
        $paths.Add($resolved)
      }
    }
  }
  return @($paths)
}

$effectiveSignToolPath = if ([string]::IsNullOrWhiteSpace($SignToolPath)) { [string]$env:VECTORGRID_SIGNTOOL_PATH } else { $SignToolPath }
$effectiveCertificateSha1 = if ([string]::IsNullOrWhiteSpace($CertificateSha1)) { [string]$env:VECTORGRID_SIGN_CERT_SHA1 } else { $CertificateSha1 }
$effectiveCertificateSubject = if ([string]::IsNullOrWhiteSpace($CertificateSubject)) { [string]$env:VECTORGRID_SIGN_CERT_SUBJECT } else { $CertificateSubject }
$effectiveTimestampUrl = if ([string]::IsNullOrWhiteSpace($TimestampUrl)) { [string]$env:VECTORGRID_SIGN_TIMESTAMP_URL } else { $TimestampUrl }
if ([string]::IsNullOrWhiteSpace($effectiveTimestampUrl)) {
  $effectiveTimestampUrl = "http://timestamp.digicert.com"
}

$resolvedSignTool = Resolve-SignTool -Candidate $effectiveSignToolPath
$CertificateSha1 = $effectiveCertificateSha1
$CertificateSubject = $effectiveCertificateSubject
$TimestampUrl = $effectiveTimestampUrl
$identityArgs = Resolve-SignIdentityArgs
$artifacts = Get-SignableArtifacts
if ($artifacts.Count -eq 0) {
  throw "No signable .exe or .msi artifacts were found."
}

foreach ($artifact in $artifacts) {
  Write-Host "Signing $artifact"
  $args = @(
    "sign",
    "/fd", $FileDigestAlgorithm,
    "/tr", $TimestampUrl,
    "/td", $FileDigestAlgorithm
  ) + $identityArgs + @($artifact)
  & $resolvedSignTool @args
  if ($LASTEXITCODE -ne 0) {
    throw "signtool failed for $artifact with exit code $LASTEXITCODE"
  }
}

Write-Host "Signed $($artifacts.Count) artifact(s) for $Variant."
