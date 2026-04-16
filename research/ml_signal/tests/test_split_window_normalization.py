from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from common import (  # noqa: E402
    DatasetBundle,
    build_fallback_split_window,
    json_file_fingerprint,
    normalize_split_windows,
    resolve_split_masks,
    resolve_walkforward_windows,
)


def _bundle(
    row_count: int,
    *,
    first_valid_bar: int = 0,
    windows: list[dict[str, int | str]] | None = None,
) -> DatasetBundle:
    dataset = pd.DataFrame(
        {
            "timestamp": np.arange(row_count, dtype=np.int64),
            "feature_a": np.linspace(0.0, 1.0, row_count, dtype=float),
        },
    )
    tmp_dir = Path(tempfile.mkdtemp(prefix="ml_signal_split_test_"))
    dataset_path = tmp_dir / "dataset.csv"
    dataset.to_csv(dataset_path, index=False)
    schema_path = tmp_dir / "feature_schema.json"
    schema_path.write_text('{"feature_names":["feature_a"],"first_valid_bar":0}\n', encoding="utf-8")
    return DatasetBundle(
        dataset_path=dataset_path,
        schema_path=schema_path,
        export_manifest_path=None,
        split_manifest_path=None,
        dataset=dataset,
        schema={"feature_names": ["feature_a"], "first_valid_bar": 0},
        export_manifest={"first_valid_bar": first_valid_bar},
        split_manifest={"windows": windows or []},
        feature_names=["feature_a"],
        first_valid_bar=0,
    )


class SplitWindowNormalizationTest(unittest.TestCase):
    def test_normalize_split_windows_offsets_and_clamps_all_windows(self) -> None:
        bundle = _bundle(
            10,
            first_valid_bar=3,
            windows=[
                {
                    "split_id": "roll_a",
                    "train_start": 0,
                    "train_end": 6,
                    "calibration_start": 6,
                    "calibration_end": 8,
                    "test_start": 8,
                    "test_end": 12,
                },
                {
                    "train_start": 5,
                    "train_end": 12,
                    "calibration_start": 12,
                    "calibration_end": 14,
                    "test_start": 14,
                    "test_end": 20,
                },
            ],
        )

        windows = normalize_split_windows(bundle)

        self.assertEqual(len(windows), 2)
        self.assertEqual(windows[0].split_id, "roll_a")
        self.assertEqual((windows[0].train_start, windows[0].train_end), (0, 3))
        self.assertEqual((windows[0].test_start, windows[0].test_end), (5, 9))
        self.assertEqual(windows[1].split_id, "roll_001")
        self.assertEqual((windows[1].train_start, windows[1].train_end), (2, 9))
        self.assertEqual((windows[1].test_start, windows[1].test_end), (10, 10))

    def test_resolve_split_masks_uses_last_window(self) -> None:
        bundle = _bundle(
            12,
            windows=[
                {
                    "split_id": "roll_000",
                    "train_start": 0,
                    "train_end": 4,
                    "calibration_start": 4,
                    "calibration_end": 6,
                    "test_start": 6,
                    "test_end": 8,
                },
                {
                    "split_id": "roll_001",
                    "train_start": 2,
                    "train_end": 7,
                    "calibration_start": 7,
                    "calibration_end": 9,
                    "test_start": 9,
                    "test_end": 12,
                },
            ],
        )

        masks = resolve_split_masks(bundle)

        self.assertEqual(int(masks["train"].sum()), 5)
        self.assertEqual(int(masks["calibration"].sum()), 2)
        self.assertEqual(int(masks["test"].sum()), 3)
        self.assertTrue(masks["train"][2])
        self.assertTrue(masks["test"][11])
        self.assertFalse(masks["test"][8])

    def test_resolve_walkforward_windows_falls_back_when_manifest_missing(self) -> None:
        bundle = _bundle(20, windows=None)

        windows = resolve_walkforward_windows(bundle)
        fallback = build_fallback_split_window(20)

        self.assertEqual(len(windows), 1)
        self.assertEqual(windows[0], fallback)

    def test_normalize_split_windows_rejects_overlapping_segments(self) -> None:
        bundle = _bundle(
            20,
            windows=[
                {
                    "split_id": "bad_overlap",
                    "train_start": 0,
                    "train_end": 8,
                    "calibration_start": 7,
                    "calibration_end": 10,
                    "test_start": 10,
                    "test_end": 12,
                },
            ],
        )

        with self.assertRaisesRegex(ValueError, "overlap"):
            normalize_split_windows(bundle)

    def test_json_file_fingerprint_is_stable_across_key_order(self) -> None:
        tmp_dir = Path(tempfile.mkdtemp(prefix="ml_signal_fingerprint_test_"))
        a = tmp_dir / "a.json"
        b = tmp_dir / "b.json"
        a.write_text('{"b":2,"a":1}\n', encoding="utf-8")
        b.write_text('{"a":1,"b":2}\n', encoding="utf-8")

        self.assertEqual(json_file_fingerprint(a), json_file_fingerprint(b))


if __name__ == "__main__":
    unittest.main()
