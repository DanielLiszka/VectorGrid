
import { beforeEach, describe, expect, it } from 'vitest'
import {
  clearArtifactHistoryPerfEntries,
  installArtifactHistoryPerfBridge,
  measureArtifactHistoryPerf,
  readArtifactHistoryPerfEntries,
  recordArtifactHistoryPerf,
} from './artifactHistoryPerf'

describe('artifactHistoryPerf', () => {
  beforeEach(() => {
    clearArtifactHistoryPerfEntries()
    delete window.__VTA_ARTIFACT_HISTORY_PERF__
  })

  it('records normalized entries and preserves the newest entries first', () => {
    recordArtifactHistoryPerf(' history.refresh ', 12.5, {
      count: 3,
      enabled: true,
      empty: '',
      missing: undefined,
      nested: { nope: true },
    })
    recordArtifactHistoryPerf('workflow.bootstrap', 4)

    expect(readArtifactHistoryPerfEntries()).toEqual([
      {
        name: 'workflow.bootstrap',
        durationMs: 4,
        atUnixMs: expect.any(Number),
        meta: undefined,
      },
      {
        name: 'history.refresh',
        durationMs: 12.5,
        atUnixMs: expect.any(Number),
        meta: {
          count: 3,
          enabled: true,
          empty: '',
          missing: null,
          nested: '[object Object]',
        },
      },
    ])
  })

  it('captures success and error outcomes through the measurement helper', async () => {
    await expect(
      measureArtifactHistoryPerf(
        'history.detail',
        async () => {
          throw new Error('boom')
        },
        {
          meta: { path: 'C:/runs/run.json' },
          onErrorMeta: (error) => ({
            error: String(error instanceof Error ? error.message : error),
          }),
        },
      ),
    ).rejects.toThrow('boom')

    const result = await measureArtifactHistoryPerf(
      'history.detail',
      async () => ({ ok: true }),
      {
        onSuccessMeta: (value) => ({ ok: value.ok }),
      },
    )

    expect(result).toEqual({ ok: true })
    expect(readArtifactHistoryPerfEntries()).toEqual([
      {
        name: 'history.detail',
        durationMs: expect.any(Number),
        atUnixMs: expect.any(Number),
        meta: {
          ok: true,
          outcome: 'ok',
        },
      },
      {
        name: 'history.detail',
        durationMs: expect.any(Number),
        atUnixMs: expect.any(Number),
        meta: {
          path: 'C:/runs/run.json',
          error: 'boom',
          outcome: 'error',
        },
      },
    ])
  })

  it('installs a read-only debug bridge on window', () => {
    installArtifactHistoryPerfBridge()
    recordArtifactHistoryPerf('main.boot', 8)

    expect(window.__VTA_ARTIFACT_HISTORY_PERF__).toBeTruthy()
    expect(window.__VTA_ARTIFACT_HISTORY_PERF__?.readEntries()).toEqual([
      {
        name: 'main.boot',
        durationMs: 8,
        atUnixMs: expect.any(Number),
      },
    ])

    window.__VTA_ARTIFACT_HISTORY_PERF__?.clearEntries()
    expect(readArtifactHistoryPerfEntries()).toEqual([])
  })

  it('caps stored entries to the most recent 128 measurements', () => {
    for (let index = 0; index < 140; index += 1) {
      recordArtifactHistoryPerf(`perf.${index}`, index)
    }

    const entries = readArtifactHistoryPerfEntries()
    expect(entries).toHaveLength(128)
    expect(entries[0]?.name).toBe('perf.139')
    expect(entries[entries.length - 1]?.name).toBe('perf.12')
  })
})
