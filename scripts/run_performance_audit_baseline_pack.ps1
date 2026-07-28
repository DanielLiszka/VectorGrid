param(
  [ValidateSet("inventory", "plan-only", "candidate", "local-baseline")]
  [string]$Mode = "inventory",
  [string]$RegistryPath = "scripts/performance_audit_case_registry.json",
  [string]$OutDir = "",
  [bool]$RunWorkflowGate = $true,
  [switch]$IncludeCpu,
  [switch]$IncludeGpu,
  [switch]$IncludeGpuValidationHeavy,
  [switch]$IncludeOptions,
  [switch]$IncludeSupport,
  [switch]$IncludeFrontendBudget,
  [switch]$IncludeFrontendInteractionLatency,
  [switch]$AllowLocalBaselineCapture,
  [int]$GateTimeoutSec = 1800,
  [switch]$FailOnCoverageGap,
  [switch]$FailOnActivePerfProcess,
  [switch]$FailOnMissingReleaseBaseline
)

$ErrorActionPreference = "Stop"

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
. (Join-Path $PSScriptRoot "performance_audit_preflight.ps1")
if ([string]::IsNullOrWhiteSpace($OutDir)) {
  $OutDir = Join-Path $RepoRoot ("output/performance_audit/{0}/authority_baseline_pack" -f (Get-Date).ToString("yyyy-MM-dd"))
} elseif (-not [System.IO.Path]::IsPathRooted($OutDir)) {
  $OutDir = Join-Path $RepoRoot $OutDir
}
$OutDir = [System.IO.Path]::GetFullPath($OutDir)

function Resolve-RepoPath {
  param([string]$Path)
  if ([string]::IsNullOrWhiteSpace($Path)) {
    return ""
  }
  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $Path))
}

function Get-RepoRelativePath {
  param([string]$Path)
  if ([string]::IsNullOrWhiteSpace($Path)) {
    return ""
  }
  $full = [System.IO.Path]::GetFullPath($Path)
  $root = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd("\") + "\"
  $rootUri = [Uri]$root
  $fileUri = [Uri]$full
  return ([Uri]::UnescapeDataString($rootUri.MakeRelativeUri($fileUri).ToString())).Replace("/", "\")
}

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value,
    [int]$Depth = 64
  )
  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  ConvertTo-Json -InputObject $Value -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Read-JsonFile {
  param([Parameter(Mandatory = $true)][string]$Path)
  return Get-Content -Raw $Path | ConvertFrom-Json
}

function Get-ArrayProperty {
  param(
    [Parameter(Mandatory = $true)]$Object,
    [Parameter(Mandatory = $true)][string]$Name
  )
  $prop = $Object.PSObject.Properties[$Name]
  if ($null -eq $prop -or $null -eq $prop.Value) {
    return @()
  }
  if ($prop.Value -is [System.Array]) {
    return @($prop.Value)
  }
  return @($prop.Value)
}

function New-StringSet {
  return ,[System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
}

function Get-TauriHandlerCommands {
  $libPath = Resolve-RepoPath "src-tauri/src/lib.rs"
  $content = Get-Content -Raw $libPath
  $match = [regex]::Match($content, "generate_handler!\[([\s\S]*?)\]\)")
  if (-not $match.Success) {
    throw "Could not find tauri::generate_handler![...] in $libPath"
  }
  return @(
    $match.Groups[1].Value -split "`n" |
      ForEach-Object { $_.Trim().TrimEnd(",") } |
      Where-Object { $_ -match "^[A-Za-z_][A-Za-z0-9_]*$" } |
      Sort-Object -Unique
  )
}

function Get-CommandSurfaceModules {
  $commandsPath = Resolve-RepoPath "src-tauri/src/commands.rs"
  $content = Get-Content -Raw $commandsPath
  $matches = [regex]::Matches($content, '#\[path\s*=\s*"([^"]+)"\]')
  return @(
    foreach ($m in $matches) {
      $m.Groups[1].Value
    }
  ) | Sort-Object -Unique
}

function Get-ToolVersion {
  param(
    [Parameter(Mandatory = $true)][string]$Command,
    [string[]]$Arguments = @()
  )
  try {
    $out = & $Command @Arguments 2>$null
    $line = @($out | Select-Object -First 1) -join " "
    if ([string]::IsNullOrWhiteSpace($line)) {
      return $null
    }
    return $line.Trim()
  } catch {
    return $null
  }
}

function Get-GitFingerprint {
  $rev = Get-ToolVersion -Command "git" -Arguments @("rev-parse", "HEAD")
  $branch = Get-ToolVersion -Command "git" -Arguments @("rev-parse", "--abbrev-ref", "HEAD")
  $statusLines = @()
  try {
    $statusLines = @(git status --short 2>$null)
  } catch {
    $statusLines = @()
  }
  return [ordered]@{
    rev = $rev
    branch = $branch
    dirty = $statusLines.Count -gt 0
    status_entry_count = $statusLines.Count
  }
}

function Get-GpuIdentity {
  $query = Get-ToolVersion -Command "nvidia-smi" -Arguments @("--query-gpu=name,driver_version,memory.total", "--format=csv,noheader")
  if ($null -eq $query) {
    return [ordered]@{
      available = $false
      query = $null
    }
  }
  return [ordered]@{
    available = $true
    query = $query
  }
}

function Get-EnvironmentFingerprint {
  return [ordered]@{
    timestamp_utc = (Get-Date).ToUniversalTime().ToString("o")
    host = [System.Environment]::MachineName
    user = [System.Environment]::UserName
    os = [System.Environment]::OSVersion.VersionString
    arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    powershell = $PSVersionTable.PSVersion.ToString()
    rustc = Get-ToolVersion -Command "rustc" -Arguments @("--version")
    cargo = Get-ToolVersion -Command "cargo" -Arguments @("--version")
    node = Get-ToolVersion -Command "node" -Arguments @("--version")
    npm = Get-ToolVersion -Command "npm" -Arguments @("--version")
    gpu = Get-GpuIdentity
  }
}

function Get-PerfProcesses {
  return @(
    Get-VectorGridActivePerformanceProcesses `
      -TimeoutMilliseconds 5000 `
      -ExcludeProcessId @($PID)
  )
}

function Get-PathStatus {
  param([string]$Path)
  if ([string]::IsNullOrWhiteSpace($Path)) {
    return [ordered]@{
      path = ""
      full_path = ""
      exists = $false
    }
  }
  $full = Resolve-RepoPath $Path
  return [ordered]@{
    path = Get-RepoRelativePath $full
    full_path = $full
    exists = Test-Path $full
  }
}

function Build-Coverage {
  param($Registry)

  $handlerCommands = @(Get-TauriHandlerCommands)
  $commandModules = @(Get-CommandSurfaceModules)
  $registeredCommands = New-StringSet
  $registeredModules = New-StringSet

  foreach ($case in @($Registry.cases)) {
    foreach ($command in @(Get-ArrayProperty -Object $case -Name "command_names")) {
      if (-not [string]::IsNullOrWhiteSpace($command)) {
        $null = $registeredCommands.Add([string]$command)
      }
    }
    foreach ($module in @(Get-ArrayProperty -Object $case -Name "command_modules")) {
      if (-not [string]::IsNullOrWhiteSpace($module)) {
        $null = $registeredModules.Add([string]$module)
      }
    }
  }

  $uncoveredCommands = @($handlerCommands | Where-Object { -not $registeredCommands.Contains($_) })
  $extraCommands = @($registeredCommands | Where-Object { $_ -notin $handlerCommands } | Sort-Object)
  $uncoveredModules = @($commandModules | Where-Object { -not $registeredModules.Contains($_) })
  $extraModules = @($registeredModules | Where-Object { $_ -notin $commandModules } | Sort-Object)

  return [ordered]@{
    tauri_handler_count = $handlerCommands.Count
    command_module_count = $commandModules.Count
    registry_command_count = $registeredCommands.Count
    registry_module_count = $registeredModules.Count
    uncovered_commands = $uncoveredCommands
    extra_registry_commands = $extraCommands
    uncovered_modules = $uncoveredModules
    extra_registry_modules = $extraModules
    handler_commands = $handlerCommands
    command_modules = $commandModules
  }
}

function Build-CaseHealth {
  param($Registry)
  return @(
    foreach ($case in @($Registry.cases)) {
      $scriptStatus = Get-PathStatus -Path $case.script
      $baselineStatus = Get-PathStatus -Path $case.baseline_path
      $outStatus = Get-PathStatus -Path $case.default_out_path
      $automationMode = [string]$case.automation_mode
      $primaryAuthority = [string]$case.primary_authority
      $hasScript = -not [string]::IsNullOrWhiteSpace([string]$case.script)
      $scriptMissing = $hasScript -and -not $scriptStatus.exists
      $baselineMissing = -not [string]::IsNullOrWhiteSpace([string]$case.baseline_path) -and -not $baselineStatus.exists
      $isMissingAuthority = $primaryAuthority -match "missing_" -or ($automationMode -notmatch "manual|coverage_inventory|planned" -and -not $hasScript)

      [ordered]@{
        id = $case.id
        workflow = $case.workflow
        surface_category = $case.surface_category
        priority = $case.priority
        automation_mode = $automationMode
        claim_class = $case.claim_class
        timing_boundary = $case.timing_boundary
        script = $scriptStatus
        baseline = $baselineStatus
        default_output = $outStatus
        script_missing = $scriptMissing
        baseline_missing = $baselineMissing
        release_required_baseline = [bool]$case.release_required_baseline
        missing_automated_authority = $isMissingAuthority
        heavy = [bool]$case.heavy
        gpu_required = [bool]$case.gpu_required
        credentials_required = [bool]$case.credentials_required
        destructive = [bool]$case.destructive
      }
    }
  )
}

function Group-CaseCounts {
  param(
    [Parameter(Mandatory = $true)]$Cases,
    [Parameter(Mandatory = $true)][string]$Property
  )
  $counts = @{}
  foreach ($case in @($Cases)) {
    if ($case -is [System.Collections.IDictionary]) {
      $name = [string]$case[$Property]
    } else {
      $prop = $case.PSObject.Properties[$Property]
      $name = if ($null -eq $prop) { "" } else { [string]$prop.Value }
    }
    if ([string]::IsNullOrWhiteSpace($name)) {
      $name = "(blank)"
    }
    if (-not $counts.ContainsKey($name)) {
      $counts[$name] = 0
    }
    $counts[$name] += 1
  }
  return @(
    $counts.Keys |
      Sort-Object |
      ForEach-Object {
        [ordered]@{
          name = $_
          count = $counts[$_]
        }
      }
  )
}

function Build-WorkflowMatrix {
  param($Registry, $CaseHealth)
  return [ordered]@{
    by_workflow = Group-CaseCounts -Cases $CaseHealth -Property "workflow"
    by_surface_category = Group-CaseCounts -Cases $CaseHealth -Property "surface_category"
    by_claim_class = Group-CaseCounts -Cases $CaseHealth -Property "claim_class"
    by_priority = Group-CaseCounts -Cases $CaseHealth -Property "priority"
    by_automation_mode = Group-CaseCounts -Cases $CaseHealth -Property "automation_mode"
    heavy_cases = @($CaseHealth | Where-Object { $_.heavy }).Count
    gpu_required_cases = @($CaseHealth | Where-Object { $_.gpu_required }).Count
    credentialed_cases = @($CaseHealth | Where-Object { $_.credentials_required }).Count
    destructive_cases = @($CaseHealth | Where-Object { $_.destructive }).Count
  }
}

function Build-MissingAuthorityActions {
  param($CaseHealth)
  $actions = @()
  foreach ($case in @($CaseHealth)) {
    if ($case.script_missing) {
      $actions += [ordered]@{
        case_id = $case.id
        severity = "high"
        action = "Script path is declared but missing"
        path = $case.script.path
      }
    }
    if ($case.baseline_missing) {
      $actions += [ordered]@{
        case_id = $case.id
        severity = "medium"
        action = "Baseline path is declared but missing"
        path = $case.baseline.path
      }
    }
    if ($case.missing_automated_authority) {
      $actions += [ordered]@{
        case_id = $case.id
        severity = "medium"
        action = "No automated performance authority is available yet"
        path = ""
      }
    }
  }
  return $actions
}

function Format-CommandLine {
  param(
    [Parameter(Mandatory = $true)][string]$Command,
    [string[]]$Arguments = @()
  )
  $parts = @($Command) + $Arguments
  return (($parts | ForEach-Object {
    $s = [string]$_
    if ($s -match "\s") {
      '"' + ($s -replace '"', '\"') + '"'
    } else {
      $s
    }
  }) -join " ")
}

function Format-ProcessArgumentString {
  param([string[]]$Arguments = @())
  return (($Arguments | ForEach-Object {
    $s = [string]$_
    if ($s.Length -eq 0) {
      '""'
    } elseif ($s -match '[\s"]') {
      '"' + (($s -replace '\\(?=("|$))', '\\') -replace '"', '\"') + '"'
    } else {
      $s
    }
  }) -join " ")
}

function New-GateRun {
  param(
    [Parameter(Mandatory = $true)][string]$Id,
    [Parameter(Mandatory = $true)][string]$Command,
    [string[]]$Arguments,
    [string]$WorkingDirectory = $RepoRoot,
    [string]$ExpectedOutput = "",
    [string]$BaselinePath = "",
    [string]$Notes = ""
  )
  return [ordered]@{
    id = $Id
    command = $Command
    arguments = $Arguments
    working_directory = $WorkingDirectory
    command_line = Format-CommandLine -Command $Command -Arguments $Arguments
    expected_output = $ExpectedOutput
    baseline_path = $BaselinePath
    notes = $Notes
  }
}

function Get-GateRuns {
  $rawDir = Join-Path $OutDir "raw"
  $localBaselineDir = Join-Path $OutDir "local_baselines"
  $gateMode = "candidate"
  if ($Mode -eq "local-baseline") {
    $gateMode = "baseline"
  }

  $runs = @()
  if ($Mode -eq "plan-only") {
    $releasePlan = Join-Path $rawDir "gpu_validation_perf_plan.json"
    $runs += New-GateRun `
      -Id "gpu_validation_release_plan" `
      -Command "powershell" `
      -Arguments @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_gpu_validation_perf_gate.ps1"), "-Mode", "candidate", "-PlanOnly", "-OutPath", $releasePlan) `
      -ExpectedOutput $releasePlan `
      -BaselinePath (Resolve-RepoPath "scripts/baselines/gpu_validation_perf_baseline.json") `
      -Notes "Plan-only inventory; does not run Criterion or structured probe."

    if ($IncludeGpuValidationHeavy) {
      $heavyPlan = Join-Path $rawDir "gpu_validation_heavy_perf_plan.json"
      $runs += New-GateRun `
        -Id "gpu_validation_heavy_plan" `
        -Command "powershell" `
        -Arguments @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_gpu_validation_perf_gate.ps1"), "-Mode", "candidate", "-PlanOnly", "-WorkloadProfile", "validation-heavy", "-OutPath", $heavyPlan) `
        -ExpectedOutput $heavyPlan `
        -BaselinePath (Resolve-RepoPath "scripts/baselines/gpu_validation_heavy_perf_baseline.json") `
        -Notes "Plan-only validation-heavy inventory."
    }
    return $runs
  }

  if ($RunWorkflowGate) {
    $workflowOut = Join-Path $rawDir "workflow_regression_summary.json"
    $workflowArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_workflow_gate.ps1"), "-Mode", $gateMode, "-OutPath", $workflowOut)
    $workflowBaseline = Resolve-RepoPath "scripts/baselines/workflow_regression_baseline.json"
    if ($Mode -eq "local-baseline") {
      $workflowBaseline = Join-Path $localBaselineDir "workflow_regression_baseline.json"
      $workflowArgs += @("-BaselinePath", $workflowBaseline)
    }
    $runs += New-GateRun `
      -Id "workflow_regression" `
      -Command "powershell" `
      -Arguments $workflowArgs `
      -ExpectedOutput $workflowOut `
      -BaselinePath $workflowBaseline `
      -Notes "Core warm workflow authority."
  }

  if ($IncludeCpu) {
    $cpuOut = Join-Path $rawDir "cpu_perf_summary.json"
    $cpuWorkflowOut = Join-Path $rawDir "cpu_inner_workflow_regression_summary.json"
    $cpuArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_cpu_perf_gate.ps1"), "-Mode", $gateMode, "-OutPath", $cpuOut, "-WorkflowSummaryPath", $cpuWorkflowOut)
    $cpuBaseline = Resolve-RepoPath "scripts/baselines/cpu_perf_baseline.json"
    if ($Mode -eq "local-baseline") {
      $cpuBaseline = Join-Path $localBaselineDir "cpu_perf_baseline.json"
      $cpuArgs += @("-BaselinePath", $cpuBaseline)
    }
    $runs += New-GateRun `
      -Id "cpu_perf" `
      -Command "powershell" `
      -Arguments $cpuArgs `
      -ExpectedOutput $cpuOut `
      -BaselinePath $cpuBaseline `
      -Notes "CPU workflow and Criterion authority."
  }

  if ($IncludeGpu) {
    $gpuOut = Join-Path $rawDir "gpu_perf_summary.json"
    $gpuArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_gpu_perf_gate.ps1"), "-Mode", $gateMode, "-OutPath", $gpuOut)
    $gpuBaseline = Resolve-RepoPath "scripts/baselines/gpu_perf_baseline.json"
    if ($Mode -eq "local-baseline") {
      $gpuBaseline = Join-Path $localBaselineDir "gpu_perf_baseline.json"
      $gpuArgs += @("-BaselinePath", $gpuBaseline)
    }
    $runs += New-GateRun `
      -Id "gpu_perf" `
      -Command "powershell" `
      -Arguments $gpuArgs `
      -ExpectedOutput $gpuOut `
      -BaselinePath $gpuBaseline `
      -Notes "GPU optimization authority."

    $gpuValidationOut = Join-Path $rawDir "gpu_validation_perf_summary.json"
    $gpuValidationArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_gpu_validation_perf_gate.ps1"), "-Mode", $gateMode, "-OutPath", $gpuValidationOut)
    $gpuValidationBaseline = Resolve-RepoPath "scripts/baselines/gpu_validation_perf_baseline.json"
    if ($Mode -eq "local-baseline") {
      $gpuValidationBaseline = Join-Path $localBaselineDir "gpu_validation_perf_baseline.json"
      $gpuValidationArgs += @("-BaselinePath", $gpuValidationBaseline)
    }
    $runs += New-GateRun `
      -Id "gpu_validation_perf" `
      -Command "powershell" `
      -Arguments $gpuValidationArgs `
      -ExpectedOutput $gpuValidationOut `
      -BaselinePath $gpuValidationBaseline `
      -Notes "GPU validation authority."

    if ($IncludeGpuValidationHeavy) {
      $gpuValidationHeavyOut = Join-Path $rawDir "gpu_validation_heavy_perf_summary.json"
      $gpuValidationHeavyArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_gpu_validation_perf_gate.ps1"), "-Mode", $gateMode, "-WorkloadProfile", "validation-heavy", "-OutPath", $gpuValidationHeavyOut)
      $gpuValidationHeavyBaseline = Resolve-RepoPath "scripts/baselines/gpu_validation_heavy_perf_baseline.json"
      if ($Mode -eq "local-baseline") {
        $gpuValidationHeavyBaseline = Join-Path $localBaselineDir "gpu_validation_heavy_perf_baseline.json"
        $gpuValidationHeavyArgs += @("-BaselinePath", $gpuValidationHeavyBaseline)
      }
      $runs += New-GateRun `
        -Id "gpu_validation_heavy_perf" `
        -Command "powershell" `
        -Arguments $gpuValidationHeavyArgs `
        -ExpectedOutput $gpuValidationHeavyOut `
        -BaselinePath $gpuValidationHeavyBaseline `
        -Notes "Validation-heavy GPU authority."
    }

    $gpuFollowupOut = Join-Path $rawDir "gpu_followup_perf_summary.json"
    $gpuFollowupArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_gpu_followup_perf_gate.ps1"), "-Mode", $gateMode, "-OutPath", $gpuFollowupOut)
    $gpuFollowupBaseline = Resolve-RepoPath "scripts/baselines/gpu_followup_perf_baseline.json"
    if ($Mode -eq "local-baseline") {
      $gpuFollowupBaseline = Join-Path $localBaselineDir "gpu_followup_perf_baseline.json"
      $gpuFollowupArgs += @("-BaselinePath", $gpuFollowupBaseline)
    }
    $runs += New-GateRun `
      -Id "gpu_followup_perf" `
      -Command "powershell" `
      -Arguments $gpuFollowupArgs `
      -ExpectedOutput $gpuFollowupOut `
      -BaselinePath $gpuFollowupBaseline `
      -Notes "GPU follow-up authority."
  }

  if ($IncludeOptions) {
    $optionsOutDir = Join-Path $rawDir "options_expression_phase05_gpu_gate"
    $optionsSummary = Join-Path $rawDir "options_expression_perf_summary.json"
    $optionsArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_options_expression_gpu_promotion_gate.ps1"), "-Mode", $gateMode, "-OutDir", $optionsOutDir, "-SummaryOutPath", $optionsSummary)
    $optionsBaseline = Resolve-RepoPath "scripts/baselines/options_expression_perf_baseline.json"
    if ($Mode -eq "local-baseline") {
      $optionsBaseline = Join-Path $localBaselineDir "options_expression_perf_baseline.json"
      $optionsArgs += @("-BaselinePath", $optionsBaseline)
    }
    $runs += New-GateRun `
      -Id "options_expression_perf" `
      -Command "powershell" `
      -Arguments $optionsArgs `
      -ExpectedOutput $optionsSummary `
      -BaselinePath $optionsBaseline `
      -Notes "Requires a configured options fixture, artifact path, dataset id, or import args."
  }

  if ($IncludeSupport) {
    $supportOut = Join-Path $rawDir "support_perf_summary.json"
    $supportMode = if ($Mode -eq "local-baseline") { "local-baseline" } else { $gateMode }
    $supportArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_support_perf_gate.ps1"), "-Mode", $supportMode, "-OutPath", $supportOut)
    $supportBaseline = Resolve-RepoPath "scripts/baselines/support_perf_baseline.json"
    if ($Mode -eq "local-baseline") {
      $supportBaseline = Join-Path $localBaselineDir "support_perf_baseline.json"
    }
    $supportArgs += @("-BaselinePath", $supportBaseline)
    $runs += New-GateRun `
      -Id "support_perf" `
      -Command "powershell" `
      -Arguments $supportArgs `
      -ExpectedOutput $supportOut `
      -BaselinePath $supportBaseline `
      -Notes "Non-credential data/options/report/artifact/portfolio/dynamic/live-status/execution support authority."

    $campaignOut = Join-Path $rawDir "ta_campaign_perf_summary.json"
    $campaignMode = if ($Mode -eq "local-baseline") { "local-baseline" } else { $gateMode }
    $campaignArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Resolve-RepoPath "scripts/run_campaign_perf_gate.ps1"), "-Mode", $campaignMode, "-OutPath", $campaignOut)
    $campaignBaseline = Resolve-RepoPath "scripts/baselines/ta_campaign_perf_baseline.json"
    if ($Mode -eq "local-baseline") {
      $campaignBaseline = Join-Path $localBaselineDir "ta_campaign_perf_baseline.json"
    }
    $campaignArgs += @("-BaselinePath", $campaignBaseline)
    $runs += New-GateRun `
      -Id "campaign_perf" `
      -Command "powershell" `
      -Arguments $campaignArgs `
      -ExpectedOutput $campaignOut `
      -BaselinePath $campaignBaseline `
      -Notes "Lightweight campaign orchestration authority."
  }

  if ($IncludeFrontendBudget) {
    $runs += New-GateRun `
      -Id "frontend_ui_perf_budget" `
      -Command "powershell" `
      -Arguments @("-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "npm run perf:ui:budget") `
      -WorkingDirectory (Resolve-RepoPath "web") `
      -ExpectedOutput (Resolve-RepoPath "scripts/baselines/ui_capture/perf_metrics.json") `
      -BaselinePath (Resolve-RepoPath "scripts/baselines/ui_capture/perf_metrics.json") `
      -Notes "Budget check only; does not run baseline:ui capture."
  }

  if ($IncludeFrontendInteractionLatency) {
    $frontendInteractionOut = Join-Path $rawDir "frontend_interaction_latency_summary.json"
    $runs += New-GateRun `
      -Id "frontend_interaction_latency" `
      -Command "powershell" `
      -Arguments @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", (Resolve-RepoPath "scripts/run_frontend_interaction_latency.ps1"),
        "-OutPath", $frontendInteractionOut
      ) `
      -ExpectedOutput $frontendInteractionOut `
      -BaselinePath (Resolve-RepoPath "scripts/frontend_interaction_latency_policy.json") `
      -Notes "Scoped warm mocked-browser trusted click-to-painted-stable authority with fixed policy budgets; full/native latency remains outside this lane."
  }

  return $runs
}

function Invoke-GateRun {
  param($Run)
  $logDir = Join-Path $OutDir "logs"
  New-Item -ItemType Directory -Force -Path $logDir | Out-Null
  $logPath = Join-Path $logDir ("{0}.log" -f $Run.id)
  $started = Get-Date
  $exitCode = 0
  $status = "passed"
  $errorText = ""

  Push-Location $Run.working_directory
  try {
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = [string]$Run.command
    $psi.Arguments = Format-ProcessArgumentString -Arguments @($Run.arguments)
    $psi.WorkingDirectory = [string]$Run.working_directory
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $psi
    try {
      $null = $process.Start()
      $stdoutTask = $process.StandardOutput.ReadToEndAsync()
      $stderrTask = $process.StandardError.ReadToEndAsync()
      if (-not $process.WaitForExit($GateTimeoutSec * 1000)) {
        $status = "failed"
        $exitCode = 124
        $errorText = "Gate run timed out after $GateTimeoutSec seconds"
        try {
          $process.Kill($true)
        } catch {
          try { $process.Kill() } catch {}
        }
        $process.WaitForExit()
      } else {
        $exitCode = [int]$process.ExitCode
      }
      $stdout = $stdoutTask.Result
      $stderr = $stderrTask.Result
      $logParts = @()
      if (-not [string]::IsNullOrWhiteSpace($stdout)) {
        $logParts += $stdout.TrimEnd()
      }
      if (-not [string]::IsNullOrWhiteSpace($stderr)) {
        $logParts += $stderr.TrimEnd()
      }
      $logParts | Out-File -Encoding UTF8 -FilePath $logPath
    } finally {
      $process.Dispose()
    }
    if ($exitCode -ne 0) {
      $status = "failed"
    }
  } catch {
    $status = "failed"
    $exitCode = 1
    $errorText = $_.Exception.Message
    @($errorText) | Out-File -Encoding UTF8 -FilePath $logPath
  } finally {
    Pop-Location
  }

  $ended = Get-Date
  return [ordered]@{
    id = $Run.id
    status = $status
    exit_code = $exitCode
    started_utc = $started.ToUniversalTime().ToString("o")
    ended_utc = $ended.ToUniversalTime().ToString("o")
    duration_ms = [math]::Round(($ended - $started).TotalMilliseconds, 3)
    command_line = $Run.command_line
    working_directory = Get-RepoRelativePath $Run.working_directory
    log_path = Get-RepoRelativePath $logPath
    expected_output = Get-PathStatus -Path $Run.expected_output
    baseline_path = Get-PathStatus -Path $Run.baseline_path
    notes = $Run.notes
    error = $errorText
  }
}

function Write-MarkdownSummary {
  param(
    [Parameter(Mandatory = $true)]$Summary,
    [Parameter(Mandatory = $true)]$Coverage,
    [Parameter(Mandatory = $true)]$WorkflowMatrix,
    [Parameter(Mandatory = $true)]$MissingAuthorityActions,
    [Parameter(Mandatory = $true)]$ActivePerfProcesses,
    [Parameter(Mandatory = $true)]$RunManifest,
    [Parameter(Mandatory = $true)][string]$Path
  )

  $lines = @()
  $lines += "# Performance Authority Baseline Pack"
  $lines += ""
  $lines += "- Mode: ``$($Summary.mode)``"
  $lines += "- Generated UTC: ``$($Summary.generated_utc)``"
  $lines += "- Registry: ``$($Summary.registry.registry_version)``"
  $lines += "- Cases: ``$($Summary.registry.case_count)``"
  $lines += "- Tauri handlers covered: ``$($Coverage.registry_command_count)`` / ``$($Coverage.tauri_handler_count)``"
  $lines += "- Command modules covered: ``$($Coverage.registry_module_count)`` / ``$($Coverage.command_module_count)``"
  $lines += "- Missing authority actions: ``$(@($MissingAuthorityActions).Count)``"
  $lines += "- Missing release-required baselines: ``$($Summary.release_baselines.missing_count)``"
  $lines += "- Active benchmark/profiler processes at inventory time: ``$(@($ActivePerfProcesses).Count)``"
  $lines += ""
  $lines += "## Coverage"
  $lines += ""
  if (@($Coverage.uncovered_commands).Count -eq 0 -and @($Coverage.uncovered_modules).Count -eq 0) {
    $lines += "No handler or command-module coverage gaps were found."
  } else {
    if (@($Coverage.uncovered_commands).Count -gt 0) {
      $lines += "- Uncovered handlers: ``$($Coverage.uncovered_commands -join ', ')``"
    }
    if (@($Coverage.uncovered_modules).Count -gt 0) {
      $lines += "- Uncovered modules: ``$($Coverage.uncovered_modules -join ', ')``"
    }
  }
  $lines += ""
  $lines += "## Case Counts"
  $lines += ""
  foreach ($group in $WorkflowMatrix.by_workflow) {
    $lines += "- ``$($group.name)``: $($group.count)"
  }
  $lines += ""
  $lines += "## Authority Gaps"
  $lines += ""
  if (@($MissingAuthorityActions).Count -eq 0) {
    $lines += "No missing authority actions were detected."
  } else {
    foreach ($action in @($MissingAuthorityActions | Select-Object -First 25)) {
      $lines += "- [$($action.severity)] ``$($action.case_id)``: $($action.action)"
    }
    if (@($MissingAuthorityActions).Count -gt 25) {
      $lines += "- ... $(@($MissingAuthorityActions).Count - 25) more actions in ``missing_authority_actions.json``"
    }
  }
  $lines += ""
  $lines += "## Gate Runs"
  $lines += ""
  if (@($RunManifest).Count -eq 0) {
    $lines += "No benchmark gates were run. This is expected for ``inventory`` mode."
  } else {
    foreach ($run in @($RunManifest)) {
      $lines += "- ``$($run.id)``: $($run.status) in $($run.duration_ms) ms, log ``$($run.log_path)``"
    }
  }
  $lines += ""
  $lines += "## Safe Next Commands"
  $lines += ""
  $lines += "- Strict inventory: ``powershell -File scripts/run_performance_audit_baseline_pack.ps1 -FailOnCoverageGap -FailOnActivePerfProcess -FailOnMissingReleaseBaseline``"
  $lines += "- GPU validation plans only: ``powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode plan-only -IncludeGpuValidationHeavy``"
  $lines += "- Core candidate gate only: ``powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode candidate``"
  $lines += "- CPU candidate pack: ``powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode candidate -IncludeCpu``"
  $lines += "- Support candidate pack: ``powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode candidate -IncludeSupport``"
  $lines += ""
  $lines | Set-Content -Encoding UTF8 -Path $Path
}

$resolvedRegistryPath = Resolve-RepoPath $RegistryPath
if (-not (Test-Path $resolvedRegistryPath)) {
  throw "Missing performance audit registry: $resolvedRegistryPath"
}

if ($Mode -eq "local-baseline" -and -not $AllowLocalBaselineCapture) {
  throw "local-baseline mode writes local baseline files under the output pack. Re-run with -AllowLocalBaselineCapture if that is intended."
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$registry = Read-JsonFile -Path $resolvedRegistryPath
$coverage = Build-Coverage -Registry $registry
$caseHealth = Build-CaseHealth -Registry $registry
$workflowMatrix = Build-WorkflowMatrix -Registry $registry -CaseHealth $caseHealth
$missingAuthorityActions = @(Build-MissingAuthorityActions -CaseHealth $caseHealth)
$releaseBaselineHealth = @(Get-VectorGridReleaseBaselineHealth -Registry $registry -RepoRoot $RepoRoot)
$missingReleaseBaselines = @($releaseBaselineHealth | Where-Object { -not $_.exists })
$activePerfProcesses = @(Get-PerfProcesses)
$environment = Get-EnvironmentFingerprint
$git = Get-GitFingerprint
$runManifest = @()

if ($FailOnActivePerfProcess -and $activePerfProcesses.Count -gt 0) {
  Write-JsonUtf8 -Path (Join-Path $OutDir "active_perf_processes.json") -Value $activePerfProcesses
  throw "Active benchmark/profiler processes were detected. See $((Join-Path $OutDir 'active_perf_processes.json'))."
}

if ($FailOnMissingReleaseBaseline -and $missingReleaseBaselines.Count -gt 0) {
  Write-JsonUtf8 -Path (Join-Path $OutDir "release_baseline_health.json") -Value $releaseBaselineHealth
  $missingSummary = ($missingReleaseBaselines | ForEach-Object {
      "$($_.case_id) [$($_.path)]"
    }) -join ", "
  throw "Release-required performance baselines are missing: $missingSummary"
}

if ($Mode -ne "inventory" -and $activePerfProcesses.Count -gt 0) {
  Write-JsonUtf8 -Path (Join-Path $OutDir "active_perf_processes.json") -Value $activePerfProcesses
  throw "Refusing to run performance gates while benchmark/profiler processes are active. See $((Join-Path $OutDir 'active_perf_processes.json'))."
}

if ($FailOnCoverageGap -and (($coverage.uncovered_commands.Count -gt 0) -or ($coverage.uncovered_modules.Count -gt 0))) {
  Write-JsonUtf8 -Path (Join-Path $OutDir "command_surface_coverage.json") -Value $coverage
  throw "Performance audit registry has command-surface coverage gaps."
}

if ($Mode -ne "inventory") {
  . (Join-Path $PSScriptRoot "benchmark_lock.ps1")
  $lock = Enter-BenchmarkLock -Context "performance audit baseline pack" -TimeoutSeconds 900
  try {
    foreach ($run in @(Get-GateRuns)) {
      $requiresDirectFrontendLock = "$($run.id)" -eq "frontend_interaction_latency"
      if ($requiresDirectFrontendLock) {
        if ($null -eq $lock -or -not [bool]$lock.acquired) {
          throw "The performance audit pack cannot delegate inherited benchmark-lock authority to frontend_interaction_latency; run that authority directly."
        }
        Exit-BenchmarkLock -Handle $lock
        $lock = $null
      }
      try {
        Write-Host "Running $($run.id): $($run.command_line)"
        $runResult = Invoke-GateRun -Run $run
        $runManifest += $runResult
        Write-Host "$($run.id): $($runResult.status)"
      } finally {
        if ($requiresDirectFrontendLock) {
          $lock = Enter-BenchmarkLock `
            -Context "performance audit baseline pack" `
            -TimeoutSeconds 900
        }
      }
      if ($runResult.status -ne "passed") {
        break
      }
    }
  } finally {
    Exit-BenchmarkLock -Handle $lock
  }
}

$summary = [ordered]@{
  schema_version = 1
  mode = $Mode
  generated_utc = (Get-Date).ToUniversalTime().ToString("o")
  repo_root = $RepoRoot
  output_dir = $OutDir
  registry = [ordered]@{
    path = Get-RepoRelativePath $resolvedRegistryPath
    registry_version = $registry.registry_version
    case_count = @($registry.cases).Count
  }
  environment = $environment
  git = $git
  command_surface = [ordered]@{
    tauri_handler_count = $coverage.tauri_handler_count
    command_module_count = $coverage.command_module_count
    registry_command_count = $coverage.registry_command_count
    registry_module_count = $coverage.registry_module_count
    uncovered_command_count = @($coverage.uncovered_commands).Count
    uncovered_module_count = @($coverage.uncovered_modules).Count
  }
  workflow_matrix = $workflowMatrix
  missing_authority_action_count = @($missingAuthorityActions).Count
  release_baselines = [ordered]@{
    required_count = $releaseBaselineHealth.Count
    missing_count = $missingReleaseBaselines.Count
  }
  active_perf_process_count = @($activePerfProcesses).Count
  gate_run_count = @($runManifest).Count
  gate_fail_count = @($runManifest | Where-Object { $_.status -ne "passed" }).Count
}

Write-JsonUtf8 -Path (Join-Path $OutDir "case_registry_snapshot.json") -Value $registry
Write-JsonUtf8 -Path (Join-Path $OutDir "command_surface_coverage.json") -Value $coverage
Write-JsonUtf8 -Path (Join-Path $OutDir "case_health.json") -Value $caseHealth
Write-JsonUtf8 -Path (Join-Path $OutDir "workflow_matrix.json") -Value $workflowMatrix
Write-JsonUtf8 -Path (Join-Path $OutDir "missing_authority_actions.json") -Value $missingAuthorityActions
Write-JsonUtf8 -Path (Join-Path $OutDir "release_baseline_health.json") -Value $releaseBaselineHealth
Write-JsonUtf8 -Path (Join-Path $OutDir "active_perf_processes.json") -Value $activePerfProcesses
Write-JsonUtf8 -Path (Join-Path $OutDir "gate_run_manifest.json") -Value $runManifest
Write-JsonUtf8 -Path (Join-Path $OutDir "summary.json") -Value $summary
Write-MarkdownSummary `
  -Summary $summary `
  -Coverage $coverage `
  -WorkflowMatrix $workflowMatrix `
  -MissingAuthorityActions $missingAuthorityActions `
  -ActivePerfProcesses $activePerfProcesses `
  -RunManifest $runManifest `
  -Path (Join-Path $OutDir "summary.md")

Write-Host "Performance audit baseline pack written to $OutDir"

if (@($runManifest | Where-Object { $_.status -ne "passed" }).Count -gt 0) {
  exit 1
}
