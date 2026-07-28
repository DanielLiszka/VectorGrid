import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

test('gpu-ma performance authority has a distinct cuda-only workload and baseline', () => {
  const genericGate = read('scripts/run_gpu_perf_gate.ps1')
  const gpuMaGate = read('scripts/run_gpu_ma_perf_gate.ps1')
  const releaseGate = read('scripts/run_release_candidate_gate.ps1')
  const backendReleaseGate = read('scripts/run_backend_release_gate.ps1')

  assert.match(gpuMaGate, /WorkloadProfile\s*=\s*"gpu-ma"/)
  assert.match(gpuMaGate, /gpu_ma_perf_baseline\.json/)
  assert.match(gpuMaGate, /gpu_ma_perf_summary\.json/)
  assert.match(gpuMaGate, /CheckDynamicStrategyRatio\s*=\s*\$false/)

  assert.match(genericGate, /"double_ma\/gpu_sweep\/sma_sma\/200k\/small"/)
  assert.match(genericGate, /"double_ma\/gpu_sweep\/alma_alma\/200k\/large"/)
  assert.match(
    genericGate,
    /\$GpuFeatureSet\s*=\s*if\s*\(\$WorkloadProfile\s*-eq\s*"gpu-ma"\)\s*\{\s*"cuda"\s*\}\s*else\s*\{\s*"cuda,cuda-backtest-kernel"/s,
  )
  assert.match(genericGate, /scripts\/baselines\/gpu_perf_baseline\.json/)
  assert.match(
    releaseGate,
    /if\s*\(\$Variant\s*-eq\s*"gpu-ma"\)[\s\S]*?run_gpu_ma_perf_gate\.ps1[\s\S]*?elseif\s*\(\$Variant\s*-eq\s*"gpu-kernel"\)[\s\S]*?run_gpu_perf_gate\.ps1/,
  )
  assert.match(releaseGate, /The gpu-ma release performance baseline is missing/)
  assert.match(
    backendReleaseGate,
    /if\s*\(\$Variant\s*-eq\s*"gpu-ma"\)[\s\S]*?run_gpu_ma_perf_gate\.ps1[\s\S]*?else\s*\{[\s\S]*?run_gpu_perf_gate\.ps1/,
  )
  assert.match(backendReleaseGate, /The gpu-ma backend release performance baseline is missing/)
})

test('options GPU promotion is explicit, gpu-kernel-only, and carries correctness evidence', () => {
  const promotionGate = read('scripts/run_options_expression_gpu_promotion_gate.ps1')
  const releaseGate = read('scripts/run_release_candidate_gate.ps1')
  const publicGate = read('scripts/run_public_release_gate.ps1')

  assert.match(
    releaseGate,
    /\$IncludeOptionsGpuPromotion\s*-and\s*\$Variant\s*-ne\s*"gpu-kernel"/,
  )
  assert.match(releaseGate, /requires an explicit -OptionsGpuArtifactPath/)
  assert.match(
    releaseGate,
    /requires exactly one of -OptionsGpuDatasetId or -OptionsGpuImportCsv/,
  )
  assert.match(releaseGate, /run_options_expression_gpu_promotion_gate\.ps1/)
  assert.match(releaseGate, /"Options GPU promotion authority"/)

  assert.match(promotionGate, /"Options expression CPU test suite"/)
  assert.match(
    promotionGate,
    /fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates/,
  )
  assert.match(promotionGate, /gpu_combined_ranking_matches_cpu_topk_on_small_surface/)
  assert.match(promotionGate, /correctness_evidence\s*=\s*\$correctnessEvidence/)
  assert.match(promotionGate, /passed_test_count\s*=\s*\$passedTestCount/)

  assert.match(publicGate, /"-IncludeOptionsGpuPromotion"/)
  assert.match(publicGate, /include_options_gpu_promotion/)
  assert.match(publicGate, /Public options GPU evidence requires passing CPU options tests/)
  assert.match(publicGate, /baseline-regression checks/)
})
