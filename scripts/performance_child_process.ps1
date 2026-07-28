if ($null -eq (Get-Command Invoke-VectorGridBoundedProcessSnapshot -ErrorAction SilentlyContinue)) {
  . (Join-Path $PSScriptRoot "performance_audit_preflight.ps1")
}

function ConvertTo-VectorGridProcessStartTicks {
  param($Value)

  if ($null -eq $Value) {
    return $null
  }
  if ($Value -is [DateTime]) {
    return [int64]$Value.ToUniversalTime().Ticks
  }
  if ($Value -is [DateTimeOffset]) {
    return [int64]$Value.UtcDateTime.Ticks
  }
  if ($Value -is [byte] -or
      $Value -is [int16] -or
      $Value -is [int32] -or
      $Value -is [int64]) {
    return [int64]$Value
  }

  $text = [string]$Value
  if ([string]::IsNullOrWhiteSpace($text)) {
    return $null
  }
  try {
    return [int64]([System.Management.ManagementDateTimeConverter]::ToDateTime($text).ToUniversalTime().Ticks)
  } catch {
  }
  try {
    return [int64]([DateTimeOffset]::Parse(
        $text,
        [System.Globalization.CultureInfo]::InvariantCulture,
        [System.Globalization.DateTimeStyles]::AssumeUniversal
      ).UtcDateTime.Ticks)
  } catch {
    return $null
  }
}

function Get-VectorGridWin32ProcessSnapshot {
  [CmdletBinding()]
  param(
    [ValidateRange(50, 60000)]
    [int]$TimeoutMilliseconds = 5000,
    [scriptblock]$SnapshotProvider = {
      Get-CimInstance Win32_Process -ErrorAction Stop |
        Select-Object Name, ProcessId, ParentProcessId, CommandLine, CreationDate
    }
  )

  $rows = @(
    Invoke-VectorGridBoundedProcessSnapshot `
      -TimeoutMilliseconds $TimeoutMilliseconds `
      -SnapshotProvider $SnapshotProvider
  )
  return @(
    foreach ($row in $rows) {
      if ($null -eq $row) {
        continue
      }
      $processIdProperty = $row.PSObject.Properties["ProcessId"]
      if ($null -eq $processIdProperty) {
        $processIdProperty = $row.PSObject.Properties["Id"]
      }
      $parentIdProperty = $row.PSObject.Properties["ParentProcessId"]
      if ($null -eq $parentIdProperty) {
        $parentIdProperty = $row.PSObject.Properties["ParentId"]
      }
      if ($null -eq $processIdProperty -or $null -eq $parentIdProperty) {
        throw "Bounded Win32 process snapshot returned a row without process identity."
      }

      $processId = [int]$processIdProperty.Value
      if ($processId -le 0) {
        continue
      }
      $nameProperty = $row.PSObject.Properties["Name"]
      if ($null -eq $nameProperty) {
        $nameProperty = $row.PSObject.Properties["ProcessName"]
      }
      $commandLineProperty = $row.PSObject.Properties["CommandLine"]
      $creationProperty = $row.PSObject.Properties["CreationDate"]
      if ($null -eq $creationProperty) {
        $creationProperty = $row.PSObject.Properties["StartTicks"]
      }
      [pscustomobject]@{
        ProcessName = [System.IO.Path]::GetFileNameWithoutExtension([string]$nameProperty.Value)
        Id = $processId
        ParentId = [int]$parentIdProperty.Value
        CommandLine = if ($null -eq $commandLineProperty) { "" } else { [string]$commandLineProperty.Value }
        StartTicks = if ($null -eq $creationProperty) {
          $null
        } else {
          ConvertTo-VectorGridProcessStartTicks -Value $creationProperty.Value
        }
      }
    }
  )
}

function Test-VectorGridBenchmarkCommandLine {
  param([string]$CommandLine)

  if ([string]::IsNullOrWhiteSpace($CommandLine)) {
    return $false
  }
  if ($CommandLine -match '(^|[ \t])--no-run([ \t]|$)') {
    return $false
  }
  return $CommandLine -match '(^|[ \t])bench([ \t]|$)' -or
    $CommandLine -match '(^|[ \t])--bench([ \t]|$)' -or
    $CommandLine -match 'criterion'
}

function Select-VectorGridGpuValidationPerformanceProcesses {
  param(
    [Parameter(Mandatory = $true)]$Snapshot,
    $CompletedProcessStartTicksById = @{}
  )

  return @(
    foreach ($process in @($Snapshot)) {
      $processName = [string]$process.ProcessName
      $commandLine = [string]$process.CommandLine
      $isShell = $processName -match '^(pwsh|powershell|cmd|conhost|WindowsTerminal)$'
      $isKnownPerformanceProcess = $processName -match '^(cargo|rustup|criterion|nsys|ncu|compute-sanitizer)$' -or
        $processName -like 'double_ma_validation_workflow*' -or
        $processName -like 'composite_validation_workflow*' -or
        $processName -like 'ta_double_ma_validation_cost*'
      $isBenchmarkCommand = -not $isShell -and
        (Test-VectorGridBenchmarkCommandLine -CommandLine $commandLine)
      if (-not $isKnownPerformanceProcess -and -not $isBenchmarkCommand) {
        continue
      }
      if ($processName -match '^(cargo|rustup)$' -and -not $isBenchmarkCommand) {
        continue
      }

      $key = "$([int]$process.Id)"
      $isKnownCompletedIdentity = $null -ne $process.StartTicks -and
        $CompletedProcessStartTicksById.ContainsKey($key) -and
        [int64]$CompletedProcessStartTicksById[$key] -eq [int64]$process.StartTicks
      if ($isKnownCompletedIdentity) {
        continue
      }

      $startTime = if ($null -eq $process.StartTicks) {
        $null
      } else {
        ([DateTime]::new([int64]$process.StartTicks, [DateTimeKind]::Utc)).ToLocalTime()
      }
      [pscustomobject]@{
        ProcessName = $processName
        Id = [int]$process.Id
        CPU = $null
        StartTime = $startTime
        CommandLine = $commandLine
      }
    }
  )
}

function Get-VectorGridLaunchedProcessTree {
  param(
    [Parameter(Mandatory = $true)]$Snapshot,
    [Parameter(Mandatory = $true)][int]$RootProcessId,
    [Parameter(Mandatory = $true)][int64]$RootStartTicks
  )

  $rows = @($Snapshot)
  $rootRows = @($rows | Where-Object { [int]$_.Id -eq $RootProcessId })
  if ($rootRows.Count -eq 0) {
    return @()
  }
  if ($rootRows.Count -ne 1 -or
      $null -eq $rootRows[0].StartTicks -or
      [int64]$rootRows[0].StartTicks -ne $RootStartTicks) {
    throw "Launched root process identity changed before cleanup (PID $RootProcessId)."
  }

  $selected = [ordered]@{}
  $selected["$RootProcessId"] = [pscustomobject]@{
    ProcessName = [string]$rootRows[0].ProcessName
    Id = $RootProcessId
    ParentId = [int]$rootRows[0].ParentId
    CommandLine = [string]$rootRows[0].CommandLine
    StartTicks = $RootStartTicks
    Depth = 0
  }

  $changed = $true
  while ($changed) {
    $changed = $false
    foreach ($row in $rows) {
      $rowId = [int]$row.Id
      if ($rowId -le 0 -or $selected.Contains("$rowId")) {
        continue
      }
      $parentKey = "$([int]$row.ParentId)"
      if (-not $selected.Contains($parentKey)) {
        continue
      }
      if ($null -eq $row.StartTicks) {
        throw "Cannot verify launched descendant identity for PID $rowId."
      }
      if ([int64]$row.StartTicks -lt $RootStartTicks) {
        continue
      }
      $selected["$rowId"] = [pscustomobject]@{
        ProcessName = [string]$row.ProcessName
        Id = $rowId
        ParentId = [int]$row.ParentId
        CommandLine = [string]$row.CommandLine
        StartTicks = [int64]$row.StartTicks
        Depth = [int]$selected[$parentKey].Depth + 1
      }
      $changed = $true
    }
  }

  return @($selected.Values)
}

function Test-VectorGridProcessIdentityAlive {
  param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    [Parameter(Mandatory = $true)][int64]$StartTicks
  )

  $process = $null
  try {
    $process = [System.Diagnostics.Process]::GetProcessById($ProcessId)
    if ($process.HasExited) {
      return $false
    }
    return [int64]$process.StartTime.ToUniversalTime().Ticks -eq $StartTicks
  } catch {
    return $false
  } finally {
    if ($null -ne $process) {
      $process.Dispose()
    }
  }
}

function Stop-VectorGridLaunchedProcessTree {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory = $true)]$RootProcess,
    [ValidateRange(50, 60000)]
    [int]$SnapshotTimeoutMilliseconds = 5000,
    [ValidateRange(1, 120)]
    [int]$CleanupTimeoutSeconds = 15,
    [scriptblock]$SnapshotProvider = {
      Get-CimInstance Win32_Process -ErrorAction Stop |
        Select-Object Name, ProcessId, ParentProcessId, CommandLine, CreationDate
    },
    [scriptblock]$TerminateProvider = {
      param($Row, $Root)

      $target = $null
      $disposeTarget = $false
      try {
        if ([int]$Row.Id -eq [int]$Root.Id) {
          $target = $Root
        } else {
          $target = [System.Diagnostics.Process]::GetProcessById([int]$Row.Id)
          $disposeTarget = $true
        }
        if ($target.HasExited) {
          return $false
        }
        $actualStartTicks = [int64]$target.StartTime.ToUniversalTime().Ticks
        if ($actualStartTicks -ne [int64]$Row.StartTicks) {
          throw "Refusing to terminate reused PID $($Row.Id)."
        }
        $target.Kill()
        return $true
      } catch [System.ArgumentException] {
        return $false
      } finally {
        if ($disposeTarget -and $null -ne $target) {
          $target.Dispose()
        }
      }
    }
  )

  $rootProcessId = [int]$RootProcess.Id
  $rootStartTicks = [int64]$RootProcess.StartTime.ToUniversalTime().Ticks
  try {
    $snapshot = @(
      Get-VectorGridWin32ProcessSnapshot `
        -TimeoutMilliseconds $SnapshotTimeoutMilliseconds `
        -SnapshotProvider $SnapshotProvider
    )
  } catch {
    $snapshotError = $_.Exception.Message
    try {
      if (-not $RootProcess.HasExited -and
          [int64]$RootProcess.StartTime.ToUniversalTime().Ticks -eq $rootStartTicks) {
        $RootProcess.Kill()
      }
    } catch {
    }
    throw "Launched child-tree cleanup failed closed during bounded snapshot; exact root termination was attempted: $snapshotError"
  }

  $tree = @(
    Get-VectorGridLaunchedProcessTree `
      -Snapshot $snapshot `
      -RootProcessId $rootProcessId `
      -RootStartTicks $rootStartTicks
  )
  if ($tree.Count -eq 0) {
    try {
      if (-not $RootProcess.HasExited -and
          [int64]$RootProcess.StartTime.ToUniversalTime().Ticks -eq $rootStartTicks) {
        $RootProcess.Kill()
      }
    } catch {
    }
    throw "Bounded process snapshot did not contain the launched root PID $rootProcessId; exact root termination was attempted, but descendant cleanup is unverifiable."
  }

  $terminationOrder = @(
    $tree | Where-Object { [int]$_.Id -eq $rootProcessId }
    $tree |
      Where-Object { [int]$_.Id -ne $rootProcessId } |
      Sort-Object -Property @{ Expression = { [int]$_.Depth }; Descending = $true }, Id
  )
  $attemptedIds = @()
  foreach ($row in $terminationOrder) {
    $terminated = & $TerminateProvider $row $RootProcess
    if ($terminated -ne $false) {
      $attemptedIds += [int]$row.Id
    }
  }

  $deadline = (Get-Date).AddSeconds($CleanupTimeoutSeconds)
  while ($true) {
    $alive = @(
      $tree | Where-Object {
        Test-VectorGridProcessIdentityAlive `
          -ProcessId ([int]$_.Id) `
          -StartTicks ([int64]$_.StartTicks)
      }
    )
    if ($alive.Count -eq 0) {
      break
    }
    if ((Get-Date) -ge $deadline) {
      $survivors = ($alive | ForEach-Object { "$($_.ProcessName)#$($_.Id)" }) -join ", "
      throw "Timed out cleaning the launched process tree rooted at PID ${rootProcessId}: $survivors"
    }
    Start-Sleep -Milliseconds 25
  }

  return [pscustomobject]@{
    root_process_id = $rootProcessId
    observed_process_ids = @($tree | ForEach-Object { [int]$_.Id })
    termination_attempted_ids = @($attemptedIds)
    cleanup_completed = $true
  }
}

function Invoke-VectorGridBoundedChildProcess {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory = $true)][string]$FilePath,
    [string[]]$ArgumentList = @(),
    [ValidateRange(1, 7200)]
    [int]$TimeoutSeconds = 1800,
    [ValidateRange(50, 60000)]
    [int]$SnapshotTimeoutMilliseconds = 5000,
    [ValidateRange(1, 120)]
    [int]$CleanupTimeoutSeconds = 15,
    [scriptblock]$CleanupSnapshotProvider = {
      Get-CimInstance Win32_Process -ErrorAction Stop |
        Select-Object Name, ProcessId, ParentProcessId, CommandLine, CreationDate
    }
  )

  $stdoutPath = [System.IO.Path]::GetTempFileName()
  $stderrPath = [System.IO.Path]::GetTempFileName()
  $process = $null
  $cleanupAttempted = $false
  try {
    $process = Start-Process -FilePath $FilePath `
      -ArgumentList $ArgumentList `
      -NoNewWindow `
      -PassThru `
      -RedirectStandardOutput $stdoutPath `
      -RedirectStandardError $stderrPath
    $processId = [int]$process.Id
    $processStartTicks = [int64]$process.StartTime.ToUniversalTime().Ticks
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    while (-not $process.HasExited) {
      if ($timer.Elapsed.TotalSeconds -ge $TimeoutSeconds) {
        $cleanupAttempted = $true
        try {
          $cleanup = Stop-VectorGridLaunchedProcessTree `
            -RootProcess $process `
            -SnapshotTimeoutMilliseconds $SnapshotTimeoutMilliseconds `
            -CleanupTimeoutSeconds $CleanupTimeoutSeconds `
            -SnapshotProvider $CleanupSnapshotProvider
        } catch {
          throw "Child process timed out after ${TimeoutSeconds}s and launched-tree cleanup failed closed: $($_.Exception.Message)"
        }
        throw [System.TimeoutException]::new(
          "Child process timed out after ${TimeoutSeconds}s: $FilePath (PID $processId; cleanup_completed=$($cleanup.cleanup_completed))."
        )
      }
      Start-Sleep -Milliseconds 25
    }

    $output = @()
    if (Test-Path -LiteralPath $stdoutPath) {
      $output += @(Get-Content -LiteralPath $stdoutPath | ForEach-Object { [string]$_ })
    }
    if (Test-Path -LiteralPath $stderrPath) {
      $output += @(Get-Content -LiteralPath $stderrPath | ForEach-Object { [string]$_ })
    }
    return [pscustomobject]@{
      process_id = $processId
      process_start_ticks = $processStartTicks
      exit_code = [int]$process.ExitCode
      output = @($output)
    }
  } catch {
    $originalError = $_
    if ($null -ne $process -and -not $cleanupAttempted) {
      try {
        if (-not $process.HasExited) {
          $cleanupAttempted = $true
          $null = Stop-VectorGridLaunchedProcessTree `
            -RootProcess $process `
            -SnapshotTimeoutMilliseconds $SnapshotTimeoutMilliseconds `
            -CleanupTimeoutSeconds $CleanupTimeoutSeconds `
            -SnapshotProvider $CleanupSnapshotProvider
        }
      } catch {
        throw "Child process failed and launched-tree cleanup also failed closed. Original error: $($originalError.Exception.Message). Cleanup error: $($_.Exception.Message)"
      }
    }
    throw $originalError
  } finally {
    if ($null -ne $process) {
      $process.Dispose()
    }
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
  }
}
