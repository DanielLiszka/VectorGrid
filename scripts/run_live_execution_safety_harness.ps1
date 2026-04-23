param(
  [string]$OutDir = ""
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
  return Join-Path (Join-Path $repoRoot "output\\release_candidates") "${stamp}-live-safety"
}

function Invoke-TestCase {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  Write-Host "==> Live safety test: $Name"
  $stdoutPath = [System.IO.Path]::GetTempFileName()
  $stderrPath = [System.IO.Path]::GetTempFileName()
  try {
    $proc = Start-Process -FilePath "cargo" `
      -ArgumentList @("test", "-p", "ta_desktop_demo_app", "--lib", $Name, "--", "--exact", "--nocapture") `
      -NoNewWindow `
      -Wait `
      -PassThru `
      -RedirectStandardOutput $stdoutPath `
      -RedirectStandardError $stderrPath
    $output = @()
    if (Test-Path $stdoutPath) {
      $output += Get-Content $stdoutPath
    }
    if (Test-Path $stderrPath) {
      $output += Get-Content $stderrPath
    }
    foreach ($line in $output) {
      Write-Host $line
    }
    if ($proc.ExitCode -ne 0) {
      throw "Live safety test '$Name' failed with exit code $($proc.ExitCode)"
    }
  } finally {
    Remove-Item $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
  }
  [ordered]@{
    name = $Name
    passed = $true
    exit_code = 0
  }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null
$summaryPath = Join-Path $resolvedOutDir "live-safety-summary.json"

$tests = @(
  "commands::commands_execution::tests::alpaca_credential_storage_status_tracks_presence_bits",
  "commands::commands_execution::tests::resolve_alpaca_credentials_reads_stored_pair_by_default",
  "commands::commands_execution::tests::resolve_alpaca_credentials_prefers_complete_typed_override",
  "commands::commands_execution::tests::normalize_execution_safety_symbol_requires_non_blank_symbol",
  "commands::commands_execution::tests::ensure_runner_inactive_for_broker_safety_action_rejects_active_runner",
  "commands::commands_execution::tests::emergency_halt_status_message_reflects_runner_and_feed_state"
)

$results = @()
try {
  foreach ($test in $tests) {
    $results += Invoke-TestCase -Name $test
  }

  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    passed = $true
    tests = $results
  }
  Write-JsonUtf8 -Path $summaryPath -Value $summary
  Write-Host "Live execution safety harness summary written to $summaryPath"
} catch {
  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    passed = $false
    error = $_.Exception.Message
    tests = $results
  }
  Write-JsonUtf8 -Path $summaryPath -Value $summary
  throw
}
