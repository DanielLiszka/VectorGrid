from __future__ import annotations

import argparse
import json
from pathlib import Path

import pandas as pd
from sklearn.linear_model import LogisticRegression

from common import apply_sigmoid


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Calibrate MlSignal model scores into stored calibration artifacts.",
    )
    parser.add_argument("--bundle-dir", required=True, help="Model bundle directory.")
    parser.add_argument("--scores-csv", default="", help="Validation score CSV.")
    return parser.parse_args()


def fit_sigmoid(raw: pd.Series, target: pd.Series) -> tuple[float, float]:
    model = LogisticRegression(solver="lbfgs", max_iter=200)
    model.fit(raw.to_frame(name="raw"), target.astype(int))
    return float(model.coef_[0][0]), float(model.intercept_[0])


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

    long_coef, long_intercept = fit_sigmoid(frame["raw_long"], frame["y_long"])
    short_coef, short_intercept = fit_sigmoid(frame["raw_short"], frame["y_short"])

    frame["p_long_cal"] = apply_sigmoid(frame["raw_long"].to_numpy(), long_coef, long_intercept)
    frame["p_short_cal"] = apply_sigmoid(frame["raw_short"].to_numpy(), short_coef, short_intercept)

    calibration = {
        "kind": "sigmoid_pair",
        "long": {"coef": long_coef, "intercept": long_intercept},
        "short": {"coef": short_coef, "intercept": short_intercept},
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
            manifest["score_contract"] = {
                "validation_scores_scope": "calibration_slice",
                "validation_scores_probability_columns": ["p_long", "p_short"],
                "validation_scores_preferred_probability_columns": [
                    "p_long_cal",
                    "p_short_cal",
                ],
                "raw_margin_columns": ["raw_long", "raw_short"],
                "desktop_validation_requires_probability_columns": True,
            }
            with manifest_path.open("w", encoding="utf-8") as handle:
                json.dump(manifest, handle, indent=2, sort_keys=True)
                handle.write("\n")

    print(f"wrote calibration -> {calibration_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
