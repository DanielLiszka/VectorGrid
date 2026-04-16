from __future__ import annotations

import argparse
import shutil
from pathlib import Path

import pandas as pd
from sklearn.metrics import log_loss, roc_auc_score

from common import (
    build_fallback_split_window,
    build_feature_frame,
    choose_sample_weight,
    dump_json,
    json_file_fingerprint,
    load_dataset_bundle,
    normalize_split_windows,
    resolve_split_masks,
)

try:
    from catboost import CatBoostClassifier
except ImportError as exc:  # pragma: no cover
    raise SystemExit(
        "catboost is required for train_catboost.py. Install research/ml_signal dependencies first.",
    ) from exc


DEFAULT_ITERATIONS = 1500
DEFAULT_LEARNING_RATE = 0.03
DEFAULT_DEPTH = 6
DEFAULT_RANDOM_SEED = 42
DEFAULT_SCORE_CONTRACT_VERSION = 1
DEFAULT_SCORE_CONTRACT = {
    "score_contract_version": DEFAULT_SCORE_CONTRACT_VERSION,
    "validation_scores_scope": "calibration_slice",
    "validation_scores_probability_columns": ["p_long", "p_short"],
    "validation_scores_preferred_probability_columns": ["p_long_cal", "p_short_cal"],
    "validation_scores_preferred_probability_columns_after_calibration": [
        "p_long_cal",
        "p_short_cal",
    ],
    "raw_margin_columns": ["raw_long", "raw_short"],
    "walkforward_scores_scope": "walkforward_test",
    "desktop_validation_requires_probability_columns": True,
    "desktop_policy_validation": "probability_threshold_only",
    "walkforward_supported_modes": ["proxy", "rolling"],
    "walkforward_recommended_mode": "rolling",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Train the offline MlSignal CatBoost long/short models.",
    )
    parser.add_argument("--dataset", required=True, help="Path to exported ML dataset CSV.")
    parser.add_argument("--out-dir", required=True, help="Output bundle directory.")
    parser.add_argument(
        "--schema",
        default="",
        help="Optional feature schema JSON path. Defaults to the sidecar next to the dataset.",
    )
    parser.add_argument("--bundle-id", default="", help="Optional explicit bundle id.")
    parser.add_argument("--iterations", type=int, default=DEFAULT_ITERATIONS)
    parser.add_argument("--learning-rate", type=float, default=DEFAULT_LEARNING_RATE)
    parser.add_argument("--depth", type=int, default=DEFAULT_DEPTH)
    parser.add_argument("--random-seed", type=int, default=DEFAULT_RANDOM_SEED)
    return parser.parse_args()


def _safe_auc(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        if y_true.nunique(dropna=True) < 2:
            return None
        return float(roc_auc_score(y_true, y_prob))
    except Exception:
        return None


def _safe_logloss(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        return float(log_loss(y_true, y_prob, labels=[0, 1]))
    except Exception:
        return None


def _require_binary_target_slice(name: str, slice_name: str, target: pd.Series) -> None:
    if target.nunique(dropna=True) < 2:
        raise ValueError(f"{name} {slice_name} target requires both classes")


def _score_contract() -> dict[str, object]:
    return {
        key: list(value) if isinstance(value, list) else value
        for key, value in DEFAULT_SCORE_CONTRACT.items()
    }


def _window_to_dict(window) -> dict[str, int | str]:
    return {
        "split_id": window.split_id,
        "train_start": int(window.train_start),
        "train_end": int(window.train_end),
        "calibration_start": int(window.calibration_start),
        "calibration_end": int(window.calibration_end),
        "test_start": int(window.test_start),
        "test_end": int(window.test_end),
    }


def _build_feature_importance_artifact(
    model: CatBoostClassifier,
    feature_cols: list[str],
    out_path: Path,
) -> tuple[str, list[dict[str, float | str]]] | None:
    if not hasattr(model, "get_feature_importance"):
        return None
    values = model.get_feature_importance()
    if len(values) != len(feature_cols):
        return None
    frame = pd.DataFrame(
        {
            "feature_name": feature_cols,
            "importance": [float(value) for value in values],
        },
    ).sort_values("importance", ascending=False, kind="stable")
    frame.to_csv(out_path, index=False)
    top_rows = frame.head(10).to_dict(orient="records")
    return out_path.name, top_rows


def train_model(
    name: str,
    frame: pd.DataFrame,
    feature_cols: list[str],
    masks: dict[str, pd.Series],
    sample_weight: pd.Series,
    iterations: int,
    learning_rate: float,
    depth: int,
    random_seed: int,
) -> tuple[CatBoostClassifier, pd.DataFrame, dict[str, float | int | None]]:
    target_col = f"y_{name}"
    keep_col = f"keep_{name}"
    required_cols = [target_col, keep_col, "timestamp"]
    for column in required_cols:
        if column not in frame.columns:
            raise ValueError(f"dataset is missing required column: {column}")

    usable = frame[keep_col].fillna(0).astype(int) == 1
    train_mask = masks["train"] & usable
    calibration_mask = masks["calibration"] & usable
    if int(train_mask.sum()) < 25:
        raise ValueError(f"not enough training rows for {name}")
    if int(calibration_mask.sum()) < 10:
        raise ValueError(f"not enough calibration rows for {name}")
    _require_binary_target_slice(name, "train", frame.loc[train_mask, target_col])
    _require_binary_target_slice(name, "calibration", frame.loc[calibration_mask, target_col])

    model = CatBoostClassifier(
        loss_function="Logloss",
        eval_metric="Logloss",
        iterations=iterations,
        learning_rate=learning_rate,
        depth=depth,
        grow_policy="SymmetricTree",
        has_time=True,
        allow_writing_files=False,
        verbose=False,
        random_seed=random_seed,
    )
    model.fit(
        frame.loc[train_mask, feature_cols],
        frame.loc[train_mask, target_col].astype(int),
        sample_weight=sample_weight.loc[train_mask],
        eval_set=(
            frame.loc[calibration_mask, feature_cols],
            frame.loc[calibration_mask, target_col].astype(int),
        ),
        use_best_model=True,
    )

    score_frame = pd.DataFrame(
        {
            "row_index": frame.index[calibration_mask].astype("int64").to_numpy(),
            "timestamp": frame.loc[calibration_mask, "timestamp"].astype("int64").to_numpy(),
            "ticker_id": frame.loc[calibration_mask, "ticker_id"].astype(str).to_numpy(),
            "split_id": frame.loc[calibration_mask, "split_id"].astype(str).to_numpy(),
            f"y_{name}": frame.loc[calibration_mask, target_col].astype(int).to_numpy(),
        },
    )
    if "ticker_holdout" in frame.columns:
        score_frame["ticker_holdout"] = (
            pd.to_numeric(frame.loc[calibration_mask, "ticker_holdout"], errors="coerce")
            .fillna(0)
            .astype(int)
            .to_numpy()
        )
    score_frame[f"p_{name}"] = model.predict_proba(frame.loc[calibration_mask, feature_cols])[:, 1]
    score_frame[f"raw_{name}"] = model.predict(
        frame.loc[calibration_mask, feature_cols],
        prediction_type="RawFormulaVal",
    )

    metrics = {
        "train_rows": int(train_mask.sum()),
        "calibration_rows": int(calibration_mask.sum()),
        "positive_rate_train": float(frame.loc[train_mask, target_col].mean()),
        "positive_rate_calibration": float(frame.loc[calibration_mask, target_col].mean()),
        "auc_calibration": _safe_auc(score_frame[f"y_{name}"], score_frame[f"p_{name}"]),
        "logloss_calibration": _safe_logloss(score_frame[f"y_{name}"], score_frame[f"p_{name}"]),
    }
    return model, score_frame, metrics


def main() -> int:
    args = parse_args()
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    bundle = load_dataset_bundle(args.dataset, args.schema or None)
    masks_np = resolve_split_masks(bundle)
    split_windows = normalize_split_windows(bundle)
    selected_window = split_windows[-1] if split_windows else build_fallback_split_window(len(bundle.dataset))
    base_frame = bundle.dataset.copy()
    features = build_feature_frame(bundle)
    for column in bundle.feature_names:
        base_frame[column] = features[column]

    masks = {name: pd.Series(mask, index=base_frame.index) for name, mask in masks_np.items()}
    sample_weight = pd.Series(choose_sample_weight(bundle), index=base_frame.index)

    long_model, long_scores, long_metrics = train_model(
        "long",
        base_frame,
        bundle.feature_names,
        masks,
        sample_weight,
        args.iterations,
        args.learning_rate,
        args.depth,
        args.random_seed,
    )
    short_model, short_scores, short_metrics = train_model(
        "short",
        base_frame,
        bundle.feature_names,
        masks,
        sample_weight,
        args.iterations,
        args.learning_rate,
        args.depth,
        args.random_seed + 1,
    )

    long_model_path = out_dir / "model_long.cbm"
    short_model_path = out_dir / "model_short.cbm"
    long_model.save_model(str(long_model_path))
    short_model.save_model(str(short_model_path))

    merge_keys = ["row_index", "timestamp", "ticker_id", "split_id"]
    if "ticker_holdout" in long_scores.columns and "ticker_holdout" in short_scores.columns:
        merge_keys.append("ticker_holdout")
    merged_scores = pd.merge(long_scores, short_scores, on=merge_keys, how="outer").sort_values(
        ["row_index", "timestamp"],
    )
    validation_scores_path = out_dir / "validation_scores.csv"
    merged_scores.to_csv(validation_scores_path, index=False)

    explainability_artifacts: dict[str, object] = {}
    long_importance = _build_feature_importance_artifact(
        long_model,
        bundle.feature_names,
        out_dir / "feature_importance_long.csv",
    )
    if long_importance is not None:
        explainability_artifacts["long_feature_importance_csv"] = long_importance[0]
        explainability_artifacts["long_top_features"] = long_importance[1]
    short_importance = _build_feature_importance_artifact(
        short_model,
        bundle.feature_names,
        out_dir / "feature_importance_short.csv",
    )
    if short_importance is not None:
        explainability_artifacts["short_feature_importance_csv"] = short_importance[0]
        explainability_artifacts["short_top_features"] = short_importance[1]

    shutil.copyfile(bundle.schema_path, out_dir / "feature_schema.json")
    if bundle.export_manifest_path:
        shutil.copyfile(bundle.export_manifest_path, out_dir / "export_manifest.json")
    if bundle.split_manifest_path:
        shutil.copyfile(bundle.split_manifest_path, out_dir / "split_manifest.json")

    bundle_id = args.bundle_id.strip() or f"mlsig_{bundle.dataset_path.stem}_cbm_v1"
    source_feature_schema_hash = str(
        bundle.schema.get("schema_hash")
        or json_file_fingerprint(bundle.schema_path)
        or ""
    )
    source_export_manifest_hash = json_file_fingerprint(bundle.export_manifest_path)
    score_contract = _score_contract()
    training_mode = "single_fit_last_window" if split_windows else "single_fit_fallback"
    selected_split_id = selected_window.split_id
    training_config = {
        "loss_function": "Logloss",
        "eval_metric": "Logloss",
        "iterations": int(args.iterations),
        "learning_rate": float(args.learning_rate),
        "depth": int(args.depth),
        "grow_policy": "SymmetricTree",
        "use_best_model": True,
        "has_time": True,
        "allow_writing_files": False,
        "long_random_seed": int(args.random_seed),
        "short_random_seed": int(args.random_seed + 1),
        "feature_count": int(len(bundle.feature_names)),
    }
    training_windows_summary = {
        "window_count": int(len(split_windows)),
        "fallback_used": not split_windows,
        "selected_window": _window_to_dict(selected_window),
        "selected_split_id": selected_split_id,
        "selected_window_index": int(len(split_windows) - 1) if split_windows else 0,
        "available_split_ids": [window.split_id for window in split_windows] if split_windows else [selected_split_id],
        "train_rows": int(masks["train"].sum()),
        "calibration_rows": int(masks["calibration"].sum()),
        "test_rows": int(masks["test"].sum()),
    }
    class_balance_policy = (
        "dataset_sample_weight_nonuniform_no_class_rebalancing"
        if sample_weight.nunique(dropna=False) > 1
        else "uniform_sample_weight_no_class_rebalancing"
    )
    calibration_summary = {
        "kind": "not_calibrated_yet",
        "status": "pending_calibration_script",
        "validation_scores_scope": "calibration_slice",
        "rows": {
            "long": int(long_metrics["calibration_rows"]),
            "short": int(short_metrics["calibration_rows"]),
        },
    }
    dump_json(
        out_dir / "manifest.json",
        {
            "bundle_id": bundle_id,
            "schema_version": 1,
            "feature_schema_version": int(bundle.schema.get("schema_version", 1) or 1),
            "model_family": "catboost_binary_pair",
            "model_format": "cbm",
            "long_model_path": "model_long.cbm",
            "short_model_path": "model_short.cbm",
            "source_export_manifest_hash": source_export_manifest_hash,
            "source_feature_schema_hash": source_feature_schema_hash,
            "training_config": training_config,
            "training_windows_summary": training_windows_summary,
            "calibration_summary": calibration_summary,
            "class_balance_policy": class_balance_policy,
            "threshold_policy_default": {
                "kind": "defer_to_desktop_validation_request",
            },
            "explainability_artifacts": explainability_artifacts or None,
            "bundle_parent_id": None,
            "score_contract": score_contract,
            "bundle_training_mode": training_mode,
            "bundle_training_split_id": selected_split_id,
            "split_window_count": len(split_windows),
            "notes": "Trained from exported MlSignal dataset CSV.",
        },
    )
    dump_json(
        out_dir / "metrics.json",
        {
            "dataset_rows": int(len(base_frame)),
            "feature_count": int(len(bundle.feature_names)),
            "splits": {name: int(mask.sum()) for name, mask in masks.items()},
            "long": long_metrics,
            "short": short_metrics,
        },
    )
    dump_json(
        out_dir / "train_metadata.json",
        {
            "dataset": str(bundle.dataset_path),
            "schema": str(bundle.schema_path),
            "export_manifest": str(bundle.export_manifest_path) if bundle.export_manifest_path else "",
            "split_manifest": str(bundle.split_manifest_path) if bundle.split_manifest_path else "",
            "bundle_id": bundle_id,
            "source_feature_schema_hash": source_feature_schema_hash,
            "source_export_manifest_hash": source_export_manifest_hash,
            "iterations": args.iterations,
            "learning_rate": args.learning_rate,
            "depth": args.depth,
            "random_seed": args.random_seed,
            "training_config": training_config,
            "training_windows_summary": training_windows_summary,
            "class_balance_policy": class_balance_policy,
            "bundle_training_mode": training_mode,
            "bundle_training_split_id": selected_split_id,
            "split_window_count": len(split_windows),
            "score_contract": score_contract,
            "calibration_summary": calibration_summary,
            "explainability_artifacts": explainability_artifacts or None,
        },
    )

    print(f"trained MlSignal bundle -> {out_dir}")
    print(f"bundle_id={bundle_id}")
    print(f"validation_scores={validation_scores_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
