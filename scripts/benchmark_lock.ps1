function Enter-BenchmarkLock {
  param(
    [string]$Context = "benchmark gate",
    [int]$TimeoutSeconds = 900,
    [string]$MutexName = "Local\ta_desktop_demo_benchmark_gate"
  )

  if ($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD -eq "1") {
    Write-Host "Benchmark lock inherited for $Context from PID $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID."
    return [pscustomobject]@{
      acquired = $false
      inherited = $true
      abandoned = $false
      mutex = $null
      previous_held = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD
      previous_owner_pid = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID
      previous_context = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT
    }
  }

  $mutex = [System.Threading.Mutex]::new($false, $MutexName)
  $acquired = $false
  $abandoned = $false
  try {
    Write-Host "Waiting for benchmark lock ($Context)..."
    try {
      $acquired = $mutex.WaitOne([TimeSpan]::FromSeconds($TimeoutSeconds))
    } catch [System.Threading.AbandonedMutexException] {
      $acquired = $true
      $abandoned = $true
      Write-Host "[WARN] Benchmark lock was abandoned by a prior process; continuing with recovered lock ($Context)."
    }
    if (-not $acquired) {
      throw "Timed out waiting for benchmark lock after ${TimeoutSeconds}s ($Context). Another benchmark/profiler gate may still be running."
    }

    $previousHeld = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD
    $previousOwnerPid = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID
    $previousContext = $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD = "1"
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID = "$PID"
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT = $Context

    Write-Host "Benchmark lock acquired ($Context)."
    return [pscustomobject]@{
      acquired = $true
      inherited = $false
      abandoned = $abandoned
      mutex = $mutex
      previous_held = $previousHeld
      previous_owner_pid = $previousOwnerPid
      previous_context = $previousContext
    }
  } catch {
    if ($acquired) {
      try {
        $mutex.ReleaseMutex() | Out-Null
      } catch {
      }
    }
    try {
      $mutex.Dispose()
    } catch {
    }
    throw
  }
}

function Restore-BenchmarkLockEnvValue {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [string]$Value
  )

  if ([string]::IsNullOrWhiteSpace($Value)) {
    Remove-Item "Env:$Name" -ErrorAction SilentlyContinue
  } else {
    Set-Item "Env:$Name" -Value $Value
  }
}

function Exit-BenchmarkLock {
  param($Handle)

  if ($null -eq $Handle -or -not $Handle.acquired) {
    return
  }

  try {
    $Handle.mutex.ReleaseMutex() | Out-Null
    Write-Host "Benchmark lock released."
  } finally {
    try {
      $Handle.mutex.Dispose()
    } catch {
    }
    Restore-BenchmarkLockEnvValue -Name "TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD" -Value $Handle.previous_held
    Restore-BenchmarkLockEnvValue -Name "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID" -Value $Handle.previous_owner_pid
    Restore-BenchmarkLockEnvValue -Name "TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT" -Value $Handle.previous_context
  }
}
