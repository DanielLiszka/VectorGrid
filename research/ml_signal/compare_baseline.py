from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from common import dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compare an MlSignal walk-forward summary against a deterministic baseline summary.",
    )
    parser.add_argument("--ml-summary", required=True, help="Path to ML walkforward_summary.json.")
    parser.add_argument(
        "--baseline-summary",
        required=True,
        help="Path to deterministic baseline summary JSON.",
    )
    parser.add_argument(
        "--label",
        default="deterministic_baseline",
        help="Label for the baseline strategy in the comparison report.",
    )
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise SystemExit(f"required file not found: {path}")
    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    if not isinstance(raw, dict):
        raise SystemExit(f"expected JSON object at: {path}")
    return raw


def as_float(value: Any) -> float | None:
    try:
        parsed = float(value)
    except Exception:
        return None
    return parsed if parsed == parsed else None


def metric_delta(ml_value: float | None, baseline_value: float | None) -> float | None:
    if ml_value is None or baseline_value is None:
        return None
    return ml_value - baseline_value


def main() -> int:
    args = parse_args()
    ml_path = Path(args.ml_summary)
    baseline_path = Path(args.baseline_summary)

    ml_summary = read_json(ml_path)
    baseline_summary = read_json(baseline_path)

    metrics = [
        "sum_policy_return",
        "mean_policy_return",
        "positive_roll_rate",
        "positive_ticker_rate",
        "active_share",
    ]
    comparison_metrics: dict[str, Any] = {}
    ml_wins = 0
    comparable = 0

    for key in metrics:
        ml_value = as_float(ml_summary.get(key))
        baseline_value = as_float(baseline_summary.get(key))
        delta = metric_delta(ml_value, baseline_value)
        if delta is not None:
            comparable += 1
            if delta > 0:
                ml_wins += 1
        comparison_metrics[key] = {
            "ml": ml_value,
            "baseline": baseline_value,
            "delta": delta,
        }

    report = {
        "ml_summary_path": str(ml_path),
        "baseline_summary_path": str(baseline_path),
        "baseline_label": args.label,
        "metrics": comparison_metrics,
        "ml_wins": ml_wins,
        "comparable_metrics": comparable,
        "ml_beats_baseline_on_majority": bool(comparable > 0 and ml_wins > (comparable / 2.0)),
    }

    output_json = ml_path.with_name("baseline_comparison.json")
    output_md = ml_path.with_name("baseline_comparison.md")
    dump_json(output_json, report)

    lines = [
        f"baseline_label: {args.label}",
        f"ml_beats_baseline_on_majority: {str(report['ml_beats_baseline_on_majority']).lower()}",
        "",
        "metrics:",
    ]
    for key in metrics:
        row = comparison_metrics[key]
        lines.append(
            f"- {key}: ml={row['ml']} baseline={row['baseline']} delta={row['delta']}"
        )
    output_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"wrote baseline comparison -> {output_json}")
    print(f"wrote baseline markdown -> {output_md}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
