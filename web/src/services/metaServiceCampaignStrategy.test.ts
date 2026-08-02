import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "../stores/runtimePinia";
import { invokeCommand } from "../tauri/invoke";
import { runBatch } from "./metaService";
import { useAnalysisStore } from "../stores/analysisStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useValidationStore } from "../stores/validationStore";
import { buildCompositeCpuOptimizationRequest } from "./optimizeRequestBuilder";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

vi.mock("../tauri/events", () => ({
  listenEvent: vi.fn(async () => () => undefined),
}));

vi.mock("./optimizeService", () => ({
  buildOptimizationRequestFromVueStores: vi.fn(async () => ({
    strategy_kind: "DoubleMa",
    data_id: "data-1",
    ma_source: "close",
  })),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
const supportedCampaignStrategies = [
  "DoubleMa",
  "Pit",
  "Set",
  "Mts",
  "Orc",
  "Marrt",
  "Vfct",
  "Rocb",
  "Vcet",
  "Mkcs",
  "Kqsr",
  "Carm",
  "Kvdf",
  "Cdmc",
  "Asr",
  "Vcc",
  "Vemr",
  "CompositeCpu",
  "CompositeMo",
  "CompositeTrend",
  "RegimeHybrid",
  "BreakoutMomentum",
] as const;

function summaryFor(strategyKind: string) {
  return {
    version: 1,
    kind: "vectorgrid_campaign_summary",
    ts: "2026-04-03T12:00:00Z",
    state: "completed",
    strategy_kind: strategyKind,
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
    request: {
      strategy_kind: strategyKind,
      datasets: [{ csv_path: "C:/a.csv", label: "AAPL", timeframe: "1m" }],
    },
  };
}

async function invokeDefaultMock(
  command: string,
  args?: Record<string, unknown>,
): Promise<unknown> {
  if (command === "polygon_get_cache_info") {
    return { datasets: [] };
  }
  if (command === "list_repo_data_csvs") {
    return {
      dir: "C:/repo-data",
      truncated: false,
      csv_entries: [
        {
          csv_path: "C:/a.csv",
          label: "AAPL",
          timeframe: "1m",
          market: "local",
          source: "repo_data",
        },
      ],
    };
  }
  if (command === "run_research_campaign") {
    const req = (args?.req ?? {}) as Record<string, unknown>;
    const strategyKind = String(req.strategy_kind ?? "DoubleMa");
    return {
      summary_json_path: "C:/runs/vectorgrid_campaign_1.json",
      summary: summaryFor(strategyKind),
    };
  }
  if (command === "list_run_artifacts") return [];
  if (command === "list_artifact_library") {
    return { meta: [], stage5: [], policyEval: [] };
  }
  return undefined;
}

function firstCampaignRequest(): Record<string, unknown> {
  const call = invokeCommandMock.mock.calls.find(
    ([command]) => command === "run_research_campaign",
  );
  expect(call).toBeTruthy();
  const args = (call?.[1] ?? {}) as Record<string, unknown>;
  return (args.req ?? {}) as Record<string, unknown>;
}

describe("meta-service campaign strategy routing", () => {
  beforeEach(() => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    invokeCommandMock.mockReset();
    invokeCommandMock.mockImplementation(invokeDefaultMock);
  });

  it.each(supportedCampaignStrategies)(
    "passes %s unchanged into the backend research campaign request",
    async (strategyKind) => {
      const optimizeStore = useOptimizeStore();
      optimizeStore.strategyKind = strategyKind;
      optimizeStore.updatedAtUnixMs = Date.now();

      await runBatch({
        request: {
          selected_paths: ["C:/a.csv"],
          holdout_pct: 0,
          holdout_tag: "holdout",
          auto_meta: false,
          skip_done: false,
          tags: ["campaign"],
          optimization_template: {
            strategy_kind: strategyKind,
            data_id: "data-1",
            ma_source: "close",
          },
          validation_config: {
            max_candidates: 8,
            seed: 1,
          },
        },
      });

      const call = invokeCommandMock.mock.calls.find(
        ([command]) => command === "run_research_campaign",
      );
      expect(call).toBeTruthy();
      expect(call?.[1]).toMatchObject({
        req: {
          strategy_kind: strategyKind,
        },
      });
    },
  );

  it("passes the selected preload policy through to the backend campaign request", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        auto_stage5: false,
        skip_done: false,
        tags: ["campaign"],
        preload_policy: "host_plus_doublema_vram",
        optimization_template: {
          strategy_kind: "DoubleMa",
          data_id: "data-1",
          ma_source: "close",
        },
        validation_config: {
          max_candidates: 8,
          seed: 1,
          walk_forward: { seeded: true },
          reopt_walk_forward: { seeded: true },
          perturb: { seeded: true },
          trade_bootstrap: { seeded: true },
          monte_carlo: { seeded: true },
          cross_ticker: { seeded: true },
          cscv_pbo: { seeded: true },
          spa: { seeded: true },
        },
      },
    });

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    expect(call).toBeTruthy();
    expect(call?.[1]).toMatchObject({
      req: {
        preload_policy: "host_plus_doublema_vram",
      },
    });
    expect(useAnalysisStore().batchPreloadPolicy).toBe(
      "host_plus_doublema_vram",
    );
  });

  it("passes the current validation admission policy through to the backend campaign request", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();
    const validationStore = useValidationStore();
    validationStore.admissionPolicy = "scalar_union";

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        auto_stage5: false,
        skip_done: false,
        tags: ["campaign"],
        optimization_template: {
          strategy_kind: "DoubleMa",
          data_id: "data-1",
          ma_source: "close",
        },
      },
    });

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    expect(call).toBeTruthy();
    expect(call?.[1]).toMatchObject({
      req: {
        validation_config: {
          admission_policy: "scalar_union",
        },
      },
    });
  });

  it("uses the Rust SPA benchmark enum in generated campaign validation config", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();
    const validationStore = useValidationStore();
    validationStore.spaEnable = true;
    validationStore.spaSlices = "12";
    validationStore.spaBootstrapSamples = "750";
    validationStore.spaBlockLength = "5";

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        skip_done: false,
        tags: ["campaign"],
        optimization_template: {
          strategy_kind: "DoubleMa",
          data_id: "data-1",
          ma_source: "close",
        },
      },
    });

    expect(firstCampaignRequest()).toMatchObject({
      validation_config: {
        spa: {
          slices: 12,
          bootstrap_samples: 750,
          block_length: 5,
          benchmark: "buy_and_hold",
        },
      },
    });
  });

  it.each([
    {
      label: "default stock component costs",
      strategyKind: "DoubleMa",
      strategyField: "strategy",
      strategy: {
        fee_bps: 0.1,
        spread_bps: 1,
        slippage_bps: 0.5,
        commission: 0.00011,
      },
    },
    {
      label: "custom non-default component costs",
      strategyKind: "CompositeMo",
      strategyField: "base_strategy",
      strategy: {
        fee_bps: 2.75,
        spread_bps: 4.5,
        slippage_bps: 1.25,
        commission: 0.000625,
      },
    },
  ])(
    "normalizes $label in a cloned campaign optimization template",
    async ({ strategyKind, strategyField, strategy }) => {
      const optimizeStore = useOptimizeStore();
      optimizeStore.strategyKind = strategyKind;
      optimizeStore.updatedAtUnixMs = Date.now();
      const sourceStrategy = { ...strategy };
      const sourceTemplate: Record<string, unknown> = {
        strategy_kind: strategyKind,
        data_id: "data-1",
        ma_source: "close",
        [strategyField]: sourceStrategy,
      };

      await runBatch({
        request: {
          selected_paths: ["C:/a.csv"],
          holdout_pct: 0,
          holdout_tag: "holdout",
          auto_meta: false,
          skip_done: false,
          tags: ["campaign"],
          optimization_template: sourceTemplate,
          validation_config: {
            max_candidates: 8,
            seed: 1,
          },
        },
      });

      const receivedTemplate = firstCampaignRequest()
        .optimization_template as Record<string, unknown>;
      const receivedStrategy = receivedTemplate[strategyField] as Record<
        string,
        unknown
      >;
      expect(receivedTemplate).not.toBe(sourceTemplate);
      expect(receivedStrategy).not.toBe(sourceStrategy);
      expect(receivedStrategy).toEqual({
        ...strategy,
        commission: 0,
      });
      expect(sourceTemplate[strategyField]).toBe(sourceStrategy);
      expect(sourceStrategy).toEqual(strategy);
    },
  );

  it("preserves commission-only legacy costs while cloning the campaign template", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();
    const sourceStrategy = {
      fee_bps: 0,
      spread_bps: 0,
      slippage_bps: 0,
      commission: 0.0007,
    };
    const sourceTemplate = {
      strategy_kind: "DoubleMa",
      data_id: "data-1",
      ma_source: "close",
      strategy: sourceStrategy,
    };

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        skip_done: false,
        tags: ["campaign"],
        optimization_template: sourceTemplate,
        validation_config: {
          max_candidates: 8,
          seed: 1,
          walk_forward: { window_bars: 4000, step_bars: 500 },
          perturb: { iterations: 8, pct_min: 0.01, pct_max: 0.05 },
          trade_bootstrap: { simulations: 100 },
          cscv_pbo: { slices: 8 },
        },
      },
    });

    const receivedTemplate = firstCampaignRequest()
      .optimization_template as Record<string, unknown>;
    expect(receivedTemplate).not.toBe(sourceTemplate);
    expect(receivedTemplate.strategy).not.toBe(sourceStrategy);
    expect(receivedTemplate.strategy).toEqual(sourceStrategy);
    expect(sourceTemplate.strategy).toBe(sourceStrategy);
    expect(sourceStrategy.commission).toBe(0.0007);
  });

  it("passes validate_only through to the backend campaign request", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        execution_mode: "validate_only",
        auto_meta: true,
        auto_stage5: true,
        skip_done: true,
        tags: ["campaign"],
        preload_policy: "host_only",
        optimization_template: {
          strategy_kind: "DoubleMa",
          data_id: "data-1",
          ma_source: "close",
        },
        validation_config: {
          max_candidates: 8,
          seed: 1,
        },
      },
    });

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    expect(call).toBeTruthy();
    expect(call?.[1]).toMatchObject({
      req: {
        execution_mode: "validate_only",
        auto_meta: true,
        auto_stage5: true,
      },
    });
    expect(useAnalysisStore().batchExecutionMode).toBe("validate_only");
  });

  it("suppresses meta and stage5 follow-up payloads for optimize_only mode", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        execution_mode: "optimize_only",
        auto_meta: true,
        auto_stage5: true,
        skip_done: false,
        tags: ["campaign"],
        preload_policy: "host_only",
        optimization_template: {
          strategy_kind: "DoubleMa",
          data_id: "data-1",
          ma_source: "close",
        },
        validation_config: {
          max_candidates: 8,
          seed: 1,
        },
      },
    });

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    expect(call).toBeTruthy();
    expect(call?.[1]).toMatchObject({
      req: {
        execution_mode: "optimize_only",
        auto_meta: false,
        auto_stage5: false,
      },
    });
    expect(useAnalysisStore().batchExecutionMode).toBe("optimize_only");
    expect(useAnalysisStore().batchAutoMeta).toBe(false);
    expect(useAnalysisStore().batchAutoStage5).toBe(false);
  });

  it("builds a strict GPU VCET fixed-candidate replay request from the current ranges", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    const optimizationTemplate = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "Vcet",
        data_id: "data-1",
        current_data_meta: {
          has_high: true,
          has_low: true,
          has_close: true,
        },
      },
      backend: "Auto",
      fastStart: "8",
      fastEnd: "10",
      fastStep: "2",
      slowStart: "11",
      slowEnd: "13",
      slowStep: "2",
      fastMaType: "vidya",
      slowMaType: "vidya",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "8",
      includeAll: "false",
      vcetMapFastToVidyaShortPeriod: true,
      vcetMapSlowToVidyaLongPeriod: true,
      vcetMapSlowToCePeriod: true,
      vcetVidyaShortPeriod: "6",
      vcetVidyaLongPeriod: "24",
      vcetVidyaAlpha: "0.2",
      vcetCePeriod: "22",
      vcetCeMult: "3",
      vcetCeUseClose: true,
      vcetRequireVidyaSlope: true,
      vcetMaxTradeBars: "120",
    });
    expect(optimizationTemplate).toBeTruthy();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        execution_mode: "evaluate_candidates",
        auto_meta: true,
        auto_stage5: true,
        skip_done: true,
        tags: ["vcet-universe"],
        optimization_template: optimizationTemplate,
        validation_config: {
          max_candidates: 8,
          seed: 1,
        },
      },
    });

    expect(firstCampaignRequest()).toMatchObject({
      strategy_kind: "Vcet",
      execution_mode: "evaluate_candidates",
      auto_meta: false,
      auto_stage5: false,
      optimization_template: {
        backend: { backend: "GpuKernel", device_id: 0 },
        fast_ma_types: ["vidya"],
        slow_ma_types: ["vidya"],
        halftrend: {
          amplitude: 2,
          atr_period: 100,
          channel_deviation: 2,
        },
        vcet: {
          map_fast_to_vidya_short_period: true,
          map_slow_to_vidya_long_period: true,
          map_slow_to_ce_period: true,
          vidya_short_period: 6,
          vidya_long_period: 24,
          vidya_alpha: 0.2,
          ce_period: 22,
          ce_mult: 3,
          ce_use_close: true,
          require_vidya_slope: true,
          max_trade_bars: 120,
        },
      },
      validation_config: {
        require_gpu_kernel: true,
        cross_universe_admission: {
          min_profitable_share_requested: 0.55,
          min_hard_gate_pass_share_requested: 0.5,
          min_pnl_p10: 0,
          min_sharpe_p10: 0,
        },
        fixed_candidates: [
          { fast_len: 8, slow_len: 11 },
          { fast_len: 8, slow_len: 13 },
          { fast_len: 10, slow_len: 11 },
          { fast_len: 10, slow_len: 13 },
        ],
        walk_forward: null,
        reopt_walk_forward: null,
        perturb: null,
        trade_bootstrap: null,
        monte_carlo: null,
        cross_ticker: null,
        cscv_pbo: null,
        spa: null,
      },
    });
    expect(useAnalysisStore().batchExecutionMode).toBe("evaluate_candidates");
    expect(useAnalysisStore().batchAutoMeta).toBe(false);
  });

  it("preserves explicit VCET cross-universe admission thresholds", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 8, 1],
          slow_range: [20, 20, 1],
        },
        validation_config: {
          max_candidates: 1,
          cross_universe_admission: {
            min_profitable_share_requested: 0.7,
            min_hard_gate_pass_share_requested: 0.65,
            min_pnl_p10: 0.01,
            min_sharpe_p10: 0.25,
          },
        },
      },
    });

    expect(firstCampaignRequest()).toMatchObject({
      validation_config: {
        cross_universe_admission: {
          min_profitable_share_requested: 0.7,
          min_hard_gate_pass_share_requested: 0.65,
          min_pnl_p10: 0.01,
          min_sharpe_p10: 0.25,
        },
      },
    });
  });

  it("uses the visible VCET admission controls for fixed-candidate replay", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    const analysisStore = useAnalysisStore();
    analysisStore.batchAdmissionMinProfitableShare = "0.60";
    analysisStore.batchAdmissionMinHardGateShare = "0.58";
    analysisStore.batchAdmissionMinPnlP10 = "0.02";
    analysisStore.batchAdmissionMinSharpeP10 = "0.30";

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 8, 1],
          slow_range: [20, 20, 1],
        },
        validation_config: { max_candidates: 1 },
      },
    });

    expect(firstCampaignRequest()).toMatchObject({
      validation_config: {
        cross_universe_admission: {
          min_profitable_share_requested: 0.6,
          min_hard_gate_pass_share_requested: 0.58,
          min_pnl_p10: 0.02,
          min_sharpe_p10: 0.3,
        },
      },
    });
  });

  it("promotes a completed VCET replay to its unique Meta cohort tag", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    invokeCommandMock.mockImplementation(async (command, args) => {
      if (command === "run_research_campaign") {
        const req = (args?.req ?? {}) as Record<string, unknown>;
        return {
          summary_json_path: "C:/runs/vcet-cohort-unique.json",
          summary: {
            version: 2,
            kind: "vectorgrid_campaign_summary",
            ts: "2026-07-31T14:00:00Z",
            state: "completed",
            strategy_kind: "Vcet",
            timeframe: "1m",
            tags: ["vcet-universe"],
            holdout_pct: 0,
            holdout_tag: null,
            manifest: {
              manifest_hash: "sha256:unique-cohort-456",
              candidate_manifest_hash: "sha256:candidates-456",
              execution_mode: "evaluate_candidates",
              strategy_kind: "Vcet",
            },
            request: req,
            counts: {
              total: 1,
              optimized: 0,
              validated: 1,
              skipped: 0,
              failed: 0,
              cancelled: false,
            },
            items: [],
            fixed_candidate_evaluation: {
              schema_version: 4,
              candidate_manifest_hash: "sha256:candidates-456",
              require_gpu_kernel: true,
              requested_datasets: 1,
              evaluated_datasets: 1,
              gpu_kernel_datasets: 1,
              missing_or_failed_datasets: 0,
              selection_status: "selected",
              admitted_candidate_count: 1,
              admitted_candidate_ids: ["vcet_candidate_8_20"],
              selected_candidate_id: "vcet_candidate_8_20",
              candidates: [],
            },
            warnings: [],
            runtime_ms: 1,
          },
        };
      }
      return invokeDefaultMock(command, args);
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        timeframe: "1m",
        execution_mode: "evaluate_candidates",
        tags: ["vcet-universe"],
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 8, 1],
          slow_range: [20, 20, 1],
        },
        validation_config: { max_candidates: 1 },
      },
    });

    expect(useAnalysisStore().metaRequiredTag).toBe(
      "campaign-cohort:unique-cohort-456",
    );
  });

  it("preserves the prior unique Meta scope when a VCET replay is cancelled", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "Vcet";
    optimizeStore.updatedAtUnixMs = Date.now();
    const analysisStore = useAnalysisStore();
    analysisStore.metaRequiredTag = "campaign-cohort:deadbeef";
    invokeCommandMock.mockImplementation(async (command, args) => {
      if (command === "run_research_campaign") {
        const req = (args?.req ?? {}) as Record<string, unknown>;
        return {
          summary_json_path: "C:/runs/vcet-cohort-cancelled.json",
          summary: {
            ...summaryFor("Vcet"),
            version: 2,
            state: "cancelled",
            timeframe: "1m",
            tags: ["vcet-universe"],
            manifest: {
              manifest_hash: "sha256:cancelled-cohort",
              candidate_manifest_hash: "sha256:cancelled-candidates",
              execution_mode: "evaluate_candidates",
              strategy_kind: "Vcet",
            },
            request: req,
            counts: {
              total: 1,
              optimized: 0,
              validated: 0,
              skipped: 0,
              failed: 0,
              cancelled: true,
            },
            fixed_candidate_evaluation: {
              schema_version: 4,
              candidate_manifest_hash: "sha256:cancelled-candidates",
              require_gpu_kernel: true,
              requested_datasets: 1,
              evaluated_datasets: 0,
              gpu_kernel_datasets: 0,
              missing_or_failed_datasets: 1,
              selection_status: "no_candidate_met_admission",
              admitted_candidate_count: 0,
              admitted_candidate_ids: [],
              selected_candidate_id: null,
              candidates: [],
            },
          },
        };
      }
      return invokeDefaultMock(command, args);
    });

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        tags: ["vcet-universe"],
        optimization_template: {
          strategy_kind: "Vcet",
          backend: { backend: "GpuKernel", device_id: 0 },
          fast_range: [8, 8, 1],
          slow_range: [20, 20, 1],
        },
        validation_config: { max_candidates: 1 },
      },
    });

    expect(analysisStore.metaRequiredTag).toBe("campaign-cohort:deadbeef");
  });

  it("rejects fixed-candidate replay for non-VCET strategies", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        execution_mode: "evaluate_candidates",
        optimization_template: {
          strategy_kind: "DoubleMa",
          data_id: "data-1",
          fast_range: [8, 8, 1],
          slow_range: [34, 34, 1],
        },
        validation_config: { max_candidates: 8 },
      },
    });

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(useAnalysisStore().statusLine).toContain("currently VCET-only");
  });

  it("rejects MlSignal before invoking the backend campaign runner", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "MlSignal";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        skip_done: false,
        tags: ["campaign"],
      },
    });

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(useAnalysisStore().statusLine).toContain("do not support MlSignal");
  });

  it("rejects DynamicDoubleMa before invoking the backend campaign runner", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DynamicDoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        skip_done: false,
        tags: ["campaign"],
      },
    });

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(useAnalysisStore().statusLine).toContain(
      "do not support DynamicDoubleMa",
    );
  });

  it("rejects unsupported future strategies before invoking the backend campaign runner", async () => {
    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "UnknownFutureStrategy";
    optimizeStore.updatedAtUnixMs = Date.now();

    await runBatch({
      request: {
        selected_paths: ["C:/a.csv"],
        holdout_pct: 0,
        holdout_tag: "holdout",
        auto_meta: false,
        skip_done: false,
        tags: ["campaign"],
      },
    });

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(useAnalysisStore().statusLine).toContain(
      "do not support strategy 'UnknownFutureStrategy'",
    );
  });
});
