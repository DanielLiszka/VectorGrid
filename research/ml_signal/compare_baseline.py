from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from baseline_walkforward import run_baseline_walkforward
from common import dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compare an MlSignal walk-forward summary against a deterministic baseline summary.",
    )
    parser.add_argument("--ml-summary", required=True, help="Path to ML walkforward_summary.json.")
    parser.add_argument(
        "--baseline-summary",
        default="",
        help="Path to deterministic baseline summary JSON.",
    )
    parser.add_argument(
        "--label",
        default="deterministic_baseline",
        help="Label for the baseline strategy in the comparison report.",
    )
    parser.add_argument(
        "--built-in-baseline",
        choices=["logistic_regression", "class_prior"],
        default="",
        help="Optional built-in baseline to generate before comparing.",
    )
    parser.add_argument("--dataset", default="", help="Dataset CSV used for built-in baseline generation.")
    parser.add_argument("--bundle-dir", default="", help="Bundle directory used for built-in baseline generation.")
    parser.add_argument(
        "--baseline-mode",
        choices=["auto", "proxy", "rolling"],
        default="auto",
        help="Evaluation mode for built-in baseline generation. Defaults to the ML summary mode.",
    )
    parser.add_argument("--t-long", type=float, default=None, help="Override long threshold for built-in baseline.")
    parser.add_argument("--t-short", type=float, default=None, help="Override short threshold for built-in baseline.")
    parser.add_argument("--delta", type=float, default=None, help="Override delta threshold for built-in baseline.")
    parser.add_argument(
        "--artifact-suffix",
        default="",
        help="Optional suffix appended to built-in baseline artifact filenames.",
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


def resolve_threshold(value: float | None, summary: dict[str, Any], key: str, fallback: float) -> float:
    if value is not None:
        return float(value)
    thresholds = summary.get("thresholds")
    if isinstance(thresholds, dict):
        parsed = thresholds.get(key)
        if parsed is not None:
            return float(parsed)
    return float(fallback)


def main() -> int:
    args = parse_args()
    ml_path = Path(args.ml_summary)
    ml_summary = read_json(ml_path)
    baseline_generation: dict[str, Any] | None = None

    if str(args.baseline_summary).strip():
        baseline_path = Path(args.baseline_summary)
        baseline_summary = read_json(baseline_path)
    else:
        if not str(args.built_in_baseline).strip():
            raise SystemExit("either --baseline-summary or --built-in-baseline is required")
        if not str(args.dataset).strip():
            raise SystemExit("--dataset is required when using --built-in-baseline")
        if not str(args.bundle_dir).strip():
            raise SystemExit("--bundle-dir is required when using --built-in-baseline")
        baseline_mode = (
            str(ml_summary.get("evaluation_mode") or "proxy")
            if args.baseline_mode == "auto"
            else args.baseline_mode
        )
        baseline_generation = run_baseline_walkforward(
            dataset=args.dataset,
            bundle_dir=args.bundle_dir,
            baseline_kind=args.built_in_baseline,
            mode=baseline_mode,
            t_long=resolve_threshold(args.t_long, ml_summary, "t_long", 0.6),
            t_short=resolve_threshold(args.t_short, ml_summary, "t_short", 0.6),
            delta=resolve_threshold(args.delta, ml_summary, "delta", 0.1),
            artifact_suffix=args.artifact_suffix,
        )
        baseline_path = Path(str(baseline_generation["summary_path"]))
        baseline_summary = baseline_generation["summary"]

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
        "ml_evaluation_mode": str(ml_summary.get("evaluation_mode") or ""),
        "baseline_evaluation_mode": str(baseline_summary.get("evaluation_mode") or ""),
        "metrics": comparison_metrics,
        "ml_wins": ml_wins,
        "comparable_metrics": comparable,
        "ml_beats_baseline_on_majority": bool(comparable > 0 and ml_wins > (comparable / 2.0)),
    }
    if baseline_generation is not None:
        report["built_in_baseline"] = {
            "kind": args.built_in_baseline,
            "generation": baseline_generation,
        }
        if args.label == "deterministic_baseline":
            report["baseline_label"] = f"built_in_{args.built_in_baseline}"

    output_json = ml_path.with_name("baseline_comparison.json")
    output_md = ml_path.with_name("baseline_comparison.md")
    dump_json(output_json, report)

    lines = [
        f"baseline_label: {report['baseline_label']}",
        f"ml_evaluation_mode: {report['ml_evaluation_mode']}",
        f"baseline_evaluation_mode: {report['baseline_evaluation_mode']}",
        f"ml_beats_baseline_on_majority: {str(report['ml_beats_baseline_on_majority']).lower()}",
        "",
        "metrics:",
    ]
    for key in metrics:
        row = comparison_metrics[key]
        lines.append(
            f"- {key}: ml={row['ml']} baseline={row['baseline']} delta={row['delta']}"
        )
    if baseline_generation is not None:
        lines.extend(
            [
                "",
                "built_in_baseline:",
                f"- kind: {args.built_in_baseline}",
                f"- summary_path: {baseline_generation['summary_path']}",
            ],
        )
    output_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"wrote baseline comparison -> {output_json}")
    print(f"wrote baseline markdown -> {output_md}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
