import { describe, expect, it } from "vitest";
import { buildMetaStage4Request } from "./metaStage4RequestBuilder";

describe("metaStage4RequestBuilder", () => {
  it("builds stage4 request with defaults and clamped numeric fields", () => {
    const req = buildMetaStage4Request({
      strategyKind: "",
      timeframe: "",
      topK: "0",
      metaWeight: "-1",
      mlWeight: "-2",
      uncertaintyWeight: "-3",
      autoTrainModel: true,
    });

    expect(req).toMatchObject({
      strategy_kind: "CompositeMo",
      timeframe: null,
      source_meta_path: null,
      top_k: 1,
      min_ml_confidence: 0.55,
      meta_weight: 0,
      ml_weight: 0,
      uncertainty_weight: 0,
      accept_confidence: 0.7,
      review_confidence: 0.55,
      max_accept_uncertainty: 0.45,
      auto_train_model: true,
      auto_retrain_stale: true,
      require_healthy_model: false,
      fallback_to_meta_if_empty: false,
      training_history_policy: "source_excluded",
    });
  });

  it("builds stage4 request from provided values", () => {
    const req = buildMetaStage4Request({
      strategyKind: "CompositeCpu",
      timeframe: " 1m ",
      sourceMetaPath: " C:/meta.json ",
      topK: "12",
      minMlConfidence: "0.61",
      metaWeight: "0.5",
      mlWeight: "0.4",
      uncertaintyWeight: "0.1",
      acceptConfidence: "0.8",
      reviewConfidence: "0.6",
      maxAcceptUncertainty: "0.3",
      autoTrainModel: false,
      requireHealthyModel: true,
      fallbackToMetaIfEmpty: true,
    });

    expect(req).toMatchObject({
      strategy_kind: "CompositeCpu",
      timeframe: "1m",
      source_meta_path: "C:/meta.json",
      top_k: 12,
      min_ml_confidence: 0.61,
      meta_weight: 0.5,
      ml_weight: 0.4,
      uncertainty_weight: 0.1,
      accept_confidence: 0.8,
      review_confidence: 0.6,
      max_accept_uncertainty: 0.3,
      auto_train_model: false,
      auto_retrain_stale: false,
      require_healthy_model: true,
      fallback_to_meta_if_empty: true,
      training_history_policy: "source_excluded",
    });
  });

  it("preserves chronological training-history policy when requested", () => {
    const req = buildMetaStage4Request({
      trainingHistoryPolicy: "chronological_before_source",
    });

    expect(req.training_history_policy).toBe("chronological_before_source");
  });

  it("normalizes string booleans", () => {
    const req = buildMetaStage4Request({
      autoTrainModel: "1",
      requireHealthyModel: "yes",
      fallbackToMetaIfEmpty: "on",
    });

    expect(req.auto_train_model).toBe(true);
    expect(req.auto_retrain_stale).toBe(true);
    expect(req.require_healthy_model).toBe(true);
    expect(req.fallback_to_meta_if_empty).toBe(true);
  });

  it("normalizes confidence percentages and keeps review below accept", () => {
    const req = buildMetaStage4Request({
      minMlConfidence: "65",
      acceptConfidence: "80%",
      reviewConfidence: "90%",
      maxAcceptUncertainty: "35",
    });

    expect(req.min_ml_confidence).toBe(0.65);
    expect(req.accept_confidence).toBe(0.8);
    expect(req.review_confidence).toBe(0.8);
    expect(req.max_accept_uncertainty).toBe(0.35);
  });
});
