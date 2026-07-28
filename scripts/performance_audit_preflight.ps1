function Test-VectorGridPerformanceProcessName {
  param([Parameter(Mandatory = $true)][string]$Name)

  $normalized = [System.IO.Path]::GetFileNameWithoutExtension($Name).ToLowerInvariant()
  if ($normalized -match '^(cargo|rustc|criterion|nsys|ncu|compute-sanitizer)$') {
    return $true
  }

  return $normalized -match '^(double_ma_opt|double_ma_metrics|composite_mo_opt|composite_cpu_opt|asr_opt|vcc_opt|vemr_opt|vcet_opt|rocb_opt|mkcs_opt|gpu_native_expansion_opt|dynamic_strategy_cpu)(-|$)' -or
    $normalized -match '^(composite_validation_workflow|double_ma_validation_workflow|double_ma_followup_workflow|meta_select_workflow|ml_signal_workflow|ml_signal_validation|e2e_workflow_ml)(-|$)' -or
    $normalized -match '^(options_expression_candidate_compile|options_expression_summary_compile|options_expression_cpu_score|options_expression_gpu_score)(-|$)' -or
    $normalized -match '^(ta_workflow_regression|ta_double_ma_validation_cost|ta_campaign_perf|ta_perf_authority_support|ta_profile_route_matrix|ta_app_path_faceoff|ta_options_expression.*)(-|$)' -or
    $normalized -match '^(profile_gpu_dynamic_runner|profile_doublema_campaign_prefetch)(-|$)'
}

function Invoke-VectorGridBoundedProcessSnapshot {
  [CmdletBinding()]
  param(
    [ValidateRange(50, 60000)]
    [int]$TimeoutMilliseconds = 5000,
    [Parameter(Mandatory = $true)]
    [scriptblock]$SnapshotProvider
  )

  $snapshotJob = $null
  $snapshotRows = @()
  try {
    $providerSource = $SnapshotProvider.ToString()
    $snapshotJob = Start-Job -ScriptBlock {
      param([string]$ProviderSource)
      $ErrorActionPreference = "Stop"
      $provider = [scriptblock]::Create($ProviderSource)
      @(& $provider)
    } -ArgumentList $providerSource -ErrorAction Stop

    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    while ($snapshotJob.State -in @("NotStarted", "Running")) {
      if ($timer.ElapsedMilliseconds -ge $TimeoutMilliseconds) {
        throw [System.TimeoutException]::new(
          "Performance process snapshot timed out after ${TimeoutMilliseconds}ms."
        )
      }
      Start-Sleep -Milliseconds 10
    }

    if ($snapshotJob.State -ne "Completed") {
      $failureDetails = @()
      if ($null -ne $snapshotJob.JobStateInfo.Reason -and
          -not [string]::IsNullOrWhiteSpace($snapshotJob.JobStateInfo.Reason.Message)) {
        $failureDetails += $snapshotJob.JobStateInfo.Reason.Message
      }
      foreach ($childJob in @($snapshotJob.ChildJobs)) {
        if ($null -ne $childJob.JobStateInfo.Reason -and
            -not [string]::IsNullOrWhiteSpace($childJob.JobStateInfo.Reason.Message)) {
          $failureDetails += $childJob.JobStateInfo.Reason.Message
        }
        foreach ($errorRecord in @($childJob.Error)) {
          if ($null -ne $errorRecord.Exception -and
              -not [string]::IsNullOrWhiteSpace($errorRecord.Exception.Message)) {
            $failureDetails += $errorRecord.Exception.Message
          }
        }
      }
      $detail = if ($failureDetails.Count -gt 0) {
        ($failureDetails | Select-Object -Unique) -join "; "
      } else {
        "background snapshot state was '$($snapshotJob.State)'"
      }
      throw [System.InvalidOperationException]::new($detail)
    }

    $snapshotRows = @(Receive-Job -Job $snapshotJob -ErrorAction Stop)
  } catch {
    throw "Performance process snapshot failed closed: $($_.Exception.Message)"
  } finally {
    if ($null -ne $snapshotJob) {
      if ($snapshotJob.State -in @("NotStarted", "Running")) {
        Stop-Job -Job $snapshotJob -ErrorAction SilentlyContinue
      }
      Remove-Job -Job $snapshotJob -Force -ErrorAction SilentlyContinue
    }
  }

  return @($snapshotRows)
}

function Get-VectorGridActivePerformanceProcesses {
  [CmdletBinding()]
  param(
    [ValidateRange(50, 60000)]
    [int]$TimeoutMilliseconds = 5000,
    [scriptblock]$SnapshotProvider = {
      foreach ($process in [System.Diagnostics.Process]::GetProcesses()) {
        $cpu = $null
        $startTime = $null
        try {
          $cpu = $process.TotalProcessorTime.TotalSeconds
        } catch {
        }
        try {
          $startTime = $process.StartTime
        } catch {
        }
        [pscustomobject]@{
          ProcessName = $process.ProcessName
          Id = $process.Id
          CPU = $cpu
          StartTime = $startTime
        }
      }
    },
    [int[]]$ExcludeProcessId = @($PID)
  )

  $snapshotRows = @(
    Invoke-VectorGridBoundedProcessSnapshot `
      -TimeoutMilliseconds $TimeoutMilliseconds `
      -SnapshotProvider $SnapshotProvider
  )

  $excludedIds = [System.Collections.Generic.HashSet[int]]::new()
  foreach ($processId in @($ExcludeProcessId)) {
    $null = $excludedIds.Add([int]$processId)
  }

  return @(
    foreach ($process in $snapshotRows) {
      if ($null -eq $process) {
        continue
      }
      $processId = [int]$process.Id
      $processName = [string]$process.ProcessName
      if ($excludedIds.Contains($processId) -or
          -not (Test-VectorGridPerformanceProcessName -Name $processName)) {
        continue
      }
      [ordered]@{
        id = $processId
        name = $processName
        command_line = ""
      }
    }
  )
}

function Get-VectorGridReleaseBaselineHealth {
  param(
    [Parameter(Mandatory = $true)]$Registry,
    [Parameter(Mandatory = $true)][string]$RepoRoot
  )

  $requiredCases = @($Registry.cases | Where-Object {
      $property = $_.PSObject.Properties["release_required_baseline"]
      $null -ne $property -and [bool]$property.Value
    })
  if ($requiredCases.Count -eq 0) {
    throw "Performance registry declares no release-required baselines."
  }

  return @(
    foreach ($case in $requiredCases) {
      $path = [string]$case.baseline_path
      if ([string]::IsNullOrWhiteSpace($path)) {
        throw "Release-required performance case '$($case.id)' has no baseline_path."
      }
      $fullPath = if ([System.IO.Path]::IsPathRooted($path)) {
        [System.IO.Path]::GetFullPath($path)
      } else {
        [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $path))
      }
      [ordered]@{
        case_id = [string]$case.id
        path = $path.Replace("\", "/")
        full_path = $fullPath
        exists = Test-Path -LiteralPath $fullPath -PathType Leaf
      }
    }
  )
}
