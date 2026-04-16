from __future__ import annotations

import argparse
import json
from pathlib import Path

import pandas as pd

from calibration_utils import (
    apply_calibration_config,
    fit_calibration_config,
    fit_sigmoid,
    infer_calibration_method,
    summarize_calibration_diagnostics,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Calibrate MlSignal model scores into stored calibration artifacts.",
    )
    parser.add_argument("--bundle-dir", required=True, help="Model bundle directory.")
    parser.add_argument("--scores-csv", default="", help="Validation score CSV.")
    parser.add_argument(
        "--method",
        choices=["sigmoid", "isotonic"],
        default="sigmoid",
        help="Calibration method fitted from raw model margins.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    bundle_dir = Path(args.bundle_dir)
    scores_csv = Path(args.scores_csv) if args.scores_csv else bundle_dir / "validation_scores.csv"

    if not bundle_dir.exists():
        raise SystemExit(f"bundle dir not found: {bundle_dir}")
    if not scores_csv.exists():
        raise SystemExit(f"scores csv not found: {scores_csv}")

    frame = pd.read_csv(scores_csv)
    required = ["raw_long", "raw_short", "y_long", "y_short"]
    missing = [column for column in required if column not in frame.columns]
    if missing:
        raise SystemExit(f"scores csv is missing columns: {missing}")

    long_valid = frame["raw_long"].notna() & frame["y_long"].notna()
    short_valid = frame["raw_short"].notna() & frame["y_short"].notna()
    if int(long_valid.sum()) == 0:
        raise SystemExit("scores csv has no valid long calibration rows")
    if int(short_valid.sum()) == 0:
        raise SystemExit("scores csv has no valid short calibration rows")

    long_config = fit_calibration_config(
        args.method,
        frame.loc[long_valid, "raw_long"],
        frame.loc[long_valid, "y_long"],
    )
    short_config = fit_calibration_config(
        args.method,
        frame.loc[short_valid, "raw_short"],
        frame.loc[short_valid, "y_short"],
    )
    method = infer_calibration_method(long_config, default=args.method)

    frame["p_long_cal"] = pd.NA
    frame["p_short_cal"] = pd.NA
    frame.loc[long_valid, "p_long_cal"] = apply_calibration_config(
        frame.loc[long_valid, "raw_long"].to_numpy(),
        long_config,
    )
    frame.loc[short_valid, "p_short_cal"] = apply_calibration_config(
        frame.loc[short_valid, "raw_short"].to_numpy(),
        short_config,
    )

    calibration = {
        "kind": f"{method}_pair",
        "method": method,
        "rows_used": {
            "long": int(long_valid.sum()),
            "short": int(short_valid.sum()),
        },
        "long": long_config,
        "short": short_config,
    }
    calibration_summary = {
        "kind": f"{method}_pair",
        "method": method,
        "validation_scores_scope": "calibration_slice",
        "long": summarize_calibration_diagnostics(
            method=method,
            y_true=frame.loc[long_valid, "y_long"],
            raw_score=frame.loc[long_valid, "raw_long"],
            raw_probability=frame.loc[long_valid, "p_long"] if "p_long" in frame.columns else None,
            calibrated_probability=frame.loc[long_valid, "p_long_cal"],
        ),
        "short": summarize_calibration_diagnostics(
            method=method,
            y_true=frame.loc[short_valid, "y_short"],
            raw_score=frame.loc[short_valid, "raw_short"],
            raw_probability=frame.loc[short_valid, "p_short"] if "p_short" in frame.columns else None,
            calibrated_probability=frame.loc[short_valid, "p_short_cal"],
        ),
    }
    calibration_path = bundle_dir / "calibration.json"
    with calibration_path.open("w", encoding="utf-8") as handle:
        json.dump(calibration, handle, indent=2, sort_keys=True)
        handle.write("\n")

    frame.to_csv(bundle_dir / "validation_scores.csv", index=False)

    manifest_path = bundle_dir / "manifest.json"
    if manifest_path.exists():
        with manifest_path.open("r", encoding="utf-8") as handle:
            manifest = json.load(handle)
        if isinstance(manifest, dict):
            manifest["calibration_path"] = "calibration.json"
            score_contract = manifest.get("score_contract")
            if not isinstance(score_contract, dict):
                score_contract = {}
            score_contract["validation_scores_scope"] = "calibration_slice"
            score_contract["validation_scores_probability_columns"] = ["p_long", "p_short"]
            score_contract["validation_scores_preferred_probability_columns"] = [
                "p_long_cal",
                "p_short_cal",
            ]
            score_contract["validation_scores_preferred_probability_columns_after_calibration"] = [
                "p_long_cal",
                "p_short_cal",
            ]
            score_contract["raw_margin_columns"] = ["raw_long", "raw_short"]
            score_contract["desktop_validation_requires_probability_columns"] = True
            score_contract["calibration_kind"] = f"{method}_pair"
            score_contract["calibration_method"] = method
            manifest["score_contract"] = score_contract
            manifest["calibration_summary"] = calibration_summary
            with manifest_path.open("w", encoding="utf-8") as handle:
                json.dump(manifest, handle, indent=2, sort_keys=True)
                handle.write("\n")

    train_metadata_path = bundle_dir / "train_metadata.json"
    if train_metadata_path.exists():
        with train_metadata_path.open("r", encoding="utf-8") as handle:
            train_metadata = json.load(handle)
        if isinstance(train_metadata, dict):
            train_metadata["calibration_path"] = "calibration.json"
            train_metadata["calibration_summary"] = calibration_summary
            with train_metadata_path.open("w", encoding="utf-8") as handle:
                json.dump(train_metadata, handle, indent=2, sort_keys=True)
                handle.write("\n")

    print(f"wrote calibration -> {calibration_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
