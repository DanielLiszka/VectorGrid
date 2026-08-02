import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  clearValidationReportPerfEntries,
  readValidationReportPerfEntries,
} from "./validationReportPerf";
import {
  crossRefresh,
  exportReportJson,
  exportReportXlsx,
  exportShortlistCsv,
  exportValidationJson,
  openReportsDir,
  previewValidationAdmissionComparisonFromStores,
  previewValidationContextCompatibilityFromStores,
  previewValidationExecutionSummaryFromStores,
  runValidation,
  toggleCrossDataset,
} from "./validationService";
import { invokeCommand } from "../tauri/invoke";
import { bindRuntimePinia } from "../stores/runtimePinia";
import { useDataStore } from "../stores/dataStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({ datasets: [] })),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
let pinia: ReturnType<typeof createPinia>;
const baseDataMeta = {
  len: 0,
  hasOpen: true,
  hasHigh: true,
  hasLow: true,
  hasClose: true,
  hasVolume: true,
  minTs: 0,
  maxTs: 0,
  timeframe: "15m",
  sourceSizeBytes: null,
  sourceMtimeUtcMs: null,
} as const;

function installDefaultInvokeCommandMock(): void {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (
      command === "run_double_ma_validation" ||
      command === "run_composite_validation"
    ) {
      return {
        backend_used: "Cpu",
        backend_route: "GpuKernel->CpuOnly",
        backend_fallback_reason:
          "GPU validation runtime not implemented yet; using CPU validation.",
        candidates: [{ score: 1 }],
        runtime_ms: 123,
        cross_ticker: { datasets: [] },
        reopt_walk_forward: {
          windows: 3,
          top_n_candidates: 2,
          unique_winners: 2,
          winner_changes: 1,
          winner_param_drift_mean: 0.15,
          winner_param_drift_max: 0.3,
          forward_positive_pnl_frac: 2 / 3,
          forward_sharpe_median: 0.42,
          forward_max_dd_worst: 0.11,
          winner_sequence: [
            JSON.stringify({
              fast_ma_type: "sma",
              fast_len: 10,
              slow_ma_type: "alma",
              slow_len: 40,
            }),
            JSON.stringify({
              fast_ma_type: "sma",
              fast_len: 12,
              slow_ma_type: "alma",
              slow_len: 44,
            }),
            JSON.stringify({
              fast_ma_type: "sma",
              fast_len: 12,
              slow_ma_type: "alma",
              slow_len: 44,
            }),
          ],
        },
        run_verdict: {
          passed: true,
          approved: false,
          candidates_tested: 1,
          candidates_survived_hard_gates: 1,
          approved_candidate_count: 0,
          final_recommendation: "evidence_too_short_min_track_record",
          top_candidate_track_record_bars: 120,
          top_candidate_min_track_record_length_bars: 240,
        },
        evidence_policy_summary: {
          version: 1,
          workflow: "general_validation",
          summary_role: "interpretation_metadata_only",
          metrics: [
            {
              metric: "psr",
              metric_class: "confidence_bounded",
              gate_role: "advisory",
              sample_unit: "raw_bars",
              interpretation: "Probabilistic Sharpe Ratio.",
            },
            {
              metric: "cscv_pbo",
              metric_class: "approximate",
              gate_role: "advisory",
              sample_unit: "candidate_set_slices",
              interpretation: "Set-level overfit evidence.",
            },
          ],
        },
        spa: {
          scope: "validation_candidates",
          candidate_count: 1,
          slice_count: 8,
          bootstrap_samples: 300,
          block_length: 4,
          benchmark: "buy_and_hold",
          best_candidate_key: "sma(10)/alma(40)",
          best_candidate_mean_diff: 0.02,
          best_candidate_t_stat: 1.4,
          p_value: 0.17,
          passed: false,
        },
      };
    }
    if (command === "save_run_artifact") {
      return {
        path: "runs/vectorgrid_run_validation_test.json",
        bytes: 256,
      };
    }
    if (command === "build_validation_shortlist_csv") {
      return "rank,fast_len,slow_len";
    }
    if (command === "save_report_xlsx") {
      return {
        path: "runs/vectorgrid_report_validation_test.xlsx",
        artifact_updated: false,
        artifact_update_warning: "",
      };
    }
    if (command === "save_report_xlsx_from_artifact") {
      return {
        path: "runs/vectorgrid_report_validation_test.xlsx",
        artifact_updated: true,
        artifact_update_warning: "",
      };
    }
    if (command === "build_report_xlsx") {
      return "AA==";
    }
    if (command === "open_path" || command === "reveal_path") {
      return true;
    }
    if (command === "polygon_get_cache_info") {
      return { datasets: [] };
    }
    return {};
  });
}

function firstValidationRunCall(): [string, Record<string, unknown>] {
  const runCall = invokeCommandMock.mock.calls.find(
    ([command]) =>
      command === "run_double_ma_validation" ||
      command === "run_composite_validation",
  );
  expect(runCall).toBeTruthy();
  return [
    String(runCall?.[0] || ""),
    (runCall?.[1] ?? {}) as Record<string, unknown>,
  ];
}

function firstValidationRequest(): Record<string, unknown> {
  const [, args] = firstValidationRunCall();
  return args.req && typeof args.req === "object"
    ? (args.req as Record<string, unknown>)
    : {};
}

describe("validationService", () => {
  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    useWorkflowArtifactsStore(pinia).clearArtifacts();
    vi.clearAllMocks();
    installDefaultInvokeCommandMock();
    clearValidationReportPerfEntries();
    if (typeof window !== "undefined") {
      window.__VTA_LEGACY_BOOTED__ = false;
    }
  });

  it("builds validation request from workflow artifacts and runs direct validation command", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "32",
      seed: "1",
      minPsr: "",
      minDsr: "",
      spaEnable: true,
      spaSlices: "8",
      spaBootstrapSamples: "300",
      spaBlockLength: "4",
      reoptWfEnable: true,
      reoptWfWindow: "12000",
      reoptWfStep: "3000",
      reoptWfTopN: "5",
      wfEnable: false,
      wfWindow: "20000",
      wfStep: "20000",
      perturbEnable: false,
      perturbIters: "8",
      perturbMin: "0.015",
      perturbMax: "0.06",
      mcEnable: false,
      mcSims: "500",
      cscvEnable: false,
      cscvSlices: "",
      crossEnable: true,
      crossMax: "2",
      crossDatasets: [{ path: "C:/data/a.csv", label: "A", checked: true }],
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-123",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["alma"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "alma",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const [command, args] = firstValidationRunCall();
    const req =
      args.req && typeof args.req === "object"
        ? (args.req as Record<string, unknown>)
        : {};
    expect(command).toBe("run_double_ma_validation");
    expect(req.data_id).toBe("data-123");
    expect(req.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req.max_candidates).toBe(32);
    expect(req.admission_policy).toBe("legacy_topk");
    expect(req.admission_provenance).toMatchObject({
      requested_policy: "legacy_topk",
      applied_policy: "legacy_topk",
      source_kind: "top_rows",
      post_cap_count: 1,
    });
    expect(req.seed).toBe(1);
    expect(req.spa).toEqual({
      slices: 8,
      bootstrap_samples: 300,
      block_length: 4,
      benchmark: "buy_and_hold",
    });
    expect(req.reopt_walk_forward).toEqual({
      window_bars: 12000,
      step_bars: 3000,
      top_n_candidates: 5,
    });
    expect(req.cross_ticker).toEqual({
      max_datasets: 2,
      mode: "preserved_finalists",
      datasets: [{ csv_path: "C:/data/a.csv", label: "A" }],
    });
    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(Array.isArray(validationState.validationColumns)).toBe(true);
    expect(Array.isArray(validationState.validationRows)).toBe(true);
    expect(Number(validationState.tableRowCount || 0)).toBeGreaterThan(0);
    expect(String(validationState.validationReoptMeta || "")).toContain(
      "reopt_wf:",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "backend=Cpu",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "admission=legacy_topk",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "heads=legacy_topk",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "route=GpuKernel->CpuOnly",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "fallback=GPU validation runtime not implemented yet; using CPU validation.",
    );
    expect(String(validationState.validationAccelerationStatus || "")).toBe(
      "GPU validation unavailable: GPU validation runtime not implemented yet; using CPU validation.",
    );
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Cpu • GpuKernel->CpuOnly • fallback: GPU validation runtime not implemented yet; using CPU validation.",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "GPU validation runtime not implemented yet; using CPU validation.",
    );
    expect(String(validationState.validationAuxCpuSummary || "")).toContain(
      "none",
    );
    expect(String(validationState.validationBootstrapSummary || "")).toContain(
      "not run",
    );
    expect(String(validationState.validationCrossSummary || "")).toContain(
      "not run",
    );
    expect(String(validationState.validationSkippedSummary || "")).toContain(
      "Cross: unavailable",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "none",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "spa=0.170 fail",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "rec=evidence_too_short_min_track_record",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "evidence=prob raw_bars, cscv set-level",
    );
    const saveCall = invokeCommandMock.mock.calls.find(
      ([commandName]) => commandName === "save_run_artifact",
    );
    const savedArtifact = (
      saveCall?.[1] as { artifact?: Record<string, unknown> } | undefined
    )?.artifact;
    expect(savedArtifact?.kind).toBe("vectorgrid_run_artifact");
    expect(typeof savedArtifact?.config_fingerprint).toBe("string");
    expect(
      String(savedArtifact?.config_fingerprint || "").length,
    ).toBeGreaterThan(0);
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_run_artifact",
      expect.any(Object),
    );
    expect(
      (validationState.validationColumns as Array<Record<string, unknown>>).map(
        (column) => String(column.label || ""),
      ),
    ).toEqual(
      expect.arrayContaining([
        "Gate",
        "Robust Pareto",
        "Cross Pareto",
        "PSR",
        "DSR",
      ]),
    );
  });

  it.each([
    {
      label: "default stock costs",
      strategy: {
        fee_bps: 0.1,
        spread_bps: 1,
        slippage_bps: 0.5,
        commission: 0.00011,
      },
    },
    {
      label: "non-default custom costs",
      strategy: {
        fee_bps: 2.75,
        spread_bps: 4.5,
        slippage_bps: 1.25,
        commission: 0.000625,
      },
    },
  ])(
    "sends $label through validation with component bps as the sole cost source",
    async ({ strategy }) => {
      (pinia.state.value as Record<string, unknown>).validation = {
        candidates: "8",
        seed: "1",
        perturbEnable: true,
        perturbIters: "8",
        perturbMin: "0.015",
        perturbMax: "0.06",
      };
      useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
        currentRun: {
          ts: "2026-07-29T13:31:15Z",
          req: {
            strategy_kind: "DoubleMa",
            data_id: "data-cost-contract",
            backend: { backend: "CpuOnly" },
            ma_source: "close",
            fast_ma_types: ["sma"],
            slow_ma_types: ["ema"],
            strategy,
          },
          res: {
            top: [
              {
                params: {
                  fast_ma_type: "sma",
                  slow_ma_type: "ema",
                  fast_len: 10,
                  slow_len: 40,
                },
              },
            ],
          },
        },
        currentValidation: null,
      });

      await runValidation({ switchTab: false });

      const req = firstValidationRequest();
      const candidates = req.candidates as Array<Record<string, unknown>>;
      expect(candidates[0]?.strategy).toEqual({
        ...strategy,
        commission: 0,
      });
      expect(strategy.commission).not.toBe(0);
    },
  );

  it("sends additive Monte Carlo config and surfaces top-candidate distribution", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_double_ma_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          runtime_ms: 42,
          candidates: [
            {
              score: 1,
              robust_score: 1.2,
              robust_pareto: true,
              monte_carlo: {
                schema_version: 1,
                simulations: 400,
                trade_count: 18,
                tail_alpha: 0.05,
                modes: {
                  trade_iid: {
                    simulations: 400,
                    prob_pnl_positive: 0.82,
                    pnl_p05: -0.08,
                    pnl_tail_mean_p05: -0.11,
                    pnl_p50: 0.12,
                    pnl_p95: 0.34,
                    max_dd_p50: 0.12,
                    max_dd_p95: 0.28,
                    max_dd_tail_mean_p95: 0.31,
                    pnl_histogram: { min: -0.2, max: 0.4, counts: [1, 3, 6] },
                  },
                  trade_stationary: {
                    simulations: 400,
                    prob_pnl_positive: 0.76,
                    pnl_p05: -0.1,
                    pnl_tail_mean_p05: -0.14,
                    pnl_p50: 0.09,
                    pnl_p95: 0.29,
                    max_dd_p50: 0.14,
                    max_dd_p95: 0.33,
                    max_dd_tail_mean_p95: 0.37,
                    stationary_mean_block_len: 9,
                    pnl_histogram: { min: -0.3, max: 0.3, counts: [2, 5, 3] },
                  },
                },
                advisory: { passed: true, warning_flags: [], fail_reasons: [] },
              },
              trade_bootstrap: {
                simulations: 400,
                trade_count: 18,
                prob_pnl_positive: 0.82,
                pnl_p05: -0.08,
                pnl_p50: 0.12,
                max_dd_p95: 0.28,
              },
            },
          ],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_mc.json",
          bytes: 256,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: true,
      mcSims: "400",
      mcMode: "compare",
      mcBlockLen: "9",
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-26T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-mc",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const req = firstValidationRequest();
    expect(req.trade_bootstrap).toEqual({
      simulations: 400,
      stationary_mean_block_len: 9,
    });
    expect(req.monte_carlo).toEqual({
      schema_version: 1,
      simulations: 400,
      modes: ["trade_iid", "trade_stationary"],
      stationary_mean_block_len: 9,
      distribution_bins: 40,
      tail_alpha: 0.05,
    });
    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationAccelerationStatus || "")).toBe(
      "GPU validation completed with CPU follow-up checks.",
    );
    expect(String(validationState.validationMonteCarloSummary || "")).toContain(
      "stationary",
    );
    expect(String(validationState.validationMonteCarloSummary || "")).toContain(
      "p_pos=76.0%",
    );
    expect(validationState.validationMonteCarloHistogram).toEqual([
      expect.objectContaining({ count: 2 }),
      expect.objectContaining({ count: 5 }),
      expect.objectContaining({ count: 3 }),
    ]);
  });

  it("previews legacy and improved admission against the same source artifact", () => {
    const dataStore = useDataStore(pinia);
    const validationStore = useValidationStore(pinia);

    dataStore.csvPath = "C:/data/aapl.csv";
    dataStore.dataTimeframe = "1m";
    validationStore.candidates = "2";
    validationStore.admissionPolicy = "scalar_union";

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T10:00:00Z",
        csv_path: "C:/data/aapl.csv",
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-admission-preview",
          objective: "Sharpe",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
              metrics: {
                pnl: 0.15,
                sharpe: 1.8,
                calmar: 0.7,
                omega: 1.02,
                max_dd: 0.32,
              },
            },
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 12,
                slow_len: 48,
              },
              metrics: {
                pnl: 0.13,
                sharpe: 1.4,
                calmar: 0.85,
                omega: 1.08,
                max_dd: 0.24,
              },
            },
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 8,
                slow_len: 26,
              },
              metrics: {
                pnl: 0.1,
                sharpe: 0.9,
                calmar: 1.6,
                omega: 1.35,
                max_dd: 0.06,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationAdmissionComparisonFromStores();

    expect(preview.summary).toBe(
      "scalar_union vs legacy • overlap=1 • current-only=1 • legacy-only=1",
    );
    expect(preview.detail).toContain("current=2");
    expect(preview.detail).toContain("legacy=2");
    expect(preview.detail).toContain("heads=Sharpe/MaxDrawdown/Omega/Calmar");
  });

  it("previews shortlist admission from the artifact even when no dataset is currently loaded", () => {
    const validationStore = useValidationStore(pinia);
    validationStore.candidates = "2";
    validationStore.admissionPolicy = "scalar_union";

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T10:30:00Z",
        csv_path: "C:/data/aapl.csv",
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-admission-preview-no-dataset",
          objective: "Sharpe",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
              metrics: {
                pnl: 0.15,
                sharpe: 1.8,
                calmar: 0.7,
                omega: 1.02,
                max_dd: 0.32,
              },
            },
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 8,
                slow_len: 26,
              },
              metrics: {
                pnl: 0.1,
                sharpe: 0.9,
                calmar: 1.6,
                omega: 1.35,
                max_dd: 0.06,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationAdmissionComparisonFromStores();

    expect(preview.summary).not.toBe("Validation preview unavailable.");
    expect(preview.summary).toContain("scalar_union");
  });

  it("previews explicit compatible validation context when dataset, timeframe, and strategy match", () => {
    const dataStore = useDataStore(pinia);
    dataStore.csvPath = "C:/data/AAPL.csv";
    dataStore.dataTimeframe = "15m";
    dataStore.baseDataMeta = { ...baseDataMeta };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T10:30:00Z",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "15m",
          label: "AAPL",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-aapl-compatible",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationContextCompatibilityFromStores();

    expect(preview.state).toBe("compatible");
    expect(preview.summary).toBe(
      "Compatible: dataset + timeframe + strategy aligned",
    );
    expect(preview.detail).toContain("AAPL");
    expect(preview.detail).toContain("15m");
    expect(preview.detail).toContain("DoubleMa");
  });

  it("previews explicit incompatible validation context when the current dataset mismatches the optimization artifact", () => {
    const dataStore = useDataStore(pinia);
    dataStore.csvPath = "C:/data/MSFT.csv";
    dataStore.dataTimeframe = "15m";
    dataStore.baseDataMeta = { ...baseDataMeta };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T10:45:00Z",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "15m",
          label: "AAPL",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-aapl-incompatible",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationContextCompatibilityFromStores();

    expect(preview.state).toBe("incompatible");
    expect(preview.summary).toBe("Incompatible: dataset mismatch");
    expect(preview.detail).toContain("Validation artifact expects dataset");
  });

  it("reports partial validation context compatibility for thin legacy artifacts without dataset metadata", () => {
    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T10:50:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-thin-artifact",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationContextCompatibilityFromStores();

    expect(preview.state).toBe("warning");
    expect(preview.summary).toBe("Partial: missing dataset + timeframe");
    expect(preview.detail).toContain("missing dataset, timeframe metadata");
  });

  it("prefers the saved validation request strategy context when it mismatches the current run", () => {
    const dataStore = useDataStore(pinia);
    dataStore.csvPath = "C:/data/AAPL.csv";
    dataStore.dataTimeframe = "15m";
    dataStore.baseDataMeta = { ...baseDataMeta };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T10:55:00Z",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "15m",
          label: "AAPL",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-aapl-compatible",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: "2026-04-04T11:00:00Z",
        req: {
          validation_context: {
            strategy_kind: "CompositeMo",
            csv_path: "C:/data/AAPL.csv",
            timeframe: "15m",
            label: "AAPL",
          },
        },
        res: {},
      },
    });

    const preview = previewValidationContextCompatibilityFromStores();

    expect(preview.state).toBe("incompatible");
    expect(preview.summary).toBe("Incompatible: strategy mismatch");
    expect(preview.detail).toContain("CompositeMo");
    expect(preview.detail).toContain("DoubleMa");
  });

  it("reports requested-policy fallback when preview admission downgrades to legacy", () => {
    const validationStore = useValidationStore(pinia);
    validationStore.candidates = "2";
    validationStore.admissionPolicy = "pareto_frontier";

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T11:00:00Z",
        csv_path: "C:/data/aapl.csv",
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-admission-preview-fallback",
          objective: "Sharpe",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
              metrics: {
                pnl: 0.15,
                sharpe: 1.8,
                calmar: 0.7,
                omega: 1.02,
                max_dd: 0.32,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationAdmissionComparisonFromStores();

    expect(preview.summary).toBe("pareto_frontier -> legacy_topk fallback");
    expect(preview.detail).toContain(
      "pareto_frontier_fallback_missing_frontier",
    );
  });

  it("surfaces skipped-module and run-note metadata in validation summary text", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_double_ma_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          candidates: [],
          runtime_ms: 88,
          cscv_pbo: {
            skipped_reason: "cscv_skipped_not_enough_bars",
          },
          spa: {
            skipped_reason: "spa_skipped_not_enough_slices",
          },
          run_verdict: {
            passed: false,
            approved: false,
            candidates_tested: 4,
            candidates_survived_hard_gates: 0,
            approved_candidate_count: 0,
            final_recommendation: "no_candidate_survived",
            warning_flags: ["cross_ticker_filtered_all_candidates"],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_skipped.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      reoptWfEnable: true,
      reoptWfWindow: "8000",
      reoptWfStep: "2000",
      reoptWfTopN: "4",
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: true,
      mcSims: "500",
      cscvEnable: true,
      spaEnable: true,
      crossEnable: true,
      crossMode: "preserved_finalists",
      crossMax: "1",
      crossDatasets: [
        {
          path: "C:/data/SPY.csv",
          label: "SPY",
          meta: "Stocks • 1h",
          checked: true,
          disabled: false,
        },
      ],
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-13T00:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-skipped-meta",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationHint || "")).toContain(
      "Validation complete with warnings:",
    );
    expect(String(validationState.validationHint || "")).toContain(
      "CSCV skipped",
    );
    expect(String(validationState.validationHint || "")).toContain(
      "SPA skipped",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "cscv skipped=cscv_skipped_not_enough_bars",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "spa skipped=spa_skipped_not_enough_slices",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "note=cross_ticker_filtered_all_candidates",
    );
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "aux CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=auto)",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain("none");
    expect(String(validationState.validationAuxCpuSummary || "")).toContain(
      "perturb + trade bootstrap compare (iid + stationary, stat block=auto)",
    );
    expect(String(validationState.validationBootstrapSummary || "")).toContain(
      "iid + stationary closed-trade compare • stat block=auto",
    );
    expect(String(validationState.validationCrossSummary || "")).toContain(
      "not run",
    );
    expect(Number(validationState.tableRowCount || 0)).toBe(0);
    expect(Boolean(validationState.tableVisible)).toBe(false);
    expect(String(validationState.validationSkippedSummary || "")).toContain(
      "Cross: unavailable",
    );
    expect(String(validationState.validationSkippedSummary || "")).toContain(
      "Cross reruns: omit walk_forward + reopt WF + SPA + perturb + trade bootstrap",
    );
    expect(String(validationState.validationSkippedSummary || "")).toContain(
      "Reopt WF: unavailable",
    );
    expect(String(validationState.validationSkippedSummary || "")).toContain(
      "CSCV: cscv_skipped_not_enough_bars",
    );
    expect(String(validationState.validationSkippedSummary || "")).toContain(
      "SPA: spa_skipped_not_enough_slices",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "cross_ticker_filtered_all_candidates",
    );
  });

  it("surfaces intentional aux CPU tails in post-run validation warnings", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_double_ma_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          runtime_ms: 77,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_aux_cpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "-10",
      perturbMax: "10",
      mcEnable: true,
      mcSims: "200",
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-double-aux-cpu",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["alma"],
          slow_ma_types: ["alma"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "aux CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=auto)",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "Aux CPU tails are intentional in the current validation route",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "perturb + trade bootstrap compare (iid + stationary, stat block=auto)",
    );
  });

  it("surfaces regime and friction summaries for the top validation finalist", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_double_ma_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          candidates: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
              validation_verdict: {
                passed: true,
                warning_flags: [],
                fail_reasons: [],
              },
              regime_slice_robustness: {
                worst_bucket_label: "range",
                worst_sharpe: -0.08,
                worst_max_dd: 0.22,
                stable_bucket_count: 3,
                stable_bucket_frac: 0.6,
                support_label: "trend_only",
                trend: { pnl: 0.12, sharpe: 0.41 },
                range: { pnl: -0.02, sharpe: -0.08 },
              },
              friction_stress: {
                pass_frac: 0.5,
                max_rank_drift: 2,
                worst_scenario_label: "spread_stress",
              },
            },
          ],
          runtime_ms: 91,
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            first_approved_rank: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_regime_friction.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "1",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T01:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-regime-friction",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["alma"],
          slow_ma_types: ["alma"],
          strategy: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRegimeSummary || "")).toContain(
      "selective",
    );
    expect(String(validationState.validationRegimeSummary || "")).toContain(
      "support=trend_only",
    );
    expect(String(validationState.validationRegimeSummary || "")).toContain(
      "worst=range",
    );
    expect(String(validationState.validationRegimeSummary || "")).toContain(
      "stable=3/5 (60%)",
    );
    expect(String(validationState.validationFrictionSummary || "")).toContain(
      "fragile",
    );
    expect(String(validationState.validationFrictionSummary || "")).toContain(
      "worst=spread_stress",
    );
    expect(String(validationState.validationFrictionSummary || "")).toContain(
      "absolute pnl/sharpe rule",
    );
  });

  it("routes composite validation kinds through run_composite_validation", async () => {
    const strategyKinds = ["Kqsr", "Carm", "Kvdf", "Cdmc"];
    for (const strategyKind of strategyKinds) {
      invokeCommandMock.mockClear();
      installDefaultInvokeCommandMock();
      (pinia.state.value as Record<string, unknown>).validation = {
        candidates: "8",
        seed: "1",
        crossEnable: false,
      };
      useWorkflowArtifactsStore(pinia).clearArtifacts();
      useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
        currentRun: {
          ts: "2026-02-25T12:00:00Z",
          req: {
            strategy_kind: strategyKind,
            data_id: `data-${strategyKind.toLowerCase()}`,
            backend: { backend: "CpuOnly" },
            ma_source: "close",
            strategy: { fee_bps: 1.0 },
          },
          res: {
            top: [
              {
                params: {
                  fast_ma_type: "ema",
                  slow_ma_type: "sma",
                  fast_len: 10,
                  slow_len: 40,
                  [strategyKind.toLowerCase()]: { alpha: 1.0 },
                },
              },
            ],
          },
        },
        currentValidation: null,
      });

      await runValidation({ switchTab: false });

      const [command] = firstValidationRunCall();
      expect(
        command,
        `strategy ${strategyKind} should use composite validation`,
      ).toBe("run_composite_validation");
    }
  });

  it("routes composite double-MA profiles through run_double_ma_validation", async () => {
    const strategyKinds = [
      "CompositeTrend",
      "RegimeHybrid",
      "BreakoutMomentum",
    ];
    for (const strategyKind of strategyKinds) {
      invokeCommandMock.mockClear();
      installDefaultInvokeCommandMock();
      (pinia.state.value as Record<string, unknown>).validation = {
        candidates: "8",
        seed: "1",
        crossEnable: false,
      };
      useWorkflowArtifactsStore(pinia).clearArtifacts();
      useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
        currentRun: {
          ts: "2026-02-25T12:00:00Z",
          req: {
            strategy_kind: strategyKind,
            data_id: `data-${strategyKind.toLowerCase()}`,
            backend: { backend: "GpuHybridKernel", device_id: 0 },
            ma_source: "close",
            fast_ma_types: ["ema"],
            slow_ma_types: ["sma"],
            strategy: { fee_bps: 1.0 },
          },
          res: {
            top: [
              {
                params: {
                  fast_ma_type: "ema",
                  slow_ma_type: "sma",
                  fast_len: 10,
                  slow_len: 40,
                },
              },
            ],
          },
        },
        currentValidation: null,
      });

      await runValidation({ switchTab: false });

      const [command] = firstValidationRunCall();
      expect(
        command,
        `strategy ${strategyKind} should use double-MA validation`,
      ).toBe("run_double_ma_validation");
    }
  });

  it("rejects unknown validation strategy kinds before invoking a validation command", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "1",
      crossEnable: false,
    };
    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DefinitelyUnknown",
          data_id: "data-unknown",
          backend: { backend: "CpuOnly" },
          ma_source: "close",
          strategy: { fee_bps: 1.0 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await expect(runValidation({ switchTab: false })).rejects.toThrow(
      "Unsupported validation strategy_kind 'DefinitelyUnknown'",
    );
    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) =>
          command === "run_double_ma_validation" ||
          command === "run_composite_validation",
      ),
    ).toBe(false);
  });

  it("surfaces composite lightweight cross rerun semantics in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          runtime_ms: 111,
          cross_ticker: {
            mode: "preserved_finalists",
            datasets: [{ label: "SPY" }, { label: "QQQ" }],
          },
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 2,
            candidates_survived_hard_gates: 2,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_composite_cross.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: true,
      crossMode: "preserved_finalists",
      crossMax: "2",
      crossDatasets: [
        {
          path: "C:/data/SPY.csv",
          label: "SPY",
          checked: true,
          disabled: false,
        },
        {
          path: "C:/data/QQQ.csv",
          label: "QQQ",
          checked: true,
          disabled: false,
        },
      ],
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "Mkcs",
          data_id: "data-composite-post-run",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          strategy: { fee_bps: 1.0 },
          mkcs: { compression_threshold: 0.01, reexpansion_threshold: 0.03 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 36,
                mkcs: { stddev_period: 20, breakout_lookback: 30 },
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Gpu • GpuKernel",
    );
    expect(String(validationState.validationCrossSummary || "")).toContain(
      "requested=preserved_finalists",
    );
    expect(String(validationState.validationCrossSummary || "")).toContain(
      "finalists only (cap=4)",
    );
    expect(String(validationState.validationSkippedSummary || "")).toContain(
      "Cross reruns: preserved_finalists omit walk_forward + reopt WF + SPA + perturb + trade bootstrap",
    );
  });

  it("surfaces CompositeMo CPU-preferred validation fallback explicitly in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Cpu",
          backend_route: "CpuOnly",
          runtime_ms: 88,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_composite_mo_cpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "CompositeMo",
          data_id: "data-composite-mo-post-run",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          strategy: { fee_bps: 1.0 },
          signal_mode: "hard_gate",
          halftrend: {},
          macz: {},
          ott: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 36,
                signal_mode: "hard_gate",
                strategy: { fee_bps: 1.0 },
                halftrend: {},
                macz: {},
                ott: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Cpu • CpuOnly • route note: CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract (uniform strategy/indicator settings, vectorized engine, no inline cross_ticker).",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract (uniform strategy/indicator settings, vectorized engine, no inline cross_ticker).",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract (uniform strategy/indicator settings, vectorized engine, no inline cross_ticker).",
    );
  });

  it("surfaces plain CompositeCpu GPU route notes explicitly in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          runtime_ms: 88,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_composite_cpu_gpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "CompositeCpu",
          data_id: "data-composite-cpu-post-run",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          strategy: { fee_bps: 1.0 },
          signal_mode: "hard_gate",
          halftrend: { amplitude: 2, channel_deviation: 2.0, atr_period: 100 },
          macz: {},
          ott: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 36,
                signal_mode: "hard_gate",
                strategy: { fee_bps: 1.0 },
                macz: {},
                ott: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Gpu • GpuKernel • route note: Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists; mixed finalist sets can fall back to CPU.",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists; mixed finalist sets can fall back to CPU.",
    );
    expect(String(validationState.validationWarningSummary || "")).toBe("none");
  });

  it("surfaces Carm GPU validation route detail explicitly in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          runtime_ms: 88,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_carm_gpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "Carm",
          data_id: "data-carm-post-run",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          signal_mode: "hard_gate",
          halftrend: {},
          macz: {},
          ott: {},
          carm: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 36,
                signal_mode: "hard_gate",
                halftrend: {},
                macz: {},
                ott: {},
                carm: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Gpu • GpuKernel",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "CARM GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route.",
    );
    expect(String(validationState.validationCategorySummary || "")).toContain(
      "GPU-native: base reruns + hard gates",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "none",
    );
  });

  it("surfaces Carm Auto CPU-preferred policy explicitly in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Cpu",
          backend_route: "Auto->CpuOnly",
          backend_fallback_reason:
            "Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on measured workloads.",
          runtime_ms: 41,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_carm_auto_cpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "Carm",
          data_id: "data-carm-auto-post-run",
          backend: { backend: "Auto", device_id: 0 },
          ma_source: "close",
          signal_mode: "hard_gate",
          halftrend: {},
          macz: {},
          ott: {},
          carm: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 36,
                signal_mode: "hard_gate",
                halftrend: {},
                macz: {},
                ott: {},
                carm: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Cpu • Auto->CpuOnly • route note: Auto composite validation currently prefers CPU for CARM",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain("Auto composite validation currently prefers CPU for CARM");
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "none",
    );
  });

  it("surfaces Cdmc Auto GPU route explicitly in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "Auto->GpuKernel",
          runtime_ms: 284,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_cdmc_auto_gpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "Cdmc",
          data_id: "data-cdmc-auto-post-run",
          backend: { backend: "Auto", device_id: 0 },
          ma_source: "close",
          signal_mode: "hard_gate",
          halftrend: {},
          macz: {},
          ott: {},
          cdmc: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 36,
                signal_mode: "hard_gate",
                halftrend: {},
                macz: {},
                ott: {},
                cdmc: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Gpu • Auto->GpuKernel",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "CDMC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route.",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "none",
    );
  });

  it("previews validation execution summary from current workflow artifacts", () => {
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "1h",
      baseDataMeta: { timeframe: "1h" },
    };
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "12",
      seed: "1",
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      reoptWfEnable: true,
      reoptWfWindow: "12000",
      reoptWfStep: "3000",
      reoptWfTopN: "4",
      spaEnable: true,
      spaSlices: "12",
      spaBootstrapSamples: "300",
      spaBlockLength: "4",
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: true,
      mcSims: "500",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: true,
      crossMax: "2",
      crossMode: "preserved_finalists",
      crossDatasets: [
        { path: "C:/data/SPY.csv", label: "SPY", checked: true },
        { path: "C:/data/QQQ.csv", label: "QQQ", checked: true },
      ],
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        csvPath: "C:/data/AAPL.csv",
        datasetMeta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "aapl_1h",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const summary = previewValidationExecutionSummaryFromStores();
    expect(summary.routeSummary).toContain("GpuKernel requested");
    expect(summary.crossSummary).toContain("2 datasets");
    expect(summary.crossSummary).toContain("finalists only (cap=4)");
    expect(summary.moduleSummary).toContain(
      "auxiliary CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=auto)",
    );
    expect(summary.categorySummary).toContain(
      "GPU-native: base reruns + hard gates + walk_forward + reopt_walk_forward + cscv_pbo + spa",
    );
    expect(summary.categorySummary).toContain(
      "GPU main + CPU tail: perturb + trade_bootstrap",
    );
    expect(summary.skippedSummary).toContain(
      "Cross reruns: preserved-finalists omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap",
    );
    expect(summary.warningSummary).toContain(
      "Preserved-finalists cross reruns keep CSCV/PBO + hard gates",
    );
  });

  it("surfaces Cdmc GPU validation route detail explicitly in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          runtime_ms: 88,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_cdmc_gpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "4",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-14T00:00:00Z",
        req: {
          strategy_kind: "Cdmc",
          data_id: "data-cdmc-post-run",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          signal_mode: "hard_gate",
          halftrend: {},
          macz: {},
          ott: {},
          cdmc: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 36,
                signal_mode: "hard_gate",
                halftrend: {},
                macz: {},
                ott: {},
                cdmc: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Gpu • GpuKernel",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "CDMC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route.",
    );
    expect(String(validationState.validationCategorySummary || "")).toContain(
      "GPU-native: base reruns + hard gates",
    );
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "none",
    );
  });

  it("distinguishes GPU route notes from true CPU fallback in validation summary text", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_double_ma_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuHybridKernel->GpuKernel",
          backend_fallback_reason:
            "GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.",
          candidates: [{ score: 1 }],
          runtime_ms: 91,
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_route_note.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "1",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-13T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-route-note",
          backend: { backend: "GpuHybridKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["sma"],
          strategy: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "sma",
                fast_len: 10,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationMeta || "")).toContain(
      "route=GpuHybridKernel->GpuKernel",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "route_note=GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.",
    );
    expect(String(validationState.validationMeta || "")).not.toContain(
      "fallback=GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.",
    );
    expect(String(validationState.validationHint || "")).toContain(
      "route note: GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.",
    );
    expect(String(validationState.validationAccelerationStatus || "")).toBe(
      "GPU validation completed.",
    );
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Gpu • GpuHybridKernel->GpuKernel • route note: GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.",
    );
    expect(String(validationState.validationCategorySummary || "")).toContain(
      "GPU-native: base reruns + hard gates",
    );
    expect(String(validationState.validationAuxCpuSummary || "")).toBe("none");
    expect(String(validationState.validationBootstrapSummary || "")).toBe(
      "not run",
    );
  });

  it("surfaces iid-only bootstrap mode in preview summaries", () => {
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "1h",
      baseDataMeta: { timeframe: "1h" },
    };
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "12",
      seed: "1",
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      reoptWfEnable: true,
      reoptWfWindow: "12000",
      reoptWfStep: "3000",
      reoptWfTopN: "4",
      spaEnable: true,
      spaSlices: "12",
      spaBootstrapSamples: "300",
      spaBlockLength: "4",
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: true,
      mcSims: "400",
      mcBlockLen: "9",
      mcMode: "iid_only",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: true,
      crossMax: "2",
      crossMode: "preserved_finalists",
      crossDatasets: [
        { path: "C:/data/SPY.csv", label: "SPY", checked: true },
        { path: "C:/data/QQQ.csv", label: "QQQ", checked: true },
      ],
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        csvPath: "C:/data/AAPL.csv",
        datasetMeta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "aapl_1h",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const summary = previewValidationExecutionSummaryFromStores();
    expect(summary.moduleSummary).toContain("trade bootstrap iid-only");
    expect(summary.categorySummary).toContain(
      "GPU main + CPU tail: perturb + trade_bootstrap",
    );
  });

  it("previews validation context compatibility when the loaded dataset matches the artifact", () => {
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "1h",
      datasetLabel: "AAPL",
      baseDataMeta: { timeframe: "1h" },
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-09T12:00:00Z",
        csvPath: "C:/data/AAPL.csv",
        datasetMeta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "aapl_1h",
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationContextCompatibilityFromStores();
    expect(preview.state).toBe("compatible");
    expect(preview.summary).toContain("Compatible");
    expect(preview.detail).toContain("matches the artifact context");
  });

  it("previews validation context incompatibility when the loaded timeframe mismatches the artifact", () => {
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "15m",
      datasetLabel: "AAPL",
      baseDataMeta: { timeframe: "15m" },
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-09T12:00:00Z",
        csvPath: "C:/data/AAPL.csv",
        datasetMeta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "aapl_1h",
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationContextCompatibilityFromStores();
    expect(preview.state).toBe("incompatible");
    expect(preview.summary).toBe("Incompatible: timeframe mismatch");
    expect(preview.detail).toContain("expects timeframe 1h");
  });

  it("previews partial compatibility when artifact metadata is thin", () => {
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "1h",
      datasetLabel: "AAPL",
      baseDataMeta: { timeframe: "1h" },
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-09T12:00:00Z",
        csvPath: "",
        datasetMeta: null,
        req: {},
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const preview = previewValidationContextCompatibilityFromStores();
    expect(preview.state).toBe("warning");
    expect(preview.summary).toBe(
      "Partial: missing dataset + timeframe + strategy",
    );
    expect(preview.detail).toContain("cannot be fully confirmed");
  });

  it("classifies VCET validation as GPU-native in preview summaries", () => {
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "1h",
      baseDataMeta: { timeframe: "1h" },
    };
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "1",
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      reoptWfEnable: true,
      reoptWfWindow: "12000",
      reoptWfStep: "3000",
      reoptWfTopN: "4",
      spaEnable: true,
      spaSlices: "12",
      spaBootstrapSamples: "300",
      spaBlockLength: "4",
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: true,
      mcSims: "400",
      mcBlockLen: "",
      mcMode: "compare",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T12:00:00Z",
        csvPath: "C:/data/AAPL.csv",
        datasetMeta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "Vcet",
          data_id: "aapl_1h_vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          signal_mode: "hard_gate",
          vcet: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
                signal_mode: "hard_gate",
                vcet: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const summary = previewValidationExecutionSummaryFromStores();
    expect(summary.categorySummary).toContain(
      "GPU-native: base reruns + hard gates + walk_forward + reopt_walk_forward + cscv_pbo + spa + perturb + trade_bootstrap",
    );
    expect(summary.categorySummary).not.toContain("GPU main + CPU tail:");
    expect(summary.routeSummary).toContain(
      "expected GPU composite route (VCET)",
    );
    expect(summary.moduleSummary).toContain("perturb");
    expect(summary.moduleSummary).toContain(
      "trade bootstrap compare (iid + stationary, stat block=auto)",
    );
    expect(summary.warningSummary).toContain(
      "VCET GPU validation now supports the current fixed-candidate validation module stack on the GpuKernel route",
    );
    expect(summary.warningSummary).not.toContain(
      "may not share the same execution route",
    );
  });

  it("surfaces VCET GPU-native route detail explicitly in post-run summaries", async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Gpu",
          backend_route: "GpuKernel",
          runtime_ms: 82,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_vcet_gpu.json",
          bytes: 128,
        };
      }
      return {};
    });
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "1",
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      reoptWfEnable: true,
      reoptWfWindow: "12000",
      reoptWfStep: "3000",
      reoptWfTopN: "4",
      spaEnable: true,
      spaSlices: "12",
      spaBootstrapSamples: "300",
      spaBlockLength: "4",
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: true,
      mcSims: "400",
      mcBlockLen: "",
      mcMode: "compare",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: false,
    };
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "1h",
      baseDataMeta: { timeframe: "1h" },
      datasetLabel: "AAPL",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T12:00:00Z",
        csvPath: "C:/data/AAPL.csv",
        datasetMeta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "Vcet",
          data_id: "aapl_1h_vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          signal_mode: "hard_gate",
          vcet: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 9,
                slow_len: 30,
                signal_mode: "hard_gate",
                vcet: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Gpu • GpuKernel",
    );
    expect(
      String(validationState.validationRouteDetailSummary || ""),
    ).toContain(
      "VCET GPU validation now supports the current fixed-candidate validation module stack on the GpuKernel route",
    );
    expect(String(validationState.validationCategorySummary || "")).toContain(
      "GPU-native: base reruns + hard gates + walk_forward + reopt_walk_forward + cscv_pbo + spa + perturb + trade_bootstrap",
    );
    expect(
      String(validationState.validationCategorySummary || ""),
    ).not.toContain("GPU main + CPU tail");
    expect(String(validationState.validationAuxCpuSummary || "")).toBe("none");
    expect(String(validationState.validationWarningSummary || "")).toContain(
      "none",
    );
  });

  it("reports validation timing truthfully and persists auto-export artifacts once", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "12",
      seed: "1",
      minPsr: "",
      minDsr: "",
      autoExportXlsx: true,
      autoRevealReport: false,
      autoExportTopN: "7",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-auto-export",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx",
      expect.objectContaining({
        max_rows: 7,
        report: expect.any(Object),
      }),
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "save_report_xlsx_from_artifact",
      expect.anything(),
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "build_report_xlsx",
      expect.anything(),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith("open_path", {
      path: "runs/vectorgrid_report_validation_test.xlsx",
    });
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "reveal_path",
      expect.anything(),
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "save_run_artifact",
      ),
    ).toHaveLength(1);

    const workflowStore = useWorkflowArtifactsStore(pinia);
    expect(workflowStore.currentRun?.reportXlsxPath).toBe(
      "runs/vectorgrid_report_validation_test.xlsx",
    );
    const saveRunCall = invokeCommandMock.mock.calls.find(
      ([command]) => command === "save_run_artifact",
    );
    const savedArtifact = (
      saveRunCall?.[1] as { artifact?: Record<string, unknown> } | undefined
    )?.artifact;
    expect(savedArtifact?.stage_completions).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ stage: "optimization" }),
        expect.objectContaining({
          stage: "validation",
          report_xlsx_path: "runs/vectorgrid_report_validation_test.xlsx",
        }),
      ]),
    );
    expect(workflowStore.currentRun?.stageCompletions).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ stage: "optimization" }),
        expect.objectContaining({
          stage: "validation",
          reportXlsxPath: "runs/vectorgrid_report_validation_test.xlsx",
        }),
      ]),
    );

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.statusLine || "")).toMatch(
      /^Validation completed in \d+ms end-to-end \(\d+ms command\)\. Saved XLSX report: runs\/vectorgrid_report_validation_test\.xlsx$/,
    );
    const perfEntries = readValidationReportPerfEntries();
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.validationCommandMs",
      )?.meta,
    ).toMatchObject({
      command: "run_double_ma_validation",
      outcome: "ok",
    });
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.autoExportReportXlsxMs",
      )?.meta,
    ).toMatchObject({
      returnedPath: true,
      outcome: "ok",
    });
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.exportReportXlsxSaveMs",
      )?.meta,
    ).toMatchObject({
      branch: "direct",
      outcome: "ok",
    });
    expect(
      perfEntries.find(
        (entry) =>
          entry.name === "validationService.persistValidationArtifactMs",
      )?.meta,
    ).toMatchObject({
      persistedPathReturned: true,
      outcome: "ok",
    });
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.runValidationEndToEndMs",
      )?.meta,
    ).toMatchObject({
      command: "run_double_ma_validation",
      autoExport: true,
      returnedReportXlsxPath: true,
    });
    expect(String(validationState.statusLine || "")).not.toContain(
      "artifact report path persistence failed",
    );
  });

  it("surfaces artifact update warnings from auto-export XLSX status", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "12",
      seed: "1",
      minPsr: "",
      minDsr: "",
      autoExportXlsx: true,
      autoRevealReport: false,
      autoExportTopN: "7",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-auto-export-warning",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const fallbackInvoke = invokeCommandMock.getMockImplementation();
    const callFallbackInvoke = fallbackInvoke as
      | ((command: string, args?: unknown) => unknown)
      | undefined;
    invokeCommandMock.mockImplementation(
      async (command: string, args?: unknown) => {
        if (command === "save_report_xlsx") {
          return {
            path: "runs/vectorgrid_report_validation_warning.xlsx",
            artifact_updated: false,
            artifact_update_warning:
              "artifact report path persistence failed: report saved but run artifact was not updated",
          };
        }
        return callFallbackInvoke ? callFallbackInvoke(command, args) : {};
      },
    );

    await runValidation({ switchTab: false });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.statusLine || "")).toContain(
      "Saved XLSX report: runs/vectorgrid_report_validation_warning.xlsx",
    );
    expect(String(validationState.statusLine || "")).toContain(
      "artifact report path persistence failed: report saved but run artifact was not updated",
    );

    const perfEntries = readValidationReportPerfEntries();
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.exportReportXlsxTotalMs",
      )?.meta,
    ).toMatchObject({
      artifactUpdateWarning: true,
      outcome: "ok",
    });
  });

  it("reuses artifact-backed dataset metadata when persisting validation artifacts", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "2",
      minPsr: "",
      minDsr: "",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/AAPL.csv",
      dataTimeframe: "1h",
      baseDataMeta: { timeframe: "1h" },
      datasetLabel: "AAPL live",
      datasetContext: "source=live",
      dataFields: ["open", "close"],
      dataLen: "1000",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-09T18:00:00Z",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          base_timeframe: "1m",
          derived_timeframe: "1h",
          resample_applied: true,
          resample_rule: "session_relative_ohlcv",
          resample_anchor: "day_first_timestamp",
          label: "AAPL artifact",
          symbol: "AAPL",
          len: 4321,
          source: "artifact-source",
          source_size_bytes: 2048,
          source_mtime_utc_ms: 1700005000000,
          source_content_fingerprint: "fp-aapl-1m",
        },
        ml_signal: {
          stage: "stage5",
          model_summary: "cached model",
        },
        optimization_metric_surface: {
          surface: "core17",
          fields: ["pnl", "trades", "max_dd"],
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-aapl-artifact",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const saveCall = invokeCommandMock.mock.calls.find(
      ([command]) => command === "save_run_artifact",
    );
    const artifact = (
      saveCall?.[1] as { artifact?: Record<string, unknown> } | undefined
    )?.artifact;
    expect(artifact?.dataset_meta).toMatchObject({
      csv_path: "C:/data/AAPL.csv",
      timeframe: "1h",
      base_timeframe: "1m",
      derived_timeframe: "1h",
      resample_applied: true,
      resample_rule: "session_relative_ohlcv",
      resample_anchor: "day_first_timestamp",
      label: "AAPL artifact",
      symbol: "AAPL",
      len: 4321,
      source: "artifact-source",
      source_size_bytes: 2048,
      source_mtime_utc_ms: 1700005000000,
      source_content_fingerprint: "fp-aapl-1m",
    });
    expect(artifact?.ml_signal).toMatchObject({
      stage: "stage5",
    });
    expect(artifact?.optimization_metric_surface).toMatchObject({
      surface: "core17",
      fields: ["pnl", "trades", "max_dd"],
    });

    const workflowStore = useWorkflowArtifactsStore(pinia);
    expect(workflowStore.currentRun?.datasetMeta).toMatchObject({
      csv_path: "C:/data/AAPL.csv",
      timeframe: "1h",
      base_timeframe: "1m",
      derived_timeframe: "1h",
      resample_applied: true,
      resample_rule: "session_relative_ohlcv",
      resample_anchor: "day_first_timestamp",
      label: "AAPL artifact",
      symbol: "AAPL",
      len: 4321,
      source: "artifact-source",
      source_size_bytes: 2048,
      source_mtime_utc_ms: 1700005000000,
      source_content_fingerprint: "fp-aapl-1m",
    });
    expect(workflowStore.currentRun?.mlSignal).toMatchObject({
      stage: "stage5",
    });
    expect(workflowStore.currentRun?.optimizationMetricSurface).toMatchObject({
      surface: "core17",
      fields: ["pnl", "trades", "max_dd"],
    });
  });

  it("rejects validation when the current dataset does not match the optimization artifact", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "1",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/MSFT.csv",
      dataTimeframe: "15m",
      baseDataMeta: { timeframe: "15m" },
      datasetLabel: "MSFT",
      datasetContext: "source=csv",
      dataFields: ["open", "high", "low", "close", "volume"],
      dataLen: "1000",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-08T18:00:00Z",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "15m",
          label: "AAPL",
          symbol: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-aapl",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await expect(runValidation({ switchTab: false })).rejects.toThrow(
      "Validation requires the optimization dataset",
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "run_double_ma_validation",
      expect.anything(),
    );
  });

  it("routes ASR validation through composite validation command", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "3",
      minPsr: "",
      minDsr: "",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-01T00:00:00Z",
        req: {
          strategy_kind: "Asr",
          data_id: "asr-data",
          ma_source: "close",
          fast_ma_types: ["alma"],
          slow_ma_types: ["alma"],
          strategy: { fee_bps: 1.0 },
          asr: { angle_deg_min: 10, z_entry: 2.0 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "alma",
                slow_ma_type: "alma",
                fast_len: 12,
                slow_len: 48,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });
    const [command, args] = firstValidationRunCall();
    expect(command).toBe("run_composite_validation");
    const req =
      args.req && typeof args.req === "object"
        ? (args.req as Record<string, unknown>)
        : {};
    const candidates = Array.isArray(req.candidates)
      ? (req.candidates as Array<Record<string, unknown>>)
      : [];
    expect(candidates[0]?.asr).toMatchObject({
      angle_deg_min: 10,
      z_entry: 2.0,
    });
  });

  it("routes Mkcs validation through composite validation command", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "8",
      seed: "5",
      minPsr: "",
      minDsr: "",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-11T00:00:00Z",
        req: {
          strategy_kind: "Mkcs",
          data_id: "mkcs-data",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["ema"],
          slow_ma_types: ["sma"],
          strategy: { fee_bps: 1.25 },
          mkcs: { compression_threshold: 0.01, reexpansion_threshold: 0.03 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 12,
                slow_len: 48,
                mkcs: { stddev_period: 20, breakout_lookback: 30 },
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const [command, args] = firstValidationRunCall();
    expect(command).toBe("run_composite_validation");
    const req =
      args.req && typeof args.req === "object"
        ? (args.req as Record<string, unknown>)
        : {};
    const candidates = Array.isArray(req.candidates)
      ? (req.candidates as Array<Record<string, unknown>>)
      : [];
    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
    expect(candidates[0]?.mkcs).toEqual({
      stddev_period: 20,
      breakout_lookback: 30,
    });
  });

  it("prefers Vue validation store inputs when store source is ready", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      source: "vue",
      candidates: "9",
      seed: "7",
      minPsr: "",
      minDsr: "",
      wfEnable: false,
      wfWindow: "12345",
      wfStep: "2345",
      perturbEnable: false,
      perturbIters: "6",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: false,
      mcSims: "400",
      cscvEnable: false,
      cscvSlices: "",
      crossEnable: true,
      crossMax: "1",
      crossDatasets: [
        { path: "C:/data/from_store.csv", label: "FromStore", checked: true },
      ],
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-store-inputs",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });
    const fields: Record<string, { value?: string; checked?: boolean }> = {
      valCandidates: { value: "32" },
      valSeed: { value: "1" },
      valCrossEnable: { checked: false },
      valCrossMax: { value: "6" },
    };
    vi.stubGlobal("document", {
      getElementById: (id: string) => fields[id] ?? null,
    });
    await runValidation({ switchTab: false });

    const req = firstValidationRequest();
    expect(req.max_candidates).toBe(9);
    expect(req.seed).toBe(7);
    expect(req.cross_ticker).toEqual({
      max_datasets: 1,
      mode: "preserved_finalists",
      datasets: [{ csv_path: "C:/data/from_store.csv", label: "FromStore" }],
    });

    vi.unstubAllGlobals();
  });

  it("throws when build context is unavailable and no run snapshot exists", async () => {
    await expect(runValidation()).rejects.toThrow("Run an optimization first.");

    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "run_double_ma_validation",
      expect.anything(),
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "run_composite_validation",
      expect.anything(),
    );
  });

  it("prefers run snapshot top rows from workflow artifact store", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "16",
      seed: "2",
      minPsr: "",
      minDsr: "",
      wfEnable: false,
      wfWindow: "20000",
      wfStep: "20000",
      perturbEnable: false,
      perturbIters: "8",
      perturbMin: "0.015",
      perturbMax: "0.06",
      mcEnable: false,
      mcSims: "500",
      cscvEnable: false,
      cscvSlices: "",
      crossEnable: false,
      crossMax: "0",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-store-1",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 2 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 8,
                slow_len: 32,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });
    await runValidation({ switchTab: false });

    const req = firstValidationRequest();
    expect(req.data_id).toBe("data-store-1");
    expect(req.max_candidates).toBe(16);
  });

  it("builds validation candidates from tuple-shaped saved top rows", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "12",
      admissionPolicy: "scalar_union",
      seed: "2",
      minPsr: "",
      minDsr: "",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
      crossMax: "0",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-tuple-top",
          objective: "Sharpe",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 2 },
        },
        res: {
          top: [
            [
              {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 8,
                slow_len: 32,
              },
              {
                pnl: 0.12,
                sharpe: 1.1,
                calmar: 0.8,
                omega: 1.02,
                max_dd: 0.2,
              },
            ],
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const req = firstValidationRequest();
    expect(req.data_id).toBe("data-tuple-top");
    expect(req.admission_policy).toBe("scalar_union");
    expect(Array.isArray(req.candidates)).toBe(true);
    expect((req.candidates as Array<Record<string, unknown>>)[0]).toMatchObject(
      {
        fast_len: 8,
        slow_len: 32,
      },
    );
    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationMeta || "")).toContain(
      "admission=scalar_union",
    );
    expect(String(validationState.validationMeta || "")).toContain(
      "heads=Sharpe/MaxDrawdown",
    );
  });

  it("keeps validation successful when auto-export XLSX fails", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "10",
      seed: "2",
      minPsr: "",
      minDsr: "",
      autoExportXlsx: true,
      autoRevealReport: false,
      autoExportTopN: "5",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-auto-export-fail",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "save_report_xlsx") throw new Error("save failed");
      if (command === "build_report_xlsx") throw new Error("fallback failed");
      if (
        command === "run_double_ma_validation" ||
        command === "run_composite_validation"
      ) {
        return {
          backend_used: "Cpu",
          candidates: [{ score: 1 }],
          runtime_ms: 123,
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_test.json",
          bytes: 256,
        };
      }
      if (command === "open_path" || command === "reveal_path") return true;
      if (command === "polygon_get_cache_info") return { datasets: [] };
      return {};
    });

    await expect(runValidation({ switchTab: false })).resolves.toBeUndefined();

    const workflowStore = useWorkflowArtifactsStore(pinia);
    expect(workflowStore.currentValidation?.res).toBeTruthy();
    expect(workflowStore.currentRun?.reportXlsxPath).toBe("");
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx",
      expect.objectContaining({
        max_rows: 5,
        report: expect.any(Object),
      }),
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "save_report_xlsx_from_artifact",
      expect.anything(),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "build_report_xlsx",
      expect.objectContaining({
        max_rows: 5,
        report: expect.any(Object),
      }),
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "save_run_artifact",
      ),
    ).toHaveLength(1);

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.statusLine || "")).toContain(
      "Validation completed in",
    );
    expect(String(validationState.statusLine || "")).toContain(
      "Auto-export XLSX failed",
    );
    expect(String(validationState.statusLine || "")).toContain("save failed");
    const perfEntries = readValidationReportPerfEntries();
    expect(
      perfEntries.find(
        (entry) =>
          entry.name === "validationService.exportReportXlsxFallbackBuildMs",
      )?.meta,
    ).toMatchObject({
      branch: "fallback_build",
      outcome: "error",
    });
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.autoExportReportXlsxMs",
      )?.meta,
    ).toMatchObject({
      outcome: "error",
    });
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.runValidationEndToEndMs",
      )?.meta,
    ).toMatchObject({
      autoExport: true,
      returnedReportXlsxPath: false,
    });
  });

  it("keeps validation successful when artifact persistence fails", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "10",
      seed: "2",
      minPsr: "",
      minDsr: "",
      autoExportXlsx: false,
      autoRevealReport: false,
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/preserved.csv",
      dataTimeframe: "1m",
      datasetLabel: "Preserved",
      datasetContext: "source=file",
      dataFields: ["open", "high", "low", "close", "volume"],
      dataLen: "1000",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        artifact_path: "runs/vectorgrid_existing_validation_test.json",
        csv_path: "C:/data/preserved.csv",
        dataset_meta: {
          csv_path: "C:/data/preserved.csv",
          timeframe: "1m",
          label: "Preserved",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-persist-fail",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (
        command === "run_double_ma_validation" ||
        command === "run_composite_validation"
      ) {
        return {
          backend_used: "Cpu",
          candidates: [{ score: 1 }],
          runtime_ms: 123,
        };
      }
      if (command === "save_run_artifact") {
        throw new Error("artifact persistence failed");
      }
      if (command === "open_path" || command === "reveal_path") return true;
      if (command === "polygon_get_cache_info") return { datasets: [] };
      return {};
    });

    await expect(runValidation({ switchTab: false })).resolves.toBeUndefined();

    const workflowStore = useWorkflowArtifactsStore(pinia);
    expect(workflowStore.currentValidation?.res).toBeTruthy();
    expect(workflowStore.currentRun?.artifactPath).toBe(
      "runs/vectorgrid_existing_validation_test.json",
    );
    expect(workflowStore.currentRun?.datasetMeta).toMatchObject({
      csv_path: "C:/data/preserved.csv",
      timeframe: "1m",
      label: "Preserved",
    });
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "save_run_artifact",
      ),
    ).toHaveLength(1);
    const saveCall = invokeCommandMock.mock.calls.find(
      ([command]) => command === "save_run_artifact",
    );
    expect(saveCall?.[1]).toMatchObject({
      existing_path: "runs/vectorgrid_existing_validation_test.json",
    });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.statusLine || "")).toContain(
      "Validation completed in",
    );
    expect(String(validationState.statusLine || "")).toContain(
      "Validation artifact persistence failed:",
    );
  });

  it("falls back to a downloadable XLSX when direct save fails", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      autoRevealReport: false,
      autoExportTopN: "9",
    };
    useValidationStore(pinia);

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-10T12:34:56Z",
        artifact_path: "",
        report_xlsx_path: "",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          label: "AAPL artifact",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-export-direct-fallback",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: "2026-03-10T12:34:56Z",
        req: {
          data_id: "data-export-direct-fallback",
          candidates: [
            {
              fast_ma_type: "sma",
              slow_ma_type: "ema",
              fast_len: 10,
              slow_len: 40,
            },
          ],
        },
        res: {
          candidates: [{ score: 1 }],
        },
      },
    });

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "save_report_xlsx") throw new Error("save failed");
      if (command === "build_report_xlsx") return "AA==";
      if (command === "open_path" || command === "reveal_path") return true;
      return {};
    });

    await exportReportXlsx();

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx",
      expect.objectContaining({
        max_rows: 9,
        report: expect.any(Object),
      }),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "build_report_xlsx",
      expect.objectContaining({
        max_rows: 9,
        report: expect.any(Object),
      }),
    );
    expect([
      "Exported XLSX report as a file download.",
      "Built XLSX report (download unavailable in this environment).",
    ]).toContain(String(validationState.statusLine || ""));
  });

  it("reports when XLSX save succeeds without returning a path", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      autoRevealReport: false,
      autoExportTopN: "5",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-10T12:34:56Z",
        artifact_path: "",
        report_xlsx_path: "",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          label: "AAPL artifact",
        },
        optimization_metric_surface: {
          surface: "core17",
          fields: ["pnl", "trades", "max_dd"],
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-export-empty-path",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: "2026-03-10T12:34:56Z",
        req: {
          data_id: "data-export-empty-path",
          candidates: [
            {
              fast_ma_type: "sma",
              slow_ma_type: "ema",
              fast_len: 10,
              slow_len: 40,
            },
          ],
        },
        res: {
          candidates: [{ score: 1 }],
        },
      },
    });

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "save_report_xlsx") return { path: "" };
      if (command === "open_path" || command === "reveal_path") return true;
      return {};
    });

    await exportReportXlsx();

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx",
      expect.objectContaining({
        report: expect.objectContaining({
          optimization_metric_surface: {
            surface: "core17",
            fields: ["pnl", "trades", "max_dd"],
          },
        }),
      }),
    );
    expect(String(validationState.statusLine || "")).toBe(
      "Saved XLSX report, but no path was returned.",
    );
  });

  it("reports CSV and JSON export status strings explicitly", async () => {
    useValidationStore(pinia);
    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:34:56Z",
        artifact_path: "runs/vectorgrid_run_validation_test.json",
        report_xlsx_path: "",
        csv_path: "C:/data/test.csv",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-export-status",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: "2026-02-25T12:34:56Z",
        req: {
          data_id: "data-export-status",
          candidates: [
            {
              fast_ma_type: "sma",
              slow_ma_type: "ema",
              fast_len: 10,
              slow_len: 40,
            },
          ],
        },
        res: {
          candidates: [{ score: 1 }],
        },
      },
    });

    await exportShortlistCsv();
    let validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect([
      "Exported validation shortlist CSV.",
      "Built validation shortlist CSV (download unavailable in this environment).",
    ]).toContain(String(validationState.statusLine || ""));

    await exportValidationJson();
    validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect([
      "Exported validation JSON.",
      "Built validation JSON (download unavailable in this environment).",
    ]).toContain(String(validationState.statusLine || ""));

    await exportReportJson();
    validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect([
      "Exported report JSON.",
      "Built report JSON (download unavailable in this environment).",
    ]).toContain(String(validationState.statusLine || ""));
  });

  it("normalizes cross-refresh datasets from polygon cache info", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      datasets: [
        {
          csv_path: " C:/data/z.csv ",
          display_symbol: "ZETA",
          market: "stocks",
          timeframe: "1M",
          min_ts: 1735689600000,
          max_ts: 1735776000000,
          last_updated_utc_ms: 1735862400000,
          row_count: 1000,
          adjusted: true,
        },
        {
          csv_path: "C:/data/a.csv",
          polygon_ticker: "ALPHA",
          market: "crypto",
          timeframe: "5m",
          min_ts: 1735689600000,
          max_ts: 1735776000000,
          row_count: 0,
        },
        {
          csv_path: "",
          display_symbol: "SKIP",
        },
      ],
    });

    const summary = await crossRefresh();
    expect(invokeCommandMock).toHaveBeenCalledWith("polygon_get_cache_info");
    expect(summary.datasets).toHaveLength(2);
    const [first, second] = summary.datasets;
    expect(first).toBeDefined();
    expect(second).toBeDefined();
    expect(first!).toMatchObject({
      path: "C:/data/a.csv",
      label: "ALPHA",
      market: "crypto",
      timeframe: "5m",
      adjusted: null,
    });
    expect(first!.meta).toContain("crypto");
    expect(first!.meta).toContain("rows=");
    expect(second!).toMatchObject({
      path: "C:/data/z.csv",
      label: "ZETA",
      market: "stocks",
      timeframe: "1m",
      adjusted: true,
    });
    expect(second!.meta).toContain("stocks (adj)");
    expect(second!.meta).toContain("updated=");
  });

  it("normalizes cross toggle payload and updates Vue fallback selection", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      crossDatasets: [
        {
          path: "C:/data/msft.csv",
          label: "MSFT",
          checked: true,
          disabled: false,
          meta: "—",
        },
        {
          path: "C:/data/aapl.csv",
          label: "AAPL",
          checked: false,
          disabled: false,
          meta: "—",
        },
      ],
      crossSelectedCount: 1,
      crossTotalCount: 2,
      source: "vue",
    };

    await toggleCrossDataset(" C:/data/aapl.csv ", true);
    await toggleCrossDataset("C:/data/msft.csv", false);
    await toggleCrossDataset("", false);

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    const rows = Array.isArray(validationState.crossDatasets)
      ? (validationState.crossDatasets as Array<Record<string, unknown>>)
      : [];
    expect(validationState.crossSelectedCount).toBe(1);
    expect(rows).toEqual([
      expect.objectContaining({ path: "C:/data/msft.csv", checked: false }),
      expect.objectContaining({ path: "C:/data/aapl.csv", checked: true }),
    ]);
  });

  it("keeps validation export flows command-driven and reports directory command direct", async () => {
    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:34:56Z",
        artifact_path: "runs/vectorgrid_run_validation_test.json",
        report_xlsx_path: "",
        csv_path: "C:/data/test.csv",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-export-test",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: "2026-02-25T12:34:56Z",
        req: {
          data_id: "data-export-test",
          candidates: [
            {
              fast_ma_type: "sma",
              slow_ma_type: "ema",
              fast_len: 10,
              slow_len: 40,
            },
          ],
        },
        res: {
          candidates: [{ score: 1 }],
        },
      },
    });

    await exportShortlistCsv();
    await exportValidationJson();
    await exportReportJson();
    await exportReportXlsx();
    await openReportsDir();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "build_validation_shortlist_csv",
      expect.any(Object),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx_from_artifact",
      {
        path: "runs/vectorgrid_run_validation_test.json",
        max_rows: null,
      },
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "save_run_artifact",
      ),
    ).toHaveLength(0);
    expect(invokeCommandMock).toHaveBeenCalledWith("open_reports_dir");
  });

  it("prefers artifact-backed XLSX export for persisted validated runs", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      autoRevealReport: false,
      autoExportTopN: "9",
    };
    (pinia.state.value as Record<string, unknown>).data = {
      csvPath: "C:/data/MSFT.csv",
      dataTimeframe: "15m",
      baseDataMeta: { timeframe: "15m" },
      datasetLabel: "MSFT live",
      datasetContext: "source=live",
      dataFields: ["open", "high", "low", "close", "volume"],
      dataLen: "1000",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-10T12:34:56Z",
        artifact_path: "runs/vectorgrid_run_validation_test.json",
        report_xlsx_path: "",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          label: "AAPL artifact",
          symbol: "AAPL",
          len: 4321,
          min_ts: 111,
          max_ts: 222,
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-export-artifact",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: "2026-03-10T12:34:56Z",
        req: {
          data_id: "data-export-artifact",
          candidates: [
            {
              fast_ma_type: "sma",
              slow_ma_type: "ema",
              fast_len: 10,
              slow_len: 40,
            },
          ],
        },
        res: {
          candidates: [{ score: 1 }],
        },
      },
    });

    await exportReportXlsx();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx_from_artifact",
      {
        path: "runs/vectorgrid_run_validation_test.json",
        max_rows: 9,
      },
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "save_report_xlsx",
      expect.anything(),
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "save_run_artifact",
      ),
    ).toHaveLength(0);
    const workflowStore = useWorkflowArtifactsStore(pinia);
    expect(workflowStore.currentRun?.reportXlsxPath).toBe(
      "runs/vectorgrid_report_validation_test.xlsx",
    );
    const perfEntries = readValidationReportPerfEntries();
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.exportReportXlsxSaveMs",
      )?.meta,
    ).toMatchObject({
      branch: "artifact",
      trigger: "manual",
      outcome: "ok",
    });
    expect(
      perfEntries.find(
        (entry) => entry.name === "validationService.exportReportXlsxTotalMs",
      )?.meta,
    ).toMatchObject({
      branch: "artifact",
      trigger: "manual",
      returnedPath: true,
      artifactUpdated: true,
      outcome: "ok",
    });
  });

  it("surfaces artifact update warnings from manual artifact-backed XLSX export", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      autoRevealReport: false,
      autoExportTopN: "9",
    };

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-10T12:34:56Z",
        artifact_path: "runs/vectorgrid_run_validation_warning.json",
        report_xlsx_path: "",
        csv_path: "C:/data/AAPL.csv",
        dataset_meta: {
          csv_path: "C:/data/AAPL.csv",
          timeframe: "1h",
          label: "AAPL artifact",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-export-warning",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: "2026-03-10T12:34:56Z",
        req: {
          data_id: "data-export-warning",
          candidates: [
            {
              fast_ma_type: "sma",
              slow_ma_type: "ema",
              fast_len: 10,
              slow_len: 40,
            },
          ],
        },
        res: {
          candidates: [{ score: 1 }],
        },
      },
    });

    const fallbackInvoke = invokeCommandMock.getMockImplementation();
    const callFallbackInvoke = fallbackInvoke as
      | ((command: string, args?: unknown) => unknown)
      | undefined;
    invokeCommandMock.mockImplementation(
      async (command: string, args?: unknown) => {
        if (command === "save_report_xlsx_from_artifact") {
          return {
            path: "runs/vectorgrid_report_validation_warning.xlsx",
            artifact_updated: false,
            artifact_update_warning:
              "artifact report path persistence failed: report saved but source artifact was not updated",
          };
        }
        return callFallbackInvoke ? callFallbackInvoke(command, args) : {};
      },
    );

    await exportReportXlsx();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx_from_artifact",
      {
        path: "runs/vectorgrid_run_validation_warning.json",
        max_rows: 9,
      },
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "save_run_artifact",
      ),
    ).toHaveLength(1);

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.statusLine || "")).toContain(
      "Saved XLSX report: runs/vectorgrid_report_validation_warning.xlsx",
    );
    expect(String(validationState.statusLine || "")).toContain(
      "artifact report path persistence failed: report saved but source artifact was not updated",
    );

    const workflowStore = useWorkflowArtifactsStore(pinia);
    expect(workflowStore.currentRun?.reportXlsxPath).toBe(
      "runs/vectorgrid_report_validation_warning.xlsx",
    );
  });

  it("keeps CompositeMo Pareto artifacts on pareto-frontier admission and artifact-backed XLSX export", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      candidates: "3",
      seed: "1",
      admissionPolicy: "auto",
      autoRevealReport: false,
      autoExportTopN: "5",
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    };

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "run_composite_validation") {
        return {
          backend_used: "Cpu",
          backend_route: "CpuOnly",
          runtime_ms: 91,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: "single_candidate_supported",
            warning_flags: [],
            fail_reasons: [],
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_composite_mo_pareto.json",
          bytes: 196,
        };
      }
      if (command === "save_report_xlsx_from_artifact") {
        return {
          path: "runs/vectorgrid_report_validation_composite_mo_pareto.xlsx",
          artifact_updated: true,
          artifact_update_warning: "",
        };
      }
      if (command === "open_path" || command === "reveal_path") {
        return true;
      }
      return {};
    });

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-15T00:00:00Z",
        artifact_path:
          "runs/vectorgrid_run_validation_composite_mo_pareto.json",
        req: {
          strategy_kind: "CompositeMo",
          data_id: "data-composite-mo-artifact",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          objectives: { maximize: "Sharpe", minimize: "MaxDrawdown" },
          base_strategy: { fee_bps: 1.0 },
        },
        res: {
          pareto: [
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 9,
                slow_len: 30,
                halftrend: { amplitude: 2 },
                macz: { fast_length: 12 },
                ott: { period: 2 },
                signal_mode: "soft_vote",
                strategy: { fee_bps: 1.0 },
              },
              metrics: { pnl: 0.12, sharpe: 1.1, max_dd: 0.12 },
              maximize_value: 1.1,
              minimize_value: 0.12,
              violation: 0,
            },
            {
              params: {
                fast_ma_type: "ema",
                slow_ma_type: "sma",
                fast_len: 7,
                slow_len: 25,
                halftrend: { amplitude: 3 },
                macz: { fast_length: 10 },
                ott: { period: 3 },
                signal_mode: "hard_gate",
                strategy: { fee_bps: 1.0 },
              },
              metrics: { pnl: 0.1, sharpe: 1.0, max_dd: 0.08 },
              maximize_value: 1.0,
              minimize_value: 0.08,
              violation: 0,
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    const req = firstValidationRequest();
    expect(req.admission_policy).toBe("pareto_frontier");
    expect(req.admission_provenance).toMatchObject({
      requested_policy: "auto",
      applied_policy: "pareto_frontier",
      source_kind: "pareto_frontier",
    });

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(String(validationState.validationMeta || "")).toContain(
      "admission=pareto_frontier",
    );
    expect(String(validationState.validationRouteSummary || "")).toContain(
      "Cpu • CpuOnly",
    );

    await exportReportXlsx();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_report_xlsx_from_artifact",
      {
        path: "runs/vectorgrid_run_validation_composite_mo_pareto.json",
        max_rows: 5,
      },
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "save_report_xlsx",
      expect.anything(),
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "save_run_artifact",
      ),
    ).toHaveLength(1);
  });

  it("preserves Stage 3 meta XLSX fallback with a single reveal when open_path fails", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      autoRevealReport: true,
    };
    vi.stubGlobal("document", {
      documentElement: {
        dataset: {
          latestMetaReportPath: "C:/reports/meta_latest.xlsx",
        },
      },
    });
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "open_path") {
        throw new Error("blocked");
      }
      if (command === "reveal_path") {
        return true;
      }
      return {};
    });

    await exportReportXlsx();

    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "save_report_xlsx",
      expect.anything(),
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "save_report_xlsx_from_artifact",
      expect.anything(),
    );
    expect(
      invokeCommandMock.mock.calls.filter(
        ([command, payload]) =>
          command === "reveal_path" &&
          (payload as Record<string, unknown>)?.path ===
            "C:/reports/meta_latest.xlsx",
      ),
    ).toHaveLength(1);

    const validationState = (pinia.state.value as Record<string, unknown>)
      .validation as Record<string, unknown>;
    expect(validationState.statusLine).toBe(
      "Opened Stage 3 XLSX report: C:/reports/meta_latest.xlsx",
    );
  });

  it("prefers the runtime Stage 3 meta XLSX path over a stale command-action store value", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      autoRevealReport: true,
    };
    (pinia.state.value as Record<string, unknown>).commandActions = {
      latestMetaReportPath: "C:/reports/stale_meta.xlsx",
    };
    vi.stubGlobal("document", {
      documentElement: {
        dataset: {
          latestMetaReportPath: "C:/reports/runtime_meta.xlsx",
        },
      },
    });

    await exportReportXlsx();

    expect(invokeCommandMock).toHaveBeenCalledWith("open_path", {
      path: "C:/reports/runtime_meta.xlsx",
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/reports/runtime_meta.xlsx",
    });
  });

  it("reveals a newly saved report path only once when open_path fails and autoRevealReport is enabled", async () => {
    (pinia.state.value as Record<string, unknown>).validation = {
      autoExportXlsx: true,
      autoRevealReport: true,
      autoExportTopN: "7",
    };
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === "save_report_xlsx") {
        return {
          path: "runs/vectorgrid_report_validation_test.xlsx",
        };
      }
      if (command === "open_path") {
        throw new Error("blocked");
      }
      if (command === "reveal_path") {
        return true;
      }
      if (
        command === "run_double_ma_validation" ||
        command === "run_composite_validation"
      ) {
        return {
          backend_used: "Cpu",
          backend_route: "GpuKernel->CpuOnly",
          backend_fallback_reason:
            "GPU validation runtime not implemented yet; using CPU validation.",
          candidates: [{ score: 1 }],
          runtime_ms: 123,
          cross_ticker: { datasets: [] },
          run_verdict: {
            passed: true,
            approved: false,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 0,
            final_recommendation: "evidence_too_short_min_track_record",
            top_candidate_track_record_bars: 120,
            top_candidate_min_track_record_length_bars: 240,
          },
        };
      }
      if (command === "save_run_artifact") {
        return {
          path: "runs/vectorgrid_run_validation_test.json",
          bytes: 256,
        };
      }
      return {};
    });

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T12:00:00Z",
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-auto-export",
          backend: { backend: "GpuKernel", device_id: 0 },
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    await runValidation({ switchTab: false });

    expect(
      invokeCommandMock.mock.calls.filter(
        ([command, payload]) =>
          command === "reveal_path" &&
          (payload as Record<string, unknown>)?.path ===
            "runs/vectorgrid_report_validation_test.xlsx",
      ),
    ).toHaveLength(1);
  });
});
