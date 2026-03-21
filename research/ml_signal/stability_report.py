from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from common import dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize MlSignal policy stability across sweep or validation artifacts.",
    )
    parser.add_argument(
        "--artifacts",
        nargs="+",
        required=True,
        help="One or more JSON files or directories containing MlSignal sweep/validation outputs.",
    )
    parser.add_argument(
        "--top-n",
        type=int,
        default=3,
        help="Number of leading candidates from each artifact to include in aggregate stability counts.",
    )
    parser.add_argument(
        "--out-path",
        default="",
        help="Optional JSON output path. Defaults next to the first artifact.",
    )
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        raw = json.load(handle)
    if not isinstance(raw, dict):
        raise ValueError(f"expected JSON object at {path}")
    return raw


def resolve_artifact_path(value: str) -> Path:
    path = Path(value)
    if path.is_dir():
        preferred = [
            path / "policy_sweep.json",
            path / "validation.json",
            path / "artifact.json",
        ]
        for candidate in preferred:
            if candidate.exists():
                return candidate
        raise FileNotFoundError(f"no supported artifact json found in {path}")
    if not path.exists():
        raise FileNotFoundError(f"artifact not found: {path}")
    return path


def as_record(value: Any) -> dict[str, Any] | None:
    return value if isinstance(value, dict) else None


def read_candidates(payload: dict[str, Any]) -> list[dict[str, Any]]:
    candidate_sources = [
        payload,
        as_record(payload.get("validation")),
        as_record(payload.get("res")),
        as_record(as_record(payload.get("validation")) or {}),
    ]
    validation = as_record(payload.get("validation"))
    if validation:
        candidate_sources.append(as_record(validation.get("res")))

    for source in candidate_sources:
        if not source:
            continue
        candidates = source.get("candidates")
        if isinstance(candidates, list):
            out = [
                entry for entry in candidates if isinstance(entry, dict) and isinstance(entry.get("params"), dict)
            ]
            if out:
                return out
    return []


def normalize_policy(params: dict[str, Any]) -> dict[str, Any]:
    return {
        "t_long": float(params.get("t_long", 0.0) or 0.0),
        "t_short": float(params.get("t_short", 0.0) or 0.0),
        "delta": float(params.get("delta", 0.0) or 0.0),
        "max_hold_bars": int(params.get("max_hold_bars", 0) or 0),
        "exit_on_flat": bool(params.get("exit_on_flat", False)),
        "exit_on_opposite": bool(params.get("exit_on_opposite", False)),
        "label": str(params.get("label", "") or "").strip(),
    }


def policy_key(policy: dict[str, Any]) -> str:
    return "|".join(
        [
            f"t_long={policy['t_long']:.6f}",
            f"t_short={policy['t_short']:.6f}",
            f"delta={policy['delta']:.6f}",
            f"hold={int(policy['max_hold_bars'])}",
            f"flat={int(bool(policy['exit_on_flat']))}",
            f"opp={int(bool(policy['exit_on_opposite']))}",
        ],
    )


def extract_metric(candidate: dict[str, Any], name: str) -> float | None:
    raw = candidate.get(name)
    try:
        value = float(raw)
    except Exception:
        return None
    return value if value == value else None


def choose_sorted_candidates(candidates: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return sorted(
        candidates,
        key=lambda row: (
            extract_metric(row, "robust_score") is not None,
            extract_metric(row, "robust_score") or float("-inf"),
        ),
        reverse=True,
    )


def main() -> int:
    args = parse_args()
    paths = [resolve_artifact_path(value) for value in args.artifacts]
    if not paths:
        raise SystemExit("no artifacts provided")

    out_path = Path(args.out_path) if str(args.out_path).strip() else paths[0].with_name("stability_report.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)

    exact_counter: Counter[str] = Counter()
    top_n_counter: Counter[str] = Counter()
    parameter_samples: dict[str, list[float]] = defaultdict(list)
    top_candidates: list[dict[str, Any]] = []
    reports_considered = 0

    for path in paths:
        payload = read_json(path)
        candidates = choose_sorted_candidates(read_candidates(payload))
        if not candidates:
            continue
        reports_considered += 1
        best = candidates[0]
        best_policy = normalize_policy(as_record(best.get("params")) or {})
        best_key = policy_key(best_policy)
        exact_counter[best_key] += 1
        top_candidates.append(
            {
                "artifact_path": str(path),
                "policy": best_policy,
                "robust_score": extract_metric(best, "robust_score"),
                "psr": extract_metric(best, "psr"),
                "dsr": extract_metric(best, "dsr"),
            },
        )
        parameter_samples["t_long"].append(float(best_policy["t_long"]))
        parameter_samples["t_short"].append(float(best_policy["t_short"]))
        parameter_samples["delta"].append(float(best_policy["delta"]))
        parameter_samples["max_hold_bars"].append(float(best_policy["max_hold_bars"]))

        for candidate in candidates[: max(1, int(args.top_n))]:
            params = as_record(candidate.get("params")) or {}
            top_n_counter[policy_key(normalize_policy(params))] += 1

    if reports_considered == 0:
        raise SystemExit("no artifacts contained MlSignal candidate rows")

    most_common_exact = exact_counter.most_common(1)[0]
    exact_stability = float(most_common_exact[1] / reports_considered)
    top_n_total = sum(top_n_counter.values())
    most_common_top_n = top_n_counter.most_common(5)

    parameter_summary = {
        name: {
            "count": len(values),
            "min": min(values) if values else 0.0,
            "max": max(values) if values else 0.0,
            "mean": (sum(values) / len(values)) if values else 0.0,
            "range": (max(values) - min(values)) if values else 0.0,
        }
        for name, values in parameter_samples.items()
    }

    report = {
        "artifacts_considered": reports_considered,
        "top_n_considered": max(1, int(args.top_n)),
        "exact_top_policy_stability": exact_stability,
        "most_common_exact_top_policy": {
            "policy_key": most_common_exact[0],
            "count": most_common_exact[1],
        },
        "parameter_summary": parameter_summary,
        "top_policy_frequency": [
            {"policy_key": key, "count": count}
            for key, count in exact_counter.most_common(10)
        ],
        "top_n_policy_frequency": [
            {
                "policy_key": key,
                "count": count,
                "share": float(count / top_n_total) if top_n_total else 0.0,
            }
            for key, count in most_common_top_n
        ],
        "best_candidates": top_candidates,
    }

    dump_json(out_path, report)

    markdown_lines = [
        f"artifacts_considered: {reports_considered}",
        f"top_n_considered: {max(1, int(args.top_n))}",
        f"exact_top_policy_stability: {exact_stability:.4f}",
        "",
        "most_common_exact_top_policy:",
        f"- policy_key: {most_common_exact[0]}",
        f"- count: {most_common_exact[1]}",
        "",
        "parameter_summary:",
    ]
    for name, summary in parameter_summary.items():
        markdown_lines.append(
            f"- {name}: min={summary['min']:.6f} max={summary['max']:.6f} mean={summary['mean']:.6f} range={summary['range']:.6f}",
        )
    markdown_lines.extend(
        [
            "",
            "top_policy_frequency:",
        ],
    )
    for entry in report["top_policy_frequency"]:
        markdown_lines.append(f"- {entry['policy_key']} -> {entry['count']}")

    md_path = out_path.with_suffix(".md")
    md_path.write_text("\n".join(markdown_lines) + "\n", encoding="utf-8")

    print(f"wrote stability report -> {out_path}")
    print(f"wrote stability markdown -> {md_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
