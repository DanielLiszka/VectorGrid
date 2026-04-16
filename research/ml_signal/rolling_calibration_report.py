from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import pandas as pd

from common import dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize per-roll MlSignal calibration diagnostics from rolling walk-forward outputs.",
    )
    parser.add_argument("--bundle-dir", required=True, help="Model bundle directory.")
    parser.add_argument(
        "--artifact-suffix",
        default="",
        help="Optional suffix for walk-forward artifacts, for example 'rolling'.",
    )
    return parser.parse_args()


def _artifact_path(bundle_dir: Path, base_name: str, suffix: str) -> Path:
    base = Path(base_name)
    clean_suffix = str(suffix).strip().strip(".")
    if not clean_suffix:
        return bundle_dir / base_name
    return bundle_dir / f"{base.stem}.{clean_suffix}{base.suffix}"


def _read_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise SystemExit(f"required file not found: {path}")
    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    if not isinstance(raw, dict):
        raise SystemExit(f"expected object JSON at: {path}")
    return raw


def _as_float(value: Any) -> float | None:
    try:
        parsed = float(value)
    except Exception:
        return None
    return parsed if parsed == parsed else None


def _aggregate_rows(rows: list[dict[str, Any]], key: str) -> dict[str, Any]:
    values = [_as_float(row.get(key)) for row in rows]
    filtered = [float(value) for value in values if value is not None]
    return {
        "mean": float(sum(filtered) / len(filtered)) if filtered else None,
        "min": float(min(filtered)) if filtered else None,
        "max": float(max(filtered)) if filtered else None,
        "count": int(len(filtered)),
    }


def main() -> int:
    args = parse_args()
    bundle_dir = Path(args.bundle_dir)
    if not bundle_dir.exists():
        raise SystemExit(f"bundle dir not found: {bundle_dir}")

    walkforward_summary_path = _artifact_path(bundle_dir, "walkforward_summary.json", args.artifact_suffix)
    walkforward_rolls_path = _artifact_path(bundle_dir, "walkforward_rolls.json", args.artifact_suffix)
    walkforward_summary = _read_json(walkforward_summary_path)
    walkforward_rolls = _read_json(walkforward_rolls_path)

    if str(walkforward_summary.get("evaluation_mode") or "proxy") != "rolling":
        raise SystemExit("rolling_calibration_report.py requires walk-forward outputs produced in rolling mode")

    rolls = walkforward_rolls.get("rolls")
    if not isinstance(rolls, list) or not rolls:
        raise SystemExit("walkforward_rolls.json does not contain roll rows")

    detail_rows: list[dict[str, Any]] = []
    for roll in rolls:
        if not isinstance(roll, dict):
            continue
        for side in ("long", "short"):
            diagnostics = roll.get(f"{side}_calibration_diagnostics")
            if not isinstance(diagnostics, dict):
                continue
            detail_rows.append(
                {
                    "split_id": str(roll.get("split_id") or ""),
                    "side": side,
                    "method": str(diagnostics.get("method") or ""),
                    "rows_used": diagnostics.get("rows_used"),
                    "raw_probability_auc": diagnostics.get("raw_probability_auc"),
                    "raw_logloss": diagnostics.get("raw_logloss"),
                    "raw_brier": diagnostics.get("raw_brier"),
                    "calibrated_auc": diagnostics.get("calibrated_auc"),
                    "calibrated_logloss": diagnostics.get("calibrated_logloss"),
                    "calibrated_brier": diagnostics.get("calibrated_brier"),
                    "auc_delta": diagnostics.get("auc_delta"),
                    "logloss_delta": diagnostics.get("logloss_delta"),
                    "brier_delta": diagnostics.get("brier_delta"),
                },
            )

    if not detail_rows:
        raise SystemExit("no per-roll calibration diagnostics found in walkforward_rolls.json")

    detail_frame = pd.DataFrame(detail_rows)
    summary: dict[str, Any] = {
        "bundle_dir": str(bundle_dir),
        "walkforward_summary_path": str(walkforward_summary_path),
        "walkforward_rolls_path": str(walkforward_rolls_path),
        "artifact_suffix": str(args.artifact_suffix or ""),
        "evaluation_mode": "rolling",
        "rows": int(len(detail_frame)),
        "rolls_considered": int(detail_frame["split_id"].nunique()),
        "side_counts": detail_frame["side"].value_counts().sort_index().to_dict(),
        "method_counts": {},
        "per_side": {},
    }

    for side in ("long", "short"):
        side_frame = detail_frame.loc[detail_frame["side"] == side].copy()
        summary["method_counts"][side] = side_frame["method"].value_counts().sort_index().to_dict()
        side_rows = side_frame.to_dict(orient="records")
        summary["per_side"][side] = {
            "raw_probability_auc": _aggregate_rows(side_rows, "raw_probability_auc"),
            "raw_logloss": _aggregate_rows(side_rows, "raw_logloss"),
            "raw_brier": _aggregate_rows(side_rows, "raw_brier"),
            "calibrated_auc": _aggregate_rows(side_rows, "calibrated_auc"),
            "calibrated_logloss": _aggregate_rows(side_rows, "calibrated_logloss"),
            "calibrated_brier": _aggregate_rows(side_rows, "calibrated_brier"),
            "auc_delta": _aggregate_rows(side_rows, "auc_delta"),
            "logloss_delta": _aggregate_rows(side_rows, "logloss_delta"),
            "brier_delta": _aggregate_rows(side_rows, "brier_delta"),
        }

    json_path = _artifact_path(bundle_dir, "rolling_calibration_report.json", args.artifact_suffix)
    csv_path = _artifact_path(bundle_dir, "rolling_calibration_report.csv", args.artifact_suffix)
    md_path = _artifact_path(bundle_dir, "rolling_calibration_report.md", args.artifact_suffix)

    dump_json(
        json_path,
        {
            "summary": summary,
            "rows": detail_rows,
        },
    )
    detail_frame.to_csv(csv_path, index=False)

    lines = [
        f"bundle_dir: {bundle_dir}",
        f"artifact_suffix: {summary['artifact_suffix']}",
        f"rolls_considered: {summary['rolls_considered']}",
        "",
        "per_side:",
    ]
    for side in ("long", "short"):
        side_summary = summary["per_side"][side]
        lines.extend(
            [
                f"- {side}:",
                f"  method_counts: {summary['method_counts'][side]}",
                f"  calibrated_auc_mean: {side_summary['calibrated_auc']['mean']}",
                f"  calibrated_logloss_mean: {side_summary['calibrated_logloss']['mean']}",
                f"  calibrated_brier_mean: {side_summary['calibrated_brier']['mean']}",
                f"  auc_delta_mean: {side_summary['auc_delta']['mean']}",
                f"  logloss_delta_mean: {side_summary['logloss_delta']['mean']}",
                f"  brier_delta_mean: {side_summary['brier_delta']['mean']}",
            ],
        )
    md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"wrote rolling calibration report -> {json_path}")
    print(f"wrote rolling calibration csv -> {csv_path}")
    print(f"wrote rolling calibration markdown -> {md_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
