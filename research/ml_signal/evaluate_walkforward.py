from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import pandas as pd

from calibrate import fit_sigmoid
from common import (
    NormalizedSplitWindow,
    apply_sigmoid,
    build_feature_frame,
    choose_sample_weight,
    dump_json,
    load_dataset_bundle,
    resolve_walkforward_windows,
    split_window_masks,
)
from train_catboost import (
    DEFAULT_DEPTH,
    DEFAULT_ITERATIONS,
    DEFAULT_LEARNING_RATE,
    DEFAULT_RANDOM_SEED,
    train_model,
)

try:
    from catboost import CatBoostClassifier
except ImportError as exc:  # pragma: no cover
    raise SystemExit(
        "catboost is required for evaluate_walkforward.py. Install research/ml_signal dependencies first.",
    ) from exc


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Evaluate MlSignal bundles on walk-forward dataset slices.",
    )
    parser.add_argument("--dataset", required=True, help="Exported ML dataset CSV.")
    parser.add_argument("--bundle-dir", required=True, help="Model bundle directory.")
    parser.add_argument(
        "--mode",
        choices=["proxy", "rolling"],
        default="proxy",
        help="proxy reuses the saved bundle models; rolling retrains and recalibrates per split window.",
    )
    parser.add_argument("--t-long", type=float, default=0.6)
    parser.add_argument("--t-short", type=float, default=0.6)
    parser.add_argument("--delta", type=float, default=0.1)
    parser.add_argument(
        "--artifact-suffix",
        default="",
        help="Optional suffix appended to walk-forward artifact filenames, for example 'proxy' or 'rolling'.",
    )
    return parser.parse_args()


def _read_json(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    if not isinstance(raw, dict):
        raise ValueError(f"expected JSON object at {path}")
    return raw


def _bundle_sidecar(bundle_dir: Path, filename: str) -> Path | None:
    path = bundle_dir / filename
    return path if path.exists() else None


def _artifact_path(bundle_dir: Path, base_name: str, suffix: str) -> Path:
    base = Path(base_name)
    clean_suffix = str(suffix).strip().strip(".")
    if not clean_suffix:
        return bundle_dir / base_name
    return bundle_dir / f"{base.stem}.{clean_suffix}{base.suffix}"


def _load_training_config(bundle_dir: Path) -> dict[str, float | int]:
    config: dict[str, float | int] = {
        "iterations": DEFAULT_ITERATIONS,
        "learning_rate": DEFAULT_LEARNING_RATE,
        "depth": DEFAULT_DEPTH,
        "random_seed": DEFAULT_RANDOM_SEED,
    }
    metadata = _read_json(bundle_dir / "train_metadata.json")
    if not isinstance(metadata, dict):
        return config

    if "iterations" in metadata:
        config["iterations"] = int(metadata["iterations"])
    if "learning_rate" in metadata:
        config["learning_rate"] = float(metadata["learning_rate"])
    if "depth" in metadata:
        config["depth"] = int(metadata["depth"])
    if "random_seed" in metadata:
        config["random_seed"] = int(metadata["random_seed"])
    return config


def _load_global_calibration(bundle_dir: Path) -> dict[str, Any] | None:
    calibration = _read_json(bundle_dir / "calibration.json")
    return calibration if isinstance(calibration, dict) else None


def _build_base_frame(bundle: Any) -> tuple[pd.DataFrame, pd.DataFrame]:
    frame = bundle.dataset.copy()
    features = build_feature_frame(bundle)
    for column in bundle.feature_names:
        frame[column] = features[column]
    return frame, features


def _safe_auc(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        valid = y_true.notna() & y_prob.notna()
        if int(valid.sum()) == 0:
            return None
        y_valid = y_true.loc[valid]
        p_valid = y_prob.loc[valid]
        if y_valid.nunique(dropna=True) < 2:
            return None
        from sklearn.metrics import roc_auc_score

        return float(roc_auc_score(y_valid, p_valid))
    except Exception:
        return None


def _safe_logloss(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        valid = y_true.notna() & y_prob.notna()
        if int(valid.sum()) == 0:
            return None
        from sklearn.metrics import log_loss

        return float(log_loss(y_true.loc[valid], y_prob.loc[valid], labels=[0, 1]))
    except Exception:
        return None


def _score_frame(
    frame: pd.DataFrame,
    long_raw: Any,
    short_raw: Any,
    long_prob: Any,
    short_prob: Any,
) -> pd.DataFrame:
    scored = frame.loc[:, ["timestamp", "ticker_id", "split_id", "r_long", "r_short"]].copy()
    scored["raw_long"] = long_raw
    scored["raw_short"] = short_raw
    scored["p_long"] = long_prob
    scored["p_short"] = short_prob
    return scored


def _maybe_apply_sigmoid_columns(scored: pd.DataFrame, calibration: dict[str, Any] | None) -> bool:
    if not isinstance(calibration, dict):
        return False
    applied = False
    long_cfg = calibration.get("long") or {}
    short_cfg = calibration.get("short") or {}
    if "coef" in long_cfg and "intercept" in long_cfg:
        scored["p_long"] = apply_sigmoid(
            scored["raw_long"].to_numpy(),
            float(long_cfg["coef"]),
            float(long_cfg["intercept"]),
        )
        applied = True
    if "coef" in short_cfg and "intercept" in short_cfg:
        scored["p_short"] = apply_sigmoid(
            scored["raw_short"].to_numpy(),
            float(short_cfg["coef"]),
            float(short_cfg["intercept"]),
        )
        applied = True
    return applied


def _fit_sigmoid_config(raw: pd.Series, target: pd.Series) -> dict[str, float] | None:
    if target.nunique(dropna=True) < 2:
        return None
    try:
        coef, intercept = fit_sigmoid(raw, target)
    except Exception:
        return None
    return {"coef": float(coef), "intercept": float(intercept)}


def _apply_policy_thresholds(
    scored: pd.DataFrame,
    t_long: float,
    t_short: float,
    delta: float,
) -> pd.DataFrame:
    long_cond = (scored["p_long"] >= t_long) & ((scored["p_long"] - scored["p_short"]) >= delta)
    short_cond = (scored["p_short"] >= t_short) & ((scored["p_short"] - scored["p_long"]) >= delta)
    scored["position"] = 0
    scored.loc[long_cond, "position"] = 1
    scored.loc[~long_cond & short_cond, "position"] = -1
    scored["policy_return"] = 0.0
    scored.loc[scored["position"] == 1, "policy_return"] = scored.loc[
        scored["position"] == 1,
        "r_long",
    ]
    scored.loc[scored["position"] == -1, "policy_return"] = scored.loc[
        scored["position"] == -1,
        "r_short",
    ]
    return scored


def summarize_roll(
    window: NormalizedSplitWindow,
    scored_slice: pd.DataFrame,
    evaluation_mode: str,
    calibration_strategy: str,
    train_rows: int,
    calibration_rows: int,
    long_auc_calibration: float | None = None,
    short_auc_calibration: float | None = None,
    long_logloss_calibration: float | None = None,
    short_logloss_calibration: float | None = None,
) -> dict[str, Any]:
    rows_scored = int(len(scored_slice))
    long_count = int((scored_slice["position"] == 1).sum())
    short_count = int((scored_slice["position"] == -1).sum())
    flat_count = int((scored_slice["position"] == 0).sum())
    active_count = long_count + short_count
    active_share = float(active_count / rows_scored) if rows_scored > 0 else 0.0
    mean_return = float(scored_slice["policy_return"].mean()) if rows_scored else 0.0
    sum_return = float(scored_slice["policy_return"].sum()) if rows_scored else 0.0

    return {
        "split_id": window.split_id,
        "evaluation_mode": evaluation_mode,
        "calibration_strategy": calibration_strategy,
        "train_start": window.train_start,
        "train_end": window.train_end,
        "calibration_start": window.calibration_start,
        "calibration_end": window.calibration_end,
        "test_start": window.test_start,
        "test_end": window.test_end,
        "train_rows": int(train_rows),
        "calibration_rows": int(calibration_rows),
        "rows_scored": rows_scored,
        "signals_long": long_count,
        "signals_short": short_count,
        "signals_flat": flat_count,
        "active_share": active_share,
        "mean_policy_return": mean_return,
        "sum_policy_return": sum_return,
        "long_auc_calibration": long_auc_calibration,
        "short_auc_calibration": short_auc_calibration,
        "long_logloss_calibration": long_logloss_calibration,
        "short_logloss_calibration": short_logloss_calibration,
    }


def summarize_ticker(
    ticker_id: str,
    scored_slice: pd.DataFrame,
) -> dict[str, Any]:
    rows_scored = int(len(scored_slice))
    long_count = int((scored_slice["position"] == 1).sum())
    short_count = int((scored_slice["position"] == -1).sum())
    flat_count = int((scored_slice["position"] == 0).sum())
    active_count = long_count + short_count
    active_share = float(active_count / rows_scored) if rows_scored > 0 else 0.0
    mean_return = float(scored_slice["policy_return"].mean()) if rows_scored else 0.0
    sum_return = float(scored_slice["policy_return"].sum()) if rows_scored else 0.0

    return {
        "ticker_id": ticker_id,
        "rows_scored": rows_scored,
        "signals_long": long_count,
        "signals_short": short_count,
        "signals_flat": flat_count,
        "active_share": active_share,
        "mean_policy_return": mean_return,
        "sum_policy_return": sum_return,
    }


def summarize_rolling_calibration(roll_summaries: list[dict[str, Any]]) -> dict[str, Any]:
    metric_names = [
        "long_auc_calibration",
        "short_auc_calibration",
        "long_logloss_calibration",
        "short_logloss_calibration",
    ]
    summary: dict[str, Any] = {}
    for metric_name in metric_names:
        values = [
            float(row[metric_name])
            for row in roll_summaries
            if row.get(metric_name) is not None
        ]
        summary[f"{metric_name}_mean"] = float(sum(values) / len(values)) if values else None
        summary[f"{metric_name}_min"] = float(min(values)) if values else None
        summary[f"{metric_name}_max"] = float(max(values)) if values else None
        summary[f"{metric_name}_count"] = int(len(values))
    return summary


def _evaluate_proxy_mode(
    frame: pd.DataFrame,
    features: pd.DataFrame,
    windows: list[NormalizedSplitWindow],
    bundle_dir: Path,
    t_long: float,
    t_short: float,
    delta: float,
) -> tuple[pd.DataFrame, list[dict[str, Any]], dict[str, Any]]:
    long_model = CatBoostClassifier()
    short_model = CatBoostClassifier()
    long_model.load_model(str(bundle_dir / "model_long.cbm"))
    short_model.load_model(str(bundle_dir / "model_short.cbm"))

    scored = _score_frame(
        frame,
        long_model.predict(features, prediction_type="RawFormulaVal"),
        short_model.predict(features, prediction_type="RawFormulaVal"),
        long_model.predict_proba(features)[:, 1],
        short_model.predict_proba(features)[:, 1],
    )
    calibration_applied = _maybe_apply_sigmoid_columns(scored, _load_global_calibration(bundle_dir))
    scored = _apply_policy_thresholds(scored, t_long, t_short, delta)

    roll_frames: list[pd.DataFrame] = []
    roll_summaries: list[dict[str, Any]] = []
    skipped_rolls: list[dict[str, str]] = []
    for window in windows:
        if window.test_end <= window.test_start:
            skipped_rolls.append({"split_id": window.split_id, "reason": "empty_test_window"})
            continue
        scored_slice = scored.iloc[window.test_start : window.test_end].copy()
        if scored_slice.empty:
            skipped_rolls.append({"split_id": window.split_id, "reason": "empty_test_slice"})
            continue
        scored_slice["evaluated_split_id"] = window.split_id
        scored_slice["evaluation_mode"] = "proxy"
        roll_frames.append(scored_slice)
        roll_summaries.append(
            summarize_roll(
                window,
                scored_slice,
                evaluation_mode="proxy",
                calibration_strategy="bundle_sigmoid" if calibration_applied else "model_probability",
                train_rows=window.train_end - window.train_start,
                calibration_rows=window.calibration_end - window.calibration_start,
            ),
        )

    if not roll_frames:
        return scored.iloc[0:0].copy(), roll_summaries, {
            "calibration_strategy": "bundle_sigmoid" if calibration_applied else "model_probability",
            "calibrated_rolls": int(calibration_applied) * len(roll_summaries),
            "uncalibrated_rolls": (0 if calibration_applied else len(roll_summaries)),
            "skipped_rolls": skipped_rolls,
        }

    return pd.concat(roll_frames, ignore_index=True), roll_summaries, {
        "calibration_strategy": "bundle_sigmoid" if calibration_applied else "model_probability",
        "calibrated_rolls": len(roll_summaries) if calibration_applied else 0,
        "uncalibrated_rolls": 0 if calibration_applied else len(roll_summaries),
        "skipped_rolls": skipped_rolls,
    }


def _evaluate_rolling_mode(
    bundle: Any,
    frame: pd.DataFrame,
    sample_weight: pd.Series,
    windows: list[NormalizedSplitWindow],
    training_config: dict[str, float | int],
    t_long: float,
    t_short: float,
    delta: float,
) -> tuple[pd.DataFrame, list[dict[str, Any]], dict[str, Any]]:
    roll_frames: list[pd.DataFrame] = []
    roll_summaries: list[dict[str, Any]] = []
    skipped_rolls: list[dict[str, str]] = []
    calibrated_rolls = 0
    partially_calibrated_rolls = 0

    for roll_idx, window in enumerate(windows):
        masks_np = split_window_masks(len(frame), window)
        masks = {name: pd.Series(mask, index=frame.index) for name, mask in masks_np.items()}
        train_rows = int(masks["train"].sum())
        calibration_rows = int(masks["calibration"].sum())
        test_rows = int(masks["test"].sum())
        if test_rows == 0:
            skipped_rolls.append({"split_id": window.split_id, "reason": "no_test_rows"})
            continue

        base_seed = int(training_config["random_seed"]) + (roll_idx * 2)
        try:
            long_model, long_scores, _ = train_model(
                "long",
                frame,
                bundle.feature_names,
                masks,
                sample_weight,
                int(training_config["iterations"]),
                float(training_config["learning_rate"]),
                int(training_config["depth"]),
                base_seed,
            )
            short_model, short_scores, _ = train_model(
                "short",
                frame,
                bundle.feature_names,
                masks,
                sample_weight,
                int(training_config["iterations"]),
                float(training_config["learning_rate"]),
                int(training_config["depth"]),
                base_seed + 1,
            )
        except Exception as exc:
            skipped_rolls.append({"split_id": window.split_id, "reason": str(exc)})
            continue

        test_features = frame.loc[masks["test"], bundle.feature_names]
        scored_slice = _score_frame(
            frame.loc[masks["test"]].copy(),
            long_model.predict(test_features, prediction_type="RawFormulaVal"),
            short_model.predict(test_features, prediction_type="RawFormulaVal"),
            long_model.predict_proba(test_features)[:, 1],
            short_model.predict_proba(test_features)[:, 1],
        )
        long_calibration = _fit_sigmoid_config(long_scores["raw_long"], long_scores["y_long"])
        short_calibration = _fit_sigmoid_config(short_scores["raw_short"], short_scores["y_short"])
        applied_count = 0
        if long_calibration is not None:
            scored_slice["p_long"] = apply_sigmoid(
                scored_slice["raw_long"].to_numpy(),
                long_calibration["coef"],
                long_calibration["intercept"],
            )
            applied_count += 1
        if short_calibration is not None:
            scored_slice["p_short"] = apply_sigmoid(
                scored_slice["raw_short"].to_numpy(),
                short_calibration["coef"],
                short_calibration["intercept"],
            )
            applied_count += 1

        if applied_count == 2:
            calibration_strategy = "per_roll_sigmoid"
            calibrated_rolls += 1
        elif applied_count == 1:
            calibration_strategy = "mixed_per_roll_sigmoid"
            partially_calibrated_rolls += 1
        else:
            calibration_strategy = "model_probability"

        long_calibration_prob = long_scores["p_long"].copy()
        short_calibration_prob = short_scores["p_short"].copy()
        if long_calibration is not None:
            long_calibration_prob = pd.Series(
                apply_sigmoid(
                    long_scores["raw_long"].to_numpy(),
                    long_calibration["coef"],
                    long_calibration["intercept"],
                ),
                index=long_scores.index,
            )
        if short_calibration is not None:
            short_calibration_prob = pd.Series(
                apply_sigmoid(
                    short_scores["raw_short"].to_numpy(),
                    short_calibration["coef"],
                    short_calibration["intercept"],
                ),
                index=short_scores.index,
            )
        long_auc_calibration = _safe_auc(long_scores["y_long"], long_calibration_prob)
        short_auc_calibration = _safe_auc(short_scores["y_short"], short_calibration_prob)
        long_logloss_calibration = _safe_logloss(long_scores["y_long"], long_calibration_prob)
        short_logloss_calibration = _safe_logloss(short_scores["y_short"], short_calibration_prob)

        scored_slice = _apply_policy_thresholds(scored_slice, t_long, t_short, delta)
        scored_slice["evaluated_split_id"] = window.split_id
        scored_slice["evaluation_mode"] = "rolling"
        roll_frames.append(scored_slice)
        roll_summaries.append(
            summarize_roll(
                window,
                scored_slice,
                evaluation_mode="rolling",
                calibration_strategy=calibration_strategy,
                train_rows=train_rows,
                calibration_rows=calibration_rows,
                long_auc_calibration=long_auc_calibration,
                short_auc_calibration=short_auc_calibration,
                long_logloss_calibration=long_logloss_calibration,
                short_logloss_calibration=short_logloss_calibration,
            ),
        )

    if not roll_frames:
        return frame.iloc[0:0].copy(), roll_summaries, {
            "calibration_strategy": "model_probability",
            "calibrated_rolls": calibrated_rolls,
            "partially_calibrated_rolls": partially_calibrated_rolls,
            "uncalibrated_rolls": 0,
            "skipped_rolls": skipped_rolls,
        }

    uncalibrated_rolls = max(
        0,
        len(roll_summaries) - calibrated_rolls - partially_calibrated_rolls,
    )
    return pd.concat(roll_frames, ignore_index=True), roll_summaries, {
        "calibration_strategy": (
            "per_roll_sigmoid"
            if partially_calibrated_rolls == 0 and uncalibrated_rolls == 0
            else "mixed"
        ),
        "calibrated_rolls": calibrated_rolls,
        "partially_calibrated_rolls": partially_calibrated_rolls,
        "uncalibrated_rolls": uncalibrated_rolls,
        "skipped_rolls": skipped_rolls,
    }


def main() -> int:
    args = parse_args()
    dataset = Path(args.dataset)
    bundle_dir = Path(args.bundle_dir)

    if not dataset.exists():
        raise SystemExit(f"dataset not found: {dataset}")
    if not bundle_dir.exists():
        raise SystemExit(f"bundle dir not found: {bundle_dir}")

    bundle = load_dataset_bundle(
        dataset,
        schema_path=_bundle_sidecar(bundle_dir, "feature_schema.json"),
        export_manifest_path=_bundle_sidecar(bundle_dir, "export_manifest.json"),
        split_manifest_path=_bundle_sidecar(bundle_dir, "split_manifest.json"),
    )
    manifest = _read_json(bundle_dir / "manifest.json") or {}
    frame, features = _build_base_frame(bundle)
    sample_weight = pd.Series(choose_sample_weight(bundle), index=frame.index)
    windows = resolve_walkforward_windows(bundle)

    if args.mode == "rolling":
        scored, roll_summaries, roll_metadata = _evaluate_rolling_mode(
            bundle,
            frame,
            sample_weight,
            windows,
            _load_training_config(bundle_dir),
            args.t_long,
            args.t_short,
            args.delta,
        )
    else:
        scored, roll_summaries, roll_metadata = _evaluate_proxy_mode(
            frame,
            features,
            windows,
            bundle_dir,
            args.t_long,
            args.t_short,
            args.delta,
        )

    if scored.empty:
        raise SystemExit("no walk-forward rows available")

    output_scores = _artifact_path(bundle_dir, "walkforward_test_scores.csv", args.artifact_suffix)
    scored.to_csv(output_scores, index=False)

    positive_rolls = sum(1 for row in roll_summaries if float(row["sum_policy_return"]) > 0.0)
    ticker_summaries = [
        summarize_ticker(str(ticker_id), ticker_frame.copy())
        for ticker_id, ticker_frame in scored.groupby("ticker_id", sort=True)
    ]
    positive_tickers = sum(
        1 for row in ticker_summaries if float(row["sum_policy_return"]) > 0.0
    )

    calibration_path = bundle_dir / "calibration.json"
    roll_summary_path = _artifact_path(bundle_dir, "walkforward_rolls.json", args.artifact_suffix)
    ticker_summary_path = _artifact_path(bundle_dir, "walkforward_tickers.json", args.artifact_suffix)
    walkforward_summary_path = _artifact_path(bundle_dir, "walkforward_summary.json", args.artifact_suffix)
    summary = {
        "bundle_id": str(manifest.get("bundle_id") or bundle_dir.name),
        "bundle_dir": str(bundle_dir),
        "dataset_path": str(bundle.dataset_path),
        "feature_schema_path": str(bundle.schema_path),
        "export_manifest_path": str(bundle.export_manifest_path) if bundle.export_manifest_path else "",
        "split_manifest_path": str(bundle.split_manifest_path) if bundle.split_manifest_path else "",
        "ticker_holdout": (bundle.export_manifest or {}).get("ticker_holdout"),
        "evaluation_mode": args.mode,
        "artifact_suffix": str(args.artifact_suffix or ""),
        "calibration_path_present": bool(calibration_path.exists()),
        "calibration_strategy": str(roll_metadata.get("calibration_strategy", "")),
        "calibrated_rolls": int(roll_metadata.get("calibrated_rolls", 0) or 0),
        "partially_calibrated_rolls": int(roll_metadata.get("partially_calibrated_rolls", 0) or 0),
        "uncalibrated_rolls": int(roll_metadata.get("uncalibrated_rolls", 0) or 0),
        "skipped_rolls": roll_metadata.get("skipped_rolls", []),
        "rows_scored": int(len(scored)),
        "expected_splits": int(len(windows)),
        "skipped_roll_count": int(len(roll_metadata.get("skipped_rolls", []))),
        "splits_scored": int(len(roll_summaries)),
        "tickers_scored": int(len(ticker_summaries)),
        "signals_long": int((scored["position"] == 1).sum()),
        "signals_short": int((scored["position"] == -1).sum()),
        "signals_flat": int((scored["position"] == 0).sum()),
        "active_share": float(((scored["position"] != 0).sum()) / len(scored)),
        "mean_policy_return": float(scored["policy_return"].mean()),
        "sum_policy_return": float(scored["policy_return"].sum()),
        "positive_roll_rate": float(positive_rolls / len(roll_summaries)),
        "positive_ticker_rate": float(positive_tickers / len(ticker_summaries))
        if ticker_summaries
        else 0.0,
        "thresholds": {
            "t_long": args.t_long,
            "t_short": args.t_short,
            "delta": args.delta,
        },
        "score_path": str(output_scores),
        "roll_summary_path": str(roll_summary_path),
        "ticker_summary_path": str(ticker_summary_path),
        "score_contract": {
            "walkforward_score_scope": "walkforward_test",
            "walkforward_probability_columns": ["p_long", "p_short"],
            "raw_margin_columns": ["raw_long", "raw_short"],
            "validation_scores_scope": "calibration_slice",
            "validation_scores_preferred_probability_columns": [
                "p_long_cal",
                "p_short_cal",
            ]
            if calibration_path.exists()
            else ["p_long", "p_short"],
            "walkforward_evaluation_mode": args.mode,
        },
        "parity_notice": {
            "desktop_parity": "proxy_only" if args.mode == "proxy" else "research_only",
            "details": (
                "walkforward_test_scores.csv is a research-side proxy evaluation and not a desktop validation or policy-sweep artifact."
                if args.mode == "proxy"
                else "walkforward_test_scores.csv was produced by per-roll retraining and recalibration, but it is still a research artifact and not desktop validation or policy-sweep parity evidence."
            ),
        },
    }
    if args.mode == "rolling":
        summary["rolling_calibration_summary"] = summarize_rolling_calibration(roll_summaries)

    dump_json(roll_summary_path, {"rolls": roll_summaries})
    dump_json(ticker_summary_path, {"tickers": ticker_summaries})
    dump_json(walkforward_summary_path, summary)

    print(f"wrote walk-forward scores -> {output_scores}")
    print(f"wrote roll summary -> {roll_summary_path}")
    print(f"wrote ticker summary -> {ticker_summary_path}")
    print(f"wrote walk-forward summary -> {walkforward_summary_path}")
    print(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
