import { describe, expect, it } from 'vitest'
import { buildMetaBatchRequest } from './metaBatchRequestBuilder'

describe('metaBatchRequestBuilder', () => {
  it('builds normalized batch request with defaults', () => {
    const req = buildMetaBatchRequest({
      selectedPaths: [' C:/a.csv ', '', ' C:/b.csv '],
      holdoutPct: '80',
      holdoutTag: '',
      autoMeta: '1',
      skipDone: true,
      tags: '',
    })

    expect(req).toEqual({
      selected_paths: ['C:/a.csv', 'C:/b.csv'],
      holdout_pct: 50,
      holdout_tag: 'holdout',
      execution_mode: 'full',
      preload_policy: 'host_only',
      auto_meta: true,
      auto_stage5: false,
      skip_done: true,
      tags: [],
    })
  })

  it('includes optional auto stage 5 toggle', () => {
    const req = buildMetaBatchRequest({
      autoMeta: true,
      autoStage5: 'true',
      executionMode: 'validate_only',
      preloadPolicy: 'host_plus_doublema_vram',
    })

    expect(req.execution_mode).toBe('validate_only')
    expect(req.auto_meta).toBe(true)
    expect(req.auto_stage5).toBe(true)
    expect(req.preload_policy).toBe('host_plus_doublema_vram')
  })

  it('parses and deduplicates tags from string input', () => {
    const req = buildMetaBatchRequest({
      tags: 'alpha, beta Alpha\nGAMMA beta',
    })

    expect(req.tags).toEqual(['alpha', 'beta', 'GAMMA'])
  })

  it('accepts tags from list input', () => {
    const req = buildMetaBatchRequest({
      tags: [' one ', 'two', 'ONE', '', 'two'],
      holdoutPct: '-2',
    })

    expect(req.tags).toEqual(['one', 'two'])
    expect(req.holdout_pct).toBe(0)
  })
})
