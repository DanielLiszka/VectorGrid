from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import pandas as pd

from common import assign_ticker_holdout, dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize MlSignal walk-forward results by ticker holdout assignment.",
    )
    parser.add_argument(
        "--scores",
        required=True,
        help="walkforward_test_scores.csv produced by evaluate_walkforward.py",
    )
    parser.add_argument(
        "--modulus",
        type=int,
        default=0,
        help="Ticker holdout modulus. Required unless --holdout-column is present.",
    )
    parser.add_argument(
        "--residue",
        type=int,
        default=0,
        help="Ticker holdout residue. Required unless --holdout-column is present.",
    )
    parser.add_argument(
        "--holdout-column",
        default="ticker_holdout",
        help="Optional explicit holdout boolean/int column to use when present.",
    )
    parser.add_argument(
        "--out-dir",
        default="",
        help="Optional output directory. Defaults to the score CSV parent directory.",
    )
    return parser.parse_args()


def summarize_group(name: str, frame: pd.DataFrame) -> dict[str, Any]:
    rows_scored = int(len(frame))
    if rows_scored == 0:
        return {
            "group": name,
            "rows_scored": 0,
            "ticker_count": 0,
            "active_share": 0.0,
            "mean_policy_return": 0.0,
            "sum_policy_return": 0.0,
            "positive_ticker_rate": 0.0,
            "signals_long": 0,
            "signals_short": 0,
            "signals_flat": 0,
        }

    ticker_sums = frame.groupby("ticker_id", sort=True)["policy_return"].sum()
    long_count = int((frame["position"] == 1).sum())
    short_count = int((frame["position"] == -1).sum())
    flat_count = int((frame["position"] == 0).sum())
    active_count = long_count + short_count
    return {
        "group": name,
        "rows_scored": rows_scored,
        "ticker_count": int(frame["ticker_id"].nunique(dropna=True)),
        "active_share": float(active_count / rows_scored),
        "mean_policy_return": float(frame["policy_return"].mean()),
        "sum_policy_return": float(frame["policy_return"].sum()),
        "positive_ticker_rate": float((ticker_sums > 0.0).mean()) if len(ticker_sums) else 0.0,
        "signals_long": long_count,
        "signals_short": short_count,
        "signals_flat": flat_count,
    }


def main() -> int:
    args = parse_args()
    scores_path = Path(args.scores)
    if not scores_path.exists():
        raise SystemExit(f"walk-forward scores not found: {scores_path}")

    out_dir = Path(args.out_dir) if str(args.out_dir).strip() else scores_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)

    scored = pd.read_csv(scores_path)
    required_cols = {"ticker_id", "position", "policy_return"}
    missing = sorted(required_cols.difference(scored.columns))
    if missing:
        raise SystemExit(f"score CSV is missing required columns: {missing}")

    holdout_column = str(args.holdout_column or "").strip()
    if holdout_column and holdout_column in scored.columns:
        holdout_raw = pd.to_numeric(scored[holdout_column], errors="coerce").fillna(0)
        scored["holdout_group"] = holdout_raw.astype(int) == 1
        holdout_source = f"column:{holdout_column}"
    else:
        if int(args.modulus) <= 0:
            raise SystemExit(
                "either provide a valid --modulus/--residue pair or include an explicit holdout column in the score CSV",
            )
        scored["holdout_group"] = scored["ticker_id"].map(
            lambda value: bool(
                assign_ticker_holdout(str(value), int(args.modulus), int(args.residue)),
            ),
        )
        holdout_source = f"hash_mod:{int(args.modulus)}:{int(args.residue)}"

    holdout_frame = scored.loc[scored["holdout_group"]].copy()
    non_holdout_frame = scored.loc[~scored["holdout_group"]].copy()
    evaluation_modes = []
    if "evaluation_mode" in scored.columns:
        evaluation_modes = sorted(
            {str(value).strip() for value in scored["evaluation_mode"].dropna().tolist() if str(value).strip()},
        )

    holdout_summary = summarize_group("holdout", holdout_frame)
    non_holdout_summary = summarize_group("non_holdout", non_holdout_frame)
    comparison = {
        "holdout_minus_non_holdout": {
            "mean_policy_return": float(holdout_summary["mean_policy_return"] - non_holdout_summary["mean_policy_return"]),
            "sum_policy_return": float(holdout_summary["sum_policy_return"] - non_holdout_summary["sum_policy_return"]),
            "active_share": float(holdout_summary["active_share"] - non_holdout_summary["active_share"]),
            "positive_ticker_rate": float(
                holdout_summary["positive_ticker_rate"] - non_holdout_summary["positive_ticker_rate"],
            ),
        },
    }

    report = {
        "scores_path": str(scores_path),
        "holdout_source": holdout_source,
        "evaluation_modes": evaluation_modes,
        "groups": {
            "holdout": holdout_summary,
            "non_holdout": non_holdout_summary,
        },
        "comparison": comparison,
    }

    dump_json(out_dir / "holdout_report.json", report)

    lines = [
        f"scores_path: {scores_path}",
        f"holdout_source: {holdout_source}",
        f"evaluation_modes: {', '.join(evaluation_modes) if evaluation_modes else 'unknown'}",
        "",
        "groups:",
        f"- holdout: rows={holdout_summary['rows_scored']} tickers={holdout_summary['ticker_count']} active_share={holdout_summary['active_share']:.4f} sum_return={holdout_summary['sum_policy_return']:.6f} positive_ticker_rate={holdout_summary['positive_ticker_rate']:.4f}",
        f"- non_holdout: rows={non_holdout_summary['rows_scored']} tickers={non_holdout_summary['ticker_count']} active_share={non_holdout_summary['active_share']:.4f} sum_return={non_holdout_summary['sum_policy_return']:.6f} positive_ticker_rate={non_holdout_summary['positive_ticker_rate']:.4f}",
        "",
        "comparison:",
        f"- holdout_minus_non_holdout.mean_policy_return={comparison['holdout_minus_non_holdout']['mean_policy_return']:.6f}",
        f"- holdout_minus_non_holdout.sum_policy_return={comparison['holdout_minus_non_holdout']['sum_policy_return']:.6f}",
        f"- holdout_minus_non_holdout.active_share={comparison['holdout_minus_non_holdout']['active_share']:.6f}",
        f"- holdout_minus_non_holdout.positive_ticker_rate={comparison['holdout_minus_non_holdout']['positive_ticker_rate']:.6f}",
    ]
    (out_dir / "holdout_report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"wrote holdout report -> {out_dir / 'holdout_report.json'}")
    print(f"wrote holdout markdown -> {out_dir / 'holdout_report.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
