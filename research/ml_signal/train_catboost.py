from __future__ import annotations

import argparse
import shutil
from pathlib import Path

import pandas as pd
from sklearn.metrics import log_loss, roc_auc_score

from common import (
    build_feature_frame,
    choose_sample_weight,
    dump_json,
    load_dataset_bundle,
    resolve_split_masks,
)

try:
    from catboost import CatBoostClassifier
except ImportError as exc:  # pragma: no cover
    raise SystemExit(
        "catboost is required for train_catboost.py. Install research/ml_signal dependencies first.",
    ) from exc


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
    parser.add_argument("--iterations", type=int, default=1500)
    parser.add_argument("--learning-rate", type=float, default=0.03)
    parser.add_argument("--depth", type=int, default=6)
    parser.add_argument("--random-seed", type=int, default=42)
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


def train_model(
    name: str,
    frame: pd.DataFrame,
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

    feature_cols = [
        column
        for column in frame.columns
        if column
        not in {
            "timestamp",
            "ticker_id",
            "split_id",
            "sample_weight",
            "r_long",
            "r_short",
            "y_long",
            "y_short",
            "keep_long",
            "keep_short",
        }
    ]

    model = CatBoostClassifier(
        loss_function="Logloss",
        eval_metric="Logloss",
        iterations=iterations,
        learning_rate=learning_rate,
        depth=depth,
        grow_policy="SymmetricTree",
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
    base_frame = bundle.dataset.copy()
    features = build_feature_frame(bundle)
    for column in bundle.feature_names:
        base_frame[column] = features[column]

    masks = {name: pd.Series(mask, index=base_frame.index) for name, mask in masks_np.items()}
    sample_weight = pd.Series(choose_sample_weight(bundle), index=base_frame.index)

    long_model, long_scores, long_metrics = train_model(
        "long",
        base_frame,
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

    shutil.copyfile(bundle.schema_path, out_dir / "feature_schema.json")
    if bundle.export_manifest_path:
        shutil.copyfile(bundle.export_manifest_path, out_dir / "export_manifest.json")
    if bundle.split_manifest_path:
        shutil.copyfile(bundle.split_manifest_path, out_dir / "split_manifest.json")

    bundle_id = args.bundle_id.strip() or f"mlsig_{bundle.dataset_path.stem}_cbm_v1"
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
            "score_contract": {
                "validation_scores_scope": "calibration_slice",
                "validation_scores_probability_columns": ["p_long", "p_short"],
                "validation_scores_preferred_probability_columns_after_calibration": [
                    "p_long_cal",
                    "p_short_cal",
                ],
                "raw_margin_columns": ["raw_long", "raw_short"],
                "desktop_validation_requires_probability_columns": True,
            },
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
            "iterations": args.iterations,
            "learning_rate": args.learning_rate,
            "depth": args.depth,
            "random_seed": args.random_seed,
            "score_contract": {
                "validation_scores_scope": "calibration_slice",
                "walkforward_scores_scope": "walkforward_test",
                "desktop_policy_validation": "probability_threshold_only",
            },
        },
    )

    print(f"trained MlSignal bundle -> {out_dir}")
    print(f"bundle_id={bundle_id}")
    print(f"validation_scores={validation_scores_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
