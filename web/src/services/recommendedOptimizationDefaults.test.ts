import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useAdvancedStore } from '../stores/advancedStore'
import {
  applyRecommendedOptimizationDefaults,
  RECOMMENDED_HEATMAP_BINS,
  RECOMMENDED_TOP_K,
} from './recommendedOptimizationDefaults'

describe('recommendedOptimizationDefaults', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('applies recommended defaults when top k and grid heatmap bins are disabled', () => {
    const store = useAdvancedStore()
    store.topK = '0'
    store.heatmapBins = '0'
    store.mode = 'Grid'

    const changed = applyRecommendedOptimizationDefaults()

    expect(changed).toBe(true)
    expect(store.topK).toBe(RECOMMENDED_TOP_K)
    expect(store.heatmapBins).toBe(RECOMMENDED_HEATMAP_BINS)
    expect(store.statusLine).toContain('Applied recommended output defaults')
  })

  it('does not force grid heatmap defaults into modes that require heatmap bins off', () => {
    const store = useAdvancedStore()
    store.topK = '0'
    store.heatmapBins = '0'
    store.mode = 'CoarseToFine'

    const changed = applyRecommendedOptimizationDefaults()

    expect(changed).toBe(true)
    expect(store.topK).toBe(RECOMMENDED_TOP_K)
    expect(store.heatmapBins).toBe('0')
  })

  it('leaves explicit nonzero values unchanged', () => {
    const store = useAdvancedStore()
    store.topK = '25'
    store.heatmapBins = '128'
    store.mode = 'Grid'

    const changed = applyRecommendedOptimizationDefaults()

    expect(changed).toBe(false)
    expect(store.topK).toBe('25')
    expect(store.heatmapBins).toBe('128')
  })
})
