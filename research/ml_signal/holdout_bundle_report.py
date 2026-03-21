from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import pandas as pd

from common import dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Aggregate MlSignal bundle summaries by ticker-holdout cohort.",
    )
    parser.add_argument(
        "--bundle-dirs",
        nargs="+",
        required=True,
        help="One or more bundle directories containing walkforward_summary.json.",
    )
    parser.add_argument(
        "--out-path",
        default="",
        help="Optional JSON output path. Defaults next to the first bundle.",
    )
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    return raw if isinstance(raw, dict) else None


def normalize_holdout_group(value: Any) -> str:
    if value is True:
        return "holdout_true"
    if value is False:
        return "holdout_false"
    return "holdout_unknown"


def summarize_group(label: str, frame: pd.DataFrame) -> dict[str, Any]:
    if frame.empty:
        return {
            "group": label,
            "bundle_count": 0,
            "rows_scored_total": 0,
            "mean_sum_policy_return": 0.0,
            "median_sum_policy_return": 0.0,
            "mean_positive_roll_rate": 0.0,
            "median_positive_roll_rate": 0.0,
            "mean_positive_ticker_rate": 0.0,
            "mean_active_share": 0.0,
            "gate_pass_rate": 0.0,
        }
    gate_values = pd.to_numeric(frame["gate_passed"], errors="coerce").fillna(0)
    return {
        "group": label,
        "bundle_count": int(len(frame)),
        "rows_scored_total": int(pd.to_numeric(frame["rows_scored"], errors="coerce").fillna(0).sum()),
        "mean_sum_policy_return": float(pd.to_numeric(frame["sum_policy_return"], errors="coerce").fillna(0).mean()),
        "median_sum_policy_return": float(pd.to_numeric(frame["sum_policy_return"], errors="coerce").fillna(0).median()),
        "mean_positive_roll_rate": float(pd.to_numeric(frame["positive_roll_rate"], errors="coerce").fillna(0).mean()),
        "median_positive_roll_rate": float(pd.to_numeric(frame["positive_roll_rate"], errors="coerce").fillna(0).median()),
        "mean_positive_ticker_rate": float(pd.to_numeric(frame["positive_ticker_rate"], errors="coerce").fillna(0).mean()),
        "mean_active_share": float(pd.to_numeric(frame["active_share"], errors="coerce").fillna(0).mean()),
        "gate_pass_rate": float(gate_values.mean()) if len(gate_values) else 0.0,
    }


def main() -> int:
    args = parse_args()
    bundle_dirs = [Path(value) for value in args.bundle_dirs]
    if not bundle_dirs:
        raise SystemExit("no bundle directories provided")

    rows: list[dict[str, Any]] = []
    for bundle_dir in bundle_dirs:
        if not bundle_dir.exists():
            continue
        walkforward = read_json(bundle_dir / "walkforward_summary.json")
        if not isinstance(walkforward, dict):
            continue
        gate_report = read_json(bundle_dir / "gate_report.json") or {}
        rows.append(
            {
                "bundle_dir": str(bundle_dir),
                "bundle_id": str(
                    walkforward.get("bundle_id") or bundle_dir.name or "",
                ),
                "ticker_holdout_group": normalize_holdout_group(
                    walkforward.get("ticker_holdout"),
                ),
                "rows_scored": walkforward.get("rows_scored"),
                "sum_policy_return": walkforward.get("sum_policy_return"),
                "positive_roll_rate": walkforward.get("positive_roll_rate"),
                "positive_ticker_rate": walkforward.get("positive_ticker_rate"),
                "active_share": walkforward.get("active_share"),
                "gate_passed": bool(gate_report.get("passed", False)),
            },
        )

    if not rows:
        raise SystemExit("no readable bundle summaries found")

    frame = pd.DataFrame(rows)
    out_path = (
        Path(args.out_path)
        if str(args.out_path).strip()
        else bundle_dirs[0].parent / "holdout_bundle_summary.json"
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)

    group_summaries = [
        summarize_group(label, group_frame.copy())
        for label, group_frame in frame.groupby("ticker_holdout_group", sort=True)
    ]

    report = {
        "bundle_count": int(len(frame)),
        "bundle_rows_path": str(out_path.with_name("holdout_bundle_rows.csv")),
        "groups": group_summaries,
    }

    dump_json(out_path, report)
    frame.to_csv(out_path.with_name("holdout_bundle_rows.csv"), index=False)

    md_lines = [
        f"bundle_count: {len(frame)}",
        "",
        "groups:",
    ]
    for row in group_summaries:
        md_lines.append(
            f"- {row['group']}: bundles={row['bundle_count']} rows_scored_total={row['rows_scored_total']} mean_sum_policy_return={row['mean_sum_policy_return']:.6f} mean_positive_roll_rate={row['mean_positive_roll_rate']:.4f} gate_pass_rate={row['gate_pass_rate']:.4f}",
        )
    out_path.with_suffix(".md").write_text("\n".join(md_lines) + "\n", encoding="utf-8")

    print(f"wrote holdout bundle summary -> {out_path}")
    print(f"wrote holdout bundle rows -> {out_path.with_name('holdout_bundle_rows.csv')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
