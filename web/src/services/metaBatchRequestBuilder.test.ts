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
      auto_meta: true,
      skip_done: true,
      tags: [],
    })
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
