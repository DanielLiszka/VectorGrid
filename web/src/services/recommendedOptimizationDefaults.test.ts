import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useAdvancedStore } from '../stores/advancedStore'
import { bindRuntimePinia } from '../stores/runtimePinia'
import {
  applyRecommendedOptimizationDefaults,
  RECOMMENDED_HEATMAP_BINS,
  RECOMMENDED_TOP_K,
} from './recommendedOptimizationDefaults'

let pinia: ReturnType<typeof createPinia>

describe('recommendedOptimizationDefaults', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
  })

  it('applies recommended defaults when top k and grid heatmap bins are disabled', () => {
    const store = useAdvancedStore(pinia)
    store.topK = '0'
    store.heatmapBins = '0'
    store.mode = 'Grid'

    const changed = applyRecommendedOptimizationDefaults()

    expect(changed).toBe(true)
    expect(store.topK).toBe(RECOMMENDED_TOP_K)
    expect(store.heatmapBins).toBe(RECOMMENDED_HEATMAP_BINS)
    expect(store.statusLine).toContain('Applied recommended output defaults')
  })

  it('normalizes legacy non-grid modes back to the grid heatmap defaults', () => {
    const store = useAdvancedStore(pinia)
    store.topK = '0'
    store.heatmapBins = '0'
    store.mode = 'CoarseToFine'

    const changed = applyRecommendedOptimizationDefaults()

    expect(changed).toBe(true)
    expect(store.topK).toBe(RECOMMENDED_TOP_K)
    expect(store.heatmapBins).toBe('0')
    expect(store.statusLine).toContain(`Top K ${RECOMMENDED_TOP_K}`)
    expect(store.statusLine).not.toContain('Heatmap bins')
  })

  it('does not add recommended heatmap bins for Auto mode', () => {
    const store = useAdvancedStore(pinia)
    store.topK = '0'
    store.heatmapBins = '0'
    store.mode = 'Auto'

    const changed = applyRecommendedOptimizationDefaults()

    expect(changed).toBe(true)
    expect(store.topK).toBe(RECOMMENDED_TOP_K)
    expect(store.heatmapBins).toBe('0')
  })

  it('leaves explicit nonzero values unchanged', () => {
    const store = useAdvancedStore(pinia)
    store.topK = '25'
    store.heatmapBins = '128'
    store.mode = 'Grid'

    const changed = applyRecommendedOptimizationDefaults()

    expect(changed).toBe(false)
    expect(store.topK).toBe('25')
    expect(store.heatmapBins).toBe('128')
  })
})
