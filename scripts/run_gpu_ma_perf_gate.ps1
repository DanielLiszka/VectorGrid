param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselinePath = "scripts/baselines/gpu_ma_perf_baseline.json",
  [string]$OutPath = "output/benchmarks/gpu_ma_perf_summary.json",
  [string]$CaseId = "",
  [string]$BenchFilter = "",
  [string]$CriterionEstimatesPath = "",
  [string]$CriterionEstimatesGlob = "",
  [double]$RegressionPct = 10.0
)

$ErrorActionPreference = "Stop"

$gateArgs = @{
  Mode = $Mode
  WorkloadProfile = "gpu-ma"
  BaselinePath = $BaselinePath
  OutPath = $OutPath
  CaseId = $CaseId
  BenchFilter = $BenchFilter
  CriterionEstimatesPath = $CriterionEstimatesPath
  CriterionEstimatesGlob = $CriterionEstimatesGlob
  RegressionPct = $RegressionPct
  CheckDynamicStrategyRatio = $false
}

& (Join-Path $PSScriptRoot "run_gpu_perf_gate.ps1") @gateArgs
