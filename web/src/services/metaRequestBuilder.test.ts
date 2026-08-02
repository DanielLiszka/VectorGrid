import { describe, expect, it } from "vitest";
import { buildMetaSelectionRequest } from "./metaRequestBuilder";

describe("metaRequestBuilder", () => {
  it("builds baseline meta request with holdout disabled", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "CompositeMo",
      timeframe: "",
      requiredTag: "",
      minSamples: "30",
      topK: "25",
      evalHoldout: false,
      groupingMode: "clustered",
      clusterIntBucket: "5",
      clusterFloatBucket: "0.25",
    });

    expect(req).toMatchObject({
      strategy_kind: "CompositeMo",
      timeframe: null,
      required_tag: null,
      eval_holdout: false,
      holdout_mode: "light",
      holdout_validation: null,
      min_samples: 30,
      top_k: 25,
      strict_shortlist: true,
      allow_shortlist_fallback: false,
    });
  });

  it("builds heavy holdout validation payload when enabled", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "CompositeMo",
      evalHoldout: true,
      holdoutMode: "heavy",
      holdoutTag: "holdout-a",
      holdoutEvalTopN: "10",
      holdoutExploreN: "3",
      holdoutRankBy: "auto",
      saveHoldoutRows: true,
      valSeed: "3",
      valWfWindow: "4000",
      valWfStep: "500",
      valPerturbEnable: true,
      valPerturbIters: "12",
      valPerturbMin: "0.01",
      valPerturbMax: "0.08",
      valMcEnable: true,
      valMcSims: "800",
    });

    expect(req).toMatchObject({
      eval_holdout: true,
      holdout_mode: "heavy",
      holdout_tag: "holdout-a",
      holdout_eval_top_n: 10,
      holdout_eval_explore_n: 3,
      save_holdout_rows: true,
      holdout_validation: {
        seed: 3,
        walk_forward: { window_bars: 4000, step_bars: 500 },
        perturb: {
          iterations: 12,
          pct_min: 0.01,
          pct_max: 0.08,
          flip_bool_prob: 0.02,
        },
        trade_bootstrap: { simulations: 800 },
        validation_engine: "iterative",
      },
    });
  });

  it("supports iid-only trade bootstrap mode for heavy holdout validation", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "DoubleMa",
      evalHoldout: true,
      holdoutMode: "heavy",
      valMcEnable: true,
      valMcSims: "800",
      valMcMode: "iid_only",
      valMcBlockLen: "9",
    });

    expect(req).toMatchObject({
      holdout_validation: {
        trade_bootstrap: { simulations: 800, stationary_mean_block_len: 0 },
      },
    });
  });

  it("disables holdout for unsupported strategy kinds", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "UnknownStrategy",
      evalHoldout: true,
      holdoutMode: "heavy",
    });

    expect(req).toMatchObject({
      strategy_kind: "UnknownStrategy",
      eval_holdout: false,
      holdout_mode: "light",
      holdout_validation: null,
    });
  });

  it("keeps holdout enabled for ASR strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Asr",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Asr",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for VEMR strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Vemr",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Vemr",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for VCC strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Vcc",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Vcc",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for VCET strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Vcet",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Vcet",
      eval_holdout: true,
      holdout_mode: "light",
      holdout_validation: {
        validation_engine: "vectorized",
        use_gpu: true,
        require_gpu: true,
        gpu_device_id: 0,
        walk_forward: null,
      },
    });
  });

  it("makes heavy VCET GPU-preferred with explicit fallback permission and CPU auxiliary stages", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Vcet",
      evalHoldout: true,
      holdoutMode: "heavy",
      holdoutGpuDeviceId: "2",
      valWfWindow: "4000",
      valWfStep: "500",
    });

    expect(req).toMatchObject({
      holdout_validation: {
        validation_engine: "vectorized",
        use_gpu: true,
        require_gpu: false,
        gpu_device_id: 2,
        walk_forward: { window_bars: 4000, step_bars: 500 },
      },
    });
  });

  it("keeps holdout enabled for Set strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Set",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Set",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for Pit strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Pit",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Pit",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for Mts strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Mts",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Mts",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for Orc strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Orc",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Orc",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for Marrt strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Marrt",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Marrt",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for Vfct strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Vfct",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Vfct",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("keeps holdout enabled for Rocb strategy", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "Rocb",
      evalHoldout: true,
      holdoutMode: "light",
    });

    expect(req).toMatchObject({
      strategy_kind: "Rocb",
      eval_holdout: true,
      holdout_mode: "light",
    });
  });

  it("clamps grouping representatives and keeps shortlist defaults hard-enabled", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "CompositeMo",
      evalHoldout: true,
      holdoutMode: "heavy",
      holdoutReps: "9",
      clusterIntBucket: "0",
      clusterFloatBucket: "",
      holdoutEvalTopN: "10",
      holdoutExploreN: "3",
      holdoutRankBy: "auto",
    });

    expect(req).toMatchObject({
      cluster_config: {
        int_bucket: 1,
        float_bucket: 0,
      },
      cluster_representatives: 3,
      strict_shortlist: true,
      allow_shortlist_fallback: false,
      holdout_eval_top_n: 10,
      holdout_eval_explore_n: 3,
      holdout_eval_rank_by: "auto",
    });
  });

  it("falls back to safe enum defaults for invalid persisted mode values", () => {
    const req = buildMetaSelectionRequest({
      strategyKind: "CompositeMo",
      evalHoldout: true,
      holdoutMode: "surprise",
      groupingMode: "unknown-grouping",
    });

    expect(req).toMatchObject({
      holdout_mode: "light",
      grouping_mode: "clustered",
    });
  });
});
