import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "./runtimePinia";
import {
  applyPreset,
  cancelOptimization,
  type GpuStatusSummary,
  presetKernel,
  previewDynamicStrategyIr,
  refreshGpuStatus,
  runGpuParityCheck,
  runOptimization,
  runValidation,
} from "../services/optimizeService";
import { useOptimizeStore } from "./optimizeStore";
import { useAdvancedStore } from "./advancedStore";
import { useDataStore } from "./dataStore";
import { useStrategyStore } from "./strategyStore";
import { useWorkflowStore } from "./workflowStore";
import { invokeCommand } from "../tauri/invoke";

vi.mock("../services/optimizeService", () => ({
  applyPreset: vi.fn(async () => undefined),
  cancelOptimization: vi.fn(async () => undefined),
  presetKernel: vi.fn(async () => undefined),
  previewDynamicStrategyIr: vi.fn(async () => undefined),
  refreshGpuStatus: vi.fn(async () => undefined),
  runGpuParityCheck: vi.fn(async () => undefined),
  runOptimization: vi.fn(async () => undefined),
  runValidation: vi.fn(async () => undefined),
}));

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({})),
}));

const applyPresetMock = vi.mocked(applyPreset);
const runOptimizationMock = vi.mocked(runOptimization);
const cancelOptimizationMock = vi.mocked(cancelOptimization);
const runValidationMock = vi.mocked(runValidation);
const presetKernelMock = vi.mocked(presetKernel);
const previewDynamicStrategyIrMock = vi.mocked(previewDynamicStrategyIr);
const refreshGpuStatusMock = vi.mocked(refreshGpuStatus);
const runGpuParityCheckMock = vi.mocked(runGpuParityCheck);
const invokeCommandMock = vi.mocked(invokeCommand);
const defaultGpuStatusSummary: GpuStatusSummary = {
  builtWithCuda: true,
  builtWithCudaBacktestKernel: true,
  cudaAvailable: true,
  autoDeviceName: "NVIDIA RTX 4090",
  autoDeviceMemoryBytes: 24 * 1024 * 1024 * 1024,
  deviceMeta: "Automatic CUDA device: NVIDIA RTX 4090 (24 GB).",
  text: "GPU: build=cuda | kernel=on | cuda_available=true",
};

interface LegacyField {
  value: string;
}

interface DocumentHarness {
  fields: Record<string, LegacyField>;
}

function installDocumentHarness(): DocumentHarness {
  const makeField = (_id: string, value = ""): LegacyField => ({ value });
  const fields: Record<string, LegacyField> = {
    fastStart: makeField("fastStart", "3"),
    backend: makeField("backend", "Auto"),
  };
  vi.stubGlobal("document", {
    getElementById: (id: string) => fields[id] ?? null,
  });
  return { fields };
}

describe("optimize store actions", () => {
  let harness: DocumentHarness;
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    harness = installDocumentHarness();
    applyPresetMock.mockClear();
    applyPresetMock.mockResolvedValue(undefined);
    runOptimizationMock.mockClear();
    runOptimizationMock.mockResolvedValue(undefined);
    cancelOptimizationMock.mockClear();
    cancelOptimizationMock.mockResolvedValue(undefined);
    runValidationMock.mockClear();
    runValidationMock.mockResolvedValue(undefined);
    presetKernelMock.mockClear();
    presetKernelMock.mockResolvedValue(undefined);
    previewDynamicStrategyIrMock.mockClear();
    previewDynamicStrategyIrMock.mockResolvedValue(undefined);
    refreshGpuStatusMock.mockClear();
    refreshGpuStatusMock.mockResolvedValue(defaultGpuStatusSummary);
    runGpuParityCheckMock.mockClear();
    runGpuParityCheckMock.mockResolvedValue(undefined);
    invokeCommandMock.mockClear();
    invokeCommandMock.mockResolvedValue({});
  });

  async function flushServiceImports(): Promise<void> {
    await vi.dynamicImportSettled();
    await Promise.resolve();
    await Promise.resolve();
  }

  it("updates Vue optimize state without legacy DOM mirroring", () => {
    const store = useOptimizeStore(pinia);
    store.setInputField("fastStart", "5");
    store.setSelectField("backend", "CpuOnly");

    expect(store.fastStart).toBe("5");
    expect(store.backend).toBe("CpuOnly");
    expect(store.source).toBe("vue");
    expect(harness.fields.fastStart?.value).toBe("3");
    expect(harness.fields.backend?.value).toBe("Auto");
  });

  it("mirrors objective and mode changes into the advanced store authority fields", () => {
    const store = useOptimizeStore(pinia);
    const advanced = useAdvancedStore(pinia);

    store.setSelectField("objective", "Omega");
    store.setSelectField("mode", "GpuVdps");

    expect(store.objective).toBe("Omega");
    expect(store.mode).toBe("GpuVdps");
    expect(advanced.objective).toBe("Omega");
    expect(advanced.mode).toBe("GpuVdps");
  });

  it("keeps advanced store parity when hydrating an optimize snapshot", () => {
    const store = useOptimizeStore(pinia);
    const advanced = useAdvancedStore(pinia);

    store.applyLegacySnapshot({
      objective: "Calmar",
      mode: "CoarseToFine",
    } as never);

    expect(store.objective).toBe("Calmar");
    expect(store.mode).toBe("CoarseToFine");
    expect(advanced.objective).toBe("Calmar");
    expect(advanced.mode).toBe("CoarseToFine");
  });

  it("dispatches optimize run/cancel/validation through optimize service", async () => {
    const store = useOptimizeStore(pinia);
    store.runOptimization();
    store.cancelOptimization();
    store.runValidation();
    await flushServiceImports()

    expect(runOptimizationMock).toHaveBeenCalled();
    expect(cancelOptimizationMock).toHaveBeenCalled();
    expect(runValidationMock).toHaveBeenCalled();
  });

  it("keeps validation service rejections inside store state", async () => {
    const store = useOptimizeStore(pinia);
    runValidationMock.mockRejectedValueOnce(new Error("dataset mismatch"));

    store.runValidation();
    await flushServiceImports();

    expect(runValidationMock).toHaveBeenCalled();
    expect(store.statusLine).toBe("Validation failed: Error: dataset mismatch");
    expect(store.source).toBe("vue");
  });

  it("routes the shared MlSignal primary action by workflow step", async () => {
    const store = useOptimizeStore(pinia);
    const data = useDataStore(pinia);
    const strategy = useStrategyStore(pinia);
    const workflow = useWorkflowStore(pinia);
    workflow.selectMode("ml-signal", { persist: false });
    store.mlSignalScoreInputPath = "C:\\data\\scores.csv";
    store.mlSignalDatasetExportPath = "C:\\data\\ml_dataset.csv";
    store.mlSignalScoreCachePath = "C:\\data\\score_cache.json";
    store.mlSignalLongThreshold = "0.60";
    store.mlSignalShortThreshold = "0.61";
    store.mlSignalDelta = "0.08";
    store.mlSignalMaxHoldBars = "12";
    data.dataId = "data-1";
    data.csvPath = "C:\\data\\prices.csv";
    data.datasetLabel = "prices.csv";
    data.dataTimeframe = "1d";
    data.baseDataMeta = {
      len: 100,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1,
      maxTs: 2,
      timeframe: "1d",
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
      sourceContentFingerprint: "fp-prices-1d",
    } as never;
    strategy.longOnly = false;

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "export_ml_signal_dataset") {
        return {
          row_count: 12,
          feature_count: 8,
          split_count: 2,
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
        };
      }
      if (command === "generate_ml_signal_score_cache") {
        return { matched_rows: 12, skipped_rows: 0 };
      }
      if (command === "run_ml_signal_validation") {
        return {
          candidates: [{ params: {} }],
          run_verdict: { approved_candidate_count: 1 },
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_run_ml_signal.json" };
      }
      if (command === "run_ml_signal_policy_sweep") {
        return {
          sweep_size: 4,
          top_k: 2,
          validation: {
            candidates: [{ params: { label: "best" }, robust_score: 1.1 }],
          },
        };
      }
      return {};
    });

    workflow.selectWorkflowStep("data", { persist: false });
    store.runOptimization();
    await vi.waitFor(() => {
      expect(invokeCommandMock).toHaveBeenCalledWith(
        "export_ml_signal_dataset",
        expect.objectContaining({ req: expect.any(Object) }),
      );
    });

    workflow.selectWorkflowStep("optimize", { persist: false });
    store.runOptimization();
    await vi.waitFor(() => {
      expect(invokeCommandMock).toHaveBeenCalledWith(
        "generate_ml_signal_score_cache",
        expect.objectContaining({ req: expect.any(Object) }),
      );
    });

    workflow.selectWorkflowStep("validate", { persist: false });
    store.runOptimization();
    await vi.waitFor(() => {
      expect(invokeCommandMock).toHaveBeenCalledWith(
        "run_ml_signal_validation",
        expect.objectContaining({ req: expect.any(Object) }),
      );
    });

    workflow.selectWorkflowStep("analyze", { persist: false });
    store.runOptimization();
    await vi.waitFor(() => {
      expect(invokeCommandMock).toHaveBeenCalledWith(
        "run_ml_signal_policy_sweep",
        expect.objectContaining({ req: expect.any(Object) }),
      );
    });

    workflow.selectWorkflowStep("finalize", { persist: false });
    store.runOptimization();
    await vi.waitFor(() => {
      expect(store.statusLine).toContain("inspect-only");
    });

    expect(runOptimizationMock).not.toHaveBeenCalled();
    expect(runValidationMock).not.toHaveBeenCalled();
  });

  it("dispatches optimize preset and gpu utility actions through optimize service", async () => {
    const store = useOptimizeStore(pinia);
    store.applyPreset();
    store.presetKernel();
    store.previewDynamicStrategyIr();
    store.refreshGpuStatus();
    store.runGpuParityCheck();
    await flushServiceImports()

    expect(applyPresetMock).toHaveBeenCalled();
    expect(presetKernelMock).toHaveBeenCalled();
    expect(previewDynamicStrategyIrMock).toHaveBeenCalled();
    expect(refreshGpuStatusMock).toHaveBeenCalled();
    expect(runGpuParityCheckMock).toHaveBeenCalled();
  });

  it("syncs MA param/sweep state from provided payload", () => {
    const store = useOptimizeStore(pinia);
    store.syncMaParamState({
      fast: {
        params: { alpha: 3 },
        sweeps: { alpha: [1, 5, 2] },
      },
      slow: {
        params: { beta: 7 },
        sweeps: { beta: [2, 10, 4] },
      },
    });

    expect(store.fastMaParams).toEqual({ alpha: 3 });
    expect(store.slowMaParams).toEqual({ beta: 7 });
    expect(store.fastMaParamSweeps).toEqual({ alpha: [1, 5, 2] });
    expect(store.slowMaParamSweeps).toEqual({ beta: [2, 10, 4] });
    expect(store.source).toBe("vue");
  });

  it("keeps new backend options when hydrating an older snapshot", () => {
    const store = useOptimizeStore(pinia);
    store.applyLegacySnapshot({
      backend: "GpuHybridKernel",
      backendOptions: [
        { value: "Auto", label: "Auto", disabled: false },
        { value: "CpuOnly", label: "CPU only", disabled: false },
        { value: "GpuOnly", label: "GPU only", disabled: false },
        { value: "GpuKernel", label: "GPU kernel", disabled: false },
      ],
    } as never);

    const optionValues = store.backendOptions.map((option) => option.value);
    expect(optionValues).not.toContain("GpuOnly");
    expect(optionValues).not.toContain("GpuHybridKernel");
    expect(optionValues).toContain("GpuKernel");
    expect(store.backend).toBe("GpuKernel");
  });

  it("includes the staged GPU-native expansion strategies in the selector options", () => {
    const store = useOptimizeStore(pinia);
    const values = store.strategyOptions.map((option) => option.value);
    expect(values).toEqual(
      expect.arrayContaining([
        "Kqsr",
        "Carm",
        "Kvdf",
        "Cdmc",
        "DynamicDoubleMa",
      ]),
    );
  });
});
