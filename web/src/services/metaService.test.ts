import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invokeCommand } from "../tauri/invoke";
import { bindRuntimePinia } from "../stores/runtimePinia";
import { useCommandActionStore } from "../stores/commandActionStore";
import { LAST_BATCH_SUMMARY_KEY } from "../utils/core/storageKeys";
import { buildOptimizationRequestFromVueStores } from "./optimizeService";
import {
  batchRefresh,
  batchRevealSummary,
  batchSelectAll,
  batchSelectNone,
  deletePolicyEvalSaved,
  deleteSaved,
  deleteStage5Saved,
  deriveVcetTemporalCampaignSourceEligibility,
  loadPolicyEvalSaved,
  loadSaved,
  loadStage5Saved,
  mlApply,
  mlRefresh,
  mlTrain,
  openMetaReport,
  openSavedReport,
  refreshPolicyEvalSaved,
  refreshSaved,
  refreshStage5Saved,
  resolveLatestVcetTemporalCampaignSource,
  revealMetaFiles,
  revealPolicyEvalSaved,
  revealStage5Saved,
  runBatch,
  runMlPolicyEvaluation,
  runMeta,
  savePolicyEvalSavedTags,
  saveSavedTags,
  saveStage5SavedTags,
  runStage5,
  toggleBatchDataset,
} from "./metaService";
import { useAnalysisStore } from "../stores/analysisStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { TauriRuntimeError } from "../tauri/runtimeError";
import {
  buildVcetSearchConfigFromDraft,
  createDefaultVcetSearchDraft,
} from "./vcetSearchRequestBuilder";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

vi.mock("./optimizeService", () => ({
  buildOptimizationRequestFromVueStores: vi.fn(async () => ({
    strategy_kind: "CompositeMo",
    data_id: "data-1",
    ma_source: "close",
    candidates: [],
  })),
}));

vi.mock("./dataService", () => ({
  loadCsv: vi.fn(async (path: string) => ({
    path,
    dataId: "data-1",
    meta: {
      len: 1000,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1700000000000,
      maxTs: 1700003600000,
      timeframe: "1m",
      sourceSizeBytes: 123,
      sourceMtimeUtcMs: 1700000000000,
    },
    cacheHit: false,
    cachePath: "",
    loadSource: "csv_parsed_cache_write_skipped",
  })),
  applyTimeframe: vi.fn(async () => ({
    path: "",
    dataId: "data-1@1m",
    meta: {
      len: 1000,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1700000000000,
      maxTs: 1700003600000,
      timeframe: "1m",
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    },
    cacheHit: false,
    cachePath: "",
    loadSource: "derived_fresh_resample",
  })),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
const buildOptimizationRequestMock = vi.mocked(
  buildOptimizationRequestFromVueStores,
);
let pinia: ReturnType<typeof createPinia>;

function createDeferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason?: unknown) => void;
} {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function invokeDefaultMock(command: string): Promise<unknown> {
  if (command === "run_meta_selection") {
    return {
      ts: "2026-02-26T10:00:00Z",
      report_xlsx_path: "C:/reports/meta.xlsx",
      meta_json_path: "C:/runs/vectorgrid_meta.json",
      result: {
        strategy_kind: "CompositeMo",
        timeframe: "1m",
        artifacts_used: 2,
        artifacts_considered: 3,
        candidates_considered: 40,
        runtime_ms: 123,
        groups: [
          {
            params: {
              fast_ma_type: "ema",
              fast_len: 10,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            pass_rate: 0.55,
            samples: 10,
            robust_score_p10: 0.12,
            robust_score_median: 0.18,
            max_dd_p90: 0.09,
            cross_score_median: 0.04,
            meta_score: 0.8,
          },
        ],
      },
    };
  }

  if (command === "run_meta_stage5_ml_confidence") {
    return {
      ts: "2026-02-26T10:02:00Z",
      stage4_json_path: "C:/runs/vectorgrid_stage5.json",
      stage5_json_path: "C:/runs/vectorgrid_stage5.json",
      result: {
        strategy_kind: "CompositeMo",
        timeframe: "1m",
        source_meta_json_path: "C:/runs/vectorgrid_meta.json",
        model_path: "C:/models/meta_model.json",
        model_state: "healthy",
        model_evidence_tier: "cross_validation",
        model_healthy: true,
        model_trained_now: false,
        groups_selected: 1,
        groups_scored: 1,
        groups_considered: 2,
        min_ml_confidence: 0.55,
        training_artifact_count: 3,
        source_artifact_sha256: "stage5-source-hash",
        warning_count: 0,
        decision_counts: { accept: 1, review: 0, reject: 1 },
        evidence_policy_summary: {
          version: 1,
          workflow: "meta_stage5_ml_confidence",
          summary_role: "interpretation_metadata_only",
          metrics: [
            {
              metric: "ml_confidence",
              metric_class: "approximate",
              gate_role: "run_recommendation",
              sample_unit: "meta_group_features",
              interpretation:
                "Model-estimated holdout success probability for a meta-selection group.",
            },
            {
              metric: "decision_bucket",
              metric_class: "metadata_only",
              gate_role: "advisory",
              sample_unit: "selected_group",
              interpretation: "Threshold review bucket.",
            },
          ],
        },
        selected_groups: [
          {
            stage4_rank: 1,
            decision_bucket: "accept",
            final_rank_score: 0.9,
            ml_confidence: 0.8,
            ml_uncertainty: 0.1,
            ml_confidence_semantics:
              "model_estimated_holdout_success_probability",
            decision_bucket_semantics:
              "threshold_review_bucket_not_deployability_proof",
            params: {
              fast_ma_type: "ema",
              fast_len: 10,
              slow_ma_type: "ema",
              slow_len: 20,
            },
          },
        ],
      },
    };
  }

  if (command === "evaluate_ml_confidence_policy") {
    return {
      ts: "2026-02-26T10:03:00Z",
      eval_json_path: "C:/runs/vectorgrid_ml_policy_eval.json",
      result: {
        strategy_kind: "CompositeMo",
        timeframe: "1m",
        artifacts_considered: 4,
        artifacts_evaluated: 3,
        artifacts_skipped_no_history: 1,
        artifacts_skipped_training_failure: 0,
        artifacts_skipped_unhealthy_model: 0,
        artifacts_skipped_no_labels: 0,
        top_k: 15,
        min_ml_confidence: 0.58,
        meta_weight: 0.6,
        ml_weight: 0.35,
        uncertainty_weight: 0.05,
        meta_only: {
          selected_total: 3,
          positives_total: 2,
          precision_at_k: 2 / 3,
        },
        ml_only: {
          selected_total: 3,
          positives_total: 1,
          precision_at_k: 1 / 3,
        },
        stage5_blended: {
          selected_total: 3,
          positives_total: 3,
          precision_at_k: 1.0,
        },
        warnings: [],
      },
    };
  }

  if (command === "ml_confidence_model_status") {
    return {
      exists: true,
      model_path: "C:/models/meta_model.json",
      bytes: 1024,
      available_meta_artifacts: 10,
      available_labeled_rows: 300,
      stale: false,
      retrain_recommended: false,
      state: "healthy",
      model_healthy: true,
      model_summary: {
        model_kind: "logistic",
        label: {
          min_holdout_pass_rate: 0.6,
          min_holdout_score_p10: 0.1,
          min_holdout_datasets: 5,
        },
        meta: {
          healthy: true,
          cv_auc: 0.71,
          cv_brier_calibrated: 0.19,
          trained_rows: 300,
          target: "holdout_dataset_pass",
        },
      },
      meta_scan_error: null,
    };
  }

  if (command === "train_ml_confidence_model") {
    return {
      model_path: "C:/models/meta_model.json",
      model_bytes: 1024,
      model_summary: {
        label: {
          min_holdout_pass_rate: 0.6,
          min_holdout_score_p10: 0.1,
          min_holdout_datasets: 5,
        },
        meta: {
          healthy: true,
          cv_auc: 0.71,
          cv_brier_calibrated: 0.19,
        },
      },
      metrics: {
        rows: 300,
        positives: 180,
        negatives: 120,
        target: "holdout_gate",
        model_kind: "logistic",
        train_auc: 0.8,
        train_brier: 0.18,
        baseline_brier: 0.24,
        healthy: true,
        warnings: [],
      },
      status: {
        exists: true,
        model_path: "C:/models/meta_model.json",
        bytes: 1024,
        available_meta_artifacts: 10,
        available_labeled_rows: 300,
        stale: false,
        retrain_recommended: false,
        state: "healthy",
        model_healthy: true,
        model_summary: {
          model_kind: "logistic",
          label: {
            min_holdout_pass_rate: 0.6,
            min_holdout_score_p10: 0.1,
            min_holdout_datasets: 5,
          },
          meta: {
            healthy: true,
            cv_auc: 0.71,
            cv_brier_calibrated: 0.19,
            trained_rows: 320,
            target: "holdout_dataset_pass",
          },
        },
        meta_scan_error: null,
      },
    };
  }

  if (command === "apply_ml_confidence_model") {
    return { confidences: [0.88], warnings: [] };
  }

  if (command === "polygon_get_cache_info") {
    return {
      datasets: [
        {
          csv_path: "C:/a.csv",
          display_symbol: "AAPL",
          market: "stocks",
          timeframe: "1m",
          min_ts: 1700000000000,
          max_ts: 1700003600000,
          last_updated_utc_ms: 1700010000000,
          row_count: 1000,
          adjusted: true,
          integrity: {
            csv_size_bytes: 456,
            csv_sha256:
              "D2683B5EF685D46C4F32E3E92D89E83A1EB70DCE166F47311266823189441D82",
          },
        },
      ],
    };
  }

  if (command === "list_repo_data_csvs") {
    return {
      dir: "C:/repo/data",
      truncated: false,
      csv_entries: [
        {
          csv_path: "C:/b.csv",
          source_size_bytes: 123,
          source_mtime_utc_ms: 1700020000000,
        },
      ],
    };
  }

  if (command === "list_run_artifacts") return [];
  if (command === "save_batch_summary")
    return { path: "C:/runs/vectorgrid_batch_summary_1.json" };
  if (command === "run_research_campaign") {
    return {
      summary_json_path: "C:/runs/vectorgrid_campaign_1.json",
      summary: {
        version: 1,
        kind: "vectorgrid_campaign_summary",
        ts: "2026-04-03T12:00:00Z",
        state: "completed",
        strategy_kind: "CompositeMo",
        counts: {
          total: 1,
          optimized: 1,
          validated: 1,
          skipped: 0,
          failed: 0,
          cancelled: false,
        },
        items: [],
        warnings: [],
        runtime_ms: 10,
        tags: ["campaign"],
        holdout_pct: 0,
        holdout_tag: "holdout",
      },
    };
  }
  if (command === "list_artifact_library") {
    return {
      meta: [],
      stage5: [],
      policyEvals: [],
    };
  }
  return undefined;
}

function mockInvokeDefaults(): void {
  invokeCommandMock.mockImplementation(invokeDefaultMock);
}

describe("meta service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.unstubAllGlobals();
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    mockInvokeDefaults();
  });

  it("admits only the retained latest exact terminal VCET evaluate_candidates source for temporal preparation", async () => {
    const candidateManifestHash = "b".repeat(64);
    const envelope = {
      path: "C:/PickerSpelling/VCET-summary.json",
      summary: {
        kind: "vectorgrid_campaign_summary",
        version: 2,
        state: "completed",
        strategy_kind: "Vcet",
        counts: { cancelled: false },
        request: {
          strategy_kind: "Vcet",
          execution_mode: "evaluate_candidates",
          datasets: [{ csv_path: "C:/a.csv" }, { csv_path: "C:/b.csv" }],
        },
        manifest: {
          strategy_kind: "Vcet",
          execution_mode: "evaluate_candidates",
          manifest_hash: "a".repeat(64),
          candidate_manifest_hash: candidateManifestHash,
        },
        fixed_candidate_evaluation: {
          schema_version: 4,
          candidate_manifest_hash: candidateManifestHash,
          admitted_candidate_count: 2,
          admitted_candidate_ids: ["candidate-1", "candidate-2"],
        },
        items: [
          {
            item_id: "dataset-a",
            is_holdout: false,
            status: "ok",
            artifact_path: "C:/runs/a.json",
            artifact_sha256: "c".repeat(64),
          },
          {
            item_id: "dataset-b",
            is_holdout: true,
            status: "ok",
            artifact_path: "C:/runs/b.json",
            artifact_sha256: "d".repeat(64),
          },
        ],
      },
    };

    expect(deriveVcetTemporalCampaignSourceEligibility(envelope)).toEqual({
      eligible: true,
      summaryPath: "C:/PickerSpelling/VCET-summary.json",
      identity: {
        nativePathKey: "c:/pickerspelling/vcet-summary.json",
        summaryContentFingerprint: expect.stringMatching(
          /^campaign-summary-v2-fnv1a64-[0-9a-f]{16}$/u,
        ),
      },
      message: expect.stringContaining("Structurally eligible"),
    });

    const legacyAlias = structuredClone(envelope);
    legacyAlias.summary.kind = "vectorbt_campaign_summary";
    expect(
      deriveVcetTemporalCampaignSourceEligibility(legacyAlias).eligible,
    ).toBe(false);

    const wrongMode = structuredClone(envelope);
    wrongMode.summary.request.execution_mode = "search";
    expect(
      deriveVcetTemporalCampaignSourceEligibility(wrongMode).eligible,
    ).toBe(false);

    const unsealedDataset = structuredClone(envelope);
    unsealedDataset.summary.items[1]!.artifact_sha256 = "not-sealed";
    expect(
      deriveVcetTemporalCampaignSourceEligibility(unsealedDataset).eligible,
    ).toBe(false);

    const samePathChangedContent = structuredClone(envelope);
    (samePathChangedContent.summary as Record<string, unknown>).operator_note =
      "changed on disk";
    const originalIdentity =
      deriveVcetTemporalCampaignSourceEligibility(envelope).identity;
    const changedIdentity = deriveVcetTemporalCampaignSourceEligibility(
      samePathChangedContent,
    ).identity;
    expect(changedIdentity?.nativePathKey).toBe(
      originalIdentity?.nativePathKey,
    );
    expect(changedIdentity?.summaryContentFingerprint).not.toBe(
      originalIdentity?.summaryContentFingerprint,
    );

    const storage = new Map<string, string>([
      [LAST_BATCH_SUMMARY_KEY, envelope.path],
    ]);
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, String(value)),
      removeItem: (key: string) => storage.delete(key),
    });
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.batchOutputPreview = "retained UI preview";
    analysisStore.batchMeta = "retained UI metadata";
    analysisStore.updatedAtUnixMs = 123;
    let diskArtifact: unknown = envelope.summary;
    invokeCommandMock.mockImplementation(async (command: string, args) => {
      if (command === "load_artifact") {
        expect(args).toEqual({ path: envelope.path });
        return diskArtifact;
      }
      return invokeDefaultMock(command);
    });

    await expect(resolveLatestVcetTemporalCampaignSource()).resolves.toEqual({
      eligible: true,
      summaryPath: envelope.path,
      identity: {
        nativePathKey: "c:/pickerspelling/vcet-summary.json",
        summaryContentFingerprint: expect.stringMatching(
          /^campaign-summary-v2-fnv1a64-[0-9a-f]{16}$/u,
        ),
      },
      message: expect.stringContaining("sealing-time verification"),
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("load_artifact", {
      path: envelope.path,
    });
    expect(analysisStore.batchOutputPreview).toBe("retained UI preview");
    expect(analysisStore.batchMeta).toBe("retained UI metadata");
    expect(analysisStore.updatedAtUnixMs).toBe(123);

    const replacedOnDisk = structuredClone(envelope.summary);
    replacedOnDisk.request.execution_mode = "search";
    diskArtifact = replacedOnDisk;
    invokeCommandMock.mockClear();

    await expect(resolveLatestVcetTemporalCampaignSource()).resolves.toEqual(
      expect.objectContaining({
        eligible: false,
        summaryPath: "",
        message: expect.stringContaining("evaluate_candidates"),
      }),
    );
    expect(invokeCommandMock).toHaveBeenCalledTimes(1);
    expect(invokeCommandMock).toHaveBeenCalledWith("load_artifact", {
      path: envelope.path,
    });
  });

  it("runs meta with auto-built request via direct tauri invoke without waiting on saved refresh", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
      metaTopK: "25",
      metaMinSamples: "30",
    });
    const validationStore = useValidationStore(pinia);
    validationStore.applyLegacySnapshot({
      seed: "1",
      wfWindow: "20000",
      wfStep: "20000",
      perturbEnable: false,
      mcEnable: false,
    });

    const deferredSaved = createDeferred<unknown>();
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") return deferredSaved.promise;
      return invokeDefaultMock(command);
    });

    const runPromise = runMeta();
    await Promise.resolve();
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(analysisStore.metaSavedSelected).toBe(
      "C:/runs/vectorgrid_meta.json",
    );
    expect(analysisStore.aggregateSummarySavedMeta).toBe("1");

    deferredSaved.resolve({
      meta: [],
      stage5: [],
      policyEvals: [],
    });
    await runPromise;

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_meta_selection",
    );
    expect(call).toBeTruthy();
    expect(
      (call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req
        ?.strategy_kind,
    ).toBe("CompositeMo");
    expect(
      (call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req
        ?.top_k,
    ).toBe(25);
    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "list_artifact_library",
      ),
    ).toBe(true);
  });

  it("renders VCET holdout GPU residency and requested-universe rates", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "Vcet",
      metaTimeframe: "1m",
      metaTopK: "25",
      metaMinSamples: "5",
    });
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_meta_selection") {
        return {
          ts: "2026-07-31T10:00:00Z",
          report_xlsx_path: "C:/reports/vcet-meta.xlsx",
          meta_json_path: "C:/runs/vcet-meta.json",
          holdout_execution: {
            requested_backend: "GpuKernel",
            require_gpu: true,
            gpu_device_id: 0,
            datasets_requested: 10,
            datasets_evaluated: 8,
            datasets_failed: 2,
            gpu_datasets: 8,
            cpu_datasets: 0,
            fallback_datasets: 0,
            pass_rate_denominator: "requested_unique_holdout_datasets",
            profitable_rate_denominator: "requested_unique_holdout_datasets",
            group_metrics: [
              {
                group_key: "vcet-candidate-1",
                datasets_requested: 10,
                datasets_evaluated: 8,
                datasets_failed: 2,
                passes: 6,
                profitable: 7,
                evaluation_rate: 0.8,
                pass_rate: 0.6,
                profitable_rate: 0.7,
              },
            ],
          },
          result: {
            strategy_kind: "Vcet",
            timeframe: "1m",
            artifacts_used: 10,
            artifacts_considered: 10,
            candidates_considered: 1,
            runtime_ms: 123,
            holdout: { datasets: 8, skipped: 2 },
            groups: [
              {
                key: "vcet-candidate-1",
                params: {
                  fast_ma_type: "ema",
                  fast_len: 10,
                  slow_ma_type: "ema",
                  slow_len: 30,
                },
                samples: 10,
                pass_rate: 0.5,
                robust_score_p10: 0.1,
                robust_score_median: 0.2,
                max_dd_p90: 0.1,
                meta_score: 0.3,
                holdout: {
                  pass_rate: 0.6,
                  score_p10: 0.01,
                  score_median: 0.02,
                  max_dd_p90: 0.08,
                },
              },
            ],
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runMeta();

    expect(analysisStore.metaMeta).toContain("holdout_route=GpuKernel");
    expect(analysisStore.metaMeta).toContain("eval=8/10");
    expect(analysisStore.metaOutputPreview).toContain("fully_gpu=8");
    expect(analysisStore.metaOutputPreview).toContain(
      "requested-universe denominators",
    );
    expect(analysisStore.aggregateSummaryLatestHoldout).toBe(
      "8/10 (GPU 8) · fail 2",
    );
    expect(
      analysisStore.metaTableColumns.map((column) => column.label),
    ).toEqual(
      expect.arrayContaining([
        "H eval (requested)",
        "H pass (requested)",
        "H PnL+ (requested)",
      ]),
    );
    expect(
      analysisStore.metaTableRows[0]?.cells.map((cell) => cell.text),
    ).toEqual(expect.arrayContaining(["80.0%", "60.0%", "70.0%"]));
  });

  it("labels legacy holdout pass rates without claiming a requested-universe denominator", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "Vcet",
      metaTimeframe: "1m",
      metaTopK: "25",
      metaMinSamples: "5",
    });
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_meta_selection") {
        return {
          ts: "2026-07-31T10:05:00Z",
          meta_json_path: "C:/runs/vcet-meta-legacy.json",
          result: {
            strategy_kind: "Vcet",
            timeframe: "1m",
            artifacts_used: 4,
            artifacts_considered: 4,
            candidates_considered: 1,
            runtime_ms: 12,
            holdout: { datasets: 3, skipped: 0 },
            groups: [
              {
                key: "legacy-vcet-candidate",
                params: {
                  fast_ma_type: "ema",
                  fast_len: 10,
                  slow_ma_type: "ema",
                  slow_len: 30,
                },
                samples: 4,
                pass_rate: 0.5,
                meta_score: 0.3,
                holdout: {
                  pass_rate: 0.75,
                  score_p10: 0.01,
                  score_median: 0.02,
                  max_dd_p90: 0.08,
                },
              },
            ],
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runMeta();

    expect(
      analysisStore.metaTableColumns.map((column) => column.label),
    ).toEqual(
      expect.arrayContaining(["H eval", "H pass (legacy reported)", "H PnL+"]),
    );
    expect(analysisStore.metaTableRows[0]?.cells[11]?.text).toBe("75.0%");
    expect(analysisStore.metaOutputPreview).not.toContain(
      "requested-universe denominators",
    );
  });

  it("runs stage5 without waiting on saved refresh or redundant ML status refresh", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
      metaStage4TopK: "15",
      metaStage4MinConf: "58",
    });

    await runMeta();
    await mlRefresh();
    invokeCommandMock.mockClear();

    const deferredSaved = createDeferred<unknown>();
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_meta_stage5_ml_confidence") {
        return {
          ts: "2026-02-26T10:02:00Z",
          stage4_json_path: "C:/runs/vectorgrid_stage5.json",
          stage5_json_path: "C:/runs/vectorgrid_stage5.json",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            source_meta_json_path: "C:/runs/vectorgrid_meta.json",
            model_path: "C:/models/meta_model.json",
            model_state: "healthy",
            model_healthy: true,
            model_trained_now: false,
            groups_selected: 1,
            groups_scored: 1,
            groups_considered: 2,
            min_ml_confidence: 0.55,
            decision_counts: { accept: 1, review: 0, reject: 1 },
            evidence_policy_summary: {
              version: 1,
              workflow: "meta_stage5_ml_confidence",
              summary_role: "interpretation_metadata_only",
              metrics: [
                {
                  metric: "ml_confidence",
                  metric_class: "approximate",
                  gate_role: "run_recommendation",
                  sample_unit: "meta_group_features",
                  interpretation:
                    "Model-estimated holdout success probability for a meta-selection group.",
                },
                {
                  metric: "decision_bucket",
                  metric_class: "metadata_only",
                  gate_role: "advisory",
                  sample_unit: "selected_group",
                  interpretation: "Threshold review bucket.",
                },
              ],
            },
            selected_groups: [
              {
                stage4_rank: 1,
                decision_bucket: "accept",
                final_rank_score: 0.9,
                ml_confidence: 0.8,
                ml_uncertainty: 0.1,
                ml_confidence_semantics:
                  "model_estimated_holdout_success_probability",
                decision_bucket_semantics:
                  "threshold_review_bucket_not_deployability_proof",
                params: {
                  fast_ma_type: "ema",
                  fast_len: 10,
                  slow_ma_type: "ema",
                  slow_len: 20,
                },
              },
            ],
          },
        };
      }
      if (command === "list_artifact_library") return deferredSaved.promise;
      return invokeDefaultMock(command);
    });

    let settled = false;
    const runPromise = runStage5({
      request: {
        strategy_kind: "CompositeMo",
        timeframe: "1m",
        source_meta_path: "C:/runs/vectorgrid_meta.json",
        top_k: 15,
        min_ml_confidence: 0.58,
        meta_weight: 0.6,
        ml_weight: 0.35,
        uncertainty_weight: 0.05,
        accept_confidence: 0.75,
        review_confidence: 0.6,
        max_accept_uncertainty: 0.25,
        require_healthy_model: true,
        fallback_to_meta_if_empty: true,
      },
    }).then(() => {
      settled = true;
    });
    await Promise.resolve();
    await Promise.resolve();

    expect(settled).toBe(true);
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "ml_confidence_model_status",
      ),
    ).toHaveLength(0);

    deferredSaved.resolve({
      meta: [],
      stage5: [],
      policyEvals: [],
    });
    await runPromise;

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_meta_stage5_ml_confidence",
    );
    expect(call).toBeTruthy();
    expect(
      (call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req
        ?.strategy_kind,
    ).toBe("CompositeMo");
    expect(
      (call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req
        ?.top_k,
    ).toBe(15);
    expect(
      (call?.[1] as { req?: { min_ml_confidence?: number } }).req
        ?.min_ml_confidence,
    ).toBe(0.58);
    expect(
      (call?.[1] as { req?: { source_meta_path?: string | null } }).req
        ?.source_meta_path,
    ).toBe("C:/runs/vectorgrid_meta.json");
    expect(analysisStore.metaStage4OutputPreview).toContain(
      "evidence=ml_confidence=model_estimated_holdout_success_probability decision_bucket=review_label_not_deployability_proof",
    );
  });

  it("surfaces Stage 5 lineage warnings in the status and preview", async () => {
    const analysisStore = useAnalysisStore(pinia);
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_meta_stage5_ml_confidence") {
        return {
          ts: "2026-02-26T10:02:00Z",
          stage4_json_path: "C:/runs/vectorgrid_stage5.json",
          stage5_json_path: "C:/runs/vectorgrid_stage5.json",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            source_meta_json_path: "C:/runs/vectorgrid_meta.json",
            model_path: "C:/models/meta_model.json",
            model_state: "warning",
            model_evidence_tier: "train_only",
            model_healthy: false,
            model_trained_now: true,
            groups_selected: 1,
            groups_scored: 1,
            groups_considered: 1,
            min_ml_confidence: 0.55,
            training_artifact_count: 1,
            source_artifact_sha256: "stage5-source-hash",
            warning_count: 2,
            decision_counts: { accept: 0, review: 1, reject: 0 },
            training_manifest: {
              policy: "source_excluded",
              training_artifact_count: 1,
              excluded_source_count: 0,
              excluded_future_or_equal_count: 0,
              excluded_missing_ts_count: 0,
            },
            warnings: [
              "Auto-train did not find the active source meta artifact in saved history.",
              "Auto-train history is thin: only 1 eligible meta artifact(s) after leakage filters.",
            ],
            selected_groups: [
              {
                stage5_rank: 1,
                decision_bucket: "review",
                final_rank_score: 0.42,
                ml_confidence: 0.6,
                ml_uncertainty: 0.2,
                params: {
                  fast_ma_type: "ema",
                  fast_len: 10,
                  slow_ma_type: "ema",
                  slow_len: 20,
                },
              },
            ],
          },
        };
      }
      if (command === "list_artifact_library") {
        return { meta: [], stage5: [], policyEvals: [] };
      }
      return invokeDefaultMock(command);
    });

    await runStage5({
      request: {
        strategy_kind: "CompositeMo",
        timeframe: "1m",
        source_meta_path: "C:/runs/vectorgrid_meta.json",
      },
    });

    expect(analysisStore.metaStage4Status).toContain("warn=2");
    expect(analysisStore.metaStage4Status).toContain("evidence=train only");
    expect(analysisStore.metaStage4OutputPreview).toContain(
      "model_evidence_tier=train only training_artifacts=1 source_sha256=stage5-source-hash",
    );
    expect(analysisStore.metaStage4OutputPreview).toContain(
      "warnings=2: Auto-train did not find the active source meta artifact",
    );
    expect(analysisStore.metaStage4OutputPreview).toContain(
      "training_history=source_excluded artifacts=1 excluded_source=0",
    );
  });

  it("runs ML policy evaluation with auto-built request without waiting on saved refresh", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
      metaStage4TopK: "15",
      metaStage4MinConf: "0.58",
    });

    await runMeta();
    invokeCommandMock.mockClear();

    const deferredSaved = createDeferred<unknown>();
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "evaluate_ml_confidence_policy") {
        return {
          ts: "2026-02-26T10:03:00Z",
          eval_json_path: "C:/runs/vectorgrid_ml_policy_eval.json",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            artifacts_considered: 4,
            artifacts_evaluated: 3,
            artifacts_skipped_no_history: 1,
            artifacts_skipped_training_failure: 0,
            artifacts_skipped_unhealthy_model: 0,
            artifacts_skipped_no_labels: 0,
            top_k: 15,
            min_ml_confidence: 0.58,
            meta_weight: 0.6,
            ml_weight: 0.35,
            uncertainty_weight: 0.05,
            meta_only: {
              selected_total: 3,
              positives_total: 2,
              precision_at_k: 2 / 3,
            },
            ml_only: {
              selected_total: 3,
              positives_total: 1,
              precision_at_k: 1 / 3,
            },
            stage5_blended: {
              selected_total: 3,
              positives_total: 3,
              precision_at_k: 1.0,
            },
            best_policy: "stage5_blended",
            stage5_vs_meta_precision_delta: 0.1,
            stage5_vs_ml_precision_delta: 0.2,
            warnings: [],
          },
        };
      }
      if (command === "list_artifact_library") return deferredSaved.promise;
      return invokeDefaultMock(command);
    });

    let settled = false;
    const runPromise = runMlPolicyEvaluation().then(() => {
      settled = true;
    });
    await Promise.resolve();
    await Promise.resolve();

    expect(settled).toBe(true);
    expect(analysisStore.policyEvalSavedSelected).toBe(
      "C:/runs/vectorgrid_ml_policy_eval.json",
    );

    deferredSaved.resolve({
      meta: [],
      stage5: [],
      policyEvals: [],
    });
    await runPromise;

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "evaluate_ml_confidence_policy",
    );
    expect(call).toBeTruthy();
    expect(
      (call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req
        ?.strategy_kind,
    ).toBe("CompositeMo");
    expect(
      (call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req
        ?.top_k,
    ).toBe(15);
    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "list_artifact_library",
      ),
    ).toBe(true);

    expect(analysisStore.metaPolicyEvalStatus).toContain(
      "Policy evaluation complete",
    );
    expect(analysisStore.metaPolicyEvalStatus).toContain(
      "file=vectorgrid_ml_policy_eval.json",
    );
    expect(analysisStore.metaPolicyEvalOutputPreview).toContain(
      "stage5_blended",
    );
    expect(analysisStore.metaPolicyEvalOutputPreview).toContain(
      "skipped(no_history/train_fail/unhealthy/no_labels)=1/0/0/0",
    );
    expect(analysisStore.metaPolicyEvalOutputPreview).toContain(
      "contract=offline scoped history over saved Meta artifacts",
    );
    expect(analysisStore.metaPolicyEvalOutputPreview).toContain(
      "request reuses current Stage 5 thresholds/weights",
    );
    expect(analysisStore.metaPolicyEvalOutputPreview).toContain(
      "arms=meta_only / ml_only / stage5_blended",
    );
    expect(analysisStore.policyEvalSavedMeta).toContain("arms=meta/ml/stage5");
    expect(analysisStore.policyEvalSavedMeta).toContain("replay=json");
  });

  it("preserves Asr strategy kind for meta selection request", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "Asr",
      metaTimeframe: "1m",
      metaTopK: "10",
      metaMinSamples: "20",
    });
    const validationStore = useValidationStore(pinia);
    validationStore.applyLegacySnapshot({
      seed: "1",
      wfWindow: "20000",
      wfStep: "20000",
      perturbEnable: false,
      mcEnable: false,
    });

    await runMeta();

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_meta_selection",
    );
    expect(call).toBeTruthy();
    expect(
      (call?.[1] as { req?: { strategy_kind?: string } }).req?.strategy_kind,
    ).toBe("Asr");
  });

  it("uses summary-only ML status payloads and minimal apply DTOs on direct command path", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
      metaUseMlConfidence: false,
    });

    await runMeta();
    await mlRefresh();
    expect(analysisStore.metaMlStatus).toContain("cv_auc=0.710");
    expect(analysisStore.metaMlStatus).toContain("model_kind=logistic");
    expect(analysisStore.metaMlStatus).toContain("target=holdout_dataset_pass");
    expect(analysisStore.metaMlStatus).toContain(
      "label: pass>=0.60, p10>=0.100, datasets>=5",
    );
    expect(analysisStore.metaMlStatus).toContain("fresh");

    invokeCommandMock.mockClear();
    await mlTrain();

    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "ml_confidence_model_status",
      ),
    ).toHaveLength(0);
    expect(analysisStore.metaMlStatus).toContain("model_kind=logistic");
    expect(analysisStore.metaMlStatus).toContain("trained_rows=320");
    expect(analysisStore.metaMlStatus).toContain("target=holdout_dataset_pass");
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "train_ml_confidence_model",
      {
        scope: { strategy_kind: "CompositeMo", timeframe: "1m" },
        cfg: null,
      },
    );

    analysisStore.metaUseMlConfidence = true;
    invokeCommandMock.mockClear();
    await mlApply();

    const applyCall = invokeCommandMock.mock.calls.find(
      ([command]) => command === "apply_ml_confidence_model",
    );
    expect(applyCall?.[1]).toEqual({
      scope: { strategy_kind: "CompositeMo", timeframe: "1m" },
      groups: [
        {
          samples: 10,
          pass_rate: 0.55,
          robust_score_p10: 0.12,
          robust_score_median: 0.18,
          max_dd_p90: 0.09,
          cross_score_median: 0.04,
          meta_score: 0.8,
        },
      ],
    });
  });

  it("includes meta scan errors in formatted ML status", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "ml_confidence_model_status") {
        return {
          exists: false,
          state: "missing",
          available_meta_artifacts: 7,
          available_labeled_rows: 140,
          meta_scan_error: "scan failed",
        };
      }
      return invokeDefaultMock(command);
    });

    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
    });

    await mlRefresh();

    expect(analysisStore.metaMlStatus).toContain("meta_scan_error=scan failed");
  });

  it("runs ML autopilot after meta when enabled and ML ranking is active", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
      metaUseMlConfidence: true,
      metaMlAutopilot: true,
    });

    await runMeta();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "ml_confidence_model_status",
      {
        scope: { strategy_kind: "CompositeMo", timeframe: "1m" },
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "apply_ml_confidence_model",
      {
        scope: { strategy_kind: "CompositeMo", timeframe: "1m" },
        groups: [
          {
            samples: 10,
            pass_rate: 0.55,
            robust_score_p10: 0.12,
            robust_score_median: 0.18,
            max_dd_p90: 0.09,
            cross_score_median: 0.04,
            meta_score: 0.8,
          },
        ],
      },
    );
  });

  it("falls back to the latest meta result scope for ML actions and stage5 when draft timeframe is blank", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "",
      metaTimeframe: "",
      metaStage4TopK: "12",
    });

    await runMeta();
    analysisStore.metaStrategyKind = "";
    analysisStore.metaTimeframe = "";
    invokeCommandMock.mockClear();

    await mlRefresh();
    await mlTrain();
    await runStage5();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "ml_confidence_model_status",
      {
        scope: { strategy_kind: "CompositeMo", timeframe: "1m" },
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "train_ml_confidence_model",
      {
        scope: { strategy_kind: "CompositeMo", timeframe: "1m" },
        cfg: null,
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "run_meta_stage5_ml_confidence",
      {
        req: expect.objectContaining({
          strategy_kind: "CompositeMo",
          timeframe: "1m",
          source_meta_path: "C:/runs/vectorgrid_meta.json",
          top_k: 12,
        }),
      },
    );
  });

  it("blocks ML refresh, train, and apply when timeframe is missing from both drafts and the latest meta result", async () => {
    vi.resetModules();

    const { bindRuntimePinia } = await import("../stores/runtimePinia");
    const { useAnalysisStore } = await import("../stores/analysisStore");
    const { invokeCommand } = await import("../tauri/invoke");
    const freshInvokeCommandMock = vi.mocked(invokeCommand);
    freshInvokeCommandMock.mockReset();
    freshInvokeCommandMock.mockImplementation(invokeDefaultMock);

    const freshPinia = createPinia();
    setActivePinia(freshPinia);
    bindRuntimePinia(freshPinia);

    const analysisStore = useAnalysisStore(freshPinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "",
    });

    const {
      mlRefresh: freshMlRefresh,
      mlTrain: freshMlTrain,
      mlApply: freshMlApply,
    } = await import("./metaService");

    await freshMlRefresh();
    expect(analysisStore.metaMlStatus).toBe(
      "Timeframe required (set Meta timeframe filter or run meta once).",
    );
    expect(freshInvokeCommandMock).not.toHaveBeenCalled();

    await freshMlTrain();
    expect(analysisStore.statusLine).toBe(
      "Timeframe required for ML model training.",
    );
    expect(freshInvokeCommandMock).not.toHaveBeenCalled();

    await freshMlApply();
    expect(analysisStore.statusLine).toBe("Timeframe required for ML apply.");
    expect(freshInvokeCommandMock).not.toHaveBeenCalled();
  });

  it("formats aggregate runtime rejections without constructor prefixes", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
    });
    invokeCommandMock.mockRejectedValueOnce(
      new TauriRuntimeError({
        code: "TAURI_COMMAND_UNKNOWN",
        operation: "ml_confidence_model_status",
        message:
          "Desktop runtime unavailable for ML confidence status. Open this action in the Tauri desktop app.",
      }),
    );

    await mlRefresh();

    expect(analysisStore.statusLine).toBe(
      "Desktop runtime unavailable for ML confidence status. Open this action in the Tauri desktop app.",
    );
    expect(analysisStore.statusLine).not.toContain("TauriRuntimeError:");
  });

  it("refreshes ML status after stage5 auto-trains a model", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_meta_selection") {
        return {
          ts: "2026-02-26T10:00:00Z",
          report_xlsx_path: "C:/reports/meta.xlsx",
          meta_json_path: "C:/runs/vectorgrid_meta.json",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            groups: [
              {
                samples: 10,
                pass_rate: 0.55,
                robust_score_p10: 0.12,
                robust_score_median: 0.18,
                max_dd_p90: 0.09,
                cross_score_median: 0.04,
                meta_score: 0.8,
              },
            ],
          },
        };
      }
      if (command === "run_meta_stage5_ml_confidence") {
        return {
          ts: "2026-02-26T10:02:00Z",
          stage4_json_path: "C:/runs/vectorgrid_stage5.json",
          stage5_json_path: "C:/runs/vectorgrid_stage5.json",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            source_meta_json_path: "C:/runs/vectorgrid_meta.json",
            model_path: "C:/models/meta_model.json",
            model_state: "healthy",
            model_healthy: true,
            model_trained_now: true,
            groups_selected: 1,
            groups_scored: 1,
            groups_considered: 1,
            min_ml_confidence: 0.55,
            decision_counts: { accept: 1, review: 0, reject: 0 },
            selected_groups: [],
          },
        };
      }
      if (command === "ml_confidence_model_status") {
        return {
          exists: true,
          model_path: "C:/models/meta_model.json",
          bytes: 1024,
          available_meta_artifacts: 10,
          available_labeled_rows: 300,
          stale: false,
          retrain_recommended: false,
          model_healthy: true,
          model_summary: {
            label: {
              min_holdout_pass_rate: 0.6,
              min_holdout_score_p10: 0.1,
              min_holdout_datasets: 5,
            },
            meta: { healthy: true },
          },
        };
      }
      if (command === "list_artifact_library") {
        return {
          meta: [],
          stage5: [],
          policyEvals: [],
        };
      }
      return undefined;
    });

    const analysisStore = useAnalysisStore(pinia);
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: "CompositeMo",
      metaTimeframe: "1m",
    });

    await runMeta();
    await mlRefresh();
    invokeCommandMock.mockClear();
    await runStage5();
    await Promise.resolve();
    await Promise.resolve();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "ml_confidence_model_status",
      {
        scope: { strategy_kind: "CompositeMo", timeframe: "1m" },
      },
    );
  });

  it("refreshes saved artifact lists from artifact library snapshot", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";
    analysisStore.stage4SavedSelected = "C:/runs/stage5_saved.json";
    analysisStore.policyEvalSavedSelected = "C:/runs/policy_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            { path: "C:/runs/meta_saved.json", ts: "2026-02-26T10:00:00Z" },
          ],
          stage5: [
            {
              path: "C:/runs/stage5_saved.json",
              ts: "2026-02-26T10:02:00Z",
              strategy_kind: "CompositeMo",
              model_evidence_tier: "cross_validation",
              training_artifact_count: 4,
              warning_count: 1,
            },
          ],
          policyEvals: [
            { path: "C:/runs/policy_saved.json", ts: "2026-02-26T10:03:00Z" },
          ],
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();
    await refreshStage5Saved();
    await refreshPolicyEvalSaved();

    expect(analysisStore.metaSavedSelected).toBe("C:/runs/meta_saved.json");
    expect(analysisStore.stage4SavedSelected).toBe("C:/runs/stage5_saved.json");
    expect(analysisStore.stage4SavedMeta).toContain(
      "evidence=cross validation",
    );
    expect(analysisStore.stage4SavedMeta).toContain("training_artifacts=4");
    expect(analysisStore.stage4SavedMeta).toContain("warn=1");
    expect(analysisStore.policyEvalSavedSelected).toBe(
      "C:/runs/policy_saved.json",
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "list_artifact_library",
      ),
    ).toHaveLength(3);
  });

  it("rebuilds compare baseline options from saved meta artifacts and clears stale selections", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaCompareSelected = "C:/runs/stale_meta.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              strategy_kind: "CompositeMo",
              timeframe: "1m",
              groups: 3,
              artifacts_used: 2,
            },
          ],
          stage5: [],
          policyEvals: [],
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();

    expect(analysisStore.metaCompareSelected).toBe("");
    expect(analysisStore.metaCompareOptions).toEqual([
      { value: "", label: "Compare against saved meta...", disabled: false },
      {
        value: "C:/runs/meta_saved.json",
        label: "2026-02-26T10:00:00Z | CompositeMo | 1m | groups=3 | arts=2",
        disabled: false,
      },
    ]);
  });

  it("normalizes snake_case aggregate artifact library fields without losing saved selections", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.stage4SavedSelected = "C:/runs/stage5_saved.json";
    analysisStore.policyEvalSavedSelected = "C:/runs/policy_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [],
          stage_5: [
            { path: "C:/runs/stage5_saved.json", ts: "2026-02-26T10:02:00Z" },
          ],
          policy_evals: [
            { path: "C:/runs/policy_saved.json", ts: "2026-02-26T10:03:00Z" },
          ],
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshStage5Saved();
    await refreshPolicyEvalSaved();

    expect(analysisStore.stage4SavedSelected).toBe("C:/runs/stage5_saved.json");
    expect(analysisStore.policyEvalSavedSelected).toBe(
      "C:/runs/policy_saved.json",
    );
  });

  it("normalizes snake_case policy_evals from artifact library snapshots", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.policyEvalSavedSelected = "C:/runs/policy_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [],
          stage5: [],
          policy_evals: [
            { path: "C:/runs/policy_saved.json", ts: "2026-02-26T10:03:00Z" },
          ],
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshPolicyEvalSaved();

    expect(analysisStore.policyEvalSavedSelected).toBe(
      "C:/runs/policy_saved.json",
    );
    expect(analysisStore.policyEvalSavedOptions).toEqual([
      {
        value: "",
        label: "Select a saved policy evaluation...",
        disabled: false,
      },
      expect.objectContaining({ value: "C:/runs/policy_saved.json" }),
    ]);
    expect(analysisStore.policyEvalSavedMeta).toContain(
      "ts=2026-02-26T10:03:00Z",
    );
  });

  it("loads saved meta/stage5/policy-eval replays through typed artifact commands", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";
    analysisStore.stage4SavedSelected = "C:/runs/stage5_saved.json";
    analysisStore.policyEvalSavedSelected = "C:/runs/policy_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              report_xlsx_path: "C:/reports/meta_saved.xlsx",
            },
          ],
          stage5: [
            { path: "C:/runs/stage5_saved.json", ts: "2026-02-26T10:02:00Z" },
          ],
          policyEvals: [
            { path: "C:/runs/policy_saved.json", ts: "2026-02-26T10:03:00Z" },
          ],
        };
      }
      if (command === "load_meta_artifact_replay") {
        return {
          ts: "2026-02-26T10:00:00Z",
          meta_json_path: "C:/runs/meta_saved.json",
          report_xlsx_path: "C:/reports/meta_saved.xlsx",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            groups: [],
          },
        };
      }
      if (command === "load_stage5_artifact_replay") {
        return {
          ts: "2026-02-26T10:02:00Z",
          stage5_json_path: "C:/runs/stage5_saved.json",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            selected_groups: [],
          },
        };
      }
      if (command === "load_ml_policy_eval_artifact_replay") {
        return {
          ts: "2026-02-26T10:03:00Z",
          eval_json_path: "C:/runs/policy_saved.json",
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            stage5_blended: { precision_at_k: 1 },
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();
    await loadSaved();
    await refreshStage5Saved();
    await loadStage5Saved();
    await refreshPolicyEvalSaved();
    await loadPolicyEvalSaved();

    if (typeof document !== "undefined") {
      expect(document.documentElement.dataset.latestMetaReportPath).toBe(
        "C:/reports/meta_saved.xlsx",
      );
    }

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "load_meta_artifact_replay",
      {
        path: "C:/runs/meta_saved.json",
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "load_stage5_artifact_replay",
      {
        path: "C:/runs/stage5_saved.json",
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "load_ml_policy_eval_artifact_replay",
      {
        path: "C:/runs/policy_saved.json",
      },
    );
    expect(analysisStore.statusLine).toContain("Loaded policy evaluation");
  });

  it("reveals saved Stage 5 and policy-evaluation artifacts through typed artifact selections", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.stage4SavedSelected = "C:/runs/stage5_saved.json";
    analysisStore.policyEvalSavedSelected = "C:/runs/policy_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [],
          stage5: [
            { path: "C:/runs/stage5_saved.json", ts: "2026-02-26T10:02:00Z" },
          ],
          policyEvals: [
            { path: "C:/runs/policy_saved.json", ts: "2026-02-26T10:03:00Z" },
          ],
        };
      }
      if (command === "reveal_path") return true;
      return invokeDefaultMock(command);
    });

    await refreshStage5Saved();
    await revealStage5Saved();
    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/runs/stage5_saved.json",
    });
    expect(analysisStore.statusLine).toBe("Revealed Stage 5 run.");

    await refreshPolicyEvalSaved();
    await revealPolicyEvalSaved();
    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/runs/policy_saved.json",
    });
    expect(analysisStore.statusLine).toBe("Revealed policy evaluation.");
  });

  it("deletes saved meta artifacts through the aggregate-library delete command", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
            },
          ],
          stage5: [],
          policyEvals: [],
        };
      }
      if (command === "delete_artifact") return undefined;
      return invokeDefaultMock(command);
    });

    await refreshSaved();
    await deleteSaved();

    expect(invokeCommandMock).toHaveBeenCalledWith("delete_artifact", {
      path: "C:/runs/meta_saved.json",
    });
    expect(analysisStore.statusLine).toBe("Deleted meta run.");
  });

  it("deletes saved Stage 5 artifacts through the aggregate-library delete command", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.stage4SavedSelected = "C:/runs/stage5_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [],
          stage5: [
            {
              path: "C:/runs/stage5_saved.json",
              ts: "2026-02-26T10:02:00Z",
            },
          ],
          policyEvals: [],
        };
      }
      if (command === "delete_artifact") return undefined;
      return invokeDefaultMock(command);
    });

    await refreshStage5Saved();
    await deleteStage5Saved();

    expect(invokeCommandMock).toHaveBeenCalledWith("delete_artifact", {
      path: "C:/runs/stage5_saved.json",
    });
    expect(analysisStore.statusLine).toBe("Deleted Stage 5 run.");
  });

  it("deletes saved policy evaluations through the aggregate-library delete command", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.policyEvalSavedSelected = "C:/runs/policy_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [],
          stage5: [],
          policyEvals: [
            {
              path: "C:/runs/policy_saved.json",
              ts: "2026-02-26T10:03:00Z",
            },
          ],
        };
      }
      if (command === "delete_artifact") return undefined;
      return invokeDefaultMock(command);
    });

    await refreshPolicyEvalSaved();
    await deletePolicyEvalSaved();

    expect(invokeCommandMock).toHaveBeenCalledWith("delete_artifact", {
      path: "C:/runs/policy_saved.json",
    });
    expect(analysisStore.statusLine).toBe("Deleted policy evaluation.");
  });

  it("reports when saved meta replay returns an empty payload instead of typed replay data", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              report_xlsx_path: "C:/reports/meta_saved.xlsx",
            },
          ],
          stage5: [],
          policyEvals: [],
        };
      }
      if (command === "load_meta_artifact_replay") {
        return null;
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();
    await loadSaved();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "load_meta_artifact_replay",
      {
        path: "C:/runs/meta_saved.json",
      },
    );
    expect(analysisStore.statusLine).toBe(
      "Saved meta artifact payload was empty.",
    );
  });

  it("opens saved meta reports through open_path and falls back to reveal_path", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              report_xlsx_path: "C:/reports/meta_saved.xlsx",
            },
          ],
          stage5: [],
          policyEvals: [],
        };
      }
      if (command === "open_path") {
        throw new Error("blocked");
      }
      if (command === "reveal_path") {
        return true;
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();
    await openSavedReport();

    expect(invokeCommandMock).toHaveBeenCalledWith("open_path", {
      path: "C:/reports/meta_saved.xlsx",
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/reports/meta_saved.xlsx",
    });
    expect(analysisStore.statusLine).toBe("Revealed report.");
  });

  it("blocks saved meta report open actions when the persisted report file is gone", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              report_xlsx_path: "C:/reports/meta_saved.xlsx",
              report_xlsx_exists: false,
            },
          ],
          stage5: [],
          policyEvals: [],
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();

    expect(analysisStore.canOpenMetaSavedReport).toBe(false);

    await openSavedReport();

    expect(invokeCommandMock).not.toHaveBeenCalledWith("open_path", {
      path: "C:/reports/meta_saved.xlsx",
    });
    expect(analysisStore.statusLine).toBe(
      "Saved meta report file no longer exists.",
    );
  });

  it("opens the latest meta report through open_path and falls back to reveal_path", async () => {
    const analysisStore = useAnalysisStore(pinia);
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "open_path") {
        throw new Error("blocked");
      }
      if (command === "reveal_path") {
        return true;
      }
      return invokeDefaultMock(command);
    });

    await runMeta();
    await openMetaReport();

    expect(invokeCommandMock).toHaveBeenCalledWith("open_path", {
      path: "C:/reports/meta.xlsx",
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/reports/meta.xlsx",
    });
    expect(analysisStore.statusLine).toBe("Revealed meta report.");
  });

  it("clears the latest meta report quick action when a replayed saved report file is gone", async () => {
    const analysisStore = useAnalysisStore(pinia);
    const commandActionStore = useCommandActionStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              report_xlsx_path: "C:/reports/meta_saved.xlsx",
              report_xlsx_exists: false,
            },
          ],
          stage5: [],
          policyEvals: [],
        };
      }
      if (command === "load_meta_artifact_replay") {
        return {
          ts: "2026-02-26T10:00:00Z",
          meta_json_path: "C:/runs/meta_saved.json",
          report_xlsx_path: "C:/reports/meta_saved.xlsx",
          report_xlsx_exists: false,
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            groups: [],
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();
    await loadSaved();

    if (typeof document !== "undefined") {
      expect(
        document.documentElement.dataset.latestMetaReportPath,
      ).toBeUndefined();
    }
    expect(analysisStore.canOpenMetaReport).toBe(false);
    expect(commandActionStore.canOpenLatestMetaReport).toBe(false);
    expect(commandActionStore.latestMetaReportPath).toBe("");

    await openMetaReport();

    expect(invokeCommandMock).not.toHaveBeenCalledWith("open_path", {
      path: "C:/reports/meta_saved.xlsx",
    });
    expect(analysisStore.statusLine).toBe(
      "Meta report file no longer exists on disk.",
    );
  });

  it("reveals the meta JSON path when the saved XLSX path is missing on disk", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              report_xlsx_path: "C:/reports/meta_saved.xlsx",
              report_xlsx_exists: false,
            },
          ],
          stage5: [],
          policyEvals: [],
        };
      }
      if (command === "load_meta_artifact_replay") {
        return {
          ts: "2026-02-26T10:00:00Z",
          meta_json_path: "C:/runs/meta_saved.json",
          report_xlsx_path: "C:/reports/meta_saved.xlsx",
          report_xlsx_exists: false,
          result: {
            strategy_kind: "CompositeMo",
            timeframe: "1m",
            groups: [],
          },
        };
      }
      if (command === "reveal_path") {
        return true;
      }
      return invokeDefaultMock(command);
    });

    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";
    await refreshSaved();
    await loadSaved();
    await revealMetaFiles();

    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/runs/meta_saved.json",
    });
    expect(analysisStore.statusLine).toBe("Revealed meta files.");
  });

  it("saves normalized tags consistently across meta, stage5, and policy-eval artifact families", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.metaSavedSelected = "C:/runs/meta_saved.json";
    analysisStore.metaSavedOptions = [
      { value: "", label: "Select a saved meta run...", disabled: false },
      { value: "C:/runs/meta_saved.json", label: "meta", disabled: false },
    ];
    analysisStore.metaSavedTagsDraft = "Holdout, holdout, review,  ";
    analysisStore.stage4SavedSelected = "C:/runs/stage5_saved.json";
    analysisStore.stage4SavedOptions = [
      { value: "", label: "Select a saved Stage 5 run...", disabled: false },
      { value: "C:/runs/stage5_saved.json", label: "stage5", disabled: false },
    ];
    analysisStore.stage4SavedTagsDraft = "Stage5,  tagged, stage5";
    analysisStore.policyEvalSavedSelected = "C:/runs/policy_saved.json";
    analysisStore.policyEvalSavedOptions = [
      {
        value: "",
        label: "Select a saved policy evaluation...",
        disabled: false,
      },
      { value: "C:/runs/policy_saved.json", label: "policy", disabled: false },
    ];
    analysisStore.policyEvalSavedTagsDraft = "Policy, review, policy";

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "update_artifact_tags") return undefined;
      if (command === "list_artifact_library") {
        return {
          meta: [
            {
              path: "C:/runs/meta_saved.json",
              ts: "2026-02-26T10:00:00Z",
              tags: ["Holdout", "review"],
            },
          ],
          stage5: [
            {
              path: "C:/runs/stage5_saved.json",
              ts: "2026-02-26T10:02:00Z",
              tags: ["Stage5", "tagged"],
            },
          ],
          policyEvals: [
            {
              path: "C:/runs/policy_saved.json",
              ts: "2026-02-26T10:03:00Z",
              tags: ["Policy", "review"],
            },
          ],
        };
      }
      return invokeDefaultMock(command);
    });

    await refreshSaved();
    await refreshStage5Saved();
    await refreshPolicyEvalSaved();
    await saveSavedTags();
    await saveStage5SavedTags();
    await savePolicyEvalSavedTags();

    expect(invokeCommandMock).toHaveBeenCalledWith("update_artifact_tags", {
      path: "C:/runs/meta_saved.json",
      tags: ["Holdout", "review"],
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("update_artifact_tags", {
      path: "C:/runs/stage5_saved.json",
      tags: ["Stage5", "tagged"],
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("update_artifact_tags", {
      path: "C:/runs/policy_saved.json",
      tags: ["Policy", "review"],
    });
    expect(analysisStore.metaSavedTagsDraft).toBe("Holdout, review");
    expect(analysisStore.stage4SavedTagsDraft).toBe("Stage5, tagged");
    expect(analysisStore.policyEvalSavedTagsDraft).toBe("Policy, review");
    expect(analysisStore.statusLine).toBe("Saved policy evaluation tags.");
  });

  it("runs batch via vue-owned orchestration path and saves summary", async () => {
    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "CompositeMo";
    optimizeStore.updatedAtUnixMs = Date.now();
    const validationStore = useValidationStore(pinia);
    validationStore.autoRun = true;
    validationStore.autoExportXlsx = true;
    validationStore.autoRevealReport = true;
    validationStore.updatedAtUnixMs = Date.now();
    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-26T10:00:00Z",
        artifact_path: "C:/runs/vectorgrid_run.json",
        csv_path: "C:/a.csv",
        dataset_meta: {
          csv_path: "C:/a.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        ml_signal: {
          stage: "stage5",
          model_summary: "cached model",
        },
        tags: ["existing"],
        req: {
          strategy_kind: "CompositeMo",
        },
        res: {
          top: [],
        },
      },
      currentValidation: null,
    });

    buildOptimizationRequestMock.mockResolvedValue({
      strategy_kind: "CompositeMo",
      data_id: "data-1",
      ma_source: "close",
      candidates: [],
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 10,
        holdout_tag: "holdout",
        auto_meta: false,
        skip_done: false,
        tags: ["batch-1"],
      },
    });

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "run_research_campaign",
      expect.objectContaining({
        req: expect.objectContaining({
          strategy_kind: "CompositeMo",
          holdout_pct: 10,
          holdout_tag: "holdout",
          auto_meta: false,
          tags: ["batch-1"],
          datasets: [
            expect.objectContaining({
              csv_path: "C:/a.csv",
              label: "AAPL",
              timeframe: "1m",
              source_size_bytes: 456,
              source_content_fingerprint: null,
            }),
          ],
        }),
      }),
    );
    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "save_batch_summary",
      ),
    ).toBe(false);

    const analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.canBatchRevealSummary).toBe(true);
    expect(analysisStore.batchOutputPreview).toContain(
      "vectorgrid_campaign_summary",
    );
    expect(validationStore.autoRun).toBe(true);
    expect(validationStore.autoExportXlsx).toBe(true);
    expect(validationStore.autoRevealReport).toBe(true);
    const workflowArtifacts = useWorkflowArtifactsStore(pinia);
    expect(workflowArtifacts.currentRun?.datasetMeta).toMatchObject({
      csv_path: "C:/a.csv",
      timeframe: "1m",
      label: "AAPL",
    });
    expect(workflowArtifacts.currentRun?.mlSignal).toMatchObject({
      stage: "stage5",
    });
  });

  it("emits the UI multidimensional VCET search with an exact candidate cap and no fixed manifest", async () => {
    const analysisStore = useAnalysisStore(pinia);
    analysisStore.setBatchVcetCandidateDesignMode("halton");
    analysisStore.setBatchVcetSearchDraft({
      ...createDefaultVcetSearchDraft(),
      budget: "17",
      seed: "42",
    });
    useOptimizeStore(pinia).strategyKind = "Vcet";

    await runBatch({
      request: {
        strategy_kind: "Vcet",
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 10, 2],
          slow_range: [20, 30, 10],
        },
        validation_config: {},
      },
    });

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    const request = (call?.[1] as { req?: Record<string, unknown> })?.req;
    const validation = request?.validation_config as
      | Record<string, unknown>
      | undefined;
    expect(validation).toMatchObject({
      max_candidates: 17,
      require_gpu_kernel: true,
      vcet_search: {
        budget: 17,
        seed: 42,
        vidya_short_period: { min: 4, max: 48, step: 2 },
        vidya_long_period: { min: 12, max: 160, step: 4 },
        ce_use_close: { values: [false, true] },
        require_vidya_slope: { values: [false, true] },
      },
    });
    expect(validation).not.toHaveProperty("fixed_candidates");
  });

  it("rejects an injected VCET request that supplies both candidate design contracts", async () => {
    const search = buildVcetSearchConfigFromDraft(
      createDefaultVcetSearchDraft(),
    ).config;
    useOptimizeStore(pinia).strategyKind = "Vcet";

    await runBatch({
      request: {
        strategy_kind: "Vcet",
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
        },
        validation_config: {
          max_candidates: 2048,
          fixed_candidates: [{}],
          vcet_search: search,
        },
      },
    });

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(useAnalysisStore(pinia).statusLine).toContain(
      "either fixed_candidates or vcet_search, not both",
    );
  });

  it("fails closed instead of truncating an injected multidimensional design", async () => {
    const search = buildVcetSearchConfigFromDraft(
      createDefaultVcetSearchDraft(),
    ).config;
    useOptimizeStore(pinia).strategyKind = "Vcet";

    await runBatch({
      request: {
        strategy_kind: "Vcet",
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
        },
        validation_config: {
          max_candidates: 128,
          vcet_search: search,
        },
      },
    });

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(useAnalysisStore(pinia).statusLine).toContain(
      "resolves to 2048 candidates",
    );
    expect(useAnalysisStore(pinia).statusLine).toContain(
      "will not be truncated",
    );
  });

  it("validates and canonicalizes an injected VCET search before invoking the backend", async () => {
    const defaults = buildVcetSearchConfigFromDraft(
      createDefaultVcetSearchDraft(),
    ).config;
    useOptimizeStore(pinia).strategyKind = "Vcet";

    await runBatch({
      request: {
        strategy_kind: "Vcet",
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
        },
        validation_config: {
          vcet_search: {
            ...defaults,
            budget: 2,
            seed: 7,
            ce_use_close: { values: [true, false, true] },
            require_vidya_slope: { values: [true, true] },
          },
        },
      },
    });

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    const request = (call?.[1] as { req?: Record<string, unknown> })?.req;
    const validation = request?.validation_config as
      | Record<string, unknown>
      | undefined;
    expect(validation?.max_candidates).toBe(2);
    expect(validation?.vcet_search).toMatchObject({
      budget: 2,
      seed: 7,
      ce_use_close: { values: [false, true] },
      require_vidya_slope: { values: [true] },
    });
    expect(validation).not.toHaveProperty("fixed_candidates");
  });

  it("summarizes VCET fixed-candidate universe coverage and ranks candidates visibly", async () => {
    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_research_campaign") {
        return {
          summary_json_path: "C:/runs/vcet-campaign.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T10:00:00Z",
            state: "completed",
            strategy_kind: "Vcet",
            counts: {
              total: 10,
              optimized: 0,
              validated: 8,
              skipped: 0,
              failed: 2,
              cancelled: false,
            },
            items: [],
            warnings: [],
            runtime_ms: 10,
            tags: ["vcet-universe"],
            holdout_pct: 0,
            fixed_candidate_evaluation: {
              schema_version: 4,
              candidate_manifest_hash: "sha256-manifest",
              require_gpu_kernel: true,
              cross_universe_admission: {
                min_profitable_share_requested: 0.55,
                min_hard_gate_pass_share_requested: 0.5,
                min_pnl_p10: 0,
                min_sharpe_p10: 0,
              },
              admitted_candidate_count: 1,
              selection_status: "selected",
              selection_cohort: "legacy",
              selected_candidate_id: "vcet-10-30",
              requested_datasets: 10,
              evaluated_datasets: 8,
              gpu_kernel_datasets: 8,
              missing_or_failed_datasets: 2,
              candidates: [
                {
                  candidate_id: "vcet-8-20",
                  fast_len: 8,
                  slow_len: 20,
                  requested_datasets: 10,
                  evaluated_datasets: 8,
                  missing_or_failed_datasets: 2,
                  profitable_datasets: 5,
                  hard_gate_pass_datasets: 4,
                  profitable_share_requested: 0.5,
                  hard_gate_pass_share_requested: 0.4,
                  pnl_p10: -0.01,
                  sharpe_p10: -0.1,
                  max_drawdown_p90: 0.2,
                },
                {
                  candidate_id: "vcet-10-30",
                  fast_len: 10,
                  slow_len: 30,
                  requested_datasets: 10,
                  evaluated_datasets: 8,
                  missing_or_failed_datasets: 2,
                  profitable_datasets: 7,
                  hard_gate_pass_datasets: 6,
                  profitable_share_requested: 0.7,
                  hard_gate_pass_share_requested: 0.6,
                  pnl_p10: 0.02,
                  sharpe_p10: 0.3,
                  max_drawdown_p90: 0.12,
                },
              ],
            },
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        tags: ["vcet-universe"],
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 10, 2],
          slow_range: [20, 30, 10],
        },
        validation_config: { max_candidates: 8 },
      },
    });

    const analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchMeta).toContain("VCET replay 8/10");
    expect(analysisStore.batchMeta).toContain("GPU 8");
    expect(analysisStore.batchOutputPreview).toContain(
      "VCET candidate-universe replay",
    );
    expect(analysisStore.batchOutputPreview).toContain(
      "datasets=8/10 fully_gpu=8 missing_or_failed=2",
    );
    expect(analysisStore.batchOutputPreview).toContain(
      "cross-universe admission: PnL+ requested >= 55.0% · hard gate requested >= 50.0% · PnL p10 >= 0.000000 · Sharpe p10 >= 0.0000 · admitted=1/2",
    );
    expect(analysisStore.batchOutputPreview).toContain(
      "p10 requested-tail rule: missing or failed training datasets rank below every finite result; if p10 falls in that missing tail, the metric is unavailable and admission fails",
    );
    expect(analysisStore.batchOutputPreview).toContain(
      "selection_status=selected selection_cohort=legacy selected_candidate_id=vcet-10-30",
    );
    expect(analysisStore.batchVcetEvidence).toMatchObject({
      state: "degraded",
      cohort: "legacy",
      candidateManifestHash: "sha256-manifest",
      selectedCandidateId: "vcet-10-30",
      admissionSource: "derived",
      admittedCandidateCount: 1,
      fastValues: [8, 10],
      slowValues: [20, 30],
    });
    const bestIndex = analysisStore.batchOutputPreview.indexOf("#1 10/30");
    const weakerIndex = analysisStore.batchOutputPreview.indexOf("#2 8/20");
    expect(bestIndex).toBeGreaterThanOrEqual(0);
    expect(weakerIndex).toBeGreaterThan(bestIndex);
  });

  it("ranks VCET campaign candidates from training only and labels holdout audit-only", async () => {
    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    const candidate = (
      id: string,
      fast: number,
      slow: number,
      profitable: number,
      gate: number,
      sharpe: number,
      requested: number,
    ) => ({
      candidate_id: id,
      fast_len: fast,
      slow_len: slow,
      requested_datasets: requested,
      evaluated_datasets: requested,
      missing_or_failed_datasets: 0,
      profitable_datasets: Math.round(profitable * requested),
      hard_gate_pass_datasets: Math.round(gate * requested),
      profitable_share_requested: profitable,
      hard_gate_pass_share_requested: gate,
      pnl_p10: profitable - 0.5,
      sharpe_p10: sharpe,
      max_drawdown_p90: 0.1,
    });
    const trainWinner = candidate("train-winner", 8, 20, 0.9, 0.8, 0.7, 6);
    const holdoutWinner = candidate("holdout-winner", 10, 30, 0.2, 0.1, 0.0, 6);
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_research_campaign") {
        return {
          summary_json_path: "C:/runs/vcet-train-holdout.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T11:00:00Z",
            state: "completed",
            strategy_kind: "Vcet",
            counts: {
              total: 10,
              optimized: 0,
              validated: 10,
              skipped: 0,
              failed: 0,
              cancelled: false,
            },
            items: [],
            warnings: [],
            runtime_ms: 10,
            tags: ["vcet-split"],
            holdout_pct: 40,
            fixed_candidate_evaluation: {
              schema_version: 2,
              candidate_manifest_hash: "sha256-split",
              require_gpu_kernel: true,
              selection_cohort: "training",
              selected_candidate_id: "train-winner",
              requested_datasets: 10,
              evaluated_datasets: 10,
              gpu_kernel_datasets: 10,
              missing_or_failed_datasets: 0,
              // Deliberately conflicts with training; this combined/root list
              // must not become a ranking source once split aggregates exist.
              candidates: [
                { ...holdoutWinner, profitable_share_requested: 1 },
                { ...trainWinner, profitable_share_requested: 0 },
              ],
              training: {
                requested_datasets: 6,
                evaluated_datasets: 6,
                gpu_kernel_datasets: 6,
                missing_or_failed_datasets: 0,
                candidates: [trainWinner, holdoutWinner],
              },
              holdout: {
                requested_datasets: 4,
                evaluated_datasets: 4,
                gpu_kernel_datasets: 4,
                missing_or_failed_datasets: 0,
                candidates: [
                  candidate("holdout-winner", 10, 30, 1, 1, 1, 4),
                  candidate("train-winner", 8, 20, 0, 0, -1, 4),
                ],
              },
            },
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        holdout_pct: 40,
        tags: ["vcet-split"],
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 10, 2],
          slow_range: [20, 30, 10],
        },
        validation_config: { max_candidates: 8 },
      },
    });

    const preview = useAnalysisStore(pinia).batchOutputPreview;
    expect(preview).toContain(
      "training-only ranking: datasets=6/6 candidates=2",
    );
    expect(preview).toContain(
      "holdout audit-only (excluded from ranking): datasets=4/4 candidates=2",
    );
    expect(preview).toContain(
      "selection_cohort=training selected_candidate_id=train-winner",
    );
    expect(preview).toContain("#1 8/20");
    expect(preview).toContain("#1 8/20 · PnL+ 90.0%");
    expect(preview).toContain("backend-selected");
    expect(preview.indexOf("#1 8/20")).toBeGreaterThanOrEqual(0);
    expect(preview.indexOf("#2 10/30")).toBeGreaterThan(
      preview.indexOf("#1 8/20"),
    );
  });

  it("shows a fail-closed VCET selection when no candidate meets cross-universe admission", async () => {
    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    const rejectedCandidate = {
      candidate_id: "vcet-8-20",
      fast_len: 8,
      slow_len: 20,
      requested_datasets: 10,
      evaluated_datasets: 10,
      missing_or_failed_datasets: 0,
      profitable_datasets: 5,
      hard_gate_pass_datasets: 4,
      profitable_share_requested: 0.5,
      hard_gate_pass_share_requested: 0.4,
      pnl_p10: -0.01,
      sharpe_p10: -0.1,
      max_drawdown_p90: 0.2,
    };
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_research_campaign") {
        return {
          summary_json_path: "C:/runs/vcet-no-admitted-candidate.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T11:15:00Z",
            state: "completed",
            strategy_kind: "Vcet",
            counts: {
              total: 10,
              optimized: 0,
              validated: 10,
              skipped: 0,
              failed: 0,
              cancelled: false,
            },
            items: [],
            warnings: [],
            runtime_ms: 10,
            tags: ["vcet-admission"],
            holdout_pct: 0,
            fixed_candidate_evaluation: {
              schema_version: 2,
              candidate_manifest_hash: "sha256-no-admission",
              require_gpu_kernel: true,
              cross_universe_admission: {
                min_profitable_share_requested: 0.55,
                min_hard_gate_pass_share_requested: 0.5,
                min_pnl_p10: 0,
                min_sharpe_p10: 0,
              },
              admitted_candidate_count: 0,
              selection_status: "no_candidate_met_admission",
              selection_cohort: "training",
              selected_candidate_id: null,
              requested_datasets: 10,
              evaluated_datasets: 10,
              gpu_kernel_datasets: 10,
              missing_or_failed_datasets: 0,
              candidates: [rejectedCandidate],
              training: {
                requested_datasets: 10,
                evaluated_datasets: 10,
                gpu_kernel_datasets: 10,
                missing_or_failed_datasets: 0,
                candidates: [rejectedCandidate],
              },
              holdout: {
                requested_datasets: 0,
                evaluated_datasets: 0,
                gpu_kernel_datasets: 0,
                missing_or_failed_datasets: 0,
                candidates: [],
              },
            },
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        tags: ["vcet-admission"],
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 8, 1],
          slow_range: [20, 20, 1],
        },
        validation_config: { max_candidates: 1 },
      },
    });

    const preview = useAnalysisStore(pinia).batchOutputPreview;
    expect(preview).toContain("admitted=0/1");
    expect(preview).toContain(
      "selection unavailable: no candidate met cross-universe admission",
    );
    expect(preview).toContain("#1 8/20");
    expect(preview).not.toContain("backend-selected");
  });

  it("matches backend VCET tie-break ordering for missing, negative, pnl-tied, and id-tied metrics", async () => {
    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    const row = (
      candidateId: string,
      fastLen: number,
      slowLen: number,
      sharpeP10: number | null,
      pnlP10: number | null,
    ) => ({
      candidate_id: candidateId,
      fast_len: fastLen,
      slow_len: slowLen,
      requested_datasets: 4,
      evaluated_datasets: 4,
      missing_or_failed_datasets: 0,
      profitable_datasets: 2,
      hard_gate_pass_datasets: 2,
      profitable_share_requested: 0.5,
      hard_gate_pass_share_requested: 0.5,
      pnl_p10: pnlP10,
      sharpe_p10: sharpeP10,
      max_drawdown_p90: 0.1,
    });
    const candidates = [
      row("missing", 8, 20, null, 100),
      row("negative", 9, 21, -0.5, -1),
      row("alpha", 10, 22, -0.5, 0),
      row("beta", 11, 23, -0.5, 0),
    ];
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_research_campaign") {
        return {
          summary_json_path: "C:/runs/vcet-ties.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T11:30:00Z",
            state: "completed",
            strategy_kind: "Vcet",
            counts: {
              total: 4,
              optimized: 0,
              validated: 4,
              skipped: 0,
              failed: 0,
              cancelled: false,
            },
            items: [],
            warnings: [],
            runtime_ms: 10,
            tags: ["vcet-ties"],
            holdout_pct: 0,
            fixed_candidate_evaluation: {
              schema_version: 2,
              candidate_manifest_hash: "sha256-ties",
              require_gpu_kernel: true,
              requested_datasets: 4,
              evaluated_datasets: 4,
              gpu_kernel_datasets: 4,
              missing_or_failed_datasets: 0,
              selection_cohort: "training",
              selected_candidate_id: "alpha",
              candidates,
              training: {
                requested_datasets: 4,
                evaluated_datasets: 4,
                gpu_kernel_datasets: 4,
                missing_or_failed_datasets: 0,
                candidates,
              },
              holdout: {
                requested_datasets: 0,
                evaluated_datasets: 0,
                gpu_kernel_datasets: 0,
                missing_or_failed_datasets: 0,
                candidates: [],
              },
            },
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        tags: ["vcet-ties"],
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 11, 1],
          slow_range: [20, 23, 1],
        },
        validation_config: { max_candidates: 32 },
      },
    });

    const preview = useAnalysisStore(pinia).batchOutputPreview;
    const first = preview.indexOf("#1 10/22");
    const second = preview.indexOf("#2 11/23");
    const third = preview.indexOf("#3 9/21");
    const fourth = preview.indexOf("#4 8/20");
    expect(first).toBeGreaterThanOrEqual(0);
    expect(second).toBeGreaterThan(first);
    expect(third).toBeGreaterThan(second);
    expect(fourth).toBeGreaterThan(third);
    expect(preview.slice(first, second)).toContain("backend-selected");
    expect(preview.slice(fourth)).toContain("Sharpe p10 -");
  });

  it("does not display a VCET winner when the training cohort has no evaluated evidence", async () => {
    useOptimizeStore(pinia).strategyKind = "Vcet";
    const unevaluatedCandidate = {
      candidate_id: "unevaluated",
      fast_len: 8,
      slow_len: 20,
      requested_datasets: 3,
      evaluated_datasets: 0,
      missing_or_failed_datasets: 3,
      profitable_datasets: 0,
      hard_gate_pass_datasets: 0,
      profitable_share_requested: 0,
      hard_gate_pass_share_requested: 0,
      pnl_p10: null,
      sharpe_p10: null,
      max_drawdown_p90: null,
    };
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_research_campaign") {
        return {
          summary_json_path: "C:/runs/vcet-no-training-evidence.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T11:45:00Z",
            state: "completed_with_failures",
            strategy_kind: "Vcet",
            counts: {
              total: 5,
              optimized: 0,
              validated: 2,
              skipped: 0,
              failed: 3,
              cancelled: false,
            },
            items: [],
            warnings: [],
            runtime_ms: 10,
            tags: ["vcet-no-training-evidence"],
            holdout_pct: 40,
            fixed_candidate_evaluation: {
              schema_version: 2,
              candidate_manifest_hash: "sha256-no-training-evidence",
              require_gpu_kernel: true,
              requested_datasets: 5,
              evaluated_datasets: 2,
              gpu_kernel_datasets: 2,
              missing_or_failed_datasets: 3,
              selection_cohort: "training",
              selected_candidate_id: null,
              candidates: [unevaluatedCandidate],
              training: {
                requested_datasets: 3,
                evaluated_datasets: 0,
                gpu_kernel_datasets: 0,
                missing_or_failed_datasets: 3,
                candidates: [unevaluatedCandidate],
              },
              holdout: {
                requested_datasets: 2,
                evaluated_datasets: 2,
                gpu_kernel_datasets: 2,
                missing_or_failed_datasets: 0,
                candidates: [
                  {
                    ...unevaluatedCandidate,
                    evaluated_datasets: 2,
                    missing_or_failed_datasets: 0,
                    profitable_datasets: 2,
                    profitable_share_requested: 1,
                    pnl_p10: 1,
                    sharpe_p10: 3,
                  },
                ],
              },
            },
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        holdout_pct: 40,
        tags: ["vcet-no-training-evidence"],
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 8, 1],
          slow_range: [20, 20, 1],
        },
        validation_config: { max_candidates: 8 },
      },
    });

    const preview = useAnalysisStore(pinia).batchOutputPreview;
    expect(preview).toContain("selection unavailable: no training evidence");
    expect(preview).toContain(
      "holdout audit-only (excluded from ranking): datasets=2/2 candidates=1",
    );
    expect(preview).not.toContain("#1 ");
    expect(preview).not.toContain("backend-selected");
  });

  it("preserves the last completed campaign and storage pointer across VCET preflight and backend failures", async () => {
    const analysisStore = useAnalysisStore(pinia);
    const storageValues = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      get length() {
        return storageValues.size;
      },
      clear: () => storageValues.clear(),
      getItem: (key: string) => storageValues.get(key) ?? null,
      key: (index: number) => Array.from(storageValues.keys())[index] ?? null,
      removeItem: (key: string) => storageValues.delete(key),
      setItem: (key: string, value: string) => storageValues.set(key, value),
    } satisfies Storage);
    let campaignCalls = 0;
    let failBackend = false;
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_research_campaign") {
        campaignCalls += 1;
        if (failBackend) throw new Error("campaign backend unavailable");
        return {
          summary_json_path: "C:/runs/last-completed.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T12:00:00Z",
            state: "completed",
            strategy_kind: "CompositeMo",
            counts: {
              total: 1,
              optimized: 1,
              validated: 1,
              skipped: 0,
              failed: 0,
              cancelled: false,
            },
            request: { datasets: [{ csv_path: "C:/a.csv" }] },
            items: [],
            warnings: [],
            runtime_ms: 10,
            tags: ["preserved"],
            holdout_pct: 0,
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runBatch({
      request: {
        strategy_kind: "CompositeMo",
        selected_paths: ["C:/a.csv"],
        auto_meta: false,
        tags: ["preserved"],
        optimization_template: { strategy_kind: "CompositeMo" },
        validation_config: {},
      },
    });
    const completedPreview = analysisStore.batchOutputPreview;
    expect(completedPreview).toContain("path=C:/runs/last-completed.json");
    expect(
      globalThis.localStorage.getItem("vectorgrid_last_batch_summary_path_v1"),
    ).toBe("C:/runs/last-completed.json");

    await runBatch({
      request: {
        strategy_kind: "Vcet",
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        auto_meta: false,
        tags: ["invalid-vcet"],
        optimization_template: {
          strategy_kind: "Vcet",
          // Invalid on purpose: fixed-manifest construction must fail before
          // invoking the backend without destroying the completed summary.
          fast_range: [10, 8, 1],
          slow_range: [20, 30, 1],
        },
        validation_config: { max_candidates: 8 },
      },
    });

    expect(campaignCalls).toBe(1);
    expect(analysisStore.batchOutputPreview).toBe(completedPreview);
    expect(analysisStore.canBatchRevealSummary).toBe(true);
    expect(
      globalThis.localStorage.getItem("vectorgrid_last_batch_summary_path_v1"),
    ).toBe("C:/runs/last-completed.json");

    failBackend = true;
    await runBatch({
      request: {
        strategy_kind: "CompositeMo",
        selected_paths: ["C:/a.csv"],
        auto_meta: false,
        tags: ["backend-failure"],
        optimization_template: { strategy_kind: "CompositeMo" },
        validation_config: {},
      },
    });
    expect(campaignCalls).toBe(2);
    expect(analysisStore.batchOutputPreview).toBe(completedPreview);
    expect(
      globalThis.localStorage.getItem("vectorgrid_last_batch_summary_path_v1"),
    ).toBe("C:/runs/last-completed.json");

    await batchRevealSummary();
    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/runs/last-completed.json",
    });
  });

  it("keeps a 10k-dataset campaign preview bounded while retaining useful failure counts", async () => {
    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "CompositeMo";
    optimizeStore.updatedAtUnixMs = Date.now();
    const items = Array.from({ length: 10_000 }, (_, index) => ({
      csv_path: `C:/datasets/dataset-${String(index).padStart(5, "0")}.csv`,
      label: `Dataset ${index}`,
      timeframe: "1m",
      is_holdout: false,
      status: "failed",
      error: `synthetic failure ${index}`,
    }));
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_research_campaign") {
        return {
          summary_json_path: "C:/runs/campaign-10k.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T13:00:00Z",
            state: "completed_with_failures",
            strategy_kind: "CompositeMo",
            counts: {
              total: 10_000,
              optimized: 0,
              validated: 0,
              skipped: 0,
              failed: 10_000,
              cancelled: false,
            },
            request: {
              datasets: items.map((item) => ({ csv_path: item.csv_path })),
            },
            items,
            warnings: Array.from(
              { length: 100 },
              (_, index) => `warning ${index} ${"x".repeat(500)}`,
            ),
            runtime_ms: 1234,
            tags: ["scale"],
            holdout_pct: 0,
          },
        };
      }
      return invokeDefaultMock(command);
    });

    await runBatch({
      request: {
        strategy_kind: "CompositeMo",
        selected_paths: ["C:/a.csv"],
        auto_meta: false,
        tags: ["scale"],
        optimization_template: { strategy_kind: "CompositeMo" },
        validation_config: {},
      },
    });

    const preview = useAnalysisStore(pinia).batchOutputPreview;
    expect(preview.length).toBeLessThan(10_000);
    expect(preview).toContain("counts total=10000");
    expect(preview).toContain("items=10000");
    expect(preview).toContain("failed items showing 12/10000");
    expect(preview).toContain("warnings showing 12/100");
    expect(preview).toContain("dataset-00000.csv");
    expect(preview).not.toContain("dataset-09999.csv");
  });

  it("refreshes, selects, clears, toggles, and reveals batch datasets in vue path", async () => {
    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "CompositeMo";
    optimizeStore.updatedAtUnixMs = Date.now();

    await batchRefresh();
    let analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchTotalCount).toBeGreaterThan(0);
    expect(analysisStore.batchDatasets.map((row) => row.path).sort()).toEqual([
      "C:/a.csv",
      "C:/b.csv",
    ]);
    const polygonRow = analysisStore.batchDatasets.find(
      (row) => row.path === "C:/a.csv",
    );
    const repoRow = analysisStore.batchDatasets.find(
      (row) => row.path === "C:/b.csv",
    );
    expect(polygonRow?.meta).toContain("stocks (adj)");
    expect(repoRow?.meta).toContain("local");

    await batchSelectAll();
    analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchSelectedCount).toBe(
      analysisStore.batchTotalCount,
    );

    await batchSelectNone();
    analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchSelectedCount).toBe(0);

    await toggleBatchDataset("C:/a.csv", true);
    analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchSelectedCount).toBe(1);

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        skip_done: false,
        tags: ["batch-2"],
      },
    });
    await batchRevealSummary();

    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/runs/vectorgrid_campaign_1.json",
    });
  });

  it("merges packaged samples into the clean-install batch catalog without duplicating existing paths", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_sample_csvs") {
        return [
          {
            id: "synthetic-ohlcv",
            label: "Synthetic OHLCV sample",
            path: "C:/package/resources/sample-data/vectorgrid_synthetic_ohlcv.csv",
            source_kind: "bundled_synthetic",
            timeframe: "1m",
            row_count: 160,
          },
          {
            id: "repo-duplicate",
            label: "Duplicate repo sample",
            path: "C:/b.csv",
            source_kind: "repo_local",
            timeframe: "1m",
          },
        ];
      }
      return invokeDefaultMock(command);
    });

    await batchRefresh();

    const analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchDatasets.map((row) => row.path).sort()).toEqual([
      "C:/a.csv",
      "C:/b.csv",
      "C:/package/resources/sample-data/vectorgrid_synthetic_ohlcv.csv",
    ]);
    const sampleRow = analysisStore.batchDatasets.find((row) =>
      row.path.endsWith("vectorgrid_synthetic_ohlcv.csv"),
    );
    expect(sampleRow?.label).toBe("Synthetic OHLCV sample");
    expect(sampleRow?.meta).toContain("sample");
    expect(sampleRow?.meta).toContain("tf=1m");
    expect(sampleRow?.meta).toContain("rows=160");
  });

  it("keeps local batch datasets and reports Polygon refresh failures", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "polygon_get_cache_info") {
        throw new Error("polygon unavailable");
      }
      return invokeDefaultMock(command);
    });

    await batchRefresh();

    const analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchDatasets.map((row) => row.path)).toEqual([
      "C:/b.csv",
    ]);
    expect(analysisStore.batchMeta).toContain("polygon error");
    expect(analysisStore.batchMeta).toContain("local=data");
  });

  it("keeps Polygon batch datasets and reports local repo-data refresh failures", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_repo_data_csvs") {
        throw new Error("repo data unavailable");
      }
      return invokeDefaultMock(command);
    });

    await batchRefresh();

    const analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchDatasets.map((row) => row.path)).toEqual([
      "C:/a.csv",
    ]);
    expect(analysisStore.batchMeta).toContain("local data error");
  });

  it("keeps Polygon and local datasets when packaged sample discovery fails", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "list_sample_csvs") {
        throw new Error("packaged samples unavailable");
      }
      return invokeDefaultMock(command);
    });

    await batchRefresh();

    const analysisStore = useAnalysisStore(pinia);
    expect(analysisStore.batchDatasets.map((row) => row.path).sort()).toEqual([
      "C:/a.csv",
      "C:/b.csv",
    ]);
    expect(analysisStore.batchMeta).toContain("sample data error");
  });
});
