from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

import calibration_slice_report  # noqa: E402
import calibrate  # noqa: E402
import gate_report  # noqa: E402
import holdout_bundle_report  # noqa: E402
import holdout_report  # noqa: E402


class ReportingContractsTest(unittest.TestCase):
    def test_calibrate_handles_side_specific_rows(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ml_signal_calibrate_") as tmp:
            root = Path(tmp)
            bundle_dir = root / "bundle"
            bundle_dir.mkdir()
            scores = bundle_dir / "validation_scores.csv"
            pd.DataFrame(
                {
                    "raw_long": [-1.0, 1.0, None, None],
                    "y_long": [0, 1, None, None],
                    "raw_short": [None, None, -1.5, 1.5],
                    "y_short": [None, None, 0, 1],
                },
            ).to_csv(scores, index=False)
            (bundle_dir / "manifest.json").write_text("{}", encoding="utf-8")
            (bundle_dir / "train_metadata.json").write_text("{}", encoding="utf-8")

            argv = [
                "calibrate.py",
                "--bundle-dir",
                str(bundle_dir),
                "--scores-csv",
                str(scores),
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = calibrate.main()

            self.assertEqual(exit_code, 0)
            calibrated = pd.read_csv(scores)
            self.assertTrue(calibrated.loc[0:1, "p_long_cal"].notna().all())
            self.assertTrue(calibrated.loc[2:3, "p_short_cal"].notna().all())
            self.assertTrue(calibrated.loc[2:3, "p_long_cal"].isna().all())
            self.assertTrue(calibrated.loc[0:1, "p_short_cal"].isna().all())
            manifest = json.loads((bundle_dir / "manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["score_contract"]["calibration_kind"], "sigmoid_pair")
            self.assertIn("calibration_summary", manifest)
            train_metadata = json.loads((bundle_dir / "train_metadata.json").read_text(encoding="utf-8"))
            self.assertEqual(train_metadata["calibration_path"], "calibration.json")

    def test_calibrate_preserves_existing_score_contract_fields(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ml_signal_calibrate_merge_") as tmp:
            root = Path(tmp)
            bundle_dir = root / "bundle"
            bundle_dir.mkdir()
            scores = bundle_dir / "validation_scores.csv"
            pd.DataFrame(
                {
                    "raw_long": [-1.0, 1.0],
                    "p_long": [0.2, 0.8],
                    "y_long": [0, 1],
                    "raw_short": [-1.5, 1.5],
                    "p_short": [0.1, 0.9],
                    "y_short": [0, 1],
                },
            ).to_csv(scores, index=False)
            (bundle_dir / "manifest.json").write_text(
                json.dumps(
                    {
                        "score_contract": {
                            "walkforward_supported_modes": ["proxy", "rolling"],
                            "walkforward_recommended_mode": "rolling",
                        },
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            argv = [
                "calibrate.py",
                "--bundle-dir",
                str(bundle_dir),
                "--scores-csv",
                str(scores),
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = calibrate.main()

            self.assertEqual(exit_code, 0)
            manifest = json.loads((bundle_dir / "manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(
                manifest["score_contract"]["walkforward_supported_modes"],
                ["proxy", "rolling"],
            )
            self.assertEqual(
                manifest["score_contract"]["walkforward_recommended_mode"],
                "rolling",
            )

    def test_calibration_slice_report_prefers_calibrated_probability_columns(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ml_signal_report_") as tmp:
            root = Path(tmp)
            scores = root / "validation_scores.csv"
            pd.DataFrame(
                {
                    "timestamp": [1, 2, 3, 4],
                    "ticker_id": ["AAA", "AAA", "BBB", "BBB"],
                    "split_id": ["roll_000", "roll_000", "roll_001", "roll_001"],
                    "y_long": [0, 1, 0, 1],
                    "p_long": [0.4, 0.6, 0.45, 0.55],
                    "p_long_cal": [0.1, 0.9, 0.2, 0.8],
                    "raw_long": [-0.5, 0.5, -0.3, 0.3],
                    "y_short": [1, 0, 1, 0],
                    "p_short": [0.6, 0.4, 0.55, 0.45],
                    "p_short_cal": [0.85, 0.15, 0.75, 0.25],
                    "raw_short": [0.4, -0.4, 0.2, -0.2],
                },
            ).to_csv(scores, index=False)

            argv = [
                "calibration_slice_report.py",
                "--scores-csv",
                str(scores),
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = calibration_slice_report.main()

            self.assertEqual(exit_code, 0)
            report = json.loads((root / "validation_slice_metrics.json").read_text(encoding="utf-8"))
            self.assertEqual(report["overall"]["long"]["probability_column"], "p_long_cal")
            self.assertEqual(report["overall"]["short"]["probability_column"], "p_short_cal")

    def test_holdout_reports_preserve_evaluation_mode_metadata(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ml_signal_holdout_") as tmp:
            root = Path(tmp)
            scores = root / "walkforward_test_scores.csv"
            pd.DataFrame(
                {
                    "ticker_id": ["AAA", "BBB", "AAA", "BBB"],
                    "position": [1, 0, -1, 1],
                    "policy_return": [0.02, 0.0, -0.01, 0.03],
                    "evaluation_mode": ["rolling", "rolling", "rolling", "rolling"],
                },
            ).to_csv(scores, index=False)

            argv = [
                "holdout_report.py",
                "--scores",
                str(scores),
                "--modulus",
                "5",
                "--residue",
                "0",
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = holdout_report.main()

            self.assertEqual(exit_code, 0)
            holdout_json = json.loads((root / "holdout_report.json").read_text(encoding="utf-8"))
            self.assertEqual(holdout_json["evaluation_modes"], ["rolling"])

            bundle_a = root / "bundle_a"
            bundle_b = root / "bundle_b"
            bundle_a.mkdir()
            bundle_b.mkdir()
            (bundle_a / "walkforward_summary.json").write_text(
                json.dumps(
                    {
                        "bundle_id": "a",
                        "ticker_holdout": True,
                        "evaluation_mode": "rolling",
                        "rows_scored": 10,
                        "sum_policy_return": 0.12,
                        "positive_roll_rate": 0.5,
                        "positive_ticker_rate": 1.0,
                        "active_share": 0.4,
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            (bundle_b / "walkforward_summary.json").write_text(
                json.dumps(
                    {
                        "bundle_id": "b",
                        "ticker_holdout": False,
                        "evaluation_mode": "proxy",
                        "rows_scored": 8,
                        "sum_policy_return": 0.08,
                        "positive_roll_rate": 0.75,
                        "positive_ticker_rate": 1.0,
                        "active_share": 0.5,
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            argv = [
                "holdout_bundle_report.py",
                "--bundle-dirs",
                str(bundle_a),
                str(bundle_b),
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = holdout_bundle_report.main()

            self.assertEqual(exit_code, 0)
            summary = json.loads((root / "holdout_bundle_summary.json").read_text(encoding="utf-8"))
            self.assertEqual(summary["evaluation_mode_counts"], {"proxy": 1, "rolling": 1})

    def test_gate_report_prefers_rolling_calibration_metrics_and_warns_on_mode_mismatch(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ml_signal_gate_") as tmp:
            root = Path(tmp)
            bundle_dir = root / "bundle"
            bundle_dir.mkdir()
            (bundle_dir / "manifest.json").write_text('{"bundle_id":"bundle_gate"}\n', encoding="utf-8")
            (bundle_dir / "metrics.json").write_text(
                json.dumps({"long": {"auc_calibration": 0.1}, "short": {"auc_calibration": 0.2}}, indent=2) + "\n",
                encoding="utf-8",
            )
            (bundle_dir / "walkforward_summary.json").write_text(
                json.dumps(
                    {
                        "evaluation_mode": "rolling",
                        "rows_scored": 40,
                        "expected_splits": 2,
                        "skipped_roll_count": 0,
                        "splits_scored": 2,
                        "positive_roll_rate": 1.0,
                        "active_share": 0.5,
                        "mean_policy_return": 0.02,
                        "sum_policy_return": 0.8,
                        "rolling_calibration_summary": {
                            "long_auc_calibration_mean": 0.71,
                            "short_auc_calibration_mean": 0.69,
                        },
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            (bundle_dir / "walkforward_rolls.json").write_text(
                json.dumps({"rolls": [{"split_id": "roll_000"}, {"split_id": "roll_001"}]}, indent=2) + "\n",
                encoding="utf-8",
            )
            (bundle_dir / "baseline_comparison.json").write_text(
                json.dumps({"ml_evaluation_mode": "proxy"}, indent=2) + "\n",
                encoding="utf-8",
            )
            (bundle_dir / "holdout_report.json").write_text(
                json.dumps({"evaluation_modes": ["proxy"]}, indent=2) + "\n",
                encoding="utf-8",
            )

            argv = [
                "gate_report.py",
                "--bundle-dir",
                str(bundle_dir),
                "--min-auc",
                "0.6",
                "--min-rows-scored",
                "10",
                "--min-positive-roll-rate",
                "0.0",
                "--min-active-share",
                "0.0",
            ]
            with mock.patch.object(sys, "argv", argv):
                exit_code = gate_report.main()

            self.assertEqual(exit_code, 0)
            report = json.loads((bundle_dir / "gate_report.json").read_text(encoding="utf-8"))
            self.assertEqual(report["summary"]["auc_source"], "rolling_calibration_summary.mean")
            self.assertEqual(report["summary"]["long_auc"], 0.71)
            self.assertTrue(report["auxiliary_mode_warnings"])


if __name__ == "__main__":
    unittest.main()
