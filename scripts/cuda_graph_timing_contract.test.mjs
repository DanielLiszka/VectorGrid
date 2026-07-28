import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')
const probePath = path.join(scriptDir, 'run_cuda_graph_timing_probe.ps1')

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

test('CUDA Graph timing probe exposes a CPU-only plan with exact boundary semantics', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-cuda-graph-plan-'))
  const outPath = path.join(tempRoot, 'plan.json')
  try {
    const result = spawnSync(
      'powershell',
      [
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        probePath,
        '-PlanOnly',
        '-OutPath',
        outPath,
      ],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        shell: false,
      },
    )
    assert.equal(
      result.status,
      0,
      `plan-only probe failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    const plan = JSON.parse(fs.readFileSync(outPath, 'utf8'))
    assert.equal(plan.schema_version, 1)
    assert.equal(plan.report_kind, 'cuda-graph-timing-boundary')
    assert.equal(plan.status, 'planned')
    assert.equal(plan.cargo_features, 'cuda,cuda-backtest-kernel')
    assert.equal(plan.timing_contract.clock, 'std_monotonic_instant')
    assert.match(plan.timing_contract.capture_boundary, /cuStreamBeginCapture/)
    assert.match(plan.timing_contract.capture_boundary, /cuStreamEndCapture/)
    assert.equal(plan.timing_contract.instantiate_boundary, 'cuGraphInstantiate_v2_host_call')
    assert.equal(plan.timing_contract.update_applicable, false)
    assert.match(plan.timing_contract.update_boundary, /no_cuGraphExecUpdate_call/)
    assert.match(plan.timing_contract.replay_boundary, /cuGraphLaunch/)
    assert.match(plan.timing_contract.replay_boundary, /completion_excluded/)
    assert.deepEqual(
      plan.runner_args.slice(
        plan.runner_args.indexOf('--cuda-graphs'),
        plan.runner_args.indexOf('--cuda-graphs') + 2,
      ),
      ['--cuda-graphs', 'on'],
    )
    assert.deepEqual(
      plan.runner_args.slice(
        plan.runner_args.indexOf('--cuda-graph-timing'),
        plan.runner_args.indexOf('--cuda-graph-timing') + 2,
      ),
      ['--cuda-graph-timing', 'on'],
    )
    assert.deepEqual(
      plan.runner_args.slice(
        plan.runner_args.indexOf('--heatmap-bins'),
        plan.runner_args.indexOf('--heatmap-bins') + 2,
      ),
      ['--heatmap-bins', '0'],
    )
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('runtime, runner, and probe share the opt-in timing contract', () => {
  const runtime = read('crates/app_core/src/gpu_backtest_kernel.rs')
  const resultContract = read('crates/app_core/src/double_ma.rs')
  const runner = read('crates/app_core/src/bin/profile_gpu_dynamic_runner.rs')
  const probe = read('scripts/run_cuda_graph_timing_probe.ps1')

  assert.match(runtime, /VECTORBT_KERNEL_CUDA_GRAPH_TIMING/)
  assert.match(runtime, /cuStreamBeginCapture_v2_through_cuStreamEndCapture/)
  assert.match(runtime, /cuGraphInstantiate_v2_host_call/)
  assert.match(runtime, /no_cuGraphExecUpdate_call/)
  assert.match(runtime, /cuGraphLaunch_host_submission_only_async_device_completion_excluded/)
  assert.match(runtime, /struct CudaGraphTileSignature/)
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
    assert.match(
      runtime,
      new RegExp(`let signature = CudaGraphTileSignature \\{[\\s\\S]*${capturedPointer}`),
    )
  }
  assert.match(
    runtime,
    /pair_fast_period_rows_ptr:\s*pair_fast_rows_ptr/,
  )
  assert.match(
    runtime,
    /pair_slow_period_rows_ptr:\s*pair_slow_rows_ptr/,
  )
  assert.match(resultContract, /pub struct CudaGraphTimingUs/)
  assert.match(resultContract, /pub cuda_graph_timing_us: Option<CudaGraphTimingUs>/)
  assert.match(runner, /--cuda-graph-timing/)
  assert.match(runner, /VECTORBT_KERNEL_CUDA_GRAPH_TIMING/)
  assert.match(probe, /Enter-BenchmarkLock -Context "CUDA Graph timing probe"/)
  assert.match(probe, /output\/benchmarks\/cuda_graph_timing_plan\.json/)
  assert.match(probe, /status = "running"/)
  assert.match(probe, /Prepare-FreshEvidenceDestination/)
  assert.match(probe, /FreshnessFailureCanary requires an explicit bounded -OutPath/)
  assert.match(probe, /Refusing to replace CUDA Graph timing evidence without explicit authority/)
  assert.match(probe, /-Value \$summary -AllowReplace/)
  assert.match(probe, /capture_success_count/)
  assert.match(probe, /instantiate_success_count/)
  assert.match(probe, /update_applicable/)
  assert.match(probe, /replay_submit_total_us/)
})

test('a failed probe cannot leave stale passing evidence at the bounded output path', () => {
  const outputDir = path.join(repoRoot, 'output', 'benchmarks')
  const outputPath = path.join(
    outputDir,
    `cuda-graph-freshness-canary-${process.pid}-${Date.now()}.json`,
  )
  fs.mkdirSync(outputDir, { recursive: true })
  fs.writeFileSync(outputPath, `${JSON.stringify({ status: 'passed', stale: true })}\n`)
  try {
    const result = spawnSync(
      'powershell',
      [
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        probePath,
        '-FreshnessFailureCanary',
        '-OutPath',
        outputPath,
      ],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        shell: false,
      },
    )
    assert.notEqual(
      result.status,
      0,
      `freshness canary unexpectedly succeeded:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    const marker = JSON.parse(fs.readFileSync(outputPath, 'utf8'))
    assert.equal(marker.status, 'running')
    assert.notEqual(marker.status, 'passed')
    assert.equal(marker.report_kind, 'cuda-graph-timing-boundary')
  } finally {
    fs.rmSync(outputPath, { force: true })
  }
})
