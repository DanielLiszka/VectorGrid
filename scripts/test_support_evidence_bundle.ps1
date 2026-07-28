<#
.SYNOPSIS
Runs terminal-only privacy canaries for build_support_evidence_bundle.ps1.
#>

[CmdletBinding()]
param(
  [switch]$KeepArtifacts
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Write-Utf8NoBom {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value
  )
  $parent = Split-Path -Parent $Path
  if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  [System.IO.File]::WriteAllText(
    $Path,
    $Value,
    (New-Object System.Text.UTF8Encoding($false))
  )
}

function Assert-True {
  param(
    [Parameter(Mandatory = $true)][bool]$Condition,
    [Parameter(Mandatory = $true)][string]$Message
  )
  if (-not $Condition) {
    throw $Message
  }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$allowedScratchRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "output\support-evidence\canary"))
$scratchRoot = [System.IO.Path]::GetFullPath((Join-Path $allowedScratchRoot ([guid]::NewGuid().ToString("N"))))
$allowedPrefix = $allowedScratchRoot.TrimEnd([char]92, [char]47) + [System.IO.Path]::DirectorySeparatorChar
if (-not $scratchRoot.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "Canary scratch path escaped the bounded output directory."
}

$fixtureRoot = Join-Path $scratchRoot "fixture"
$bundleRoot = Join-Path $scratchRoot "bundle"
$expandedRoot = Join-Path $scratchRoot "expanded"
$negativeRoot = Join-Path $scratchRoot "negative"
$automaticBundleRoot = Join-Path $scratchRoot "automatic-bundle"
$fakeManifestRoot = Join-Path $scratchRoot "fake-manifest-fixture"
$fakeManifestOut = Join-Path $scratchRoot "fake-manifest-output"
$textValidationRoot = Join-Path $scratchRoot "text-validation"
$ciEvidenceRoot = Join-Path $scratchRoot "ci-current-run-fixture"
$ciEvidenceOut = Join-Path $scratchRoot "ci-current-run-bundle"
$builder = Join-Path $PSScriptRoot "build_support_evidence_bundle.ps1"

function ConvertFrom-PrivacyCanaryCodePoints {
  param(
    [Parameter(Mandatory = $true)][int[]]$CodePoints
  )

  return (($CodePoints | ForEach-Object { [char]$_ }) -join "")
}

# Reconstruct deterministic credential-shaped privacy canaries only at runtime.
# Numeric code points preserve exact redaction coverage without storing
# contiguous token-shaped literals in Git objects.
$apiSecret = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(67, 65, 78, 65, 82, 89, 95, 80, 79, 76, 89, 71, 79, 78, 95, 83, 69, 67, 82, 69, 84, 95, 55, 97, 98, 57, 51, 101, 52, 97)
$bearerSecret = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(67, 65, 78, 65, 82, 89, 95, 66, 69, 65, 82, 69, 82, 95, 84, 79, 75, 69, 78, 95, 57, 100, 57, 102, 54, 97, 51, 49)
$passwordSecret = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(67, 65, 78, 65, 82, 89, 95, 80, 65, 83, 83, 87, 79, 82, 68, 95, 49, 99, 97, 56, 98, 48, 54, 50)
$githubToken = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(103, 104, 112, 95, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74)
$slackToken = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(120, 111, 120, 98, 45, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 49, 50, 45, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112)
$projectToken = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(115, 107, 45, 112, 114, 111, 106, 45, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120)
$jwtToken = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(101, 121, 74, 104, 98, 71, 99, 105, 79, 105, 74, 73, 85, 122, 73, 49, 78, 105, 74, 57, 46, 101, 121, 74, 122, 100, 87, 73, 105, 79, 105, 74, 119, 99, 109, 108, 50, 89, 88, 82, 108, 76, 88, 86, 122, 90, 88, 73, 105, 102, 81, 46, 115, 105, 103, 110, 97, 116, 117, 114, 101, 99, 97, 110, 97, 114, 121)
$polygonEnvSecret = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(67, 65, 78, 65, 82, 89, 95, 66, 65, 82, 69, 95, 80, 79, 76, 89, 71, 79, 78, 95, 69, 78, 86, 95, 57, 51, 102, 56, 100, 48)
$alpacaKeyIdEnvSecret = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(67, 65, 78, 65, 82, 89, 95, 66, 65, 82, 69, 95, 65, 76, 80, 65, 67, 65, 95, 73, 68, 95, 69, 78, 86, 95, 102, 50, 52, 49, 99, 57)
$alpacaSecretEnvSecret = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(67, 65, 78, 65, 82, 89, 95, 66, 65, 82, 69, 95, 65, 76, 80, 65, 67, 65, 95, 83, 69, 67, 82, 69, 84, 95, 69, 78, 86, 95, 50, 99, 49, 53, 55, 97)
$genericCredentialSecret = ConvertFrom-PrivacyCanaryCodePoints -CodePoints @(67, 65, 78, 65, 82, 89, 95, 71, 69, 78, 69, 82, 73, 67, 95, 67, 82, 69, 68, 69, 78, 84, 73, 65, 76, 95, 57, 49, 99, 55, 101, 102)
$privatePath = "C:\Users\Private Person\VectorGrid Project\input.csv"
$forwardSlashPath = "D:/Users/Unicode User/Private Folder/report.json"
$extendedPath = "\\?\C:\Users\Private Person\VectorGrid Project\extended.log"
$uncPath = "\\private-server\alice\crashes\vectorgrid.log"
$unixPath = "/home/private-user/vectorgrid/config.json"
$wslPath = "/mnt/c/Users/private-wsl-user/vectorgrid/settings.json"
$rootPath = "/root/private-user/vectorgrid/report.json"
$optPath = "/opt/private-vendor/vectorgrid/settings.json"
$pemFence = "-" * 5
$pemKind = "PRIVATE KEY"
$pemBody = @(
  "${pemFence}BEGIN ${pemKind}${pemFence}"
  "CANARY_PRIVATE_KEY_BODY_5d5b8fe7"
  "${pemFence}END ${pemKind}${pemFence}"
) -join [Environment]::NewLine
$sensitiveEnvironmentNames = @("POLYGON_KEY", "ALPACA_KEY_ID", "ALPACA_SECRET_KEY")
$savedSensitiveEnvironment = @{}
foreach ($name in $sensitiveEnvironmentNames) {
  $savedSensitiveEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

try {
  $env:POLYGON_KEY = $polygonEnvSecret
  $env:ALPACA_KEY_ID = $alpacaKeyIdEnvSecret
  $env:ALPACA_SECRET_KEY = $alpacaSecretEnvSecret
  New-Item -ItemType Directory -Force -Path $fixtureRoot | Out-Null

  $manifestFixture = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = "cpu"
    publishable = $false
    unsigned_beta = $true
    artifact_source_kind = "bundle"
    product_name = "VectorGrid"
    version = "1.2.3"
    identifier = "dev.vectoralpha.vectorgrid"
    release_binary_path = $privatePath
    required_resources = [ordered]@{
      api_key = $apiSecret
      authorization = "Bearer $bearerSecret"
      safe_marker = "SAFE_DIAGNOSTIC_MARKER"
      password = $passwordSecret
      ordinary_value = "preserve-this-value"
    }
    staged_artifacts = @(
      [ordered]@{
        source_kind = "bundle"
        source_path = $privatePath
        staged_path = $privatePath
        staged_name = "vectorgrid-1.2.3-windows-x64-cpu.exe"
        extension = ".exe"
        size_bytes = 1234
        sha256 = "a" * 64
      }
    )
  }
  Write-Utf8NoBom -Path (Join-Path $fixtureRoot "manifest.json") -Value (($manifestFixture | ConvertTo-Json -Depth 8) + [Environment]::NewLine)

  $readinessFixture = [ordered]@{
    supported = $true
    diagnostic = "request failed: apiKey=$apiSecret; repo=$privatePath"
    private_key_dump = $pemBody
  }
  Write-Utf8NoBom -Path (Join-Path $fixtureRoot "readiness.json") -Value (($readinessFixture | ConvertTo-Json -Depth 8) + [Environment]::NewLine)

  Write-Utf8NoBom -Path (Join-Path $fixtureRoot "credentials.json") -Value "SHOULD_NOT_BE_INCLUDED"
  Write-Utf8NoBom -Path (Join-Path $fixtureRoot "raw-data.csv") -Value "SHOULD_NOT_BE_INCLUDED"
  Write-Utf8NoBom -Path (Join-Path $fixtureRoot "arbitrary-exfiltration-summary.json") -Value '{"marker":"ARBITRARY_SUMMARY_MUST_NOT_BE_INCLUDED"}'

  $crashLogPath = Join-Path $fixtureRoot "VectorGrid-crash.log"
  $crashText = @"
VectorGrid crash canary
Authorization: Bearer $bearerSecret
ALPACA_API_SECRET=$apiSecret
password=$passwordSecret
token=$githubToken
Cookie: session=$slackToken
connectionString=Server=private-db;Password=$passwordSecret
project_token=$projectToken
webhook_secret=$genericCredentialSecret
client_credential=$genericCredentialSecret
jwt=$jwtToken
signed_url=https://private-user:$passwordSecret@example.test/archive?X-Amz-Signature=$apiSecret
database_url=postgres://private-db-user:$passwordSecret@db.example.test/vectorgrid
input="$privatePath"
forward="$forwardSlashPath"
extended="$extendedPath"
network=$uncPath
unix=$unixPath
wsl=$wslPath
root=$rootPath
opt=$optPath
home=%USERPROFILE%\private\settings.json
$polygonEnvSecret
$alpacaKeyIdEnvSecret
$alpacaSecretEnvSecret
$pemBody
SAFE_CRASH_MARKER
"@
  Write-Utf8NoBom -Path $crashLogPath -Value $crashText

  $reproductionPath = Join-Path $fixtureRoot "reproduction.md"
  Write-Utf8NoBom -Path $reproductionPath -Value @"
# Reproduction

Run with --api-key $apiSecret and inspect "$privatePath".
SAFE_REPRODUCTION_MARKER
"@

  Push-Location $fixtureRoot
  try {
    & $builder `
      -EvidenceRoot $fixtureRoot `
      -CrashLogPath $crashLogPath `
      -ReproductionMetadataPath $reproductionPath `
      -OutDir $bundleRoot
  } finally {
    Pop-Location
  }

  $archivePath = Join-Path $bundleRoot "vectorgrid-support-evidence.zip"
  $payloadRoot = Join-Path $bundleRoot "payload"
  Assert-True -Condition (Test-Path -LiteralPath $archivePath -PathType Leaf) -Message "Canary bundle ZIP was not created."
  Assert-True -Condition (Test-Path -LiteralPath (Join-Path $payloadRoot "support-bundle-manifest.json") -PathType Leaf) -Message "Bundle manifest is missing."
  Assert-True -Condition (Test-Path -LiteralPath (Join-Path $payloadRoot "README.txt") -PathType Leaf) -Message "Bundle privacy/read-before-sharing notice is missing."

  $allPayloadText = (
    Get-ChildItem -LiteralPath $payloadRoot -Recurse -File |
      ForEach-Object { [System.IO.File]::ReadAllText($_.FullName) }
  ) -join [Environment]::NewLine

  foreach ($forbidden in @(
      $apiSecret,
      $bearerSecret,
      $passwordSecret,
      $githubToken,
      $slackToken,
      $projectToken,
      $jwtToken,
      $polygonEnvSecret,
      $alpacaKeyIdEnvSecret,
      $alpacaSecretEnvSecret,
      $genericCredentialSecret,
      "CANARY_PRIVATE_KEY_BODY_5d5b8fe7",
      "Private Person",
      "Unicode User",
      "private-server",
      "private-user",
      "private-wsl-user",
      "private-db",
      "private-db-user",
      "SHOULD_NOT_BE_INCLUDED",
      "ARBITRARY_SUMMARY_MUST_NOT_BE_INCLUDED"
    )) {
    Assert-True -Condition ($allPayloadText.IndexOf($forbidden, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) -Message "Canary leak remained in the payload: $forbidden"
  }
  foreach ($required in @(
      "[redacted]",
      "<local_path>",
      "SAFE_DIAGNOSTIC_MARKER",
      "SAFE_CRASH_MARKER",
      "SAFE_REPRODUCTION_MARKER",
      "preserve-this-value"
    )) {
    Assert-True -Condition ($allPayloadText.IndexOf($required, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) -Message "Expected redacted or diagnostic marker is missing: $required"
  }

  $manifest = Get-Content -Raw -LiteralPath (Join-Path $payloadRoot "support-bundle-manifest.json") | ConvertFrom-Json
  Assert-True -Condition ($manifest.status -eq "complete") -Message "Bundle manifest status is not complete."
  Assert-True -Condition ($manifest.selection.skipped_not_allowlisted_count -ge 3) -Message "Non-allowlisted fixture files were not accounted for."
  Assert-True -Condition (-not [bool]$manifest.policy.binary_inputs_allowed) -Message "Bundle policy unexpectedly allows binary inputs."
  Assert-True -Condition (-not [bool]$manifest.policy.absolute_paths_allowed) -Message "Bundle policy unexpectedly allows absolute paths."
  Assert-True -Condition (-not [bool]$manifest.policy.automatic_upload) -Message "Bundle policy unexpectedly allows automatic upload."
  Assert-True -Condition (-not [bool]$manifest.policy.builder_uploads_archive) -Message "Bundle policy claims that the builder uploads archives."
  Assert-True -Condition ($manifest.policy.caller_context -eq "manual") -Message "Default caller context is not recorded as manual."
  Assert-True -Condition (-not [bool]$manifest.policy.release_authority) -Message "Bundle policy unexpectedly treats support evidence as release authority."
  Assert-True -Condition ([bool]$manifest.policy.review_before_sharing) -Message "Bundle policy does not require review before sharing."

  Expand-Archive -LiteralPath $archivePath -DestinationPath $expandedRoot
  $payloadEntries = @(
    Get-ChildItem -LiteralPath $payloadRoot -Recurse -File |
      ForEach-Object { $_.FullName.Substring($payloadRoot.Length + 1).Replace([char]92, [char]47) } |
      Sort-Object
  )
  $archiveEntries = @(
    Get-ChildItem -LiteralPath $expandedRoot -Recurse -File |
      ForEach-Object { $_.FullName.Substring($expandedRoot.Length + 1).Replace([char]92, [char]47) } |
      Sort-Object
  )
  Assert-True -Condition (($payloadEntries -join "`n") -eq ($archiveEntries -join "`n")) -Message "ZIP contents do not exactly match the redacted payload."

  & $builder `
    -EvidenceRoot $fixtureRoot `
    -OutDir $automaticBundleRoot `
    -CallerContext "ci-required-cpu-quality" `
    -AutomaticUpload
  $automaticManifest = Get-Content -Raw -LiteralPath (Join-Path $automaticBundleRoot "payload\support-bundle-manifest.json") | ConvertFrom-Json
  $automaticReadme = Get-Content -Raw -LiteralPath (Join-Path $automaticBundleRoot "payload\README.txt")
  Assert-True -Condition ([bool]$automaticManifest.policy.automatic_upload) -Message "CI caller automatic-upload intent is missing."
  Assert-True -Condition (-not [bool]$automaticManifest.policy.builder_uploads_archive) -Message "CI bundle incorrectly claims that the builder uploads it."
  Assert-True -Condition ($automaticManifest.policy.automatic_upload_actor -eq "caller") -Message "Automatic-upload actor is not the caller."
  Assert-True -Condition ($automaticManifest.policy.caller_context -eq "ci-required-cpu-quality") -Message "CI caller context is missing."
  Assert-True -Condition ($automaticReadme -match "builder never uploads") -Message "CI README does not distinguish builder behavior."
  Assert-True -Condition ($automaticReadme -match "may automatically upload") -Message "CI README does not disclose caller upload behavior."

  New-Item -ItemType Directory -Force -Path $ciEvidenceRoot | Out-Null
  $now = (Get-Date).ToUniversalTime().ToString("o")
  $ciJsonReports = [ordered]@{
    "frontend-qa-dashboard.json" = [ordered]@{ schema_version = 1; generated_at_utc = $now; status = "passed" }
    "headless-output-parity-manifest.json" = [ordered]@{ schema_version = 2; generated_at_utc = $now; status = "passed" }
    "manifest-toolchain-contract.json" = [ordered]@{ schema_version = 1; generated_at_utc = $now; status = "passed" }
    "node-reproducibility.json" = [ordered]@{ schema_version = 1; generated_at_utc = $now; status = "passed" }
    "release-authority-matrix-contract.json" = [ordered]@{ schema_version = 1; report_kind = "release-authority-matrix-contract"; status = "passed" }
    "release-secret-hygiene.json" = [ordered]@{ schema_version = 1; generated_at_utc = $now; status = "passed" }
    "required-cpu-quality-summary.json" = [ordered]@{
      schema_version = 1
      generated_at_utc = $now
      status = "passed"
      run_id = "focused-canary"
      caller_automatic_upload = $true
    }
  }
  foreach ($entry in $ciJsonReports.GetEnumerator()) {
    Write-Utf8NoBom `
      -Path (Join-Path $ciEvidenceRoot $entry.Key) `
      -Value (($entry.Value | ConvertTo-Json -Depth 8) + [Environment]::NewLine)
  }
  Write-Utf8NoBom `
    -Path (Join-Path $ciEvidenceRoot "frontend-qa-dashboard.md") `
    -Value "# Current-run frontend QA`n`nStatus: passed`n"
  Write-Utf8NoBom `
    -Path (Join-Path $ciEvidenceRoot "old-arbitrary-summary.json") `
    -Value '{"marker":"STALE_ARBITRARY_SUMMARY"}'

  & $builder `
    -EvidenceRoot $ciEvidenceRoot `
    -OutDir $ciEvidenceOut `
    -CallerContext "github-actions-required-cpu-quality" `
    -AutomaticUpload
  $ciBundleManifest = Get-Content -Raw -LiteralPath (Join-Path $ciEvidenceOut "payload\support-bundle-manifest.json") | ConvertFrom-Json
  $ciEvidenceTypes = @($ciBundleManifest.files | ForEach-Object { $_.evidence_type })
  foreach ($requiredType in @(
      "frontend-qa-dashboard",
      "headless-output-parity-manifest",
      "manifest-toolchain-contract",
      "node-reproducibility",
      "release-authority-matrix-contract",
      "required-cpu-quality-summary",
      "secret-hygiene",
      "system-metadata"
    )) {
    Assert-True -Condition ($ciEvidenceTypes -contains $requiredType) -Message "Focused current-run bundle is missing $requiredType."
  }
  $ciPayloadText = (
    Get-ChildItem -LiteralPath (Join-Path $ciEvidenceOut "payload") -Recurse -File |
      ForEach-Object { [System.IO.File]::ReadAllText($_.FullName) }
  ) -join [Environment]::NewLine
  Assert-True -Condition ($ciPayloadText -notmatch "STALE_ARBITRARY_SUMMARY") -Message "Focused current-run bundle included an arbitrary summary."
  Assert-True -Condition ([bool]$ciBundleManifest.policy.automatic_upload) -Message "Focused current-run bundle did not mark CI caller upload."

  $unsupportedPath = Join-Path $fixtureRoot "crash.dmp"
  Write-Utf8NoBom -Path $unsupportedPath -Value "not a real dump"
  $unsupportedRejected = $false
  try {
    & $builder -CrashLogPath $unsupportedPath -OutDir $negativeRoot
  } catch {
    $unsupportedRejected = $_.Exception.Message -match "unsupported extension"
  }
  Assert-True -Condition $unsupportedRejected -Message "An explicitly selected binary crash-dump extension was not rejected."

  $outsideRoot = Join-Path $repoRoot ("output\support-evidence-evil\" + [guid]::NewGuid().ToString("N"))
  $outsideRejected = $false
  try {
    & $builder -EvidenceRoot $fixtureRoot -OutDir $outsideRoot
  } catch {
    $outsideRejected = $_.Exception.Message -match "must be a child"
  }
  Assert-True -Condition $outsideRejected -Message "A lexical sibling of the fixed support-evidence root was not rejected."

  $malformedRoot = Join-Path $scratchRoot "malformed-fixture"
  $malformedOut = Join-Path $scratchRoot "malformed-output"
  Write-Utf8NoBom -Path (Join-Path $malformedRoot "manifest.json") -Value '{ "schema_version": 1, broken'
  $malformedRejected = $false
  try {
    & $builder -EvidenceRoot $malformedRoot -OutDir $malformedOut
  } catch {
    $malformedRejected = $_.Exception.Message -match "Refusing malformed JSON"
  }
  Assert-True -Condition $malformedRejected -Message "Malformed allowlisted JSON did not fail closed."

  New-Item -ItemType Directory -Force -Path $fakeManifestRoot | Out-Null
  Write-Utf8NoBom -Path (Join-Path $fakeManifestRoot "readiness.json") -Value '{"supported":true}'
  $fakeManifest = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = "cpu"
    artifact_source_kind = "bundle"
    product_name = "VectorGrid"
    version = "1.2.3"
    market_data = @(
      [ordered]@{ ticker = "PRIVATE"; close = 123.45 }
    )
    staged_artifacts = @(
      [ordered]@{
        staged_name = "vectorgrid.exe"
        size_bytes = 1
        sha256 = "b" * 64
      }
    )
  }
  Write-Utf8NoBom -Path (Join-Path $fakeManifestRoot "manifest.json") -Value (($fakeManifest | ConvertTo-Json -Depth 8) + [Environment]::NewLine)
  $fakeManifestRejected = $false
  try {
    & $builder -EvidenceRoot $fakeManifestRoot -OutDir $fakeManifestOut
  } catch {
    $fakeManifestRejected = $_.Exception.Message -match "fixed release schema|non-release data field"
  }
  Assert-True -Condition $fakeManifestRejected -Message "A fake manifest containing market data was not rejected."
  Assert-True -Condition (-not (Test-Path -LiteralPath (Join-Path $fakeManifestOut "vectorgrid-support-evidence.zip") -PathType Leaf)) -Message "A fake market-data manifest was included in a bundle."

  New-Item -ItemType Directory -Force -Path $textValidationRoot | Out-Null
  $invalidUtf8Path = Join-Path $textValidationRoot "invalid-utf8.log"
  [System.IO.File]::WriteAllBytes($invalidUtf8Path, [byte[]]@(0x56, 0x47, 0xc3, 0x28))
  $invalidUtf8Rejected = $false
  try {
    & $builder -CrashLogPath $invalidUtf8Path -OutDir (Join-Path $scratchRoot "invalid-utf8-output")
  } catch {
    $invalidUtf8Rejected = $_.Exception.Message -match "strict UTF-8"
  }
  Assert-True -Condition $invalidUtf8Rejected -Message "Invalid UTF-8 text evidence did not fail closed."

  $replacementPath = Join-Path $textValidationRoot "replacement.log"
  Write-Utf8NoBom -Path $replacementPath -Value "disguised$([char]0xfffd)text"
  $replacementRejected = $false
  try {
    & $builder -CrashLogPath $replacementPath -OutDir (Join-Path $scratchRoot "replacement-output")
  } catch {
    $replacementRejected = $_.Exception.Message -match "replacement characters"
  }
  Assert-True -Condition $replacementRejected -Message "A Unicode replacement character was not rejected."

  $binaryLogPath = Join-Path $textValidationRoot "disguised-binary.log"
  [System.IO.File]::WriteAllBytes(
    $binaryLogPath,
    [byte[]]@(0x56, 0x47, 0x01, 0x02, 0x03, 0x04, 0x05, 0x0a)
  )
  $binaryLogRejected = $false
  try {
    & $builder -CrashLogPath $binaryLogPath -OutDir (Join-Path $scratchRoot "binary-log-output")
  } catch {
    $binaryLogRejected = $_.Exception.Message -match "excessive control characters"
  }
  Assert-True -Condition $binaryLogRejected -Message "A disguised binary .log file was not rejected."

  $builderText = Get-Content -Raw -LiteralPath $builder
  foreach ($forbiddenCollector in @(
      "Invoke-WebRequest",
      "Invoke-RestMethod",
      "Start-BitsTransfer",
      "Get-ChildItem Env:",
      "Get-ComputerInfo",
      "Get-WinEvent"
    )) {
    Assert-True -Condition ($builderText.IndexOf($forbiddenCollector, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) -Message "Builder contains forbidden collector or network primitive: $forbiddenCollector"
  }
  $cpuCiGateText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "run_cpu_ci_gate.ps1")
  Assert-True -Condition ($cpuCiGateText -match '-EvidenceRoot\s+\$currentEvidenceRoot') -Message "CPU CI does not pass its explicit current-run evidence root to the builder."
  Assert-True -Condition ($cpuCiGateText -match '-AutomaticUpload') -Message "CPU CI does not mark GitHub Actions caller upload."
  Assert-True -Condition ($cpuCiGateText -match 'required-cpu-quality-summary\.json') -Message "CPU CI does not emit its fixed current-run summary."
  Assert-True -Condition ($cpuCiGateText -notmatch '-EvidenceRoot\s+\(Join-Path\s+\$repoRoot\s+["'']output[\\/]qa["'']\)') -Message "CPU CI still bundles the accumulated output/qa directory."

  Write-Host "Support-evidence canaries passed."
  Write-Host "  fixed allowlist excluded arbitrary JSON and CSV"
  Write-Host "  fake market-data manifest was rejected by the release schema"
  Write-Host "  credential aliases, bare environment secrets, tokens, private keys, and absolute paths were redacted"
  Write-Host "  safe diagnostic content survived"
  Write-Host "  ZIP entries exactly matched the redacted payload"
  Write-Host "  caller context and CI automatic-upload disclosure were accurate"
  Write-Host "  focused eight-report current-run CI staging and bundling passed without Cargo or frontend execution"
  Write-Host "  binary crash dump, invalid UTF-8, replacement characters, and disguised binary log were rejected"
  Write-Host "  output-root escape and malformed JSON were rejected"
} finally {
  foreach ($name in $sensitiveEnvironmentNames) {
    $value = $savedSensitiveEnvironment[$name]
    if ($null -eq $value) {
      Remove-Item "Env:$name" -ErrorAction SilentlyContinue
    } else {
      [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
  }
  if ($KeepArtifacts) {
    Write-Host "Canary artifacts retained at $scratchRoot"
  } elseif (
    (Test-Path -LiteralPath $scratchRoot) -and
    $scratchRoot.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)
  ) {
    Remove-Item -LiteralPath $scratchRoot -Recurse -Force
  }
}
