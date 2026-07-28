param(
  [Parameter(Mandatory = $true)]
  [ValidateSet(
    "inherit",
    "nested",
    "holder",
    "direct",
    "spoof-holder",
    "wait-inherited"
  )]
  [string]$Mode,
  [Parameter(Mandatory = $true)]
  [string]$RepoRoot,
  [string]$ReadyPath = "",
  [string]$StopPath = "",
  [string]$DescriptorPath = "",
  [string]$ResultPath = "",
  [string]$MutexName = "Local\ta_desktop_demo_benchmark_gate",
  [int]$TimeoutSeconds = 3
)

$ErrorActionPreference = "Stop"
$lockScript = Join-Path $RepoRoot "scripts\benchmark_lock.ps1"
. $lockScript

function Get-ContractEnvironment {
  $values = [ordered]@{}
  foreach ($name in $script:BenchmarkLockEnvironmentNames) {
    $values[$name] = [System.Environment]::GetEnvironmentVariable($name, "Process")
  }
  return $values
}

function Write-ContractResult {
  param([Parameter(Mandatory = $true)]$Value)

  $json = $Value | ConvertTo-Json -Depth 10 -Compress
  if (-not [string]::IsNullOrWhiteSpace($ResultPath)) {
    $parent = Split-Path -Parent $ResultPath
    [System.IO.Directory]::CreateDirectory($parent) | Out-Null
    [System.IO.File]::WriteAllText(
      $ResultPath,
      $json,
      [System.Text.UTF8Encoding]::new($false)
    )
  }
  Write-Output ("BENCHMARK_LOCK_CONTRACT_JSON:" + $json)
}

function Get-ChildContractResult {
  param([Parameter(Mandatory = $true)][object[]]$Output)

  $prefix = "BENCHMARK_LOCK_CONTRACT_JSON:"
  $lines = @($Output | Where-Object { "$_".StartsWith($prefix) })
  if ($lines.Count -ne 1) {
    throw "Nested benchmark-lock contract emitted $($lines.Count) result lines."
  }
  return "$($lines[0])".Substring($prefix.Length) | ConvertFrom-Json
}

function Write-ReadyPayload {
  param([Parameter(Mandatory = $true)]$Value)

  if ([string]::IsNullOrWhiteSpace($ReadyPath)) {
    throw "Holder mode requires ReadyPath."
  }
  $parent = Split-Path -Parent $ReadyPath
  [System.IO.Directory]::CreateDirectory($parent) | Out-Null
  $json = $Value | ConvertTo-Json -Depth 10 -Compress
  [System.IO.File]::WriteAllText(
    $ReadyPath,
    $json,
    [System.Text.UTF8Encoding]::new($false)
  )
}

function Wait-ForStopSignal {
  if ([string]::IsNullOrWhiteSpace($StopPath)) {
    throw "Holder mode requires StopPath."
  }
  $timer = [System.Diagnostics.Stopwatch]::StartNew()
  while (-not (Test-Path -LiteralPath $StopPath)) {
    if ($timer.Elapsed.TotalSeconds -ge 20) {
      throw "Timed out waiting for the benchmark-lock contract stop signal."
    }
    Start-Sleep -Milliseconds 20
  }
}

try {
  switch ($Mode) {
    "inherit" {
      $before = Get-ContractEnvironment
      $handle = Enter-BenchmarkLock `
        -Context "benchmark lock contract child" `
        -MutexName $MutexName `
        -TimeoutSeconds $TimeoutSeconds
      try {
        $during = Get-ContractEnvironment
        $result = [ordered]@{
          ok = $true
          acquired = [bool]$handle.acquired
          inherited = [bool]$handle.inherited
          owner_pid = [int]$handle.owner_pid
          owner_started_at = [string]$handle.owner_started_at
          lease_path = [string]$handle.lease_path
          lease_sha256 = [string]$handle.lease_sha256
          lease_identity = [string]$handle.lease_identity
          write_probe_win32_error_code =
            [int]$handle.write_probe_win32_error_code
          before = $before
          during = $during
        }
      } finally {
        Exit-BenchmarkLock -Handle $handle
      }
      $result.after = Get-ContractEnvironment
      Write-ContractResult -Value $result
    }

    "nested" {
      $before = Get-ContractEnvironment
      $handle = Enter-BenchmarkLock `
        -Context "benchmark lock nested contract" `
        -MutexName $MutexName `
        -TimeoutSeconds $TimeoutSeconds
      try {
        $during = Get-ContractEnvironment
        $childScript = $MyInvocation.MyCommand.Path
        $grandchildOutput = @(
          & powershell -NoProfile -ExecutionPolicy Bypass `
            -File $childScript `
            -Mode "inherit" `
            -RepoRoot $RepoRoot `
            -MutexName $MutexName `
            -TimeoutSeconds $TimeoutSeconds
        )
        if ($LASTEXITCODE -ne 0) {
          throw "Grandchild inheritance process failed with exit code $LASTEXITCODE."
        }
        $grandchild = Get-ChildContractResult -Output $grandchildOutput
      } finally {
        Exit-BenchmarkLock -Handle $handle
      }
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          acquired = [bool]$handle.acquired
          inherited = [bool]$handle.inherited
          before = $before
          during = $during
          after = Get-ContractEnvironment
          grandchild = $grandchild
        })
    }

    "holder" {
      foreach ($name in $script:BenchmarkLockEnvironmentNames) {
        Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
      }
      $handle = Enter-BenchmarkLock `
        -Context "benchmark lock contract holder" `
        -MutexName $MutexName `
        -TimeoutSeconds $TimeoutSeconds
      try {
        Write-ReadyPayload -Value ([ordered]@{
            ok = $true
            actual_pid = $PID
            actual_started_at = Get-BenchmarkLockProcessStartIdentity `
              -ProcessId $PID
            environment = Get-ContractEnvironment
          })
        Wait-ForStopSignal
      } finally {
        Exit-BenchmarkLock -Handle $handle
      }
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          released = $true
        })
    }

    "direct" {
      foreach ($name in $script:BenchmarkLockEnvironmentNames) {
        Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
      }
      $handle = Enter-BenchmarkLock `
        -Context "benchmark lock independent direct contract" `
        -MutexName $MutexName `
        -TimeoutSeconds $TimeoutSeconds
      try {
        $lease = Read-BenchmarkLockLease -Path $handle.lease_path
        $result = [ordered]@{
          ok = $true
          acquired = [bool]$handle.acquired
          inherited = [bool]$handle.inherited
          acquisition_id = [string]$lease.payload.acquisition_id
          lease_path = [string]$handle.lease_path
          lease_identity = [string]$handle.lease_identity
        }
      } finally {
        Exit-BenchmarkLock -Handle $handle
      }
      Write-ContractResult -Value $result
    }

    "spoof-holder" {
      if (
        [string]::IsNullOrWhiteSpace($DescriptorPath) -or
        -not (Test-Path -LiteralPath $DescriptorPath -PathType Leaf)
      ) {
        throw "Spoof-holder mode requires an existing DescriptorPath."
      }
      $descriptor = Get-Content -Raw -LiteralPath $DescriptorPath |
        ConvertFrom-Json
      $mutex = [System.Threading.Mutex]::new($false, $MutexName)
      $mutexAcquired = $false
      $leaseStream = $null
      try {
        $mutexAcquired = $mutex.WaitOne(
          [TimeSpan]::FromSeconds($TimeoutSeconds)
        )
        if (-not $mutexAcquired) {
          throw "Spoof-holder could not acquire its secondary named mutex."
        }
        $leaseDirectory = Get-BenchmarkLockLeaseDirectory
        $leasePath = Assert-BenchmarkLockLeasePath `
          -Path (Join-Path $leaseDirectory "active.json")
        $leaseStream = Open-BenchmarkLockActiveLease `
          -Path $leasePath `
          -TimeoutSeconds $TimeoutSeconds
        $identity = [VectorGrid.BenchmarkLockNative]::GetFileIdentity(
          $leaseStream.SafeFileHandle
        )
        $tokenSha256 = Get-BenchmarkLockSha256Text `
          -Value ([string]$descriptor.token)
        $payload = [ordered]@{
          schema = $script:BenchmarkLockLeaseSchema
          acquisition_id = [guid]::NewGuid().ToString("N")
          created_at = (Get-Date).ToUniversalTime().ToString("o")
          owner_pid = [int]$descriptor.claimed_owner_pid
          owner_started_at = [string]$descriptor.claimed_owner_started_at
          context = [string]$descriptor.context
          token_sha256 = $tokenSha256
          mutex_name = $MutexName
          released = $false
          file_identity = $identity
        }
        $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes(
          ($payload | ConvertTo-Json -Compress)
        )
        $leaseSha256 = Get-BenchmarkLockSha256Bytes -Bytes $bytes
        $leaseStream.SetLength(0)
        $leaseStream.Position = 0
        $leaseStream.Write($bytes, 0, $bytes.Length)
        $leaseStream.Flush($true)

        Write-ReadyPayload -Value ([ordered]@{
            ok = $true
            actual_pid = $PID
            actual_started_at = Get-BenchmarkLockProcessStartIdentity `
              -ProcessId $PID
            environment = [ordered]@{
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD = "1"
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID =
                "$([int]$descriptor.claimed_owner_pid)"
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT =
                [string]$descriptor.claimed_owner_started_at
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT =
                [string]$descriptor.context
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN =
                [string]$descriptor.token
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256 = $tokenSha256
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH = $leasePath
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256 = $leaseSha256
              TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME = $MutexName
            }
          })
        Wait-ForStopSignal
      } finally {
        if ($null -ne $leaseStream) {
          $leaseStream.Dispose()
        }
        if ($mutexAcquired) {
          $mutex.ReleaseMutex() | Out-Null
        }
        $mutex.Dispose()
      }
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          released = $true
        })
    }

    "wait-inherited" {
      $handle = Enter-BenchmarkLock `
        -Context "benchmark lock delayed inherited contract" `
        -MutexName $MutexName `
        -TimeoutSeconds $TimeoutSeconds
      Write-ReadyPayload -Value ([ordered]@{
          ok = $true
          inherited = [bool]$handle.inherited
          owner_pid = [int]$handle.owner_pid
        })
      Wait-ForStopSignal
      try {
        Exit-BenchmarkLock -Handle $handle
        $result = [ordered]@{
          ok = $true
          exit_revalidated = $true
        }
      } catch {
        $result = [ordered]@{
          ok = $false
          exit_revalidated = $false
          error = $_.Exception.Message
        }
      }
      Write-ContractResult -Value $result
    }
  }
} catch {
  Write-ContractResult -Value ([ordered]@{
      ok = $false
      mode = $Mode
      error = $_.Exception.Message
      environment = Get-ContractEnvironment
    })
}
