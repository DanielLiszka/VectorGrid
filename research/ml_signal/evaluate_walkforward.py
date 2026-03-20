from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import pandas as pd

from common import (
    apply_sigmoid,
    build_feature_frame,
    dump_json,
    load_dataset_bundle,
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
    parser.add_argument("--t-long", type=float, default=0.6)
    parser.add_argument("--t-short", type=float, default=0.6)
    parser.add_argument("--delta", type=float, default=0.1)
    return parser.parse_args()


def normalize_windows(
    bundle: Any,
) -> list[dict[str, int | str]]:
    split_manifest = bundle.split_manifest or {}
    windows = split_manifest.get("windows")
    if not isinstance(windows, list) or not windows:
        return []

    export_manifest = bundle.export_manifest or {}
    first_valid_bar = int(export_manifest.get("first_valid_bar", bundle.first_valid_bar) or 0)
    row_count = len(bundle.dataset)
    normalized: list[dict[str, int | str]] = []

    for idx, raw_window in enumerate(windows):
        if not isinstance(raw_window, dict):
            continue
        split_id = str(raw_window.get("split_id", f"roll_{idx:03}")).strip() or f"roll_{idx:03}"
        test_start = max(0, int(raw_window.get("test_start", 0)) - first_valid_bar)
        test_end = max(test_start, int(raw_window.get("test_end", row_count)) - first_valid_bar)
        normalized.append(
            {
                "split_id": split_id,
                "test_start": min(test_start, row_count),
                "test_end": min(test_end, row_count),
            },
        )
    return normalized


def summarize_roll(
    split_id: str,
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
        "split_id": split_id,
        "rows_scored": rows_scored,
        "signals_long": long_count,
        "signals_short": short_count,
        "signals_flat": flat_count,
        "active_share": active_share,
        "mean_policy_return": mean_return,
        "sum_policy_return": sum_return,
    }


def main() -> int:
    args = parse_args()
    dataset = Path(args.dataset)
    bundle_dir = Path(args.bundle_dir)

    if not dataset.exists():
        raise SystemExit(f"dataset not found: {dataset}")
    if not bundle_dir.exists():
        raise SystemExit(f"bundle dir not found: {bundle_dir}")

    bundle = load_dataset_bundle(dataset)
    frame = bundle.dataset.copy()
    features = build_feature_frame(bundle)

    long_model = CatBoostClassifier()
    short_model = CatBoostClassifier()
    long_model.load_model(str(bundle_dir / "model_long.cbm"))
    short_model.load_model(str(bundle_dir / "model_short.cbm"))

    calibration_path = bundle_dir / "calibration.json"
    calibration = None
    if calibration_path.exists():
        import json

        with calibration_path.open("r", encoding="utf-8") as handle:
            calibration = json.load(handle)

    scored = frame.loc[:, ["timestamp", "ticker_id", "split_id", "r_long", "r_short"]].copy()
    scored["raw_long"] = long_model.predict(features, prediction_type="RawFormulaVal")
    scored["raw_short"] = short_model.predict(features, prediction_type="RawFormulaVal")
    scored["p_long"] = long_model.predict_proba(features)[:, 1]
    scored["p_short"] = short_model.predict_proba(features)[:, 1]

    if isinstance(calibration, dict):
        long_cfg = calibration.get("long") or {}
        short_cfg = calibration.get("short") or {}
        if "coef" in long_cfg and "intercept" in long_cfg:
            scored["p_long"] = apply_sigmoid(
                scored["raw_long"].to_numpy(),
                float(long_cfg["coef"]),
                float(long_cfg["intercept"]),
            )
        if "coef" in short_cfg and "intercept" in short_cfg:
            scored["p_short"] = apply_sigmoid(
                scored["raw_short"].to_numpy(),
                float(short_cfg["coef"]),
                float(short_cfg["intercept"]),
            )

    long_cond = (scored["p_long"] >= args.t_long) & (
        (scored["p_long"] - scored["p_short"]) >= args.delta
    )
    short_cond = (scored["p_short"] >= args.t_short) & (
        (scored["p_short"] - scored["p_long"]) >= args.delta
    )
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

    windows = normalize_windows(bundle)
    roll_summaries: list[dict[str, Any]] = []
    if windows:
        roll_frames: list[pd.DataFrame] = []
        for window in windows:
            test_start = int(window["test_start"])
            test_end = int(window["test_end"])
            split_id = str(window["split_id"])
            if test_end <= test_start:
                continue
            scored_slice = scored.iloc[test_start:test_end].copy()
            if scored_slice.empty:
                continue
            scored_slice["evaluated_split_id"] = split_id
            roll_frames.append(scored_slice)
            roll_summaries.append(summarize_roll(split_id, scored_slice))
        scored = pd.concat(roll_frames, ignore_index=True) if roll_frames else scored.iloc[0:0].copy()
    else:
        fallback_start = max(0, int(len(scored) * 0.85))
        scored = scored.iloc[fallback_start:].copy()
        roll_summaries.append(summarize_roll("fallback_test", scored))

    if scored.empty:
        raise SystemExit("no walk-forward rows available")

    output_scores = bundle_dir / "walkforward_test_scores.csv"
    scored.to_csv(output_scores, index=False)

    positive_rolls = sum(1 for row in roll_summaries if float(row["sum_policy_return"]) > 0.0)
    summary = {
        "rows_scored": int(len(scored)),
        "splits_scored": int(len(roll_summaries)),
        "signals_long": int((scored["position"] == 1).sum()),
        "signals_short": int((scored["position"] == -1).sum()),
        "signals_flat": int((scored["position"] == 0).sum()),
        "active_share": float(((scored["position"] != 0).sum()) / len(scored)),
        "mean_policy_return": float(scored["policy_return"].mean()),
        "sum_policy_return": float(scored["policy_return"].sum()),
        "positive_roll_rate": float(positive_rolls / len(roll_summaries)),
        "thresholds": {
            "t_long": args.t_long,
            "t_short": args.t_short,
            "delta": args.delta,
        },
        "score_path": str(output_scores),
        "roll_summary_path": str(bundle_dir / "walkforward_rolls.json"),
    }

    dump_json(bundle_dir / "walkforward_rolls.json", {"rolls": roll_summaries})
    dump_json(bundle_dir / "walkforward_summary.json", summary)

    print(f"wrote walk-forward scores -> {output_scores}")
    print(f"wrote roll summary -> {bundle_dir / 'walkforward_rolls.json'}")
    print(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
