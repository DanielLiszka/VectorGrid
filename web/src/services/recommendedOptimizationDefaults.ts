import { useAdvancedStore } from '../stores/advancedStore'
import { resolveRuntimePinia } from '../stores/runtimePinia'

export const RECOMMENDED_TOP_K = '50'
export const RECOMMENDED_HEATMAP_BINS = '64'

function parseNonNegativeInt(value: unknown): number {
  const next = Number(value)
  if (!Number.isFinite(next)) return 0
  return Math.max(0, Math.floor(next))
}

function modeSupportsRecommendedGridHeatmap(value: unknown): boolean {
  return String(value || '').trim() === 'Grid'
}

export function applyRecommendedOptimizationDefaults(): boolean {
  const pinia = resolveRuntimePinia()
  if (!pinia) return false
  const advancedStore = useAdvancedStore(pinia)
  let changed = false
  const appliedDefaults: string[] = []

  const topK = parseNonNegativeInt(advancedStore.topK)
  if (topK <= 0) {
    advancedStore.topK = RECOMMENDED_TOP_K
    changed = true
    appliedDefaults.push(`Top K ${advancedStore.topK}`)
  }

  const heatmapBins = parseNonNegativeInt(advancedStore.heatmapBins)
  if (heatmapBins <= 0 && modeSupportsRecommendedGridHeatmap(advancedStore.mode)) {
    advancedStore.heatmapBins = RECOMMENDED_HEATMAP_BINS
    changed = true
    appliedDefaults.push(`Heatmap bins ${advancedStore.heatmapBins}`)
  }

  if (changed) {
    advancedStore.statusLine = `Applied recommended output defaults: ${appliedDefaults.join(', ')}.`
    advancedStore.source = 'vue'
    advancedStore.updatedAtUnixMs = Date.now()
  }

  return changed
}
