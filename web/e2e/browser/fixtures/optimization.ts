import type { FixtureFamilyModule, MockTauriScenario } from './types'
import {
  emptyAppScenario,
  savedOptionsArtifactFixturePack,
  validationSuccessScenario,
} from './scenarios'

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? (value as Record<string, unknown>) : {}
}

function sampleLoadedDataResult(): Record<string, unknown> {
  const latestHydration = asRecord(validationSuccessScenario.commandResults.load_latest_run_artifact_hydration)
  const hydration = asRecord(latestHydration.hydration)
  const currentRun = asRecord(hydration.currentRun)
  return {
    id: 'fixture_aapl_1m',
    meta: asRecord(currentRun.dataset_meta),
    cache_hit: true,
    cache_path: 'C:/VectorGridData/cache/aapl_1m.vta',
    load_source: 'binary_cache',
  }
}

function buildHeatmapFixture(bins = 32): Record<string, unknown> {
  const values = Array.from({ length: bins * bins }, (_, index) => {
    const logicalRow = Math.floor(index / bins)
    const col = index % bins
    if (logicalRow <= col) return null
    const ridge = Math.cos((logicalRow - col) / 5) * 0.18
    const slope = (logicalRow / Math.max(1, bins - 1)) * 0.12 - (col / Math.max(1, bins - 1)) * 0.06
    return Number((ridge + slope).toFixed(4))
  })
  return {
    bins_fast: bins,
    bins_slow: bins,
    fast_min: 5,
    fast_max: 36,
    slow_min: 5,
    slow_max: 36,
    values,
  }
}

function buildHeatmapSliceRows(): { all: unknown[]; all_scores: number[] } {
  const all: unknown[] = []
  const all_scores: number[] = []
  for (const offset of [0.75, 0.85]) {
    for (let fast = 5; fast <= 35; fast += 1) {
      for (let slow = fast + 1; slow <= 36; slow += 1) {
        const score = Number((
          Math.cos((slow - fast) / 5) * 0.18
          + (offset === 0.75 ? 0.08 : -0.04)
          + slow / 360
          - fast / 520
        ).toFixed(4))
        all.push([
          {
            fast_len: fast,
            slow_len: slow,
            fast_ma_type: 'alma',
            slow_ma_type: 'sma',
            fast_ma_params: { offset, sigma: 6 },
            slow_ma_params: null,
          },
          {
            pnl: score * 1000,
            sharpe: score,
            max_dd: Math.max(0.01, 0.12 - score / 10),
            trades: 24,
            exposure: 0.5,
            net_exposure: 0.1,
          },
        ])
        all_scores.push(score)
      }
    }
  }
  return { all, all_scores }
}

export const optimizationSuccessScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'optimization-success-ready',
  family: 'optimization',
  description: 'A matching sample dataset can run DoubleMa optimization and render the resulting shortlist.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_price_data: sampleLoadedDataResult(),
    run_double_ma_optimization:
      savedOptionsArtifactFixturePack.parentRunDetail.runRes,
    save_run_artifact: {
      path: 'C:/runs/vectorgrid_run_optimization_success.json',
      bytes: 73728,
    },
  },
  expectedInvokes: ['load_price_data', 'run_double_ma_optimization'],
}

export const optimizationFailureScenario: MockTauriScenario = {
  ...optimizationSuccessScenario,
  name: 'optimization-command-failure',
  family: 'optimization',
  description: 'A loaded sample dataset surfaces a recoverable DoubleMa optimization command failure.',
  commandResults: {
    ...optimizationSuccessScenario.commandResults,
    run_double_ma_optimization: {
      __mockFailure: 'Optimization fixture failed.',
    },
  },
  expectedInvokes: ['load_price_data', 'run_double_ma_optimization'],
  forbiddenInvokes: ['save_run_artifact'],
}

export const taskCenterProgressReadyScenario: MockTauriScenario = {
  ...optimizationSuccessScenario,
  name: 'task-center-progress-ready',
  family: 'optimization',
  description:
    'A delayed optimization invoke emits progress so the Task Center can prove running and completed lifecycle states.',
  commandDelays: {
    run_double_ma_optimization: 900,
  },
  commandEvents: {
    run_double_ma_optimization: [
      {
        event: 'double_ma_progress',
        delayMs: 80,
        payload: {
          processed_pairs: 12,
          total_pairs: 100,
          phase: 'building grid',
        },
      },
      {
        event: 'double_ma_progress',
        delayMs: 360,
        payload: {
          processed_pairs: 64,
          total_pairs: 100,
          phase: 'scoring candidates',
        },
      },
      {
        event: 'double_ma_progress',
        delayMs: 760,
        payload: {
          processed_pairs: 100,
          total_pairs: 100,
          phase: 'finalizing',
        },
      },
    ],
  },
}

export const compositeRouteSummaryReadyScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'composite-route-summary-ready',
  family: 'optimization',
  description:
    'Composite profile route preview controls can show CPU, GPU hybrid, and GPU kernel route summaries without running an optimization.',
  localStorage: {
    ...emptyAppScenario.localStorage,
    vta_workspace_tab_v1: 'workflow',
    vta_workflow_step_v1: 'optimize',
    vectorgrid_controls_tab_v1: 'opt',
  },
}

export const candidateDetailReadyScenario: MockTauriScenario = {
  ...validationSuccessScenario,
  name: 'candidate-detail-ready',
  family: 'optimization',
  description:
    'A saved DoubleMa run can open candidate detail from the Top workspace.',
  commandResults: {
    ...validationSuccessScenario.commandResults,
    compute_double_ma_drilldown: {
      equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
      drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
      trades: [],
      trade_stats: null,
      indicator_bars: [],
    },
    compute_double_ma_local_stability: {
      base: {
        fast_len: 8,
        slow_len: 32,
        fast_ma_type: 'sma',
        slow_ma_type: 'sma',
      },
      objective: 'Sharpe',
      radius: 1,
      step: 1,
      start_t: 1,
      points: [],
      summary: { base: 0, min: 0, median: 0, max: 0 },
    },
    run_double_ma_optimization:
      savedOptionsArtifactFixturePack.parentRunDetail.runRes,
    get_price_bars: [],
  },
  expectedInvokes: ['load_price_data', 'run_double_ma_optimization'],
}

export const heatmapLayoutScenario: MockTauriScenario = (() => {
  const latestHydration = asRecord(validationSuccessScenario.commandResults.load_latest_run_artifact_hydration)
  const hydration = asRecord(latestHydration.hydration)
  const currentRun = asRecord(hydration.currentRun)
  const req = asRecord(currentRun.req)
  const res = asRecord(currentRun.res)
  const sliceRows = buildHeatmapSliceRows()

  return {
    ...validationSuccessScenario,
    name: 'heatmap-layout-ready',
    family: 'optimization',
    description:
      'A saved DoubleMa run has a wide desktop heatmap surface with unavailable parameter bins.',
    commandResults: {
      ...validationSuccessScenario.commandResults,
      load_latest_run_artifact_hydration: {
        ...latestHydration,
        hydration: {
          ...hydration,
          currentRun: {
            ...currentRun,
            req: {
              ...req,
              heatmap_bins: 32,
              include_all: true,
              objective: 'Sharpe',
            },
            res: {
              ...res,
              heatmap: buildHeatmapFixture(32),
              all: sliceRows.all,
              all_scores: sliceRows.all_scores,
            },
          },
        },
      },
    },
    expectedInvokes: ['load_latest_run_artifact_hydration'],
  }
})()

export const optimizationFixtureFamily: FixtureFamilyModule = {
  family: 'optimization',
  description: 'Optimization result, top-candidate, and route-detail browser QA scenarios.',
  scenarios: [
    optimizationSuccessScenario,
    optimizationFailureScenario,
    compositeRouteSummaryReadyScenario,
    taskCenterProgressReadyScenario,
    candidateDetailReadyScenario,
    heatmapLayoutScenario,
  ],
}
