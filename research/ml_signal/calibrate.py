from __future__ import annotations

import argparse
import json
from pathlib import Path

import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import brier_score_loss, log_loss, roc_auc_score

from common import apply_sigmoid


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Calibrate MlSignal model scores into stored calibration artifacts.",
    )
    parser.add_argument("--bundle-dir", required=True, help="Model bundle directory.")
    parser.add_argument("--scores-csv", default="", help="Validation score CSV.")
    return parser.parse_args()


def fit_sigmoid(raw: pd.Series, target: pd.Series) -> tuple[float, float]:
    if len(raw) == 0 or len(target) == 0:
        raise ValueError("calibration slice is empty")
    if target.nunique(dropna=True) < 2:
        raise ValueError("calibration target requires both classes")
    model = LogisticRegression(solver="lbfgs", max_iter=200)
    model.fit(raw.to_frame(name="raw"), target.astype(int))
    return float(model.coef_[0][0]), float(model.intercept_[0])


def _safe_auc(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        if y_true.nunique(dropna=True) < 2:
            return None
        return float(roc_auc_score(y_true.astype(int), y_prob.astype(float)))
    except Exception:
        return None


def _safe_logloss(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        return float(log_loss(y_true.astype(int), y_prob.astype(float), labels=[0, 1]))
    except Exception:
        return None


def _safe_brier(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        return float(brier_score_loss(y_true.astype(int), y_prob.astype(float)))
    except Exception:
        return None


def _optional_series(frame: pd.DataFrame, column: str, mask: pd.Series) -> pd.Series | None:
    if column not in frame.columns:
        return None
    return frame.loc[mask, column]


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

    long_coef, long_intercept = fit_sigmoid(frame.loc[long_valid, "raw_long"], frame.loc[long_valid, "y_long"])
    short_coef, short_intercept = fit_sigmoid(frame.loc[short_valid, "raw_short"], frame.loc[short_valid, "y_short"])

    frame["p_long_cal"] = pd.NA
    frame["p_short_cal"] = pd.NA
    frame.loc[long_valid, "p_long_cal"] = apply_sigmoid(
        frame.loc[long_valid, "raw_long"].to_numpy(),
        long_coef,
        long_intercept,
    )
    frame.loc[short_valid, "p_short_cal"] = apply_sigmoid(
        frame.loc[short_valid, "raw_short"].to_numpy(),
        short_coef,
        short_intercept,
    )

    calibration = {
        "kind": "sigmoid_pair",
        "rows_used": {
            "long": int(long_valid.sum()),
            "short": int(short_valid.sum()),
        },
        "long": {"coef": long_coef, "intercept": long_intercept},
        "short": {"coef": short_coef, "intercept": short_intercept},
    }
    calibration_summary = {
        "kind": "sigmoid_pair",
        "validation_scores_scope": "calibration_slice",
        "long": {
            "rows_used": int(long_valid.sum()),
            "raw_auc": _safe_auc(frame.loc[long_valid, "y_long"], frame.loc[long_valid, "raw_long"]),
            "calibrated_auc": _safe_auc(frame.loc[long_valid, "y_long"], frame.loc[long_valid, "p_long_cal"]),
            "raw_logloss": _safe_logloss(
                frame.loc[long_valid, "y_long"],
                _optional_series(frame, "p_long", long_valid),
            )
            if "p_long" in frame.columns
            else None,
            "calibrated_logloss": _safe_logloss(frame.loc[long_valid, "y_long"], frame.loc[long_valid, "p_long_cal"]),
            "raw_brier": _safe_brier(
                frame.loc[long_valid, "y_long"],
                _optional_series(frame, "p_long", long_valid),
            )
            if "p_long" in frame.columns
            else None,
            "calibrated_brier": _safe_brier(frame.loc[long_valid, "y_long"], frame.loc[long_valid, "p_long_cal"]),
        },
        "short": {
            "rows_used": int(short_valid.sum()),
            "raw_auc": _safe_auc(frame.loc[short_valid, "y_short"], frame.loc[short_valid, "raw_short"]),
            "calibrated_auc": _safe_auc(frame.loc[short_valid, "y_short"], frame.loc[short_valid, "p_short_cal"]),
            "raw_logloss": _safe_logloss(
                frame.loc[short_valid, "y_short"],
                _optional_series(frame, "p_short", short_valid),
            )
            if "p_short" in frame.columns
            else None,
            "calibrated_logloss": _safe_logloss(frame.loc[short_valid, "y_short"], frame.loc[short_valid, "p_short_cal"]),
            "raw_brier": _safe_brier(
                frame.loc[short_valid, "y_short"],
                _optional_series(frame, "p_short", short_valid),
            )
            if "p_short" in frame.columns
            else None,
            "calibrated_brier": _safe_brier(frame.loc[short_valid, "y_short"], frame.loc[short_valid, "p_short_cal"]),
        },
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
            score_contract["calibration_kind"] = "sigmoid_pair"
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
