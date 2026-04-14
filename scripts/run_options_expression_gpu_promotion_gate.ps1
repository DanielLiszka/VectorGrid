param(
  [string]$ArtifactPath,
  [string]$DatasetId,
  [string]$ImportCsv,
  [string]$ImportRootSymbol,
  [string]$AppLocalDataDir = "$env:LOCALAPPDATA\dev.vectoralpha.vectorgrid",
  [string]$OutDir = "output/options_expression_phase05_gpu_gate",
  [ValidateSet("auto", "cpu_reference", "gpu_compiled_tensor")]
  [string]$BackendPreference = "auto",
  [ValidateSet("event_chronological", "session_chronological")]
  [string]$SplitAssignmentMode = "event_chronological",
  [int]$TopK = 100,
  [int]$SealedFinalistCount = 10,
  [int]$WarmRepeats = 1,
  [int]$BenchmarkIterations = 5,
  [string[]]$RequestArgs = @(),
  [double]$PreviewWallSpeedupGate = 1.0,
  [switch]$InspectTop
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
  return (Split-Path -Parent $PSScriptRoot)
}

function Join-IfRelative {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Root
  )

  if ([System.IO.Path]::IsPathRooted($Path)) {
    return $Path
  }
  return Join-Path $Root $Path
}

function Format-Status([bool]$Passed) {
  if ($Passed) {
    return "PASS"
  }
  return "FAIL"
}

$repoRoot = Resolve-RepoRoot
$phase05Script = Join-Path $repoRoot "scripts\run_options_expression_phase05.ps1"
$resolvedOutRoot = Join-IfRelative -Path $OutDir -Root $repoRoot
$timestamp = Get-Date -Format "yyyy-MM-ddTHH-mm-ss"
$runOutRoot = Join-Path $resolvedOutRoot $timestamp

$phase05Args = @(
  "-ExecutionPolicy",
  "Bypass",
  "-File",
  $phase05Script,
  "-AppLocalDataDir",
  $AppLocalDataDir,
  "-OutDir",
  $runOutRoot,
  "-Cuda",
  "-BackendPreference",
  $BackendPreference,
  "-SplitAssignmentMode",
  $SplitAssignmentMode,
  "-TopK",
  [string][Math]::Max(1, $TopK),
  "-SealedFinalistCount",
  [string][Math]::Max(1, [Math]::Min($SealedFinalistCount, [Math]::Max(1, $TopK))),
  "-WarmRepeats",
  [string][Math]::Max(0, $WarmRepeats),
  "-BenchmarkIterations",
  [string][Math]::Max(1, $BenchmarkIterations),
  "-CompareBackends"
)

if ($ArtifactPath) {
  $phase05Args += @("-ArtifactPath", $ArtifactPath)
}
if ($DatasetId) {
  $phase05Args += @("-DatasetId", $DatasetId)
}
if ($ImportCsv) {
  $phase05Args += @("-ImportCsv", $ImportCsv)
}
if ($ImportRootSymbol) {
  $phase05Args += @("-ImportRootSymbol", $ImportRootSymbol)
}
if ($RequestArgs -and $RequestArgs.Count -gt 0) {
  $phase05Args += @("-RequestArgs") + $RequestArgs
}
if ($InspectTop) {
  $phase05Args += "-InspectTop"
}

Push-Location $repoRoot
try {
  & powershell @phase05Args
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
} finally {
  Pop-Location
}

$summaryJson = Get-ChildItem $runOutRoot -Recurse -Filter "phase05_summary.json" -File |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1

if ($null -eq $summaryJson) {
  throw "Phase 0.5 summary JSON was not produced under $runOutRoot"
}

$summary = Get-Content -Raw $summaryJson.FullName | ConvertFrom-Json
$phase05 = $summary.phase05_gates
$comparison = $summary.backend_comparison
$previewWallSpeedup = $null
if ($null -ne $comparison) {
  if ($null -ne $comparison.estimated_backend_preview_wall_speedup_vs_cpu) {
    $previewWallSpeedup = [double]$comparison.estimated_backend_preview_wall_speedup_vs_cpu
  } elseif ($null -ne $comparison.preview_wall_speedup_vs_cpu) {
    $previewWallSpeedup = [double]$comparison.preview_wall_speedup_vs_cpu
  }
}

$checks = @(
  [pscustomobject]@{
    Label = "Candidate density gate"
    Value = "{0:P1}" -f [double]$phase05.valid_slot_density_observed
    Detail = ">= {0:P1}" -f [double]$phase05.valid_slot_density_gate_min
    Passed = [bool]$phase05.valid_slot_density_pass
  },
  [pscustomobject]@{
    Label = "Invalid-slot gate"
    Value = "{0:P1}" -f [double]$phase05.invalid_slot_rate_observed
    Detail = "<= {0:P1}" -f [double]$phase05.invalid_slot_rate_gate_max
    Passed = [bool]$phase05.invalid_slot_rate_pass
  },
  [pscustomobject]@{
    Label = "Structural cache reuse gate"
    Value = if ($null -eq $phase05.structural_cache_reuse_observed) { "n/a" } else { "{0:P1}" -f [double]$phase05.structural_cache_reuse_observed }
    Detail = ">= {0:P1}" -f [double]$phase05.cache_reuse_gate_min
    Passed = [bool]($phase05.structural_cache_reuse_pass -eq $true)
  },
  [pscustomobject]@{
    Label = "Summary cache reuse gate"
    Value = if ($null -eq $phase05.summary_cache_reuse_observed) { "n/a" } else { "{0:P1}" -f [double]$phase05.summary_cache_reuse_observed }
    Detail = ">= {0:P1}" -f [double]$phase05.cache_reuse_gate_min
    Passed = [bool]($phase05.summary_cache_reuse_pass -eq $true)
  },
  [pscustomobject]@{
    Label = "GPU hybrid-ranking speed gate"
    Value = if ($null -eq $phase05.gpu_speedup_observed) { "n/a" } else { "{0:N2}x" -f [double]$phase05.gpu_speedup_observed }
    Detail = ">= {0:N2}x" -f [double]$phase05.gpu_speedup_gate_min
    Passed = [bool]($phase05.gpu_speedup_pass -eq $true)
  },
  [pscustomobject]@{
    Label = "Estimated backend preview wall gate"
    Value = if ($null -eq $previewWallSpeedup) { "n/a" } else { "{0:N2}x" -f $previewWallSpeedup }
    Detail = "> {0:N2}x" -f $PreviewWallSpeedupGate
    Passed = ($null -ne $previewWallSpeedup) -and ($previewWallSpeedup -gt $PreviewWallSpeedupGate)
  }
)

Write-Host ""
Write-Host "== Options GPU Promotion Gate =="
Write-Host ("summary: {0}" -f $summaryJson.FullName)
foreach ($check in $checks) {
  Write-Host ("[{0}] {1}: {2} ({3})" -f (Format-Status $check.Passed), $check.Label, $check.Value, $check.Detail)
}

if ($summary.recommendations) {
  Write-Host ""
  Write-Host "Recommendations:"
  foreach ($recommendation in $summary.recommendations) {
    Write-Host ("- {0}" -f $recommendation)
  }
}

$failedChecks = $checks | Where-Object { -not $_.Passed }
if ($failedChecks.Count -gt 0) {
  $failedLabels = ($failedChecks | ForEach-Object { $_.Label }) -join "; "
  throw "Options GPU promotion gate failed: $failedLabels"
}

return
