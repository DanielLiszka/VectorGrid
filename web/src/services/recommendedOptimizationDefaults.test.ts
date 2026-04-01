import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAdvancedStore } from "../stores/advancedStore";
import { bindRuntimePinia } from "../stores/runtimePinia";
import {
  applyRecommendedOptimizationDefaults,
  RECOMMENDED_HEATMAP_BINS,
  RECOMMENDED_TOP_K,
  resolveRecommendedOptimizationDefaults,
} from "./recommendedOptimizationDefaults";

let pinia: ReturnType<typeof createPinia>;

describe("recommendedOptimizationDefaults", () => {
  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
  });

  it("repairs missing top k and blank grid heatmap bins", () => {
    const store = useAdvancedStore(pinia);
    store.topK = "0";
    store.heatmapBins = "";
    store.mode = "Grid";

    const changed = applyRecommendedOptimizationDefaults();

    expect(changed).toBe(true);
    expect(store.topK).toBe(RECOMMENDED_TOP_K);
    expect(store.heatmapBins).toBe(RECOMMENDED_HEATMAP_BINS);
    expect(store.statusLine).toContain("Applied recommended output defaults");
  });

  it("preserves explicit zero grid heatmap bins as disable", () => {
    const store = useAdvancedStore(pinia);
    store.topK = "0";
    store.heatmapBins = "0";
    store.mode = "Grid";

    const changed = applyRecommendedOptimizationDefaults();

    expect(changed).toBe(true);
    expect(store.topK).toBe(RECOMMENDED_TOP_K);
    expect(store.heatmapBins).toBe("0");
    expect(store.statusLine).toContain(`Top K ${RECOMMENDED_TOP_K}`);
    expect(store.statusLine).not.toContain("Heatmap bins");
  });

  it("normalizes legacy non-grid modes back to the grid heatmap defaults", () => {
    const store = useAdvancedStore(pinia);
    store.topK = "0";
    store.heatmapBins = "0";
    store.mode = "CoarseToFine";

    const changed = applyRecommendedOptimizationDefaults();

    expect(changed).toBe(true);
    expect(store.topK).toBe(RECOMMENDED_TOP_K);
    expect(store.heatmapBins).toBe("0");
    expect(store.statusLine).toContain(`Top K ${RECOMMENDED_TOP_K}`);
    expect(store.statusLine).not.toContain("Heatmap bins");
  });

  it("does not add recommended heatmap bins for Auto mode", () => {
    const store = useAdvancedStore(pinia);
    store.topK = "0";
    store.heatmapBins = "0";
    store.mode = "Auto";

    const changed = applyRecommendedOptimizationDefaults();

    expect(changed).toBe(true);
    expect(store.topK).toBe(RECOMMENDED_TOP_K);
    expect(store.heatmapBins).toBe("0");
  });

  it("leaves explicit nonzero values unchanged", () => {
    const store = useAdvancedStore(pinia);
    store.topK = "25";
    store.heatmapBins = "128";
    store.mode = "Grid";

    const changed = applyRecommendedOptimizationDefaults();

    expect(changed).toBe(false);
    expect(store.topK).toBe("25");
    expect(store.heatmapBins).toBe("128");
  });

  it("exposes quiet resolution without mutating the store", () => {
    const store = useAdvancedStore(pinia);
    store.topK = "0";
    store.heatmapBins = "";
    store.mode = "Grid";
    store.statusLine = "Ready.";

    const resolved = resolveRecommendedOptimizationDefaults({
      topK: store.topK,
      heatmapBins: store.heatmapBins,
      mode: store.mode,
    });

    expect(resolved).toEqual({
      topK: RECOMMENDED_TOP_K,
      heatmapBins: RECOMMENDED_HEATMAP_BINS,
      applied: [
        `Top K ${RECOMMENDED_TOP_K}`,
        `Heatmap bins ${RECOMMENDED_HEATMAP_BINS}`,
      ],
    });
    expect(store.topK).toBe("0");
    expect(store.heatmapBins).toBe("");
    expect(store.statusLine).toBe("Ready.");
  });
});
