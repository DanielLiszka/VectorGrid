from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from common import dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate a research gate report for an MlSignal bundle.",
    )
    parser.add_argument("--bundle-dir", required=True, help="Model bundle directory.")
    parser.add_argument(
        "--min-positive-roll-rate",
        type=float,
        default=0.55,
        help="Minimum acceptable positive walk-forward roll rate.",
    )
    parser.add_argument(
        "--min-active-share",
        type=float,
        default=0.02,
        help="Minimum acceptable non-flat signal share.",
    )
    parser.add_argument(
        "--min-auc",
        type=float,
        default=0.52,
        help="Minimum acceptable calibration AUC for long/short models.",
    )
    parser.add_argument(
        "--min-rows-scored",
        type=int,
        default=100,
        help="Minimum acceptable number of evaluated walk-forward rows.",
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


def as_int(value: Any) -> int | None:
    try:
        return int(value)
    except Exception:
        return None


def summarize_check(name: str, passed: bool, detail: str) -> dict[str, Any]:
    return {
        "name": name,
        "passed": bool(passed),
        "detail": detail,
    }


def main() -> int:
    args = parse_args()
    bundle_dir = Path(args.bundle_dir)
    if not bundle_dir.exists():
        raise SystemExit(f"bundle dir not found: {bundle_dir}")

    manifest = read_json(bundle_dir / "manifest.json")
    metrics = read_json(bundle_dir / "metrics.json")
    walkforward = read_json(bundle_dir / "walkforward_summary.json")
    walkforward_rolls = read_json(bundle_dir / "walkforward_rolls.json")
    holdout_report = (
        read_json(bundle_dir / "holdout_report.json")
        if (bundle_dir / "holdout_report.json").exists()
        else None
    )
    baseline_comparison = (
        read_json(bundle_dir / "baseline_comparison.json")
        if (bundle_dir / "baseline_comparison.json").exists()
        else None
    )
    stability_report = (
        read_json(bundle_dir / "stability_report.json")
        if (bundle_dir / "stability_report.json").exists()
        else None
    )

    long_metrics = metrics.get("long") if isinstance(metrics.get("long"), dict) else {}
    short_metrics = metrics.get("short") if isinstance(metrics.get("short"), dict) else {}
    rolls = walkforward_rolls.get("rolls") if isinstance(walkforward_rolls.get("rolls"), list) else []

    rows_scored = as_int(walkforward.get("rows_scored")) or 0
    positive_roll_rate = as_float(walkforward.get("positive_roll_rate")) or 0.0
    active_share = as_float(walkforward.get("active_share")) or 0.0
    mean_policy_return = as_float(walkforward.get("mean_policy_return")) or 0.0
    sum_policy_return = as_float(walkforward.get("sum_policy_return")) or 0.0
    long_auc = as_float(long_metrics.get("auc_calibration"))
    short_auc = as_float(short_metrics.get("auc_calibration"))

    checks = [
        summarize_check(
            "rows_scored",
            rows_scored >= args.min_rows_scored,
            f"rows_scored={rows_scored} threshold={args.min_rows_scored}",
        ),
        summarize_check(
            "positive_roll_rate",
            positive_roll_rate >= args.min_positive_roll_rate,
            f"positive_roll_rate={positive_roll_rate:.4f} threshold={args.min_positive_roll_rate:.4f}",
        ),
        summarize_check(
            "active_share",
            active_share >= args.min_active_share,
            f"active_share={active_share:.4f} threshold={args.min_active_share:.4f}",
        ),
        summarize_check(
            "long_auc",
            long_auc is not None and long_auc >= args.min_auc,
            f"long_auc={long_auc if long_auc is not None else 'n/a'} threshold={args.min_auc:.4f}",
        ),
        summarize_check(
            "short_auc",
            short_auc is not None and short_auc >= args.min_auc,
            f"short_auc={short_auc if short_auc is not None else 'n/a'} threshold={args.min_auc:.4f}",
        ),
        summarize_check(
            "aggregate_policy_return",
            sum_policy_return > 0.0 and mean_policy_return > 0.0,
            f"sum_policy_return={sum_policy_return:.6f} mean_policy_return={mean_policy_return:.6f}",
        ),
    ]

    passed = all(check["passed"] for check in checks)
    failed_names = [str(check["name"]) for check in checks if not check["passed"]]

    report = {
        "bundle_id": manifest.get("bundle_id", ""),
        "bundle_dir": str(bundle_dir),
        "passed": passed,
        "failed_checks": failed_names,
        "checks": checks,
        "summary": {
            "rows_scored": rows_scored,
            "splits_scored": as_int(walkforward.get("splits_scored")) or 0,
            "positive_roll_rate": positive_roll_rate,
            "active_share": active_share,
            "mean_policy_return": mean_policy_return,
            "sum_policy_return": sum_policy_return,
            "long_auc": long_auc,
            "short_auc": short_auc,
            "roll_count": len(rolls),
        },
        "baseline_comparison_present": bool(baseline_comparison),
        "holdout_report_present": bool(holdout_report),
        "stability_report_present": bool(stability_report),
        "notes": [
            "This report is a research gate only. It does not replace deterministic baseline comparison.",
            "Ticker-holdout comparison remains a separate research step.",
            "Threshold-stability review still requires comparing multiple policy outputs across rolls.",
        ],
    }
    if isinstance(baseline_comparison, dict):
        report["baseline_summary"] = {
            "baseline_label": baseline_comparison.get("baseline_label"),
            "ml_beats_baseline_on_majority": baseline_comparison.get(
                "ml_beats_baseline_on_majority",
            ),
            "ml_wins": baseline_comparison.get("ml_wins"),
            "comparable_metrics": baseline_comparison.get("comparable_metrics"),
        }
    if isinstance(holdout_report, dict):
        holdout_groups = holdout_report.get("groups")
        if isinstance(holdout_groups, dict):
            report["holdout_summary"] = holdout_groups
    if isinstance(stability_report, dict):
        report["stability_summary"] = {
            "artifacts_considered": stability_report.get("artifacts_considered"),
            "exact_top_policy_stability": stability_report.get("exact_top_policy_stability"),
            "most_common_exact_top_policy": stability_report.get("most_common_exact_top_policy"),
        }

    dump_json(bundle_dir / "gate_report.json", report)

    lines = [
        f"bundle_id: {report['bundle_id']}",
        f"passed: {str(passed).lower()}",
        "",
        "checks:",
    ]
    for check in checks:
        marker = "PASS" if check["passed"] else "FAIL"
        lines.append(f"- [{marker}] {check['name']}: {check['detail']}")
    lines.extend(
        [
            "",
            "notes:",
            "- This report is a research gate only.",
            "- Deterministic baseline and ticker-holdout comparisons are still required.",
        ],
    )
    if isinstance(holdout_report, dict):
        lines.extend(
            [
                "",
                "holdout_report:",
                "- present: true",
            ],
        )
    if isinstance(baseline_comparison, dict):
        lines.extend(
            [
                "",
                "baseline_comparison:",
                "- present: true",
                f"- baseline_label: {baseline_comparison.get('baseline_label', '')}",
                f"- ml_beats_baseline_on_majority: {baseline_comparison.get('ml_beats_baseline_on_majority', False)}",
            ],
        )
    if isinstance(stability_report, dict):
        exact_top_policy_stability = as_float(
            stability_report.get("exact_top_policy_stability"),
        )
        lines.extend(
            [
                "",
                "stability_report:",
                f"- present: true",
                f"- exact_top_policy_stability: {exact_top_policy_stability:.4f}"
                if exact_top_policy_stability is not None
                else "- exact_top_policy_stability: n/a",
            ],
        )
    (bundle_dir / "gate_report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"wrote gate report -> {bundle_dir / 'gate_report.json'}")
    print(f"wrote gate markdown -> {bundle_dir / 'gate_report.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
