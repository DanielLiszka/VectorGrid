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


@dataclass(slots=True, frozen=True)
class NormalizedSplitWindow:
    split_id: str
    train_start: int
    train_end: int
    calibration_start: int
    calibration_end: int
    test_start: int
    test_end: int


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


def _clamp_window(start: int, end: int, row_count: int) -> tuple[int, int]:
    clamped_start = min(max(0, int(start)), row_count)
    clamped_end = min(max(clamped_start, int(end)), row_count)
    return clamped_start, clamped_end


def _validate_window_sequence(
    *,
    split_id: str,
    train_start: int,
    train_end: int,
    calibration_start: int,
    calibration_end: int,
    test_start: int,
    test_end: int,
) -> None:
    if train_start > train_end:
        raise ValueError(f"{split_id}: train_start must be <= train_end")
    if calibration_start > calibration_end:
        raise ValueError(f"{split_id}: calibration_start must be <= calibration_end")
    if test_start > test_end:
        raise ValueError(f"{split_id}: test_start must be <= test_end")
    if train_end > calibration_start:
        raise ValueError(f"{split_id}: train/calibration windows overlap")
    if calibration_end > test_start:
        raise ValueError(f"{split_id}: calibration/test windows overlap")


def normalize_split_windows(bundle: DatasetBundle) -> list[NormalizedSplitWindow]:
    row_count = len(bundle.dataset)
    if row_count == 0:
        return []

    split_manifest = bundle.split_manifest or {}
    windows = split_manifest.get("windows")
    if not isinstance(windows, list) or not windows:
        return []

    export_manifest = bundle.export_manifest or {}
    first_valid_bar = int(export_manifest.get("first_valid_bar", bundle.first_valid_bar) or 0)
    normalized: list[NormalizedSplitWindow] = []

    last_test_start: int | None = None
    for idx, raw_window in enumerate(windows):
        if not isinstance(raw_window, dict):
            raise ValueError(f"split window #{idx} must be an object")
        split_id = str(raw_window.get("split_id", f"roll_{idx:03}")).strip() or f"roll_{idx:03}"
        shifted_train_start = int(raw_window.get("train_start", 0)) - first_valid_bar
        shifted_train_end = int(raw_window.get("train_end", 0)) - first_valid_bar
        shifted_calibration_start = int(raw_window.get("calibration_start", 0)) - first_valid_bar
        shifted_calibration_end = int(raw_window.get("calibration_end", 0)) - first_valid_bar
        shifted_test_start = int(raw_window.get("test_start", 0)) - first_valid_bar
        shifted_test_end = int(raw_window.get("test_end", row_count)) - first_valid_bar
        _validate_window_sequence(
            split_id=split_id,
            train_start=shifted_train_start,
            train_end=shifted_train_end,
            calibration_start=shifted_calibration_start,
            calibration_end=shifted_calibration_end,
            test_start=shifted_test_start,
            test_end=shifted_test_end,
        )
        if last_test_start is not None and shifted_test_start < last_test_start:
            raise ValueError(f"{split_id}: split windows must be ordered by nondecreasing test_start")
        train_start, train_end = _clamp_window(
            shifted_train_start,
            shifted_train_end,
            row_count,
        )
        calibration_start, calibration_end = _clamp_window(
            shifted_calibration_start,
            shifted_calibration_end,
            row_count,
        )
        test_start, test_end = _clamp_window(
            shifted_test_start,
            shifted_test_end,
            row_count,
        )
        normalized.append(
            NormalizedSplitWindow(
                split_id=split_id,
                train_start=train_start,
                train_end=train_end,
                calibration_start=calibration_start,
                calibration_end=calibration_end,
                test_start=test_start,
                test_end=test_end,
            ),
        )
        last_test_start = shifted_test_start
    return normalized


def build_fallback_split_window(row_count: int) -> NormalizedSplitWindow:
    train_start, train_end = _clamp_window(0, max(1, int(row_count * 0.7)), row_count)
    calibration_start, calibration_end = _clamp_window(
        train_end,
        max(train_end + 1, int(row_count * 0.85)),
        row_count,
    )
    test_start, test_end = _clamp_window(calibration_end, row_count, row_count)
    return NormalizedSplitWindow(
        split_id="fallback_test",
        train_start=train_start,
        train_end=train_end,
        calibration_start=calibration_start,
        calibration_end=calibration_end,
        test_start=test_start,
        test_end=test_end,
    )


def split_window_masks(row_count: int, window: NormalizedSplitWindow) -> dict[str, np.ndarray]:
    idx = np.arange(row_count)
    return {
        "train": (idx >= window.train_start) & (idx < window.train_end),
        "calibration": (idx >= window.calibration_start) & (idx < window.calibration_end),
        "test": (idx >= window.test_start) & (idx < window.test_end),
    }


def resolve_walkforward_windows(bundle: DatasetBundle) -> list[NormalizedSplitWindow]:
    windows = normalize_split_windows(bundle)
    if windows:
        return windows
    return [build_fallback_split_window(len(bundle.dataset))]


def resolve_split_masks(bundle: DatasetBundle) -> dict[str, np.ndarray]:
    row_count = len(bundle.dataset)
    if row_count == 0:
        raise ValueError("dataset is empty")

    windows = normalize_split_windows(bundle)
    if windows:
        return split_window_masks(row_count, windows[-1])
    return split_window_masks(row_count, build_fallback_split_window(row_count))


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


def stable_json_dumps(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def fnv1a64_hex_bytes(raw: bytes) -> str:
    hash_value = 0xCBF29CE484222325
    for byte in raw:
        hash_value ^= byte
        hash_value = (hash_value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv1a64-{hash_value:016x}"


def json_fingerprint(value: Any) -> str:
    return fnv1a64_hex_bytes(stable_json_dumps(value).encode("utf-8"))


def json_file_fingerprint(path: Path | None) -> str | None:
    if path is None or not path.exists():
        return None
    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    return json_fingerprint(raw)


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
