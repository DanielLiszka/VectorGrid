param(
  [ValidateSet("baseline", "candidate", "local-baseline")]
  [string]$Mode = "candidate",
  [string]$OutPath = "output/benchmarks/ta_campaign_perf_summary.json",
  [string]$BaselinePath = "scripts/baselines/ta_campaign_perf_baseline.json",
  [string]$DatasetPath = "_write_test/perf_authority_support/campaign_synthetic.csv",
  [int]$Rows = 8000,
  [double]$RegressionPct = 50.0,
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

function Read-JsonFile {
  param([Parameter(Mandatory = $true)][string]$Path)
  return Get-Content -Raw $Path | ConvertFrom-Json
}

function New-SyntheticOhlcvCsv {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][int]$RowCount
  )
  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $writer = [System.IO.StreamWriter]::new($Path, $false, [System.Text.UTF8Encoding]::new($false))
  try {
    $writer.WriteLine("timestamp,open,high,low,close,volume")
    $baseTs = [int64]1700000000000
    for ($i = 0; $i -lt $RowCount; $i++) {
      $ts = $baseTs + ([int64]$i * 60000)
      $trend = 100.0 + ([double]$i * 0.001)
      $wave = [Math]::Sin([double]$i * 0.037) * 0.4
      $close = $trend + $wave
      $open = $close - 0.05
      $high = $close + 0.15
      $low = $close - 0.20
      $volume = 10000.0 + ($i % 97)
      $writer.WriteLine(("{0},{1:F6},{2:F6},{3:F6},{4:F6},{5:F2}" -f $ts, $open, $high, $low, $close, $volume))
    }
  } finally {
    $writer.Dispose()
  }
}

function Assert-Compatible {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [AllowNull()]$Candidate,
    [AllowNull()]$Baseline
  )
  if ("$Candidate" -ne "$Baseline") {
    throw "Incompatible campaign perf baseline for ${Name}: candidate='$Candidate' baseline='$Baseline'"
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
    [Parameter(Mandatory = $true)][double]$AllowedRegressionPct,
    [Parameter(Mandatory = $true)][double]$MinMs
  )
  if ($null -eq $CandidateMs -or $null -eq $BaselineMs) {
    return
  }
  if ([double]$BaselineMs -lt $MinMs) {
    return
  }
  $allowed = [double]$BaselineMs * (1.0 + ($AllowedRegressionPct / 100.0))
  if ([double]$CandidateMs -gt $allowed) {
    throw ("Campaign perf regression for {0}: candidate={1:N3}ms baseline={2:N3}ms allowed={3:N3}ms (+{4:N1}%)" -f $Name, [double]$CandidateMs, [double]$BaselineMs, $allowed, $AllowedRegressionPct)
  }
}

$resolvedOut = Resolve-RepoPath $OutPath
$resolvedDataset = Resolve-RepoPath $DatasetPath
$resolvedDataset2 = [System.IO.Path]::Combine(
  [System.IO.Path]::GetDirectoryName($resolvedDataset),
  ([System.IO.Path]::GetFileNameWithoutExtension($resolvedDataset) + "_b" + [System.IO.Path]::GetExtension($resolvedDataset))
)
$outDir = Split-Path -Parent $resolvedOut
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}
New-SyntheticOhlcvCsv -Path $resolvedDataset -RowCount $Rows
New-SyntheticOhlcvCsv -Path $resolvedDataset2 -RowCount $Rows

$lock = Enter-BenchmarkLock -Context "campaign perf gate"
try {
  $cargoArgs = @("run")
  if (-not $DevBuild) {
    $cargoArgs += "--release"
  }
  $cargoArgs += @(
    "-p", "ta_desktop_demo_app",
    "--bin", "ta_campaign_perf",
    "--",
    "--output-json", $resolvedOut,
    "--dataset", $resolvedDataset,
    "--dataset", $resolvedDataset2,
    "--mode", "host-only",
    "--scope", "optimize-only",
    "--backend", "cpu-only",
    "--warmup-trials", "0",
    "--measured-trials", "1",
    "--fast-range", "5:15:5",
    "--slow-range", "20:40:10",
    "--top-k", "4",
    "--validation-max-candidates", "1",
    "--cleanup-artifacts", "true"
  )

  & cargo @cargoArgs
  if ($LASTEXITCODE -ne 0) {
    throw "campaign perf harness failed with exit code $LASTEXITCODE"
  }
} finally {
  Exit-BenchmarkLock $lock
}

if (!(Test-Path $resolvedOut)) {
  throw "Campaign perf gate did not produce the requested output: $resolvedOut"
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
  Write-Host "Campaign perf baseline copied to $resolvedBaseline"
} elseif (-not [string]::IsNullOrWhiteSpace($BaselinePath)) {
  $resolvedBaseline = Resolve-RepoPath $BaselinePath
  if (Test-Path $resolvedBaseline) {
    $baseline = Read-JsonFile -Path $resolvedBaseline
    Assert-Compatible -Name "schema_version" -Candidate $summary.schema_version -Baseline $baseline.schema_version
    Assert-Compatible -Name "mode" -Candidate $summary.mode -Baseline $baseline.mode
    Assert-Compatible -Name "scope" -Candidate $summary.scope -Baseline $baseline.scope
    Assert-Compatible -Name "backend" -Candidate $summary.backend -Baseline $baseline.backend
    Assert-Compatible -Name "top_k" -Candidate $summary.top_k -Baseline $baseline.top_k
    Assert-Compatible -Name "validation_max_candidates" -Candidate $summary.validation_max_candidates -Baseline $baseline.validation_max_candidates
    Assert-Compatible -Name "measured_trials" -Candidate $summary.measured_trials -Baseline $baseline.measured_trials
    Assert-TimingNotRegressed `
      -Name "mean_total_wall_ms" `
      -CandidateMs (Get-NumberProperty -Object $summary -Name "mean_total_wall_ms") `
      -BaselineMs (Get-NumberProperty -Object $baseline -Name "mean_total_wall_ms") `
      -AllowedRegressionPct $RegressionPct `
      -MinMs $MinComparableMs
    Assert-TimingNotRegressed `
      -Name "mean_campaign_runtime_ms" `
      -CandidateMs (Get-NumberProperty -Object $summary -Name "mean_campaign_runtime_ms") `
      -BaselineMs (Get-NumberProperty -Object $baseline -Name "mean_campaign_runtime_ms") `
      -AllowedRegressionPct $RegressionPct `
      -MinMs $MinComparableMs
  } else {
    throw "Campaign perf candidate mode requires an existing baseline: $resolvedBaseline"
  }
}

Write-Host "Campaign perf summary written to $resolvedOut"
