param(
  [ValidateSet("all", "polygon", "alpaca")]
  [string]$Providers = "all",
  [switch]$IncludeCredentialed,
  [switch]$RequireCredentialedEvidence,
  [ValidateRange(250, 120000)]
  [int]$TimeoutMs = 12000,
  [ValidatePattern("^[A-Z][A-Z0-9.-]{0,14}$")]
  [string]$PolygonSymbol = "AAPL",
  [string]$OutDir = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$runId = "$([DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ'))-$([guid]::NewGuid().ToString('N'))"
$providerEvidenceParent = [System.IO.Path]::GetFullPath(
  (Join-Path $repoRoot "output\qa\provider-sandbox")
)
$resolvedOutDir = if ([string]::IsNullOrWhiteSpace($OutDir)) {
  Join-Path $providerEvidenceParent $runId
} else {
  [System.IO.Path]::GetFullPath($OutDir)
}

if ([string]::IsNullOrWhiteSpace($OutDir)) {
  $providerPrefix = $providerEvidenceParent.TrimEnd([char]92, [char]47) +
    [System.IO.Path]::DirectorySeparatorChar
  if (
    -not $resolvedOutDir.StartsWith(
      $providerPrefix,
      [System.StringComparison]::OrdinalIgnoreCase
    )
  ) {
    throw "The generated provider sandbox evidence directory escaped its fixed parent."
  }
}
if (Test-Path -LiteralPath $resolvedOutDir) {
  throw "Refusing to reuse an existing provider sandbox evidence directory: $resolvedOutDir"
}
New-Item -ItemType Directory -Path $resolvedOutDir | Out-Null

$offlineStdout = Join-Path $resolvedOutDir "offline-contract-tests.stdout.txt"
$offlineStderr = Join-Path $resolvedOutDir "offline-contract-tests.stderr.txt"
$offlineSummaryPath = Join-Path $resolvedOutDir "offline-contract-summary.json"
$preflightSummaryPath = Join-Path $resolvedOutDir "provider-sandbox-summary.json"
$wrapperSummaryPath = Join-Path $resolvedOutDir "provider-sandbox-qa-summary.json"

function Write-Utf8NoBom {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value
  )
  [System.IO.File]::WriteAllText(
    $Path,
    (($Value | ConvertTo-Json -Depth 20) + [Environment]::NewLine),
    (New-Object System.Text.UTF8Encoding($false))
  )
}

function Invoke-NodeProcess {
  param(
    [Parameter(Mandatory = $true)][string[]]$Arguments,
    [Parameter(Mandatory = $true)][string]$StdoutPath,
    [Parameter(Mandatory = $true)][string]$StderrPath
  )
  Push-Location $repoRoot
  try {
    & node @Arguments 1> $StdoutPath 2> $StderrPath
    return $LASTEXITCODE
  } finally {
    Pop-Location
  }
}

$startedAtUtc = [DateTime]::UtcNow
$offlineExitCode = -1
$preflightExitCode = -1
$preflightStatus = "not_run"
$failureMessage = ""

try {
  Write-Host "==> Offline provider sandbox contract tests (local mocks only)"
  $offlineExitCode = Invoke-NodeProcess `
    -Arguments @("--test", "scripts/provider_sandbox_preflight.test.mjs") `
    -StdoutPath $offlineStdout `
    -StderrPath $offlineStderr
  Write-Utf8NoBom -Path $offlineSummaryPath -Value ([ordered]@{
      schema_version = 1
      generated_at_utc = [DateTime]::UtcNow.ToString("o")
      status = if ($offlineExitCode -eq 0) { "passed" } else { "failed" }
      exit_code = $offlineExitCode
      network_mode = "loopback-mocks-only"
      live_orders_possible = $false
    })
  if ($offlineExitCode -ne 0) {
    throw "Offline provider sandbox contract tests failed with exit code $offlineExitCode."
  }
  Write-Host "[PASS] Offline provider sandbox contract tests"

  if ($RequireCredentialedEvidence) {
    $IncludeCredentialed = $true
  }

  $preflightArgs = @(
    "scripts/provider_sandbox_preflight.mjs",
    "--providers", $Providers,
    "--timeout-ms", "$TimeoutMs",
    "--polygon-symbol", $PolygonSymbol,
    "--out", $preflightSummaryPath
  )
  if ($IncludeCredentialed) {
    $preflightArgs += "--allow-network"
    Write-Host "==> Credentialed provider preflight: safe GET requests only"
  } else {
    Write-Host "==> Credential awareness check (network disabled by default)"
  }
  if ($RequireCredentialedEvidence) {
    $preflightArgs += "--require-evidence"
  }

  $preflightStdout = Join-Path $resolvedOutDir "provider-preflight.stdout.txt"
  $preflightStderr = Join-Path $resolvedOutDir "provider-preflight.stderr.txt"
  $preflightExitCode = Invoke-NodeProcess `
    -Arguments $preflightArgs `
    -StdoutPath $preflightStdout `
    -StderrPath $preflightStderr
  if (-not (Test-Path -LiteralPath $preflightSummaryPath -PathType Leaf)) {
    throw "Provider preflight did not produce its evidence summary."
  }
  $preflight = Get-Content -Raw -LiteralPath $preflightSummaryPath |
    ConvertFrom-Json -ErrorAction Stop
  $preflightStatus = [string]$preflight.status
  if ($preflightExitCode -ne 0) {
    throw "Provider preflight failed with exit code $preflightExitCode; inspect the redacted evidence summary."
  }
  Write-Host "[PASS] Provider preflight status: $preflightStatus"
} catch {
  $failureMessage = $_.Exception.Message
} finally {
  $finishedAtUtc = [DateTime]::UtcNow
  $wrapperStatus = if (
    $offlineExitCode -eq 0 -and
    $preflightExitCode -eq 0 -and
    [string]::IsNullOrWhiteSpace($failureMessage)
  ) {
    if ($preflightStatus -eq "passed") { "passed" } else { "skipped" }
  } else {
    "failed"
  }
  Write-Utf8NoBom -Path $wrapperSummaryPath -Value ([ordered]@{
      schema_version = 1
      report_kind = "provider-sandbox-qa"
      run_id = $runId
      generated_at_utc = $finishedAtUtc.ToString("o")
      started_at_utc = $startedAtUtc.ToString("o")
      status = $wrapperStatus
      providers = $Providers
      credentialed_network_requested = [bool]$IncludeCredentialed
      credentialed_evidence_required = [bool]$RequireCredentialedEvidence
      offline_contract_exit_code = $offlineExitCode
      credentialed_preflight_exit_code = $preflightExitCode
      credentialed_preflight_status = $preflightStatus
      safety_contract = [ordered]@{
        alpaca_environment = "paper-only"
        live_alpaca_host_allowed = $false
        request_methods = @("GET")
        order_submission_allowed = $false
        raw_provider_payloads_persisted = $false
        credential_values_persisted = $false
      }
      error = $failureMessage
    })
}

Write-Host "Provider sandbox QA evidence: $resolvedOutDir"
if (-not [string]::IsNullOrWhiteSpace($failureMessage)) {
  throw $failureMessage
}
