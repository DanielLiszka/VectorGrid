import { describe, expect, it } from 'vitest'
import { resolveShellResultCommandReadiness } from './shellCommandReadiness'

function currentInput(overrides: Record<string, unknown> = {}) {
  return {
    workspace: 'workflow',
    mode: 'conventional',
    datasetReady: true,
    activeDataId: 'dataset-a',
    activeDatasetPath: 'C:/market/AAPL.csv',
    activeTimeframe: '15m',
    currentRun: {
      req: {
        data_id: 'dataset-a',
        strategy_kind: 'DoubleMa',
        timeframe: '15m',
      },
      dataset_meta: { csv_path: 'C:/market/AAPL.csv' },
      res: { rows: [{}] },
    },
    runDisabled: false,
    validateDisabled: false,
    canExportCsv: true,
    canExportJson: true,
    ...overrides,
  }
}

describe('resolveShellResultCommandReadiness', () => {
  it('enables result actions only for the matching active Conventional run', () => {
    expect(resolveShellResultCommandReadiness(currentInput())).toMatchObject({
      canRun: true,
      canValidate: true,
      canExportCsv: true,
      canExportJson: true,
    })
  })

  it('blocks result commands outside Research even when retained flags are true', () => {
    const readiness = resolveShellResultCommandReadiness(
      currentInput({ workspace: 'pro' }),
    )
    expect(readiness).toMatchObject({
      canRun: false,
      canValidate: false,
      canExportCsv: false,
      canExportJson: false,
    })
    expect(readiness.reasons.validate).toContain('Open Research')
  })

  it('blocks retained Conventional output in ML Signal mode', () => {
    const readiness = resolveShellResultCommandReadiness(
      currentInput({ mode: 'ml-signal' }),
    )
    expect(readiness.canValidate).toBe(false)
    expect(readiness.canExportCsv).toBe(false)
    expect(readiness.reasons.export_csv).toContain('Conventional')
  })

  it('blocks stale output when the active dataset differs', () => {
    const readiness = resolveShellResultCommandReadiness(
      currentInput({ activeDataId: 'dataset-b' }),
    )
    expect(readiness.canValidate).toBe(false)
    expect(readiness.canExportJson).toBe(false)
    expect(readiness.reasons.validate).toContain('different dataset')
  })

  it('blocks stale output when a process-local data id is reused for another path', () => {
    const readiness = resolveShellResultCommandReadiness(
      currentInput({ activeDatasetPath: 'C:/samples/vectorgrid_synthetic_ohlcv.csv' }),
    )
    expect(readiness.canValidate).toBe(false)
    expect(readiness.canExportCsv).toBe(false)
    expect(readiness.reasons.validate).toContain('different dataset path')
  })

  it('keeps backend and export capability flags authoritative after context matches', () => {
    const readiness = resolveShellResultCommandReadiness(
      currentInput({
        validateDisabled: true,
        canExportCsv: false,
        canExportJson: false,
      }),
    )
    expect(readiness.canValidate).toBe(false)
    expect(readiness.canExportCsv).toBe(false)
    expect(readiness.canExportJson).toBe(false)
  })
})
