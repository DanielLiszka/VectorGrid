param(
  [ValidateSet("baseline", "candidate", "local-baseline")]
  [string]$Mode = "candidate",
  [string]$OutPath = "output/benchmarks/support_perf_summary.json",
  [string]$BaselinePath = "scripts/baselines/support_perf_baseline.json",
  [string]$WorkDir = "_write_test/perf_authority_support",
  [string[]]$Case = @(),
  [double]$CaseRegressionPct = 50.0,
  [double]$StepRegressionPct = 75.0,
  [double]$MinComparableMs = 2.0,
  [switch]$DevBuild
)

$ErrorActionPreference = "Stop"

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")

function Resolve-RepoPath {
  param([string]$Path)
  if ([string]::IsNullOrWhiteSpace($Path)) {
    return ""
  }
  if ([System.IO.Path]::IsPathRooted($Path)) {
    return [System.IO.Path]::GetFullPath($Path)
  }
  return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
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
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Read-JsonFile {
  param([Parameter(Mandatory = $true)][string]$Path)
  return Get-Content -Raw $Path | ConvertFrom-Json
}

function Assert-Compatible {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [AllowNull()]$Candidate,
    [AllowNull()]$Baseline
  )
  if ("$Candidate" -ne "$Baseline") {
    throw "Incompatible support perf baseline for ${Name}: candidate='$Candidate' baseline='$Baseline'"
  }
}

function Get-NumberProperty {
  param(
    [Parameter(Mandatory = $true)]$Object,
    [Parameter(Mandatory = $true)][string]$Name
  )
  $prop = $Object.PSObject.Properties[$Name]
  if ($null -eq $prop -or $null -eq $prop.Value) {
    return $null
  }
  $value = [double]$prop.Value
  if ([double]::IsNaN($value) -or [double]::IsInfinity($value)) {
    return $null
  }
  return $value
}

function Assert-TimingNotRegressed {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [AllowNull()]$CandidateMs,
    [AllowNull()]$BaselineMs,
    [Parameter(Mandatory = $true)][double]$RegressionPct,
    [Parameter(Mandatory = $true)][double]$MinMs
  )
  if ($null -eq $CandidateMs -or $null -eq $BaselineMs) {
    return
  }
  if ([double]$BaselineMs -lt $MinMs) {
    return
  }
  $allowed = [double]$BaselineMs * (1.0 + ($RegressionPct / 100.0))
  if ([double]$CandidateMs -gt $allowed) {
    throw ("Support perf regression for {0}: candidate={1:N3}ms baseline={2:N3}ms allowed={3:N3}ms (+{4:N1}%)" -f $Name, [double]$CandidateMs, [double]$BaselineMs, $allowed, $RegressionPct)
  }
}

function Assert-SupportTimings {
  param(
    [Parameter(Mandatory = $true)]$Candidate,
    [Parameter(Mandatory = $true)]$Baseline
  )
  $baselineById = @{}
  foreach ($case in @($Baseline.cases)) {
    $baselineById[[string]$case.id] = $case
  }
  foreach ($case in @($Candidate.cases)) {
    $caseId = [string]$case.id
    if (-not $baselineById.ContainsKey($caseId)) {
      throw "Support perf baseline is missing case '$caseId'"
    }
    $baselineCase = $baselineById[$caseId]
    Assert-TimingNotRegressed `
      -Name $caseId `
      -CandidateMs (Get-NumberProperty -Object $case -Name "wall_ms") `
      -BaselineMs (Get-NumberProperty -Object $baselineCase -Name "wall_ms") `
      -RegressionPct $CaseRegressionPct `
      -MinMs $MinComparableMs

    $baselineSteps = @{}
    foreach ($step in @($baselineCase.steps)) {
      $baselineSteps[[string]$step.id] = $step
    }
    foreach ($step in @($case.steps)) {
      $stepId = [string]$step.id
      if (-not $baselineSteps.ContainsKey($stepId)) {
        throw "Support perf baseline case '$caseId' is missing step '$stepId'"
      }
      Assert-TimingNotRegressed `
        -Name "$caseId/$stepId" `
        -CandidateMs (Get-NumberProperty -Object $step -Name "wall_ms") `
        -BaselineMs (Get-NumberProperty -Object $baselineSteps[$stepId] -Name "wall_ms") `
        -RegressionPct $StepRegressionPct `
        -MinMs $MinComparableMs
    }
  }
}

$resolvedOut = Resolve-RepoPath $OutPath
$resolvedWorkDir = Resolve-RepoPath $WorkDir
$resolvedWriteTestRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "_write_test"))
$outDir = Split-Path -Parent $resolvedOut
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$workDirIsDisposable = $resolvedWorkDir -eq $resolvedWriteTestRoot -or
  $resolvedWorkDir.StartsWith($resolvedWriteTestRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)
if ($workDirIsDisposable -and (Test-Path -LiteralPath $resolvedWorkDir)) {
  Remove-Item -LiteralPath $resolvedWorkDir -Recurse -Force
} elseif (-not $workDirIsDisposable) {
  Write-Warning "Support perf work dir is outside _write_test; skipping cleanup: $resolvedWorkDir"
}
New-Item -ItemType Directory -Force -Path $resolvedWorkDir | Out-Null

$lock = Enter-BenchmarkLock -Context "support perf gate"
try {
  $cargoArgs = @("run")
  if (-not $DevBuild) {
    $cargoArgs += "--release"
  }
  $cargoArgs += @(
    "-p", "ta_desktop_demo_app",
    "--features", "maintainer-tools",
    "--bin", "ta_perf_authority_support",
    "--",
    "--out", $resolvedOut,
    "--work-dir", $resolvedWorkDir
  )
  foreach ($caseName in $Case) {
    if (-not [string]::IsNullOrWhiteSpace($caseName)) {
      $cargoArgs += @("--case", $caseName)
    }
  }

  & cargo @cargoArgs
  if ($LASTEXITCODE -ne 0) {
    throw "support perf harness failed with exit code $LASTEXITCODE"
  }
} finally {
  Exit-BenchmarkLock $lock
}

if (!(Test-Path $resolvedOut)) {
  throw "Support perf gate did not produce the requested output: $resolvedOut"
}

$summary = Read-JsonFile -Path $resolvedOut

if ($Mode -eq "local-baseline" -or $Mode -eq "baseline") {
  if ([string]::IsNullOrWhiteSpace($BaselinePath)) {
    throw "$Mode mode requires -BaselinePath"
  }
  $resolvedBaseline = Resolve-RepoPath $BaselinePath
  $baselineDir = Split-Path -Parent $resolvedBaseline
  if ($baselineDir) {
    New-Item -ItemType Directory -Force -Path $baselineDir | Out-Null
  }
  Copy-Item -LiteralPath $resolvedOut -Destination $resolvedBaseline -Force
  Write-Host "Support perf baseline copied to $resolvedBaseline"
} elseif (-not [string]::IsNullOrWhiteSpace($BaselinePath)) {
  $resolvedBaseline = Resolve-RepoPath $BaselinePath
  if (Test-Path $resolvedBaseline) {
    $baseline = Read-JsonFile -Path $resolvedBaseline
    Assert-Compatible -Name "schema_version" -Candidate $summary.schema_version -Baseline $baseline.schema_version
    Assert-Compatible -Name "timing_boundary" -Candidate $summary.timing_boundary -Baseline $baseline.timing_boundary
    Assert-SupportTimings -Candidate $summary -Baseline $baseline
  } else {
    throw "Support perf candidate mode requires an existing baseline: $resolvedBaseline"
  }
}

Write-Host "Support perf summary written to $resolvedOut"
