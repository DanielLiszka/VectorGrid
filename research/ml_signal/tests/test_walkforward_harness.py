from __future__ import annotations

import importlib
import json
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest import mock

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


class FakeCatBoostClassifier:
    fit_history: list[dict[str, object]] = []

    def __init__(self, *args, **kwargs) -> None:
        self.args = args
        self.kwargs = kwargs

    def fit(self, features, target, sample_weight=None, eval_set=None, use_best_model=None):
        eval_features, eval_target = eval_set
        FakeCatBoostClassifier.fit_history.append(
            {
                "fit_features": features.copy(),
                "fit_target": target.copy(),
                "sample_weight": sample_weight.copy() if sample_weight is not None else None,
                "eval_features": eval_features.copy(),
                "eval_target": eval_target.copy(),
                "use_best_model": use_best_model,
            },
        )
        self._feature_names = list(features.columns)
        return self

    def predict_proba(self, features):
        signal = pd.to_numeric(features.iloc[:, 0], errors="coerce").fillna(0.0).to_numpy(dtype=float)
        clipped = np.clip(signal, -8.0, 8.0)
        prob = 1.0 / (1.0 + np.exp(-clipped))
        return np.column_stack([1.0 - prob, prob])

    def predict(self, features, prediction_type=None):
        raw = pd.to_numeric(features.iloc[:, 0], errors="coerce").fillna(0.0).to_numpy(dtype=float)
        if prediction_type == "RawFormulaVal":
            return raw
        return (raw >= 0.0).astype(int)

    def save_model(self, path: str) -> None:
        Path(path).write_text("fake-model\n", encoding="utf-8")

    def load_model(self, path: str) -> None:
        self.loaded_path = path


def _import_module(module_name: str):
    fake_catboost = types.ModuleType("catboost")
    fake_catboost.CatBoostClassifier = FakeCatBoostClassifier
    sys.modules["catboost"] = fake_catboost
    for stale in ("train_catboost", "evaluate_walkforward"):
        sys.modules.pop(stale, None)
    return importlib.import_module(module_name)


def _write_json(path: Path, payload: dict[str, object]) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _build_dataset(path: Path, row_count: int) -> None:
    frame = pd.DataFrame(
        {
            "timestamp": np.arange(row_count, dtype=np.int64),
            "ticker_id": np.where(np.arange(row_count) % 2 == 0, "AAA", "BBB"),
            "split_id": ["" for _ in range(row_count)],
            "sample_weight": np.ones(row_count, dtype=float),
            "feat_a": np.linspace(-2.5, 2.5, row_count, dtype=float),
            "r_long": np.linspace(0.01, 0.03, row_count, dtype=float),
            "r_short": np.linspace(-0.02, 0.02, row_count, dtype=float),
            "y_long": (np.arange(row_count) % 2).astype(int),
            "y_short": ((np.arange(row_count) + 1) % 2).astype(int),
            "keep_long": np.ones(row_count, dtype=int),
            "keep_short": np.ones(row_count, dtype=int),
        },
    )
    frame.to_csv(path, index=False)


class WalkforwardHarnessTest(unittest.TestCase):
    def setUp(self) -> None:
        FakeCatBoostClassifier.fit_history.clear()

    def test_train_model_uses_only_train_and_calibration_rows(self) -> None:
        module = _import_module("train_catboost")
        row_count = 50
        frame = pd.DataFrame(
            {
                "timestamp": np.arange(row_count, dtype=np.int64),
                "ticker_id": ["AAA"] * row_count,
                "split_id": ["roll_000"] * row_count,
                "feature_a": np.linspace(-1.0, 1.0, row_count, dtype=float),
                "feature_b": np.linspace(1.0, 2.0, row_count, dtype=float),
                "sample_weight": np.linspace(1.0, 2.0, row_count, dtype=float),
                "r_long": np.zeros(row_count, dtype=float),
                "r_short": np.zeros(row_count, dtype=float),
                "y_long": (np.arange(row_count) % 2).astype(int),
                "y_short": ((np.arange(row_count) + 1) % 2).astype(int),
                "keep_long": np.ones(row_count, dtype=int),
                "keep_short": np.ones(row_count, dtype=int),
            },
        )
        masks = {
            "train": pd.Series(np.arange(row_count) < 30, index=frame.index),
            "calibration": pd.Series((np.arange(row_count) >= 30) & (np.arange(row_count) < 42), index=frame.index),
            "test": pd.Series(np.arange(row_count) >= 42, index=frame.index),
        }
        sample_weight = frame["sample_weight"].copy()

        _, score_frame, metrics = module.train_model(
            "long",
            frame,
            ["feature_a", "feature_b"],
            masks,
            sample_weight,
            iterations=25,
            learning_rate=0.1,
            depth=4,
            random_seed=7,
        )

        self.assertEqual(len(FakeCatBoostClassifier.fit_history), 1)
        fit_call = FakeCatBoostClassifier.fit_history[0]
        self.assertEqual(list(fit_call["fit_features"].columns), ["feature_a", "feature_b"])
        self.assertEqual(len(fit_call["fit_features"]), 30)
        self.assertEqual(len(fit_call["eval_features"]), 12)
        self.assertEqual(len(score_frame), 12)
        self.assertEqual(metrics["train_rows"], 30)
        self.assertEqual(metrics["calibration_rows"], 12)
        self.assertEqual(score_frame["row_index"].iloc[0], 30)
        self.assertEqual(score_frame["row_index"].iloc[-1], 41)

    def test_train_model_rejects_one_class_train_slice(self) -> None:
        module = _import_module("train_catboost")
        row_count = 50
        frame = pd.DataFrame(
            {
                "timestamp": np.arange(row_count, dtype=np.int64),
                "ticker_id": ["AAA"] * row_count,
                "split_id": ["roll_000"] * row_count,
                "feature_a": np.linspace(-1.0, 1.0, row_count, dtype=float),
                "sample_weight": np.ones(row_count, dtype=float),
                "r_long": np.zeros(row_count, dtype=float),
                "r_short": np.zeros(row_count, dtype=float),
                "y_long": np.concatenate([np.zeros(30, dtype=int), np.ones(20, dtype=int)]),
                "y_short": np.concatenate([np.ones(30, dtype=int), np.zeros(20, dtype=int)]),
                "keep_long": np.ones(row_count, dtype=int),
                "keep_short": np.ones(row_count, dtype=int),
            },
        )
        frame.loc[:29, "y_long"] = 0
        masks = {
            "train": pd.Series(np.arange(row_count) < 30, index=frame.index),
            "calibration": pd.Series((np.arange(row_count) >= 30) & (np.arange(row_count) < 42), index=frame.index),
            "test": pd.Series(np.arange(row_count) >= 42, index=frame.index),
        }
        sample_weight = frame["sample_weight"].copy()

        with self.assertRaisesRegex(ValueError, "requires both classes"):
            module.train_model(
                "long",
                frame,
                ["feature_a"],
                masks,
                sample_weight,
                iterations=25,
                learning_rate=0.1,
                depth=4,
                random_seed=7,
            )

    def test_train_main_writes_additive_provenance_metadata(self) -> None:
        module = _import_module("train_catboost")
        with tempfile.TemporaryDirectory(prefix="ml_signal_train_bundle_") as tmp:
            root = Path(tmp)
            dataset_path = root / "dataset.csv"
            out_dir = root / "bundle"
            _build_dataset(dataset_path, 70)
            _write_json(
                dataset_path.with_suffix(".feature_schema.json"),
                {
                    "schema_version": 1,
                    "schema_hash": "schemahash-source-abcdef",
                    "feature_names": ["feat_a"],
                    "first_valid_bar": 0,
                },
            )
            _write_json(
                dataset_path.with_suffix(".export_manifest.json"),
                {
                    "first_valid_bar": 0,
                    "feature_schema_hash": "schemahash-source-abcdef",
                },
            )
            _write_json(
                dataset_path.with_suffix(".split_manifest.json"),
                {
                    "windows": [
                        {
                            "split_id": "roll_000",
                            "train_start": 0,
                            "train_end": 40,
                            "calibration_start": 40,
                            "calibration_end": 55,
                            "test_start": 55,
                            "test_end": 65,
                        },
                    ],
                },
            )

            argv = [
                "train_catboost.py",
                "--dataset",
                str(dataset_path),
                "--out-dir",
                str(out_dir),
                "--bundle-id",
                "bundle_provenance_test",
                "--iterations",
                "20",
                "--learning-rate",
                "0.1",
                "--depth",
                "4",
                "--random-seed",
                "7",
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = module.main()

            self.assertEqual(exit_code, 0)
            manifest = json.loads((out_dir / "manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["bundle_id"], "bundle_provenance_test")
            self.assertEqual(manifest["source_feature_schema_hash"], "schemahash-source-abcdef")
            self.assertIn("source_export_manifest_hash", manifest)
            self.assertEqual(manifest["training_config"]["has_time"], True)
            self.assertEqual(manifest["class_balance_policy"], "uniform_sample_weight_no_class_rebalancing")
            self.assertEqual(
                manifest["training_windows_summary"]["selected_split_id"],
                "roll_000",
            )
            self.assertEqual(
                manifest["score_contract"]["walkforward_recommended_mode"],
                "rolling",
            )
            train_metadata = json.loads((out_dir / "train_metadata.json").read_text(encoding="utf-8"))
            self.assertEqual(
                train_metadata["source_feature_schema_hash"],
                "schemahash-source-abcdef",
            )
            self.assertIn("score_contract", train_metadata)

    def test_proxy_mode_uses_bundle_sidecars_and_only_scores_test_windows(self) -> None:
        module = _import_module("evaluate_walkforward")
        with tempfile.TemporaryDirectory(prefix="ml_signal_proxy_") as tmp:
            root = Path(tmp)
            dataset_path = root / "dataset.csv"
            bundle_dir = root / "bundle"
            bundle_dir.mkdir()
            _build_dataset(dataset_path, 10)

            _write_json(dataset_path.with_suffix(".feature_schema.json"), {"feature_names": ["wrong_feature"]})
            _write_json(bundle_dir / "feature_schema.json", {"feature_names": ["feat_a"], "first_valid_bar": 0})
            _write_json(bundle_dir / "export_manifest.json", {"first_valid_bar": 0})
            _write_json(
                bundle_dir / "split_manifest.json",
                {
                    "windows": [
                        {
                            "split_id": "roll_000",
                            "train_start": 0,
                            "train_end": 3,
                            "calibration_start": 3,
                            "calibration_end": 5,
                            "test_start": 5,
                            "test_end": 7,
                        },
                        {
                            "split_id": "roll_001",
                            "train_start": 2,
                            "train_end": 5,
                            "calibration_start": 5,
                            "calibration_end": 7,
                            "test_start": 7,
                            "test_end": 10,
                        },
                    ],
                },
            )
            _write_json(bundle_dir / "manifest.json", {"bundle_id": "bundle_proxy_test"})
            (bundle_dir / "model_long.cbm").write_text("fake\n", encoding="utf-8")
            (bundle_dir / "model_short.cbm").write_text("fake\n", encoding="utf-8")

            argv = [
                "evaluate_walkforward.py",
                "--dataset",
                str(dataset_path),
                "--bundle-dir",
                str(bundle_dir),
                "--mode",
                "proxy",
                "--artifact-suffix",
                "proxy",
                "--t-long",
                "0.0",
                "--t-short",
                "1.0",
                "--delta",
                "0.0",
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = module.main()

            self.assertEqual(exit_code, 0)
            scores = pd.read_csv(bundle_dir / "walkforward_test_scores.proxy.csv")
            summary = json.loads((bundle_dir / "walkforward_summary.proxy.json").read_text(encoding="utf-8"))
            self.assertEqual(len(scores), 5)
            self.assertEqual(scores["evaluated_split_id"].value_counts().to_dict(), {"roll_001": 3, "roll_000": 2})
            self.assertEqual(summary["bundle_id"], "bundle_proxy_test")
            self.assertEqual(summary["evaluation_mode"], "proxy")
            self.assertEqual(summary["artifact_suffix"], "proxy")

    def test_proxy_mode_records_skipped_empty_windows(self) -> None:
        module = _import_module("evaluate_walkforward")
        with tempfile.TemporaryDirectory(prefix="ml_signal_proxy_skip_") as tmp:
            root = Path(tmp)
            dataset_path = root / "dataset.csv"
            bundle_dir = root / "bundle"
            bundle_dir.mkdir()
            _build_dataset(dataset_path, 12)

            _write_json(bundle_dir / "feature_schema.json", {"feature_names": ["feat_a"], "first_valid_bar": 0})
            _write_json(bundle_dir / "export_manifest.json", {"first_valid_bar": 0})
            _write_json(
                bundle_dir / "split_manifest.json",
                {
                    "windows": [
                        {
                            "split_id": "roll_empty",
                            "train_start": 0,
                            "train_end": 4,
                            "calibration_start": 4,
                            "calibration_end": 6,
                            "test_start": 6,
                            "test_end": 6,
                        },
                        {
                            "split_id": "roll_real",
                            "train_start": 2,
                            "train_end": 6,
                            "calibration_start": 6,
                            "calibration_end": 8,
                            "test_start": 8,
                            "test_end": 10,
                        },
                    ],
                },
            )
            _write_json(bundle_dir / "manifest.json", {"bundle_id": "bundle_proxy_skip"})
            (bundle_dir / "model_long.cbm").write_text("fake\n", encoding="utf-8")
            (bundle_dir / "model_short.cbm").write_text("fake\n", encoding="utf-8")

            argv = [
                "evaluate_walkforward.py",
                "--dataset",
                str(dataset_path),
                "--bundle-dir",
                str(bundle_dir),
                "--mode",
                "proxy",
                "--t-long",
                "0.0",
                "--t-short",
                "1.0",
                "--delta",
                "0.0",
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = module.main()

            self.assertEqual(exit_code, 0)
            summary = json.loads((bundle_dir / "walkforward_summary.json").read_text(encoding="utf-8"))
            self.assertEqual(summary["skipped_roll_count"], 1)
            self.assertEqual(summary["expected_splits"], 2)
            self.assertEqual(summary["skipped_rolls"][0]["split_id"], "roll_empty")

    def test_rolling_mode_retrains_per_roll(self) -> None:
        module = _import_module("evaluate_walkforward")
        with tempfile.TemporaryDirectory(prefix="ml_signal_rolling_") as tmp:
            root = Path(tmp)
            dataset_path = root / "dataset.csv"
            bundle_dir = root / "bundle"
            bundle_dir.mkdir()
            _build_dataset(dataset_path, 70)

            _write_json(bundle_dir / "feature_schema.json", {"feature_names": ["feat_a"], "first_valid_bar": 0})
            _write_json(bundle_dir / "export_manifest.json", {"first_valid_bar": 0})
            _write_json(
                bundle_dir / "split_manifest.json",
                {
                    "windows": [
                        {
                            "split_id": "roll_000",
                            "train_start": 0,
                            "train_end": 30,
                            "calibration_start": 30,
                            "calibration_end": 45,
                            "test_start": 45,
                            "test_end": 55,
                        },
                        {
                            "split_id": "roll_001",
                            "train_start": 10,
                            "train_end": 40,
                            "calibration_start": 40,
                            "calibration_end": 55,
                            "test_start": 55,
                            "test_end": 65,
                        },
                    ],
                },
            )
            _write_json(
                bundle_dir / "train_metadata.json",
                {
                    "iterations": 20,
                    "learning_rate": 0.1,
                    "depth": 4,
                    "random_seed": 11,
                },
            )
            _write_json(bundle_dir / "manifest.json", {"bundle_id": "bundle_rolling_test"})

            argv = [
                "evaluate_walkforward.py",
                "--dataset",
                str(dataset_path),
                "--bundle-dir",
                str(bundle_dir),
                "--mode",
                "rolling",
                "--t-long",
                "0.0",
                "--t-short",
                "1.0",
                "--delta",
                "0.0",
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = module.main()

            self.assertEqual(exit_code, 0)
            self.assertEqual(len(FakeCatBoostClassifier.fit_history), 4)
            fit_sizes = [(len(call["fit_features"]), len(call["eval_features"])) for call in FakeCatBoostClassifier.fit_history]
            self.assertEqual(fit_sizes, [(30, 15), (30, 15), (30, 15), (30, 15)])

            scores = pd.read_csv(bundle_dir / "walkforward_test_scores.csv")
            summary = json.loads((bundle_dir / "walkforward_summary.json").read_text(encoding="utf-8"))
            self.assertEqual(len(scores), 20)
            self.assertEqual(summary["evaluation_mode"], "rolling")
            self.assertEqual(summary["splits_scored"], 2)
            self.assertEqual(summary["calibrated_rolls"], 2)


if __name__ == "__main__":
    unittest.main()
