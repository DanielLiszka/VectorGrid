from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import math
import pandas as pd
from sklearn.linear_model import LogisticRegression

from common import (
    NormalizedSplitWindow,
    build_feature_frame,
    choose_sample_weight,
    dump_json,
    load_dataset_bundle,
    resolve_walkforward_windows,
    split_window_masks,
)


def _artifact_path(base_dir: Path, stem: str, suffix: str, extension: str) -> Path:
    clean_suffix = str(suffix).strip().strip(".")
    if clean_suffix:
        return base_dir / f"{stem}.{clean_suffix}.{extension}"
    return base_dir / f"{stem}.{extension}"


def _read_json(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    import json

    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    return raw if isinstance(raw, dict) else None


def _score_frame(
    frame: pd.DataFrame,
    long_raw: pd.Series,
    short_raw: pd.Series,
    long_prob: pd.Series,
    short_prob: pd.Series,
) -> pd.DataFrame:
    scored = frame.loc[:, ["timestamp", "ticker_id", "split_id", "r_long", "r_short"]].copy()
    scored["raw_long"] = long_raw.to_numpy(dtype=float)
    scored["raw_short"] = short_raw.to_numpy(dtype=float)
    scored["p_long"] = long_prob.to_numpy(dtype=float)
    scored["p_short"] = short_prob.to_numpy(dtype=float)
    return scored


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
    scored.loc[scored["position"] == 1, "policy_return"] = scored.loc[scored["position"] == 1, "r_long"]
    scored.loc[scored["position"] == -1, "policy_return"] = scored.loc[scored["position"] == -1, "r_short"]
    return scored


def _require_binary_target_slice(name: str, target: pd.Series) -> None:
    if target.nunique(dropna=True) < 2:
        raise ValueError(f"{name} target requires both classes")


def _fit_logistic_baseline(
    name: str,
    frame: pd.DataFrame,
    feature_names: list[str],
    train_mask: pd.Series,
    sample_weight: pd.Series,
) -> LogisticRegression:
    target_col = f"y_{name}"
    keep_col = f"keep_{name}"
    usable_mask = train_mask & (frame[keep_col].fillna(0).astype(int) == 1)
    if int(usable_mask.sum()) < 25:
        raise ValueError(f"not enough training rows for {name}")
    _require_binary_target_slice(name, frame.loc[usable_mask, target_col])
    model = LogisticRegression(max_iter=300, solver="lbfgs")
    model.fit(
        frame.loc[usable_mask, feature_names],
        frame.loc[usable_mask, target_col].astype(int),
        sample_weight=sample_weight.loc[usable_mask],
    )
    return model


def _predict_logistic_baseline(
    model: LogisticRegression,
    features: pd.DataFrame,
) -> tuple[pd.Series, pd.Series]:
    probability = pd.Series(model.predict_proba(features)[:, 1], index=features.index, dtype=float)
    raw = pd.Series(model.decision_function(features), index=features.index, dtype=float)
    return raw, probability


def _fit_prior_probability(
    name: str,
    frame: pd.DataFrame,
    train_mask: pd.Series,
) -> float:
    target_col = f"y_{name}"
    keep_col = f"keep_{name}"
    usable_mask = train_mask & (frame[keep_col].fillna(0).astype(int) == 1)
    if int(usable_mask.sum()) < 1:
        raise ValueError(f"not enough training rows for {name}")
    return float(frame.loc[usable_mask, target_col].astype(int).mean())


def _predict_prior_probability(
    probability: float,
    index: pd.Index,
) -> tuple[pd.Series, pd.Series]:
    clipped_probability = min(max(float(probability), 1e-6), 1.0 - 1e-6)
    raw_margin = float(math.log(clipped_probability / (1.0 - clipped_probability)))
    raw = pd.Series([raw_margin] * len(index), index=index, dtype=float)
    prob = pd.Series([clipped_probability] * len(index), index=index, dtype=float)
    return raw, prob


def summarize_roll(window: NormalizedSplitWindow, scored_slice: pd.DataFrame, evaluation_mode: str) -> dict[str, Any]:
    rows_scored = int(len(scored_slice))
    long_count = int((scored_slice["position"] == 1).sum())
    short_count = int((scored_slice["position"] == -1).sum())
    flat_count = int((scored_slice["position"] == 0).sum())
    active_count = long_count + short_count
    return {
        "split_id": window.split_id,
        "evaluation_mode": evaluation_mode,
        "rows_scored": rows_scored,
        "signals_long": long_count,
        "signals_short": short_count,
        "signals_flat": flat_count,
        "active_share": float(active_count / rows_scored) if rows_scored else 0.0,
        "mean_policy_return": float(scored_slice["policy_return"].mean()) if rows_scored else 0.0,
        "sum_policy_return": float(scored_slice["policy_return"].sum()) if rows_scored else 0.0,
        "train_start": int(window.train_start),
        "train_end": int(window.train_end),
        "calibration_start": int(window.calibration_start),
        "calibration_end": int(window.calibration_end),
        "test_start": int(window.test_start),
        "test_end": int(window.test_end),
    }


def summarize_ticker(ticker_id: str, scored_slice: pd.DataFrame) -> dict[str, Any]:
    rows_scored = int(len(scored_slice))
    long_count = int((scored_slice["position"] == 1).sum())
    short_count = int((scored_slice["position"] == -1).sum())
    flat_count = int((scored_slice["position"] == 0).sum())
    active_count = long_count + short_count
    return {
        "ticker_id": ticker_id,
        "rows_scored": rows_scored,
        "signals_long": long_count,
        "signals_short": short_count,
        "signals_flat": flat_count,
        "active_share": float(active_count / rows_scored) if rows_scored else 0.0,
        "mean_policy_return": float(scored_slice["policy_return"].mean()) if rows_scored else 0.0,
        "sum_policy_return": float(scored_slice["policy_return"].sum()) if rows_scored else 0.0,
    }


def run_baseline_walkforward(
    *,
    dataset: str | Path,
    bundle_dir: str | Path,
    baseline_kind: str,
    mode: str,
    t_long: float,
    t_short: float,
    delta: float,
    artifact_suffix: str = "",
) -> dict[str, Any]:
    dataset_path = Path(dataset)
    bundle_path = Path(bundle_dir)
    bundle = load_dataset_bundle(
        dataset_path,
        schema_path=(bundle_path / "feature_schema.json") if (bundle_path / "feature_schema.json").exists() else None,
        export_manifest_path=(bundle_path / "export_manifest.json")
        if (bundle_path / "export_manifest.json").exists()
        else None,
        split_manifest_path=(bundle_path / "split_manifest.json")
        if (bundle_path / "split_manifest.json").exists()
        else None,
    )
    manifest = _read_json(bundle_path / "manifest.json") or {}
    frame = bundle.dataset.copy()
    features = build_feature_frame(bundle)
    for column in bundle.feature_names:
        frame[column] = features[column]
    sample_weight = pd.Series(choose_sample_weight(bundle), index=frame.index)
    windows = resolve_walkforward_windows(bundle)

    roll_frames: list[pd.DataFrame] = []
    roll_summaries: list[dict[str, Any]] = []
    skipped_rolls: list[dict[str, str]] = []

    if mode == "proxy":
        selected_window = windows[-1]
        masks = {name: pd.Series(mask, index=frame.index) for name, mask in split_window_masks(len(frame), selected_window).items()}
        try:
            if baseline_kind == "logistic_regression":
                long_model = _fit_logistic_baseline("long", frame, bundle.feature_names, masks["train"], sample_weight)
                short_model = _fit_logistic_baseline("short", frame, bundle.feature_names, masks["train"], sample_weight)
                long_raw, long_prob = _predict_logistic_baseline(long_model, frame.loc[:, bundle.feature_names])
                short_raw, short_prob = _predict_logistic_baseline(short_model, frame.loc[:, bundle.feature_names])
            elif baseline_kind == "class_prior":
                long_prior = _fit_prior_probability("long", frame, masks["train"])
                short_prior = _fit_prior_probability("short", frame, masks["train"])
                long_raw, long_prob = _predict_prior_probability(long_prior, frame.index)
                short_raw, short_prob = _predict_prior_probability(short_prior, frame.index)
            else:
                raise ValueError(f"unsupported baseline kind: {baseline_kind}")
        except Exception as exc:
            raise SystemExit(f"failed to fit baseline {baseline_kind}: {exc}") from exc

        scored = _apply_policy_thresholds(
            _score_frame(frame, long_raw, short_raw, long_prob, short_prob),
            t_long,
            t_short,
            delta,
        )
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
            roll_summaries.append(summarize_roll(window, scored_slice, "proxy"))
    else:
        for window in windows:
            masks = {name: pd.Series(mask, index=frame.index) for name, mask in split_window_masks(len(frame), window).items()}
            test_mask = masks["test"]
            if int(test_mask.sum()) == 0:
                skipped_rolls.append({"split_id": window.split_id, "reason": "no_test_rows"})
                continue
            try:
                if baseline_kind == "logistic_regression":
                    long_model = _fit_logistic_baseline("long", frame, bundle.feature_names, masks["train"], sample_weight)
                    short_model = _fit_logistic_baseline("short", frame, bundle.feature_names, masks["train"], sample_weight)
                    test_features = frame.loc[test_mask, bundle.feature_names]
                    long_raw, long_prob = _predict_logistic_baseline(long_model, test_features)
                    short_raw, short_prob = _predict_logistic_baseline(short_model, test_features)
                elif baseline_kind == "class_prior":
                    long_prior = _fit_prior_probability("long", frame, masks["train"])
                    short_prior = _fit_prior_probability("short", frame, masks["train"])
                    test_index = frame.index[test_mask]
                    long_raw, long_prob = _predict_prior_probability(long_prior, test_index)
                    short_raw, short_prob = _predict_prior_probability(short_prior, test_index)
                else:
                    raise ValueError(f"unsupported baseline kind: {baseline_kind}")
            except Exception as exc:
                skipped_rolls.append({"split_id": window.split_id, "reason": str(exc)})
                continue

            scored_slice = _apply_policy_thresholds(
                _score_frame(
                    frame.loc[test_mask].copy(),
                    long_raw,
                    short_raw,
                    long_prob,
                    short_prob,
                ),
                t_long,
                t_short,
                delta,
            )
            scored_slice["evaluated_split_id"] = window.split_id
            scored_slice["evaluation_mode"] = "rolling"
            roll_frames.append(scored_slice)
            roll_summaries.append(summarize_roll(window, scored_slice, "rolling"))

    if not roll_frames:
        raise SystemExit("no baseline walk-forward rows available")

    scored = pd.concat(roll_frames, ignore_index=True)
    positive_rolls = sum(1 for row in roll_summaries if float(row["sum_policy_return"]) > 0.0)
    ticker_summaries = [
        summarize_ticker(str(ticker_id), ticker_frame.copy())
        for ticker_id, ticker_frame in scored.groupby("ticker_id", sort=True)
    ]
    positive_tickers = sum(1 for row in ticker_summaries if float(row["sum_policy_return"]) > 0.0)

    stem = f"built_in_baseline_{baseline_kind}"
    score_path = _artifact_path(bundle_path, f"{stem}_scores", artifact_suffix, "csv")
    rolls_path = _artifact_path(bundle_path, f"{stem}_rolls", artifact_suffix, "json")
    tickers_path = _artifact_path(bundle_path, f"{stem}_tickers", artifact_suffix, "json")
    summary_path = _artifact_path(bundle_path, f"{stem}_summary", artifact_suffix, "json")
    scored.to_csv(score_path, index=False)

    summary = {
        "bundle_id": str(manifest.get("bundle_id") or bundle_path.name),
        "bundle_dir": str(bundle_path),
        "dataset_path": str(bundle.dataset_path),
        "feature_schema_path": str(bundle.schema_path),
        "export_manifest_path": str(bundle.export_manifest_path) if bundle.export_manifest_path else "",
        "split_manifest_path": str(bundle.split_manifest_path) if bundle.split_manifest_path else "",
        "evaluation_mode": mode,
        "artifact_suffix": str(artifact_suffix or ""),
        "baseline_kind": baseline_kind,
        "baseline_label": f"built_in_{baseline_kind}",
        "baseline_model_family": (
            "sklearn_logistic_regression_binary_pair"
            if baseline_kind == "logistic_regression"
            else "class_prior_probability_pair"
        ),
        "rows_scored": int(len(scored)),
        "expected_splits": int(len(windows)),
        "skipped_roll_count": int(len(skipped_rolls)),
        "skipped_rolls": skipped_rolls,
        "splits_scored": int(len(roll_summaries)),
        "tickers_scored": int(len(ticker_summaries)),
        "signals_long": int((scored["position"] == 1).sum()),
        "signals_short": int((scored["position"] == -1).sum()),
        "signals_flat": int((scored["position"] == 0).sum()),
        "active_share": float(((scored["position"] != 0).sum()) / len(scored)),
        "mean_policy_return": float(scored["policy_return"].mean()),
        "sum_policy_return": float(scored["policy_return"].sum()),
        "positive_roll_rate": float(positive_rolls / len(roll_summaries)) if roll_summaries else 0.0,
        "positive_ticker_rate": float(positive_tickers / len(ticker_summaries))
        if ticker_summaries
        else 0.0,
        "thresholds": {
            "t_long": float(t_long),
            "t_short": float(t_short),
            "delta": float(delta),
        },
        "score_path": str(score_path),
        "roll_summary_path": str(rolls_path),
        "ticker_summary_path": str(tickers_path),
        "score_contract": {
            "walkforward_score_scope": "walkforward_test",
            "walkforward_probability_columns": ["p_long", "p_short"],
            "raw_margin_columns": ["raw_long", "raw_short"],
            "walkforward_evaluation_mode": mode,
        },
    }

    dump_json(rolls_path, {"rolls": roll_summaries})
    dump_json(tickers_path, {"tickers": ticker_summaries})
    dump_json(summary_path, summary)
    return {
        "summary": summary,
        "summary_path": str(summary_path),
        "score_path": str(score_path),
        "roll_summary_path": str(rolls_path),
        "ticker_summary_path": str(tickers_path),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate a simple built-in MlSignal walk-forward baseline.",
    )
    parser.add_argument("--dataset", required=True, help="Exported ML dataset CSV.")
    parser.add_argument("--bundle-dir", required=True, help="Model bundle directory.")
    parser.add_argument(
        "--baseline-kind",
        choices=["logistic_regression", "class_prior"],
        default="logistic_regression",
        help="Built-in baseline kind to generate.",
    )
    parser.add_argument(
        "--mode",
        choices=["proxy", "rolling"],
        default="proxy",
        help="Evaluation mode for the built-in baseline.",
    )
    parser.add_argument("--t-long", type=float, default=0.6)
    parser.add_argument("--t-short", type=float, default=0.6)
    parser.add_argument("--delta", type=float, default=0.1)
    parser.add_argument(
        "--artifact-suffix",
        default="",
        help="Optional suffix appended to baseline artifact filenames.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    result = run_baseline_walkforward(
        dataset=args.dataset,
        bundle_dir=args.bundle_dir,
        baseline_kind=args.baseline_kind,
        mode=args.mode,
        t_long=args.t_long,
        t_short=args.t_short,
        delta=args.delta,
        artifact_suffix=args.artifact_suffix,
    )
    print(f"wrote built-in baseline summary -> {result['summary_path']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
