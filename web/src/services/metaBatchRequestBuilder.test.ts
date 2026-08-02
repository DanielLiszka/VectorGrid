import { describe, expect, it } from 'vitest'
import {
  buildMetaBatchRequest,
  buildVcetFixedCandidateManifest,
} from './metaBatchRequestBuilder'

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

  it('preserves the fixed-candidate evaluation execution mode', () => {
    const req = buildMetaBatchRequest({
      selectedPaths: ['C:/data/AAPL.csv'],
      executionMode: 'evaluate_candidates',
    })

    expect(req.execution_mode).toBe('evaluate_candidates')
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

  it('falls back to conservative defaults for invalid execution and preload modes', () => {
    const req = buildMetaBatchRequest({
      executionMode: 'strange_mode',
      preloadPolicy: 'vram_everything',
      holdoutTag: '',
      tags: 'alpha alpha beta',
    })

    expect(req).toMatchObject({
      holdout_tag: 'holdout',
      execution_mode: 'full',
      preload_policy: 'host_only',
      tags: ['alpha', 'beta'],
    })
  })

  it('expands the current VCET ranges into a stable fixed-candidate manifest', () => {
    expect(
      buildVcetFixedCandidateManifest(
        {
          fast_range: [8, 10, 2],
          slow_range: [9, 13, 2],
        },
        8,
      ),
    ).toEqual([
      { fast_len: 8, slow_len: 9 },
      { fast_len: 8, slow_len: 11 },
      { fast_len: 8, slow_len: 13 },
      { fast_len: 10, slow_len: 11 },
      { fast_len: 10, slow_len: 13 },
    ])
  })

  it('rejects a VCET candidate manifest that exceeds the validation budget', () => {
    expect(() =>
      buildVcetFixedCandidateManifest(
        {
          fast_range: [2, 8, 2],
          slow_range: [10, 20, 2],
        },
        3,
      ),
    ).toThrow('more than 3 candidates')
  })

  it('rejects VCET fixed-candidate Fast ranges outside the VIDYA contract', () => {
    for (const fastRange of [
      [1, 4, 1],
      [2, 65_536, 1],
    ]) {
      expect(() =>
        buildVcetFixedCandidateManifest(
          {
            fast_range: fastRange,
            slow_range: [10, 20, 2],
          },
          32,
        ),
      ).toThrow(/valid fast range.*\[2, 65535\]/i)
    }
  })
})
