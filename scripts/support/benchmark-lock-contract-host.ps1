param(
  [Parameter(Mandatory = $true)]
  [ValidateSet(
    "valid",
    "forged-environment",
    "forged-unheld",
    "unrelated-owner",
    "dead-owner",
    "pid-start-mismatch",
    "acl-denied",
    "release",
    "nested",
    "split-holder",
    "fixed-contention",
    "hardlink-rejected",
    "parent-release-during-child"
  )]
  [string]$Mode,
  [Parameter(Mandatory = $true)]
  [string]$RepoRoot
)

$ErrorActionPreference = "Stop"
$lockScript = Join-Path $RepoRoot "scripts\benchmark_lock.ps1"
$childScript = Join-Path $RepoRoot "scripts\support\benchmark-lock-contract-child.ps1"
. $lockScript

function Write-ContractResult {
  param([Parameter(Mandatory = $true)]$Value)

  Write-Output (
    "BENCHMARK_LOCK_CONTRACT_JSON:" +
    ($Value | ConvertTo-Json -Depth 12 -Compress)
  )
}

function Get-ChildContractResult {
  param([Parameter(Mandatory = $true)][object[]]$Output)

  $prefix = "BENCHMARK_LOCK_CONTRACT_JSON:"
  $lines = @($Output | Where-Object { "$_".StartsWith($prefix) })
  if ($lines.Count -ne 1) {
    throw "Benchmark-lock child emitted $($lines.Count) contract result lines."
  }
  return "$($lines[0])".Substring($prefix.Length) | ConvertFrom-Json
}

function Get-ContractEnvironment {
  $values = [ordered]@{}
  foreach ($name in $script:BenchmarkLockEnvironmentNames) {
    $values[$name] = [System.Environment]::GetEnvironmentVariable($name, "Process")
  }
  return $values
}

function Invoke-ContractChild {
  param(
    [Parameter(Mandatory = $true)][string]$ChildMode,
    [string]$MutexName = "Local\ta_desktop_demo_benchmark_gate",
    [int]$TimeoutSeconds = 3
  )

  $output = @(
    & powershell -NoProfile -ExecutionPolicy Bypass `
      -File $childScript `
      -Mode $ChildMode `
      -RepoRoot $RepoRoot `
      -MutexName $MutexName `
      -TimeoutSeconds $TimeoutSeconds
  )
  if ($LASTEXITCODE -ne 0) {
    throw "Benchmark-lock child process failed with exit code $LASTEXITCODE."
  }
  return Get-ChildContractResult -Output $output
}

function Set-ContractEnvironment {
  param([Parameter(Mandatory = $true)]$Values)

  foreach ($property in $Values.PSObject.Properties) {
    Set-Item `
      -LiteralPath "Env:$($property.Name)" `
      -Value ([string]$property.Value
      )
  }
}

function Clear-ContractEnvironment {
  foreach ($name in $script:BenchmarkLockEnvironmentNames) {
    Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
  }
}

function Quote-ContractProcessArgument {
  param([Parameter(Mandatory = $true)][string]$Value)

  return '"' + $Value.Replace('"', '\"') + '"'
}

function Start-ContractHolder {
  param(
    [string]$HolderMode = "holder",
    [string]$MutexName = "Local\ta_desktop_demo_benchmark_gate",
    [string]$DescriptorPath = ""
  )

  $contractDirectory = Join-Path (
    Get-BenchmarkLockLeaseDirectory
  ) "contracts"
  [System.IO.Directory]::CreateDirectory($contractDirectory) | Out-Null
  $id = [guid]::NewGuid().ToString("N")
  $readyPath = Join-Path $contractDirectory "$id.ready.json"
  $stopPath = Join-Path $contractDirectory "$id.stop"
  $resultPath = Join-Path $contractDirectory "$id.result.json"

  $arguments = @(
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-File", (Quote-ContractProcessArgument -Value $childScript),
    "-Mode", (Quote-ContractProcessArgument -Value $HolderMode),
    "-RepoRoot", (Quote-ContractProcessArgument -Value $RepoRoot),
    "-ReadyPath", (Quote-ContractProcessArgument -Value $readyPath),
    "-StopPath", (Quote-ContractProcessArgument -Value $stopPath),
    "-ResultPath", (Quote-ContractProcessArgument -Value $resultPath),
    "-MutexName", (Quote-ContractProcessArgument -Value $MutexName),
    "-TimeoutSeconds", "3"
  )
  if (-not [string]::IsNullOrWhiteSpace($DescriptorPath)) {
    $arguments += @(
      "-DescriptorPath",
      (Quote-ContractProcessArgument -Value $DescriptorPath)
    )
  }

  $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = (Get-Command powershell).Source
  $startInfo.Arguments = $arguments -join " "
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
  $process = [System.Diagnostics.Process]::Start($startInfo)

  $timer = [System.Diagnostics.Stopwatch]::StartNew()
  while (-not (Test-Path -LiteralPath $readyPath -PathType Leaf)) {
    if ($process.HasExited) {
      throw "Benchmark-lock holder exited before publishing readiness (exit $($process.ExitCode))."
    }
    if ($timer.Elapsed.TotalSeconds -ge 8) {
      throw "Timed out waiting for benchmark-lock holder readiness."
    }
    Start-Sleep -Milliseconds 20
  }

  return [pscustomobject]@{
    process = $process
    ready_path = $readyPath
    stop_path = $stopPath
    result_path = $resultPath
    descriptor_path = $DescriptorPath
    ready = Get-Content -Raw -LiteralPath $readyPath | ConvertFrom-Json
  }
}

function Stop-ContractHolder {
  param([Parameter(Mandatory = $true)]$Holder)

  $result = $null
  try {
    [System.IO.File]::WriteAllText(
      $Holder.stop_path,
      "stop",
      [System.Text.UTF8Encoding]::new($false)
    )
    if (-not $Holder.process.WaitForExit(8000)) {
      $Holder.process.Kill()
      $Holder.process.WaitForExit()
      throw "Benchmark-lock holder did not stop after its signal."
    }
    if ($Holder.process.ExitCode -ne 0) {
      throw "Benchmark-lock holder exited with code $($Holder.process.ExitCode)."
    }
    if (Test-Path -LiteralPath $Holder.result_path -PathType Leaf) {
      $result = Get-Content -Raw -LiteralPath $Holder.result_path |
        ConvertFrom-Json
    }
  } finally {
    $Holder.process.Dispose()
    Remove-Item -LiteralPath $Holder.ready_path -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Holder.stop_path -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Holder.result_path -Force -ErrorAction SilentlyContinue
    if (-not [string]::IsNullOrWhiteSpace($Holder.descriptor_path)) {
      Remove-Item `
        -LiteralPath $Holder.descriptor_path `
        -Force `
        -ErrorAction SilentlyContinue
    }
  }
  return $result
}

$originalEnvironment = Get-BenchmarkLockEnvironmentSnapshot
$activeHandle = $null
$holder = $null
try {
  Clear-ContractEnvironment
  switch ($Mode) {
    "valid" {
      $activeHandle = Enter-BenchmarkLock -Context "benchmark lock valid contract"
      $child = Invoke-ContractChild -ChildMode "inherit"
      $leaseDuring = Read-BenchmarkLockLease -Path $activeHandle.lease_path
      $path = $activeHandle.lease_path
      Exit-BenchmarkLock -Handle $activeHandle
      $activeHandle = $null
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          child = $child
          lease_path = $path
          lease_exists_after_release = Test-Path -LiteralPath $path -PathType Leaf
          stale_sha256 = (Read-BenchmarkLockLease -Path $path).sha256
          active_sha256 = $leaseDuring.sha256
        })
    }

    "forged-unheld" {
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock unheld contract"
      $activeHandle.lease_stream.Dispose()
      $activeHandle.lease_stream = $null
      $child = Invoke-ContractChild -ChildMode "inherit"
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          child = $child
      })
    }

    "forged-environment" {
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock forged environment contract"
      $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256 = "0" * 64
      $child = Invoke-ContractChild -ChildMode "inherit"
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          child = $child
        })
    }

    "unrelated-owner" {
      $holder = Start-ContractHolder
      Set-ContractEnvironment -Values $holder.ready.environment
      try {
        $null = Enter-BenchmarkLock -Context "unrelated-owner contract"
        $attempt = [ordered]@{ ok = $true; error = "" }
      } catch {
        $attempt = [ordered]@{
          ok = $false
          error = $_.Exception.Message
        }
      }
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          holder_pid = [int]$holder.ready.actual_pid
          attempt = $attempt
        })
    }

    "dead-owner" {
      $holder = Start-ContractHolder
      $copiedEnvironment = $holder.ready.environment
      Stop-ContractHolder -Holder $holder
      $holder = $null
      Set-ContractEnvironment -Values $copiedEnvironment
      try {
        $null = Enter-BenchmarkLock -Context "dead-owner contract"
        $attempt = [ordered]@{ ok = $true; error = "" }
      } catch {
        $attempt = [ordered]@{
          ok = $false
          error = $_.Exception.Message
        }
      }
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          attempt = $attempt
        })
    }

    "pid-start-mismatch" {
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock PID-start mismatch contract"
      $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT =
        "2000-01-01T00:00:00.0000000Z"
      $child = Invoke-ContractChild -ChildMode "inherit"
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          child = $child
        })
    }

    "acl-denied" {
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock ACL contract"
      $activeHandle.lease_stream.Dispose()
      $activeHandle.lease_stream = $null
      $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
      $denyRule = "${identity}:(W)"
      & icacls.exe $activeHandle.lease_path /deny $denyRule /Q | Out-Null
      if ($LASTEXITCODE -ne 0) {
        throw "Could not install the benchmark-lock ACL-denial contract rule."
      }
      try {
        $probeCode = Get-BenchmarkLockLeaseWriteProbeCode `
          -Path $activeHandle.lease_path
        $child = Invoke-ContractChild -ChildMode "inherit"
      } finally {
        & icacls.exe $activeHandle.lease_path /remove:d $identity /Q | Out-Null
        if ($LASTEXITCODE -ne 0) {
          throw "Could not remove the benchmark-lock ACL-denial contract rule."
        }
      }
      Exit-BenchmarkLock -Handle $activeHandle
      $activeHandle = $null
      $afterFailureEnvironment = Get-ContractEnvironment
      $recovery = Invoke-ContractChild -ChildMode "direct"
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          probe_code = $probeCode
          child = $child
          after_failure_environment = $afterFailureEnvironment
          recovery = $recovery
        })
    }

    "release" {
      $seeded = [ordered]@{}
      foreach ($name in $script:BenchmarkLockEnvironmentNames) {
        $value = if ($name -eq "TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD") {
          "0"
        } else {
          "prior-$name"
        }
        Set-Item -LiteralPath "Env:$name" -Value $value
        $seeded[$name] = $value
      }
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock release contract"
      $firstLease = Read-BenchmarkLockLease -Path $activeHandle.lease_path
      $path = $activeHandle.lease_path
      $identity = $activeHandle.lease_identity
      Exit-BenchmarkLock -Handle $activeHandle
      Exit-BenchmarkLock -Handle $activeHandle
      $restored = [ordered]@{}
      foreach ($name in $script:BenchmarkLockEnvironmentNames) {
        $restored[$name] = [System.Environment]::GetEnvironmentVariable(
          $name,
          "Process"
        )
      }
      $activeHandle = $null
      $second = Invoke-ContractChild -ChildMode "direct"
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          lease_exists_after_release = Test-Path -LiteralPath $path -PathType Leaf
          first_acquisition_id = [string]$firstLease.payload.acquisition_id
          first_identity = $identity
          second = $second
          seeded = $seeded
          restored = $restored
        })
    }

    "nested" {
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock nested root contract"
      $rootEnvironment = Get-ContractEnvironment
      $child = Invoke-ContractChild -ChildMode "nested"
      $afterEnvironment = Get-ContractEnvironment
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          child = $child
          root_environment = $rootEnvironment
          after_environment = $afterEnvironment
        })
    }

    "split-holder" {
      $contractDirectory = Join-Path (
        Get-BenchmarkLockLeaseDirectory
      ) "contracts"
      [System.IO.Directory]::CreateDirectory($contractDirectory) | Out-Null
      $descriptorPath = Join-Path $contractDirectory (
        "$([guid]::NewGuid().ToString('N')).descriptor.json"
      )
      $descriptor = [ordered]@{
        claimed_owner_pid = $PID
        claimed_owner_started_at = Get-BenchmarkLockProcessStartIdentity `
          -ProcessId $PID
        context = "benchmark lock split-holder contract"
        token = [guid]::NewGuid().ToString("N")
      }
      [System.IO.File]::WriteAllText(
        $descriptorPath,
        ($descriptor | ConvertTo-Json -Compress),
        [System.Text.UTF8Encoding]::new($false)
      )
      $holder = Start-ContractHolder `
        -HolderMode "spoof-holder" `
        -DescriptorPath $descriptorPath
      Set-ContractEnvironment -Values $holder.ready.environment
      $claimedOwnerReadStream = [System.IO.FileStream]::new(
        [string]$holder.ready.environment.
          TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::ReadWrite
      )
      try {
        $child = Invoke-ContractChild -ChildMode "inherit"
      } finally {
        $claimedOwnerReadStream.Dispose()
      }
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          claimed_owner_pid = $PID
          actual_holder_pid = [int]$holder.ready.actual_pid
          claimed_owner_held_read_handle = $true
          child = $child
        })
    }

    "fixed-contention" {
      $rootMutex = "Local\ta_desktop_demo_benchmark_contract_root_" +
        [guid]::NewGuid().ToString("N")
      $contenderMutex = "Local\ta_desktop_demo_benchmark_contract_contender_" +
        [guid]::NewGuid().ToString("N")
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock fixed contention root" `
        -MutexName $rootMutex `
        -TimeoutSeconds 2
      $contender = Invoke-ContractChild `
        -ChildMode "direct" `
        -MutexName $contenderMutex `
        -TimeoutSeconds 1
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          root_mutex = $rootMutex
          contender_mutex = $contenderMutex
          contender = $contender
        })
    }

    "hardlink-rejected" {
      $leaseCandidate = Join-Path (
        Get-BenchmarkLockLeaseDirectory
      ) "active.json"
      if (-not (Test-Path -LiteralPath $leaseCandidate -PathType Leaf)) {
        $seedHandle = Enter-BenchmarkLock `
          -Context "benchmark lock hard-link seed contract"
        Exit-BenchmarkLock -Handle $seedHandle
      }
      $leasePath = Assert-BenchmarkLockLeasePath `
        -Path $leaseCandidate `
        -MustExist
      $contractDirectory = Join-Path (
        Get-BenchmarkLockLeaseDirectory
      ) "contracts"
      [System.IO.Directory]::CreateDirectory($contractDirectory) | Out-Null
      $hardlinkPath = Join-Path $contractDirectory (
        "$([guid]::NewGuid().ToString('N')).hardlink.json"
      )
      New-Item `
        -ItemType HardLink `
        -Path $hardlinkPath `
        -Target $leasePath | Out-Null
      try {
        try {
          $null = Enter-BenchmarkLock `
            -Context "benchmark lock hard-link rejection contract" `
            -TimeoutSeconds 1
          $attempt = [ordered]@{ ok = $true; error = "" }
        } catch {
          $attempt = [ordered]@{
            ok = $false
            error = $_.Exception.Message
          }
        }
      } finally {
        Remove-Item -LiteralPath $hardlinkPath -Force -ErrorAction SilentlyContinue
      }
      $afterFailureEnvironment = Get-ContractEnvironment
      $recovery = Invoke-ContractChild -ChildMode "direct"
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          attempt = $attempt
          active_exists = Test-Path -LiteralPath $leasePath -PathType Leaf
          after_failure_environment = $afterFailureEnvironment
          recovery = $recovery
        })
    }

    "parent-release-during-child" {
      $activeHandle = Enter-BenchmarkLock `
        -Context "benchmark lock parent-release contract"
      $holder = Start-ContractHolder -HolderMode "wait-inherited"
      $holderReady = $holder.ready
      Exit-BenchmarkLock -Handle $activeHandle
      $activeHandle = $null
      $childExit = Stop-ContractHolder -Holder $holder
      $holder = $null
      Write-ContractResult -Value ([ordered]@{
          ok = $true
          child_entered_inherited = [bool]$holderReady.inherited
          child_exit = $childExit
        })
    }
  }
} catch {
  Write-ContractResult -Value ([ordered]@{
      ok = $false
      mode = $Mode
      error = $_.Exception.Message
    })
} finally {
  if ($null -ne $activeHandle) {
    try {
      Exit-BenchmarkLock -Handle $activeHandle
    } catch {
    }
  }
  if ($null -ne $holder) {
    try {
      Stop-ContractHolder -Holder $holder
    } catch {
    }
  }
  Restore-BenchmarkLockEnvironment -Snapshot $originalEnvironment
}
