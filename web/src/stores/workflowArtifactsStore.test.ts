import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "./runtimePinia";
import {
  applyWorkflowArtifactsPayload,
  readLatestWorkflowRunIdentitySnapshot,
  useWorkflowArtifactsStore,
} from "./workflowArtifactsStore";

let pinia: ReturnType<typeof createPinia>;

describe("workflowArtifactsStore", () => {
  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    vi.restoreAllMocks();
    useWorkflowArtifactsStore(pinia).clearArtifacts();
  });

  it("treats identical normalized payloads as a no-op without timestamp churn", () => {
    const nowSpy = vi
      .spyOn(Date, "now")
      .mockReturnValueOnce(1_000)
      .mockReturnValueOnce(2_000);
    const store = useWorkflowArtifactsStore(pinia);
    const payload = {
      currentRun: {
        ts: "2026-04-01T12:00:00Z",
        artifact_path: "C:/runs/run.json",
        req: { strategy_kind: "DoubleMa" },
        res: { top: [] },
      },
      currentValidation: null,
    };

    const first = applyWorkflowArtifactsPayload(payload, "vue");
    const firstUpdatedAt = store.updatedAtUnixMs;
    const second = applyWorkflowArtifactsPayload(payload, "vue");

    expect(first).toMatchObject({
      applied: true,
      runChanged: true,
      validationChanged: false,
      sourceChanged: false,
    });
    expect(second).toMatchObject({
      applied: false,
      runChanged: false,
      validationChanged: false,
      sourceChanged: false,
    });
    expect(store.updatedAtUnixMs).toBe(firstUpdatedAt);
    expect(readLatestWorkflowRunIdentitySnapshot()).toEqual({
      ts: "2026-04-01T12:00:00Z",
      artifactPath: "C:/runs/run.json",
      hasValidation: false,
    });
    expect(nowSpy).toHaveBeenCalledTimes(1);
  });

  it("reports validation changes when validation arrives later for the same run", () => {
    vi.spyOn(Date, "now")
      .mockReturnValueOnce(1_000)
      .mockReturnValueOnce(2_000);
    const store = useWorkflowArtifactsStore(pinia);
    const baseRun = {
      ts: "2026-04-01T12:00:00Z",
      artifact_path: "C:/runs/run.json",
      req: { strategy_kind: "DoubleMa" },
      res: { top: [] },
    };

    applyWorkflowArtifactsPayload(
      {
        currentRun: baseRun,
        currentValidation: null,
      },
      "vue",
    );

    const next = applyWorkflowArtifactsPayload(
      {
        currentRun: baseRun,
        currentValidation: {
          ts: "2026-04-01T12:00:00Z",
          req: { candidates: 16 },
          res: { rows: [] },
        },
      },
      "vue",
    );

    expect(next).toMatchObject({
      applied: true,
      runChanged: false,
      validationChanged: true,
      sourceChanged: false,
    });
    expect(store.currentValidation?.req).toMatchObject({ candidates: 16 });
    expect(readLatestWorkflowRunIdentitySnapshot()).toEqual({
      ts: "2026-04-01T12:00:00Z",
      artifactPath: "C:/runs/run.json",
      hasValidation: true,
    });
    expect(store.updatedAtUnixMs).toBe(2_000);
  });

  it("normalizes optimization metric surface metadata from hydrated artifacts", () => {
    const store = useWorkflowArtifactsStore(pinia);

    applyWorkflowArtifactsPayload(
      {
        currentRun: {
          ts: "2026-04-01T12:00:00Z",
          artifact_path: "C:/runs/run.json",
          optimization_metric_surface: {
            surface: "core17",
            fields: ["pnl", "trades", "max_dd"],
          },
          req: { strategy_kind: "CompositeTrend" },
          res: { top: [] },
        },
        currentValidation: null,
      },
      "vue",
    );

    expect(store.currentRun?.optimizationMetricSurface).toMatchObject({
      surface: "core17",
      fields: ["pnl", "trades", "max_dd"],
    });
  });

  it("clears stale store state even when the latest global signatures are already null", () => {
    const store = useWorkflowArtifactsStore(pinia);
    store.currentRun = {
      ts: "2026-04-01T12:00:00Z",
      artifactPath: "C:/runs/run.json",
      reportXlsxPath: "",
      csvPath: "",
      datasetMeta: null,
      mlSignal: null,
      optimizationMetricSurface: null,
      tags: [],
      req: null,
      res: null,
    };
    store.currentValidation = {
      ts: "2026-04-01T12:00:00Z",
      req: { candidates: 16 },
      res: { rows: [] },
    };

    store.clearArtifacts();

    expect(store.currentRun).toBeNull();
    expect(store.currentValidation).toBeNull();
    expect(store.source).toBe("vue");
  });
});
