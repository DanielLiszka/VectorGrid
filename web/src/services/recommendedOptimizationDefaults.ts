import { useAdvancedStore } from "../stores/advancedStore";
import { resolveRuntimePinia } from "../stores/runtimePinia";

export const RECOMMENDED_TOP_K = "50";
export const RECOMMENDED_HEATMAP_BINS = "64";

export interface RecommendedOptimizationDefaultsResolution {
  topK: string | null;
  heatmapBins: string | null;
  applied: string[];
}

function parseOptionalNonNegativeInt(value: unknown): number | null {
  const text = String(value ?? "").trim();
  if (!text) return null;
  const next = Number(text);
  if (!Number.isFinite(next) || next < 0) return null;
  return Math.floor(next);
}

export function modeSupportsRecommendedGridHeatmap(value: unknown): boolean {
  return String(value || "").trim() === "Grid";
}

export function resolveRecommendedOptimizationDefaults(input: {
  topK: unknown;
  heatmapBins: unknown;
  mode: unknown;
}): RecommendedOptimizationDefaultsResolution {
  const applied: string[] = [];
  let topK: string | null = null;
  let heatmapBins: string | null = null;

  const parsedTopK = parseOptionalNonNegativeInt(input.topK);
  if (parsedTopK == null || parsedTopK <= 0) {
    topK = RECOMMENDED_TOP_K;
    applied.push(`Top K ${topK}`);
  }

  const parsedHeatmapBins = parseOptionalNonNegativeInt(input.heatmapBins);
  if (
    parsedHeatmapBins == null &&
    modeSupportsRecommendedGridHeatmap(input.mode)
  ) {
    heatmapBins = RECOMMENDED_HEATMAP_BINS;
    applied.push(`Heatmap bins ${heatmapBins}`);
  }

  return { topK, heatmapBins, applied };
}

export function applyRecommendedOptimizationDefaults(): boolean {
  const pinia = resolveRuntimePinia();
  if (!pinia) return false;
  const advancedStore = useAdvancedStore(pinia);
  const resolved = resolveRecommendedOptimizationDefaults({
    topK: advancedStore.topK,
    heatmapBins: advancedStore.heatmapBins,
    mode: advancedStore.mode,
  });
  let changed = false;

  // These are fallback defaults for missing/invalid values, not preset profiles.
  if (resolved.topK != null) {
    advancedStore.topK = resolved.topK;
    changed = true;
  }

  if (resolved.heatmapBins != null) {
    advancedStore.heatmapBins = resolved.heatmapBins;
    changed = true;
  }

  if (changed) {
    advancedStore.statusLine = `Applied recommended output defaults: ${resolved.applied.join(", ")}.`;
    advancedStore.source = "vue";
    advancedStore.updatedAtUnixMs = Date.now();
  }

  return changed;
}
