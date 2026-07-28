param(
  [Parameter(Mandatory = $true)][ValidateSet(
    "positive",
    "spoof",
    "stale",
    "dead-owner",
    "acl-denied",
    "released-mutex",
    "token-mismatch"
  )]
  [string]$Mode,
  [Parameter(Mandatory = $true)][string]$RepoRoot,
  [Parameter(Mandatory = $true)][string]$LeasePath,
  [Parameter(Mandatory = $true)][string]$Nonce,
  [Parameter(Mandatory = $true)][string]$RunId,
  [Parameter(Mandatory = $true)][string]$LeaseModule
)

$ErrorActionPreference = "Stop"
$authorityMutexName = "Local\ta_desktop_demo_frontend_interaction_authority"
$benchmarkMutexName = "Local\ta_desktop_demo_benchmark_gate"
$context = "scoped warm mocked-browser click-to-painted-stable latency"
$token = [guid]::NewGuid().ToString("N")
$mutex = $null
$benchmarkMutex = $null
$stream = $null
$benchmarkLeaseStream = $null

function Get-Sha256Text {
  param([Parameter(Mandatory = $true)][string]$Value)
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return (
      [System.BitConverter]::ToString($sha.ComputeHash($bytes))
    ).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha.Dispose()
  }
}

function Get-Sha256Bytes {
  param([Parameter(Mandatory = $true)][byte[]]$Bytes)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return (
      [System.BitConverter]::ToString($sha.ComputeHash($Bytes))
    ).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha.Dispose()
  }
}

try {
  if ($Mode -ne "spoof") {
    $mutex = [System.Threading.Mutex]::new($false, $authorityMutexName)
    if (-not $mutex.WaitOne([TimeSpan]::FromSeconds(5))) {
      throw "test helper could not acquire authority mutex"
    }
    $benchmarkMutex = [System.Threading.Mutex]::new($false, $benchmarkMutexName)
    if (-not $benchmarkMutex.WaitOne([TimeSpan]::FromSeconds(5))) {
      throw "test helper could not acquire benchmark mutex"
    }
  }

  $ownerPid = if ($Mode -eq "dead-owner") { 2147480000 } else { $PID }
  $ownerStartedAt = if ($Mode -eq "dead-owner") {
    "2000-01-01T00:00:00.0000000Z"
  } else {
    (Get-Process -Id $PID).StartTime.ToUniversalTime().ToString("o")
  }
  $createdAt = if ($Mode -eq "stale") {
    [DateTimeOffset]::UtcNow.AddHours(-4).ToString("o")
  } else {
    [DateTimeOffset]::UtcNow.ToString("o")
  }
  $benchmarkLeasePath = Join-Path (
    Join-Path $RepoRoot "output\qa\benchmark-lock"
  ) "active.json"
  $benchmarkLeaseIdentity = "01234567:89abcdef01234567"
  $benchmarkLeaseSha256 = ""
  if ($Mode -ne "spoof") {
    New-Item -ItemType Directory -Force -Path (
      Split-Path -Parent $benchmarkLeasePath
    ) | Out-Null
    $benchmarkLeasePayload = [ordered]@{
      schema = "ta_desktop_demo_benchmark_lock_active_lease_v1"
      acquisition_id = [guid]::NewGuid().ToString("N")
      created_at = [DateTimeOffset]::UtcNow.ToString("o")
      owner_pid = $PID
      owner_started_at = (
        (Get-Process -Id $PID).StartTime.ToUniversalTime().ToString("o")
      )
      context = $context
      token_sha256 = Get-Sha256Text -Value $token
      mutex_name = $benchmarkMutexName
      released = $false
      file_identity = $benchmarkLeaseIdentity
    }
    $benchmarkLeaseBytes = [System.Text.UTF8Encoding]::new($false).GetBytes(
      ($benchmarkLeasePayload | ConvertTo-Json -Compress)
    )
    $benchmarkLeaseSha256 = Get-Sha256Bytes `
      -Bytes $benchmarkLeaseBytes
    $benchmarkLeaseStream = [System.IO.FileStream]::new(
      $benchmarkLeasePath,
      [System.IO.FileMode]::CreateNew,
      [System.IO.FileAccess]::ReadWrite,
      [System.IO.FileShare]::Read
    )
    $benchmarkLeaseStream.Write(
      $benchmarkLeaseBytes,
      0,
      $benchmarkLeaseBytes.Length
    )
    $benchmarkLeaseStream.Flush($true)
    $benchmarkLeaseStream.Position = 0
  }
  $payload = [ordered]@{
    schema_version = 1
    lease_kind = "frontend-interaction-authority-lease"
    created_at = $createdAt
    run_id = $RunId
    nonce = $Nonce
    wrapper_path = "scripts/run_frontend_interaction_latency.ps1"
    wrapper_context = $context
    wrapper_pid = $PID
    wrapper_started_at = (
      (Get-Process -Id $PID).StartTime.ToUniversalTime().ToString("o")
    )
    mutex_name = $authorityMutexName
    mutex_owner_pid = $ownerPid
    mutex_owner_started_at = $ownerStartedAt
    mutex_ownership_mode = "acquired"
    benchmark_mutex_name = $benchmarkMutexName
    benchmark_mutex_owner_pid = $PID
    benchmark_mutex_owner_started_at = (
      (Get-Process -Id $PID).StartTime.ToUniversalTime().ToString("o")
    )
    benchmark_mutex_ownership_mode = "acquired"
    benchmark_lock_context = $context
    benchmark_lock_token_sha256 = Get-Sha256Text -Value $token
    benchmark_lease_path = "output/qa/benchmark-lock/active.json"
    benchmark_lease_sha256 = $benchmarkLeaseSha256
    benchmark_lease_identity = $benchmarkLeaseIdentity
    benchmark_lease_write_probe_win32_error_code = 32
  }
  $json = ($payload | ConvertTo-Json -Depth 5) + "`n"
  $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($json)
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LeasePath) |
    Out-Null
  if ($Mode -in @("spoof", "acl-denied")) {
    [System.IO.File]::WriteAllBytes($LeasePath, $bytes)
    if ($Mode -eq "acl-denied") {
      $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
      & icacls.exe $LeasePath /deny "${identity}:(WD)" | Out-Null
      if ($LASTEXITCODE -ne 0) {
        throw "failed to install the ACL-denial lease canary"
      }
    }
  } else {
    $stream = [System.IO.FileStream]::new(
      $LeasePath,
      [System.IO.FileMode]::CreateNew,
      [System.IO.FileAccess]::ReadWrite,
      [System.IO.FileShare]::Read
    )
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Flush($true)
    $stream.Position = 0
  }

  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD = "1"
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID = "$PID"
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT = (
    (Get-Process -Id $PID).StartTime.ToUniversalTime().ToString("o")
  )
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT = $context
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN = $token
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256 = Get-Sha256Text `
    -Value $token
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH = $benchmarkLeasePath
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256 = $benchmarkLeaseSha256
  $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME = $benchmarkMutexName
  if ($Mode -eq "token-mismatch") {
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN = [guid]::NewGuid().ToString("N")
  }
  if ($Mode -eq "released-mutex") {
    $mutex.ReleaseMutex() | Out-Null
    $mutex.Dispose()
    $mutex = $null
  }
  & node $LeaseModule probe `
    --repo-root $RepoRoot `
    --lease $LeasePath `
    --nonce $Nonce `
    --run-id $RunId `
    --max-age-minutes 30
  exit $LASTEXITCODE
} finally {
  if ($null -ne $stream) {
    $stream.Dispose()
  }
  if ($null -ne $benchmarkLeaseStream) {
    $benchmarkLeaseStream.Dispose()
  }
  if ($null -ne $mutex) {
    try {
      $mutex.ReleaseMutex() | Out-Null
    } catch {
    }
    $mutex.Dispose()
  }
  if ($null -ne $benchmarkMutex) {
    try {
      $benchmarkMutex.ReleaseMutex() | Out-Null
    } catch {
    }
    $benchmarkMutex.Dispose()
  }
}
