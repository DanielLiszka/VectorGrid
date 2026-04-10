import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "./runtimePinia";
import { useAdvancedStore } from "./advancedStore";
import { useOptimizeStore } from "./optimizeStore";

const invokeCommandMock = vi.fn();
const resetMoSearchSpaceMock = vi.fn();

vi.mock("../tauri/invoke", () => ({
  invokeCommand: (...args: unknown[]) => invokeCommandMock(...args),
}));

vi.mock("../services/advancedService", () => ({
  resetMoSearchSpace: (...args: unknown[]) => resetMoSearchSpaceMock(...args),
}));

describe("advanced store actions", () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    invokeCommandMock.mockReset();
    resetMoSearchSpaceMock.mockReset();
  });

  it("mirrors objective and mode changes back into the optimize store summary fields", () => {
    const advanced = useAdvancedStore(pinia);
    const optimize = useOptimizeStore(pinia);

    advanced.setSelectField("objective", "Sortino");
    advanced.setSelectField("mode", "CoarseToFine");

    expect(advanced.objective).toBe("Sortino");
    expect(advanced.mode).toBe("CoarseToFine");
    expect(optimize.objective).toBe("Sortino");
    expect(optimize.mode).toBe("CoarseToFine");
  });

  it("keeps optimize store parity when hydrating an advanced snapshot", () => {
    const advanced = useAdvancedStore(pinia);
    const optimize = useOptimizeStore(pinia);

    advanced.applyLegacySnapshot({
      objective: "Calmar",
      mode: "GpuVdps",
    });

    expect(advanced.objective).toBe("Calmar");
    expect(advanced.mode).toBe("GpuVdps");
    expect(optimize.objective).toBe("Calmar");
    expect(optimize.mode).toBe("GpuVdps");
  });

  it("normalizes legacy approximate mode aliases when hydrating advanced snapshots", () => {
    const advanced = useAdvancedStore(pinia);
    const optimize = useOptimizeStore(pinia);

    advanced.applyLegacySnapshot({
      mode: "GpuProgressive",
      modeOptions: [
        { value: "GpuProgressive", label: "GpuProgressive", disabled: false },
        { value: "Grid", label: "Grid", disabled: false },
      ],
    });

    expect(advanced.mode).toBe("Grid");
    expect(advanced.modeOptions.map((option) => option.value)).toEqual(["Grid"]);
    expect(optimize.mode).toBe("Grid");
  });

  it("browses and stores the export-all CSV path through the Vue store boundary", async () => {
    const advanced = useAdvancedStore(pinia);
    invokeCommandMock.mockResolvedValue("C:\\exports\\all_pairs.csv");

    advanced.browseExportAll();
    await Promise.resolve();
    await Promise.resolve();

    expect(invokeCommandMock).toHaveBeenCalledWith("pick_save_csv", {
      defaultName: "all_pairs.csv",
    });
    expect(advanced.exportAllCsvPath).toBe("C:\\exports\\all_pairs.csv");
    expect(advanced.source).toBe("vue");
    expect(advanced.updatedAtUnixMs).toBeGreaterThan(0);
  });

  it("resets Composite MO search space only when the helper is enabled", async () => {
    const advanced = useAdvancedStore(pinia);
    advanced.canResetMoSearchSpace = true;
    resetMoSearchSpaceMock.mockResolvedValue({ json: "{\"sampler\":\"nsga2\"}" });

    advanced.resetMoSearchSpace();
    await Promise.resolve();
    await Promise.resolve();

    expect(resetMoSearchSpaceMock).toHaveBeenCalledTimes(1);
    expect(advanced.moSearchSpace).toBe("{\"sampler\":\"nsga2\"}");
    expect(advanced.statusLine).toBe("Reset Composite MO search space.");
    expect(advanced.source).toBe("vue");
    expect(advanced.updatedAtUnixMs).toBeGreaterThan(0);
  });
});
