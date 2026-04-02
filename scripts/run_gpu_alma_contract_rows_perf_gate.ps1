param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$BaselineDir = "scripts/baselines/gpu_alma_contract_rows",
  [string]$SummaryPath = "target/gpu_alma_contract_rows_perf_summary.json",
  [double]$RegressionPct = 10.0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "Specialized ALMA contract-row GPU perf gate (not the default PR perf authority)." -ForegroundColor Cyan

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$gateScript = Join-Path $PSScriptRoot "run_gpu_perf_gate.ps1"

$specs = @(
  [ordered]@{
    name = "auto"
    bench_filter = "double_ma/gpu_kernel_contract_rows/alma_alma/trending_200k/large/58300pairs/auto"
    criterion_estimates_path = "target/criterion/double_ma_gpu_kernel_contract_rows_alma_alma_trending_200k/large_58300pairs_auto/new/estimates.json"
    baseline_path = Join-Path $BaselineDir "auto.json"
    out_path = "target/gpu_alma_contract_rows_perf/auto.json"
  },
  [ordered]@{
    name = "full17"
    bench_filter = "double_ma/gpu_kernel_contract_rows/alma_alma/trending_200k/large/58300pairs/full17"
    criterion_estimates_path = "target/criterion/double_ma_gpu_kernel_contract_rows_alma_alma_trending_200k/large_58300pairs_full17/new/estimates.json"
    baseline_path = Join-Path $BaselineDir "full17.json"
    out_path = "target/gpu_alma_contract_rows_perf/full17.json"
  },
  [ordered]@{
    name = "trade22"
    bench_filter = "double_ma/gpu_kernel_contract_rows/alma_alma/trending_200k/large/58300pairs/trade22"
    criterion_estimates_path = "target/criterion/double_ma_gpu_kernel_contract_rows_alma_alma_trending_200k/large_58300pairs_trade22/new/estimates.json"
    baseline_path = Join-Path $BaselineDir "trade22.json"
    out_path = "target/gpu_alma_contract_rows_perf/trade22.json"
  }
)

$summaryDir = Split-Path -Parent $SummaryPath
if ($summaryDir) {
  New-Item -ItemType Directory -Force -Path $summaryDir | Out-Null
}
New-Item -ItemType Directory -Force -Path $BaselineDir | Out-Null

$results = @()
foreach ($spec in $specs) {
  Write-Host "Running ALMA contract-row GPU perf gate for $($spec.name)..."
  & $gateScript `
    -Mode $Mode `
    -BaselinePath $spec.baseline_path `
    -OutPath $spec.out_path `
    -CaseId "gpu_contract_rows_$($spec.name)" `
    -BenchFilter $spec.bench_filter `
    -CriterionEstimatesPath $spec.criterion_estimates_path `
    -RegressionPct $RegressionPct `
    -CheckDynamicStrategyRatio:$false

  $summary = Get-Content -Raw $spec.out_path | ConvertFrom-Json
  $criterion = if ($summary.PSObject.Properties.Name -contains "criterion") { $summary.criterion } else { $summary }
  $results += [ordered]@{
    name = $spec.name
    bench_filter = $criterion.bench_filter
    criterion_mean_ms = $criterion.mean_ms
    criterion_mean_ci_lower_ms = $criterion.mean_ci_lower_ms
    criterion_mean_ci_upper_ms = $criterion.mean_ci_upper_ms
    criterion_median_ms = $criterion.median_ms
    criterion_estimates_json = $criterion.estimates_json
    gpu = $summary.gpu
  }
}

$aggregate = [ordered]@{
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  regression_pct = $RegressionPct
  benchmarks = $results
}

$aggregate | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $SummaryPath
Write-Host "ALMA contract-row GPU perf summary written to $SummaryPath"
