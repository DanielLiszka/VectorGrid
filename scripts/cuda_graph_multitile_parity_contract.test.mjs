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

test('multi-tile CUDA Graph parity lane fixes one deterministic large Double MA case', () => {
  const probe = read('scripts/run_cuda_graph_multitile_parity.ps1')

  assert.match(probe, /\$workload = "large"/)
  assert.match(probe, /\$CandlesLen = 20000/)
  assert.match(probe, /\$VramBudgetMb = 160/)
  assert.match(probe, /"--strategy", "double_ma"/)
  assert.match(probe, /"--candle-shape", "synth"/)
  assert.match(probe, /"--fast-ma", "sma"/)
  assert.match(probe, /"--slow-ma", "sma"/)
  assert.match(probe, /"--metric-contract", "full17"/)
  assert.match(probe, /"--heatmap-bins", "0"/)
  assert.match(probe, /"--include-all", "off"/)
  assert.match(probe, /\$baselineRunnerArgs = \$commonRunnerArgs \+ @\(/)
  assert.match(probe, /"--cuda-graphs", "off"/)
  assert.match(probe, /"--cuda-graph-timing", "off"/)
  assert.match(probe, /\$graphRunnerArgs = \$commonRunnerArgs \+ @\(/)
  assert.match(probe, /"--cuda-graphs", "on"/)
  assert.match(probe, /"--cuda-graph-timing", "on"/)
})

test('multi-tile lane fail-closes on stale evidence, concurrency, and weak geometry', () => {
  const probe = read('scripts/run_cuda_graph_multitile_parity.ps1')

  assert.match(probe, /status = "running"/)
  assert.match(probe, /Prepare-FreshEvidenceDestination/)
  assert.match(probe, /Refusing stale or concurrently reserved CUDA Graph multi-tile parity OutPath/)
  assert.match(probe, /Assert-OwnedRunningMarker/)
  assert.match(probe, /running marker ownership changed/)
  assert.match(probe, /FreshnessFailureCanary requires an explicit fresh -OutPath/)
  assert.match(probe, /Enter-BenchmarkLock -Context "CUDA Graph multi-tile correctness parity"/)
  assert.match(probe, /Wait-NoPerfProcesses/)
  assert.match(probe, /VECTORBT_KERNEL_VRAM_BUDGET_MB/)
  assert.match(probe, /VECTORBT_KERNEL_TILE_DEBUG/)
  assert.match(probe, /Get-TileGeometry/)
  assert.match(probe, /Get-RepeatedCoarseTileShapeEvidence/)
  assert.match(probe, /max_same_coarse_signature_multiplicity -lt 2/)
  assert.match(probe, /captured-pointer regression was not exercised/)
})

test('multi-tile lane requires exact winners, metric parity, and pointer-aware graph success', () => {
  const probe = read('scripts/run_cuda_graph_multitile_parity.ps1')
  const runtime = read('crates/app_core/src/gpu_backtest_kernel.rs')

  assert.match(probe, /Assert-ResultParity/)
  assert.match(probe, /CUDA Graph parity changed exact best parameter/)
  assert.match(probe, /MetricAbsoluteTolerance/)
  assert.match(probe, /MetricRelativeTolerance/)
  assert.match(probe, /CUDA Graph parity exceeded tolerance/)
  assert.match(probe, /Every CUDA Graph capture must succeed, with at least two captures/)
  assert.match(probe, /Every CUDA Graph capture must be instantiated successfully/)
  assert.match(probe, /Every CUDA Graph replay must succeed, with at least two replays/)
  assert.match(probe, /gpu_kernel_timing_ms\.graph_replay_count must be at least two/)
  assert.match(probe, /\$captureSuccesses -ne \$replaySuccesses/)
  assert.match(probe, /Pointer-aware immutable graphs must recapture each full tile/)
  assert.match(probe, /Legacy graph replay count disagrees with replay-success timing/)
  assert.match(probe, /update must remain not applicable/i)
  assert.match(probe, /report_kind = "cuda-graph-multitile-correctness-parity"/)
  assert.match(probe, /capture_failure_count =/)
  assert.match(probe, /instantiate_failure_count =/)
  assert.match(probe, /replay_failure_count =/)
  assert.match(probe, /exact_best_params_match = \$true/)
  assert.match(probe, /continuous_metrics_within_tolerance = \$true/)

  for (const capturedPointer of [
    'pair_index_map_ptr',
    'pair_fast_period_rows_ptr',
    'pair_slow_period_rows_ptr',
    'pair_fast_local_rows_ptr',
    'pair_slow_local_rows_ptr',
  ]) {
    assert.match(
      runtime,
      new RegExp(`struct CudaGraphTileSignature[\\s\\S]*${capturedPointer}`),
    )
  }
})
