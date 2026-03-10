import { useAdvancedStore } from '../stores/advancedStore'

export const RECOMMENDED_TOP_K = '50'
export const RECOMMENDED_HEATMAP_BINS = '64'

function parseNonNegativeInt(value: unknown): number {
  const next = Number(value)
  if (!Number.isFinite(next)) return 0
  return Math.max(0, Math.floor(next))
}

export function applyRecommendedOptimizationDefaults(): boolean {
  const advancedStore = useAdvancedStore()
  let changed = false

  const topK = parseNonNegativeInt(advancedStore.topK)
  if (topK <= 0) {
    advancedStore.topK = RECOMMENDED_TOP_K
    changed = true
  }

  const heatmapBins = parseNonNegativeInt(advancedStore.heatmapBins)
  if (heatmapBins <= 0) {
    advancedStore.heatmapBins = RECOMMENDED_HEATMAP_BINS
    changed = true
  }

  if (changed) {
    advancedStore.statusLine = `Applied recommended output defaults: Top K ${advancedStore.topK}, Heatmap bins ${advancedStore.heatmapBins}.`
    advancedStore.source = 'vue'
    advancedStore.updatedAtUnixMs = Date.now()
  }

  return changed
}
