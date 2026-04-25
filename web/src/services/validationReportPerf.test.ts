
import { describe, expect, it } from 'vitest'
import {
  clearValidationReportPerfEntries,
  installValidationReportPerfBridge,
  measureValidationReportPerf,
  readValidationReportPerfEntries,
  recordValidationReportPerf,
} from './validationReportPerf'

describe('validationReportPerf', () => {
  it('records normalized perf entries and bridges them to the window object', async () => {
    clearValidationReportPerfEntries()
    installValidationReportPerfBridge()

    recordValidationReportPerf(' validation.phase ', 12.4, {
      branch: ' direct ',
      ok: true,
    })
    await measureValidationReportPerf(
      'validation.otherPhase',
      async () => 'done',
      {
        meta: { trigger: 'manual' },
        onSuccessMeta: (result) => ({ result }),
      },
    )

    const entries = readValidationReportPerfEntries()
    expect(entries).toHaveLength(2)
    expect(entries[0]).toMatchObject({
      name: 'validation.otherPhase',
      meta: {
        trigger: 'manual',
        result: 'done',
        outcome: 'ok',
      },
    })
    expect(entries[1]).toMatchObject({
      name: 'validation.phase',
      meta: {
        branch: 'direct',
        ok: true,
      },
    })
    expect(window.__VTA_VALIDATION_REPORT_PERF__?.readEntries()).toHaveLength(2)
  })

  it('records error outcomes and keeps only the newest bounded entries', async () => {
    clearValidationReportPerfEntries()

    for (let index = 0; index < 140; index += 1) {
      recordValidationReportPerf(`phase.${index}`, index)
    }

    await expect(
      measureValidationReportPerf(
        'validation.errorPhase',
        async () => {
          throw new Error('boom')
        },
        {
          meta: { trigger: 'auto' },
          onErrorMeta: (error) => ({ reason: String(error) }),
        },
      ),
    ).rejects.toThrow('boom')

    const entries = readValidationReportPerfEntries()
    expect(entries).toHaveLength(128)
    expect(entries[entries.length - 1]?.name).toBe('phase.13')
    expect(entries.find((entry) => entry.name === 'validation.errorPhase')?.meta).toMatchObject({
      trigger: 'auto',
      outcome: 'error',
    })
  })
})
