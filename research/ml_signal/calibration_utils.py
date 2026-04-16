from __future__ import annotations

from typing import Any

import numpy as np
import pandas as pd
from sklearn.isotonic import IsotonicRegression
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import brier_score_loss, log_loss, roc_auc_score

from common import apply_sigmoid

PROBABILITY_EPSILON = 1e-6


def clip_probability(values: np.ndarray | pd.Series) -> np.ndarray:
    return np.clip(np.asarray(values, dtype=float), PROBABILITY_EPSILON, 1.0 - PROBABILITY_EPSILON)


def fit_sigmoid(raw: pd.Series, target: pd.Series) -> tuple[float, float]:
    if len(raw) == 0 or len(target) == 0:
        raise ValueError("calibration slice is empty")
    if target.nunique(dropna=True) < 2:
        raise ValueError("calibration target requires both classes")
    model = LogisticRegression(solver="lbfgs", max_iter=200)
    model.fit(raw.to_frame(name="raw"), target.astype(int))
    return float(model.coef_[0][0]), float(model.intercept_[0])


def fit_isotonic(raw: pd.Series, target: pd.Series) -> tuple[list[float], list[float]]:
    if len(raw) == 0 or len(target) == 0:
        raise ValueError("calibration slice is empty")
    if target.nunique(dropna=True) < 2:
        raise ValueError("calibration target requires both classes")
    model = IsotonicRegression(out_of_bounds="clip", y_min=0.0, y_max=1.0)
    model.fit(raw.astype(float), target.astype(int))
    return (
        [float(value) for value in model.X_thresholds_],
        [float(value) for value in model.y_thresholds_],
    )


def infer_calibration_method(config: dict[str, Any] | None, default: str = "sigmoid") -> str:
    if not isinstance(config, dict):
        return default
    method = str(config.get("method") or "").strip().lower()
    if method in {"sigmoid", "isotonic", "none"}:
        return method
    kind = str(config.get("kind") or "").strip().lower()
    if "isotonic" in kind:
        return "isotonic"
    if "sigmoid" in kind:
        return "sigmoid"
    if "coef" in config and "intercept" in config:
        return "sigmoid"
    if "x_thresholds" in config and "y_thresholds" in config:
        return "isotonic"
    return default


def fit_calibration_config(method: str, raw: pd.Series, target: pd.Series) -> dict[str, Any]:
    normalized_method = infer_calibration_method({"method": method}, default="sigmoid")
    if normalized_method == "none":
        raise ValueError("calibration method 'none' cannot be fit")
    if normalized_method == "sigmoid":
        coef, intercept = fit_sigmoid(raw, target)
        return {
            "method": "sigmoid",
            "coef": float(coef),
            "intercept": float(intercept),
        }
    if normalized_method == "isotonic":
        x_thresholds, y_thresholds = fit_isotonic(raw, target)
        return {
            "method": "isotonic",
            "x_thresholds": x_thresholds,
            "y_thresholds": y_thresholds,
        }
    raise ValueError(f"unsupported calibration method: {method}")


def apply_calibration_config(raw: np.ndarray | pd.Series, config: dict[str, Any] | None) -> np.ndarray:
    if not isinstance(config, dict):
        raise ValueError("calibration config must be an object")
    method = infer_calibration_method(config)
    raw_values = np.asarray(raw, dtype=float)
    if method == "sigmoid":
        if "coef" not in config or "intercept" not in config:
            raise ValueError("sigmoid calibration config requires coef/intercept")
        return clip_probability(
            apply_sigmoid(
                raw_values,
                float(config["coef"]),
                float(config["intercept"]),
            ),
        )
    if method == "isotonic":
        x_thresholds = np.asarray(config.get("x_thresholds", []), dtype=float)
        y_thresholds = np.asarray(config.get("y_thresholds", []), dtype=float)
        if x_thresholds.size == 0 or y_thresholds.size == 0 or x_thresholds.size != y_thresholds.size:
            raise ValueError("isotonic calibration config requires aligned x_thresholds/y_thresholds")
        interpolated = np.interp(
            raw_values,
            x_thresholds,
            y_thresholds,
            left=float(y_thresholds[0]),
            right=float(y_thresholds[-1]),
        )
        return clip_probability(interpolated)
    raise ValueError(f"unsupported calibration method: {method}")


def safe_auc(y_true: pd.Series, y_score: pd.Series) -> float | None:
    try:
        valid = y_true.notna() & y_score.notna()
        if int(valid.sum()) == 0:
            return None
        y_valid = y_true.loc[valid].astype(int)
        score_valid = y_score.loc[valid].astype(float)
        if y_valid.nunique(dropna=True) < 2:
            return None
        return float(roc_auc_score(y_valid, score_valid))
    except Exception:
        return None


def safe_logloss(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        valid = y_true.notna() & y_prob.notna()
        if int(valid.sum()) == 0:
            return None
        return float(
            log_loss(
                y_true.loc[valid].astype(int),
                clip_probability(y_prob.loc[valid]),
                labels=[0, 1],
            ),
        )
    except Exception:
        return None


def safe_brier(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        valid = y_true.notna() & y_prob.notna()
        if int(valid.sum()) == 0:
            return None
        return float(
            brier_score_loss(
                y_true.loc[valid].astype(int),
                clip_probability(y_prob.loc[valid]),
            ),
        )
    except Exception:
        return None


def calibration_curve_bins(
    y_true: pd.Series,
    y_prob: pd.Series,
    bins: int = 10,
) -> list[dict[str, Any]]:
    if int(bins) <= 0:
        return []
    valid = y_true.notna() & y_prob.notna()
    if int(valid.sum()) == 0:
        return []
    frame = pd.DataFrame(
        {
            "y_true": y_true.loc[valid].astype(int),
            "y_prob": clip_probability(y_prob.loc[valid]),
        },
    )
    if frame.empty:
        return []
    bucket_edges = np.linspace(0.0, 1.0, int(bins) + 1)
    bucket_index = np.minimum(
        int(bins) - 1,
        np.digitize(frame["y_prob"].to_numpy(dtype=float), bucket_edges[1:-1], right=False),
    )
    frame["bucket_index"] = bucket_index.astype(int)

    rows: list[dict[str, Any]] = []
    for bucket in range(int(bins)):
        bucket_frame = frame.loc[frame["bucket_index"] == bucket]
        if bucket_frame.empty:
            continue
        rows.append(
            {
                "bucket_index": int(bucket),
                "probability_min": float(bucket_edges[bucket]),
                "probability_max": float(bucket_edges[bucket + 1]),
                "rows": int(len(bucket_frame)),
                "mean_probability": float(bucket_frame["y_prob"].mean()),
                "positive_rate": float(bucket_frame["y_true"].mean()),
            },
        )
    return rows


def summarize_probability_metrics(
    y_true: pd.Series,
    y_prob: pd.Series,
    *,
    bins: int = 0,
) -> dict[str, Any]:
    valid = y_true.notna() & y_prob.notna()
    rows = int(valid.sum())
    summary: dict[str, Any] = {
        "rows": rows,
        "positive_rate": float(y_true.loc[valid].astype(int).mean()) if rows else 0.0,
        "mean_probability": float(y_prob.loc[valid].astype(float).mean()) if rows else 0.0,
        "auc": safe_auc(y_true, y_prob),
        "logloss": safe_logloss(y_true, y_prob),
        "brier": safe_brier(y_true, y_prob),
    }
    if int(bins) > 0:
        summary["calibration_bins"] = calibration_curve_bins(y_true, y_prob, bins=bins)
    return summary


def summarize_calibration_diagnostics(
    *,
    method: str,
    y_true: pd.Series,
    raw_score: pd.Series,
    raw_probability: pd.Series | None,
    calibrated_probability: pd.Series | None,
    bins: int = 0,
) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "method": infer_calibration_method({"method": method}),
        "rows_used": int((y_true.notna() & raw_score.notna()).sum()),
        "raw_auc": safe_auc(y_true, raw_score),
        "raw_probability_auc": safe_auc(y_true, raw_probability) if raw_probability is not None else None,
        "raw_logloss": safe_logloss(y_true, raw_probability) if raw_probability is not None else None,
        "raw_brier": safe_brier(y_true, raw_probability) if raw_probability is not None else None,
        "calibrated_auc": safe_auc(y_true, calibrated_probability) if calibrated_probability is not None else None,
        "calibrated_logloss": safe_logloss(y_true, calibrated_probability)
        if calibrated_probability is not None
        else None,
        "calibrated_brier": safe_brier(y_true, calibrated_probability)
        if calibrated_probability is not None
        else None,
    }
    raw_logloss = summary["raw_logloss"]
    calibrated_logloss = summary["calibrated_logloss"]
    raw_brier = summary["raw_brier"]
    calibrated_brier = summary["calibrated_brier"]
    raw_probability_auc = summary["raw_probability_auc"]
    calibrated_auc = summary["calibrated_auc"]
    summary["logloss_delta"] = (
        float(calibrated_logloss - raw_logloss)
        if raw_logloss is not None and calibrated_logloss is not None
        else None
    )
    summary["brier_delta"] = (
        float(calibrated_brier - raw_brier)
        if raw_brier is not None and calibrated_brier is not None
        else None
    )
    summary["auc_delta"] = (
        float(calibrated_auc - raw_probability_auc)
        if raw_probability_auc is not None and calibrated_auc is not None
        else None
    )
    if int(bins) > 0 and calibrated_probability is not None:
        summary["calibrated_bins"] = calibration_curve_bins(y_true, calibrated_probability, bins=bins)
    return summary
