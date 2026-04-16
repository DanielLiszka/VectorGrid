from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import pandas as pd

from calibration_utils import summarize_probability_metrics
from common import dump_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize MlSignal validation-score calibration quality by slice.",
    )
    parser.add_argument(
        "--scores-csv",
        required=True,
        help="validation_scores.csv produced by train_catboost.py",
    )
    parser.add_argument(
        "--top-tickers",
        type=int,
        default=25,
        help="Maximum number of ticker slices to include in the report.",
    )
    parser.add_argument(
        "--out-dir",
        default="",
        help="Optional output directory. Defaults to the score CSV parent directory.",
    )
    return parser.parse_args()


def preferred_probability_column(name: str, frame: pd.DataFrame) -> str:
    calibrated = f"p_{name}_cal"
    return calibrated if calibrated in frame.columns else f"p_{name}"


def raw_probability_column(name: str, frame: pd.DataFrame) -> str | None:
    column = f"p_{name}"
    return column if column in frame.columns else None


def summarize_target(name: str, frame: pd.DataFrame) -> dict[str, Any]:
    y_col = f"y_{name}"
    p_col = preferred_probability_column(name, frame)
    raw_p_col = raw_probability_column(name, frame)
    raw_col = f"raw_{name}"
    valid = frame[y_col].notna() & frame[p_col].notna()
    valid_frame = frame.loc[valid].copy()
    rows = len(valid_frame)
    summary = summarize_probability_metrics(
        valid_frame[y_col] if rows else pd.Series(dtype=int),
        valid_frame[p_col] if rows else pd.Series(dtype=float),
        bins=10,
    )
    target_summary = {
        "rows": int(rows),
        "preferred_probability_column": p_col,
        "probability_column": p_col,
        "raw_probability_column": raw_p_col,
        "positive_rate": summary["positive_rate"],
        "auc": summary["auc"],
        "logloss": summary["logloss"],
        "brier": summary["brier"],
        "mean_probability": summary["mean_probability"],
        "mean_raw_score": float(valid_frame[raw_col].mean()) if rows and raw_col in valid_frame.columns else 0.0,
        "preferred_probability_metrics": summary,
    }
    if raw_p_col and raw_p_col != p_col and rows:
        raw_metrics = summarize_probability_metrics(valid_frame[y_col], valid_frame[raw_p_col], bins=10)
        target_summary["raw_probability_metrics"] = raw_metrics
        target_summary["logloss_delta_vs_raw"] = (
            float(summary["logloss"] - raw_metrics["logloss"])
            if summary["logloss"] is not None and raw_metrics["logloss"] is not None
            else None
        )
        target_summary["brier_delta_vs_raw"] = (
            float(summary["brier"] - raw_metrics["brier"])
            if summary["brier"] is not None and raw_metrics["brier"] is not None
            else None
        )
        target_summary["auc_delta_vs_raw"] = (
            float(summary["auc"] - raw_metrics["auc"])
            if summary["auc"] is not None and raw_metrics["auc"] is not None
            else None
        )
    return target_summary


def summarize_slice(label: str, frame: pd.DataFrame) -> dict[str, Any]:
    return {
        "slice": label,
        "long": summarize_target("long", frame),
        "short": summarize_target("short", frame),
    }


def main() -> int:
    args = parse_args()
    scores_path = Path(args.scores_csv)
    if not scores_path.exists():
        raise SystemExit(f"validation scores not found: {scores_path}")

    out_dir = Path(args.out_dir) if str(args.out_dir).strip() else scores_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)

    frame = pd.read_csv(scores_path)
    required = {
        "timestamp",
        "ticker_id",
        "split_id",
        "y_long",
        "raw_long",
        "y_short",
        "raw_short",
    }
    missing = sorted(required.difference(frame.columns))
    if missing:
        raise SystemExit(f"validation scores are missing required columns: {missing}")
    if "p_long" not in frame.columns and "p_long_cal" not in frame.columns:
        raise SystemExit("validation scores are missing both p_long and p_long_cal")
    if "p_short" not in frame.columns and "p_short_cal" not in frame.columns:
        raise SystemExit("validation scores are missing both p_short and p_short_cal")

    summary = {
        "scores_path": str(scores_path),
        "overall": summarize_slice("overall", frame),
        "by_split": [],
        "by_holdout": [],
        "by_ticker": [],
    }

    split_rows = [
        summarize_slice(str(split_id), split_frame.copy())
        for split_id, split_frame in frame.groupby("split_id", sort=True)
    ]
    summary["by_split"] = split_rows

    if "ticker_holdout" in frame.columns:
        holdout_numeric = pd.to_numeric(frame["ticker_holdout"], errors="coerce").fillna(0).astype(int)
        frame = frame.copy()
        frame["ticker_holdout"] = holdout_numeric
        summary["by_holdout"] = [
            summarize_slice("holdout", frame.loc[frame["ticker_holdout"] == 1].copy()),
            summarize_slice("non_holdout", frame.loc[frame["ticker_holdout"] != 1].copy()),
        ]

    ticker_groups = list(frame.groupby("ticker_id", sort=True))
    ticker_groups.sort(key=lambda item: len(item[1]), reverse=True)
    top_tickers = ticker_groups[: max(1, int(args.top_tickers))]
    summary["by_ticker"] = [
        summarize_slice(str(ticker_id), ticker_frame.copy())
        for ticker_id, ticker_frame in top_tickers
    ]

    csv_rows: list[dict[str, Any]] = []
    for section_name in ("overall",):
        row = summary[section_name]
        if isinstance(row, dict):
            csv_rows.append(
                {
                    "section": section_name,
                    "slice": row["slice"],
                    "long_auc": row["long"]["auc"],
                    "long_logloss": row["long"]["logloss"],
                    "long_brier": row["long"]["brier"],
                    "short_auc": row["short"]["auc"],
                    "short_logloss": row["short"]["logloss"],
                    "short_brier": row["short"]["brier"],
                    "rows_long": row["long"]["rows"],
                    "rows_short": row["short"]["rows"],
                },
            )
    for section_name in ("by_split", "by_holdout", "by_ticker"):
        rows = summary[section_name]
        if not isinstance(rows, list):
            continue
        for row in rows:
            csv_rows.append(
                {
                    "section": section_name,
                    "slice": row["slice"],
                    "long_auc": row["long"]["auc"],
                    "long_logloss": row["long"]["logloss"],
                    "long_brier": row["long"]["brier"],
                    "short_auc": row["short"]["auc"],
                    "short_logloss": row["short"]["logloss"],
                    "short_brier": row["short"]["brier"],
                    "rows_long": row["long"]["rows"],
                    "rows_short": row["short"]["rows"],
                },
            )

    dump_json(out_dir / "validation_slice_metrics.json", summary)
    pd.DataFrame(csv_rows).to_csv(out_dir / "validation_slice_metrics.csv", index=False)

    markdown_lines = [
        f"scores_path: {scores_path}",
        f"preferred_long_probability_column: {summary['overall']['long']['probability_column']}",
        f"preferred_short_probability_column: {summary['overall']['short']['probability_column']}",
        "",
        "overall:",
        f"- long_auc: {summary['overall']['long']['auc']}",
        f"- short_auc: {summary['overall']['short']['auc']}",
        f"- long_logloss: {summary['overall']['long']['logloss']}",
        f"- short_logloss: {summary['overall']['short']['logloss']}",
        f"- long_brier: {summary['overall']['long']['brier']}",
        f"- short_brier: {summary['overall']['short']['brier']}",
        "",
        f"split_slices: {len(summary['by_split'])}",
        f"holdout_slices: {len(summary['by_holdout'])}",
        f"ticker_slices: {len(summary['by_ticker'])}",
    ]
    (out_dir / "validation_slice_metrics.md").write_text(
        "\n".join(markdown_lines) + "\n",
        encoding="utf-8",
    )

    print(f"wrote calibration slice json -> {out_dir / 'validation_slice_metrics.json'}")
    print(f"wrote calibration slice csv -> {out_dir / 'validation_slice_metrics.csv'}")
    print(f"wrote calibration slice markdown -> {out_dir / 'validation_slice_metrics.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
