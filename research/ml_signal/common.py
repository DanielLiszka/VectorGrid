from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd


@dataclass(slots=True)
class DatasetBundle:
    dataset_path: Path
    schema_path: Path
    export_manifest_path: Path | None
    split_manifest_path: Path | None
    dataset: pd.DataFrame
    schema: dict[str, Any]
    export_manifest: dict[str, Any] | None
    split_manifest: dict[str, Any] | None
    feature_names: list[str]
    first_valid_bar: int


def _read_json(path: Path | None) -> dict[str, Any] | None:
    if path is None or not path.exists():
        return None
    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    if not isinstance(raw, dict):
        raise ValueError(f"expected object JSON at {path}")
    return raw


def _default_sidecar(dataset: Path, suffix: str) -> Path:
    stem = dataset.with_suffix("")
    return Path(f"{stem}.{suffix}.json")


def load_dataset_bundle(
    dataset_path: str | Path,
    schema_path: str | Path | None = None,
    export_manifest_path: str | Path | None = None,
    split_manifest_path: str | Path | None = None,
) -> DatasetBundle:
    dataset = Path(dataset_path)
    if not dataset.exists():
        raise FileNotFoundError(f"dataset not found: {dataset}")

    schema = Path(schema_path) if schema_path else _default_sidecar(dataset, "feature_schema")
    if not schema.exists():
        raise FileNotFoundError(f"feature schema not found: {schema}")

    export_manifest = Path(export_manifest_path) if export_manifest_path else _default_sidecar(dataset, "export_manifest")
    split_manifest = Path(split_manifest_path) if split_manifest_path else _default_sidecar(dataset, "split_manifest")

    df = pd.read_csv(dataset)
    schema_json = _read_json(schema)
    if schema_json is None:
        raise ValueError(f"empty schema: {schema}")
    feature_names = [str(name).strip() for name in schema_json.get("feature_names", []) if str(name).strip()]
    if not feature_names:
        raise ValueError("feature schema is missing feature_names")
    missing = [name for name in feature_names if name not in df.columns]
    if missing:
        raise ValueError(f"dataset is missing schema features: {missing[:10]}")

    return DatasetBundle(
        dataset_path=dataset,
        schema_path=schema,
        export_manifest_path=export_manifest if export_manifest.exists() else None,
        split_manifest_path=split_manifest if split_manifest.exists() else None,
        dataset=df,
        schema=schema_json,
        export_manifest=_read_json(export_manifest if export_manifest.exists() else None),
        split_manifest=_read_json(split_manifest if split_manifest.exists() else None),
        feature_names=feature_names,
        first_valid_bar=int(schema_json.get("first_valid_bar", 0) or 0),
    )


def resolve_split_masks(bundle: DatasetBundle) -> dict[str, np.ndarray]:
    row_count = len(bundle.dataset)
    if row_count == 0:
        raise ValueError("dataset is empty")

    split_manifest = bundle.split_manifest or {}
    windows = split_manifest.get("windows")
    if isinstance(windows, list) and windows:
        export_manifest = bundle.export_manifest or {}
        first_valid_bar = int(export_manifest.get("first_valid_bar", bundle.first_valid_bar) or 0)
        last_window = windows[-1]
        train_start = max(0, int(last_window.get("train_start", 0)) - first_valid_bar)
        train_end = max(train_start, int(last_window.get("train_end", 0)) - first_valid_bar)
        calibration_start = max(0, int(last_window.get("calibration_start", 0)) - first_valid_bar)
        calibration_end = max(calibration_start, int(last_window.get("calibration_end", 0)) - first_valid_bar)
        test_start = max(0, int(last_window.get("test_start", 0)) - first_valid_bar)
        test_end = max(test_start, int(last_window.get("test_end", row_count)) - first_valid_bar)
        idx = np.arange(row_count)
        return {
            "train": (idx >= train_start) & (idx < min(train_end, row_count)),
            "calibration": (idx >= calibration_start) & (idx < min(calibration_end, row_count)),
            "test": (idx >= test_start) & (idx < min(test_end, row_count)),
        }

    train_end = max(1, int(row_count * 0.7))
    calibration_end = max(train_end + 1, int(row_count * 0.85))
    idx = np.arange(row_count)
    return {
        "train": idx < train_end,
        "calibration": (idx >= train_end) & (idx < calibration_end),
        "test": idx >= calibration_end,
    }


def build_feature_frame(bundle: DatasetBundle) -> pd.DataFrame:
    return bundle.dataset.loc[:, bundle.feature_names].apply(pd.to_numeric, errors="coerce").fillna(0.0)


def choose_sample_weight(bundle: DatasetBundle) -> np.ndarray:
    if "sample_weight" not in bundle.dataset.columns:
        return np.ones(len(bundle.dataset), dtype=float)
    values = pd.to_numeric(bundle.dataset["sample_weight"], errors="coerce").fillna(1.0)
    return values.to_numpy(dtype=float)


def apply_sigmoid(raw: np.ndarray, coef: float, intercept: float) -> np.ndarray:
    margin = (raw.astype(float) * coef) + intercept
    clipped = np.clip(margin, -40.0, 40.0)
    return 1.0 / (1.0 + np.exp(-clipped))


def dump_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")


def stable_hash_mod(text: str, modulus: int) -> int | None:
    if modulus <= 0:
        return None
    value = str(text).strip()
    if not value:
        return None
    hash_value = 0xCBF29CE484222325
    for byte in value.encode("utf-8"):
        hash_value ^= byte
        hash_value = (hash_value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return int(hash_value % modulus)


def assign_ticker_holdout(ticker_id: str, modulus: int, residue: int) -> bool | None:
    mod_value = stable_hash_mod(ticker_id, modulus)
    if mod_value is None:
        return None
    return mod_value == (int(residue) % max(1, int(modulus)))
