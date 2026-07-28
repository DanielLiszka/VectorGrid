import { describe, expect, it } from 'vitest'
import {
  activeRunCompatibility,
  workflowRunDataId,
  workflowRunMode,
  workflowRunTimeframe,
} from './activeContextCompatibility'

function run(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    req: {
      data_id: 'dataset-a@15m',
      strategy_kind: 'DoubleMa',
      timeframe: '15m',
    },
    res: { rows: [{}] },
    ...overrides,
  }
}

describe('activeContextCompatibility', () => {
  it('accepts a run only when dataset, mode, and timeframe match', () => {
    expect(
      activeRunCompatibility({
        activeDataId: 'dataset-a@15m',
        activeMode: 'conventional',
        activeTimeframe: '15m',
        run: run(),
      }),
    ).toMatchObject({ compatible: true, code: 'compatible' })
  })

  it('blocks retained output when no dataset is active', () => {
    expect(
      activeRunCompatibility({
        activeDataId: '—',
        activeMode: 'conventional',
        run: run(),
      }),
    ).toMatchObject({
      compatible: false,
      code: 'missing_active_dataset',
      activeDataId: '',
      runDataId: 'dataset-a@15m',
    })
  })

  it('blocks retained output from another dataset', () => {
    expect(
      activeRunCompatibility({
        activeDataId: 'dataset-b@15m',
        activeMode: 'conventional',
        run: run(),
      }),
    ).toMatchObject({ compatible: false, code: 'dataset_mismatch' })
  })

  it('blocks a reused process-local data id when the dataset paths differ', () => {
    expect(
      activeRunCompatibility({
        activeDataId: 'data-1',
        activeDatasetPath: '\\\\?\\C:\\samples\\vectorgrid_synthetic_ohlcv.csv',
        activeMode: 'conventional',
        run: run({
          req: {
            data_id: 'data-1',
            strategy_kind: 'DoubleMa',
            timeframe: '15m',
          },
          dataset_meta: { csv_path: 'C:/market/AAPL.csv' },
        }),
      }),
    ).toMatchObject({
      compatible: false,
      code: 'dataset_mismatch',
      activeDatasetPath: '\\\\?\\C:\\samples\\vectorgrid_synthetic_ohlcv.csv',
      runDatasetPath: 'C:/market/AAPL.csv',
    })
  })

  it('blocks cross-mode output even when the dataset matches', () => {
    expect(
      activeRunCompatibility({
        activeDataId: 'dataset-a@15m',
        activeMode: 'ml-signal',
        run: run(),
      }),
    ).toMatchObject({
      compatible: false,
      code: 'mode_mismatch',
      runMode: 'conventional',
    })
  })

  it('requires a source dataset identity on legacy artifacts before actions run', () => {
    expect(
      activeRunCompatibility({
        activeDataId: 'dataset-a@15m',
        activeMode: 'conventional',
        run: run({ req: { strategy_kind: 'DoubleMa' } }),
      }),
    ).toMatchObject({ compatible: false, code: 'missing_run_dataset_identity' })
  })

  it('normalizes snake/camel artifact identity fields and mode families', () => {
    const artifact = {
      artifact_kind: 'ml_signal_policy_artifact',
      request: { dataId: 'dataset-ml' },
      dataset_meta: { derivedTimeframe: '30m' },
      ml_signal: { score_space: 'probability' },
    }
    expect(workflowRunDataId(artifact)).toBe('dataset-ml')
    expect(workflowRunMode(artifact)).toBe('ml-signal')
    expect(workflowRunTimeframe(artifact)).toBe('30m')
  })
})
