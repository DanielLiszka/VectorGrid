from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import pandas as pd
from sklearn.metrics import log_loss, roc_auc_score

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


def safe_auc(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        if y_true.nunique(dropna=True) < 2:
            return None
        return float(roc_auc_score(y_true, y_prob))
    except Exception:
        return None


def safe_logloss(y_true: pd.Series, y_prob: pd.Series) -> float | None:
    try:
        return float(log_loss(y_true, y_prob, labels=[0, 1]))
    except Exception:
        return None


def summarize_target(name: str, frame: pd.DataFrame) -> dict[str, Any]:
    y_col = f"y_{name}"
    p_col = f"p_{name}"
    raw_col = f"raw_{name}"
    rows = len(frame)
    return {
        "rows": int(rows),
        "positive_rate": float(frame[y_col].mean()) if rows else 0.0,
        "auc": safe_auc(frame[y_col], frame[p_col]) if rows else None,
        "logloss": safe_logloss(frame[y_col], frame[p_col]) if rows else None,
        "mean_probability": float(frame[p_col].mean()) if rows else 0.0,
        "mean_raw_score": float(frame[raw_col].mean()) if rows and raw_col in frame.columns else 0.0,
    }


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
        "p_long",
        "raw_long",
        "y_short",
        "p_short",
        "raw_short",
    }
    missing = sorted(required.difference(frame.columns))
    if missing:
        raise SystemExit(f"validation scores are missing required columns: {missing}")

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
                    "short_auc": row["short"]["auc"],
                    "short_logloss": row["short"]["logloss"],
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
                    "short_auc": row["short"]["auc"],
                    "short_logloss": row["short"]["logloss"],
                    "rows_long": row["long"]["rows"],
                    "rows_short": row["short"]["rows"],
                },
            )

    dump_json(out_dir / "validation_slice_metrics.json", summary)
    pd.DataFrame(csv_rows).to_csv(out_dir / "validation_slice_metrics.csv", index=False)

    markdown_lines = [
        f"scores_path: {scores_path}",
        "",
        "overall:",
        f"- long_auc: {summary['overall']['long']['auc']}",
        f"- short_auc: {summary['overall']['short']['auc']}",
        f"- long_logloss: {summary['overall']['long']['logloss']}",
        f"- short_logloss: {summary['overall']['short']['logloss']}",
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
