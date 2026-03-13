#!/usr/bin/env python3
import argparse
import json
import math
import platform
import sys
import time
from pathlib import Path

import numpy as np
import pandas as pd

try:
    import numba as nb
except Exception:
    nb = None

try:
    import vectorbt as vbt
except Exception as exc:
    raise SystemExit(f"vectorbt import failed: {exc}")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="CPU-only VectorBT-side exact-grid double-ALMA crossover benchmark."
    )
    p.add_argument("--csv", default="", help="Path to CSV with at least a close column.")
    p.add_argument("--synth-len", type=int, default=0, help="Synthetic series length when --csv is omitted.")
    p.add_argument("--close-column", default="close", help="Close column name.")
    p.add_argument("--fast-range", default="", help="Range spec start:end:step.")
    p.add_argument("--slow-range", default="", help="Range spec start:end:step.")
    p.add_argument("--fast-start", type=int, default=5)
    p.add_argument("--fast-end", type=int, default=200)
    p.add_argument("--fast-step", type=int, default=1)
    p.add_argument("--slow-start", type=int, default=10)
    p.add_argument("--slow-end", type=int, default=400)
    p.add_argument("--slow-step", type=int, default=1)
    p.add_argument("--alma-offset", type=float, default=0.85)
    p.add_argument("--alma-sigma", type=float, default=6.0)
    p.add_argument("--fees", type=float, default=0.0)
    p.add_argument("--init-cash", type=float, default=10000.0)
    p.add_argument("--chunk-size", type=int, default=256)
    p.add_argument("--batch-size", type=int, default=0, help="Alias for --chunk-size.")
    p.add_argument("--output-json", default="")
    p.add_argument("--out", default="", help="Alias for --output-json.")
    return p.parse_args()


def parse_range(spec: str, start: int, end: int, step: int) -> tuple[int, int, int]:
    text = spec.strip()
    if not text:
        return start, end, step
    parts = text.split(":")
    if len(parts) != 3:
        raise ValueError(f"invalid range spec '{spec}', expected start:end:step")
    return int(parts[0]), int(parts[1]), int(parts[2])


def expand_range(start: int, end: int, step: int) -> np.ndarray:
    if step <= 0:
        raise ValueError("step must be > 0")
    return np.arange(start, end + 1, step, dtype=np.int32)


def build_pairs(fast_periods: np.ndarray, slow_periods: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    fast_vals = []
    slow_vals = []
    for f in fast_periods.tolist():
        for s in slow_periods.tolist():
            if f < s:
                fast_vals.append(f)
                slow_vals.append(s)
    return np.asarray(fast_vals, dtype=np.int32), np.asarray(slow_vals, dtype=np.int32)


def make_synth_close(length: int) -> np.ndarray:
    close = np.empty(length, dtype=np.float64)
    px = 100.0
    for i in range(length):
        drift = 0.00005
        noise = math.sin(i * 0.017) * 0.001
        px *= 1.0 + drift + noise
        close[i] = px
    return close


if nb is not None:
    @nb.njit(cache=True)
    def alma_series(close: np.ndarray, period: int, offset: float, sigma: float) -> np.ndarray:
        out = np.empty(close.shape[0], dtype=np.float64)
        out[:] = np.nan
        if period <= 0 or close.shape[0] < period:
            return out
        m = offset * (period - 1)
        s = period / sigma
        w = np.empty(period, dtype=np.float64)
        w_sum = 0.0
        for i in range(period):
            wi = math.exp(-((i - m) * (i - m)) / (2.0 * s * s))
            w[i] = wi
            w_sum += wi
        for i in range(period):
            w[i] /= w_sum
        for t in range(period - 1, close.shape[0]):
            acc = 0.0
            base = t - period + 1
            for i in range(period):
                acc += close[base + i] * w[i]
            out[t] = acc
        return out
else:
    def alma_series(close: np.ndarray, period: int, offset: float, sigma: float) -> np.ndarray:
        out = np.empty(close.shape[0], dtype=np.float64)
        out[:] = np.nan
        if period <= 0 or close.shape[0] < period:
            return out
        m = offset * (period - 1)
        s = period / sigma
        w = np.array(
            [math.exp(-((i - m) * (i - m)) / (2.0 * s * s)) for i in range(period)],
            dtype=np.float64,
        )
        w /= w.sum()
        for t in range(period - 1, close.shape[0]):
            window = close[t - period + 1 : t + 1]
            out[t] = np.dot(window, w)
        return out


def build_alma_cache(close: np.ndarray, periods: np.ndarray, offset: float, sigma: float) -> dict[int, np.ndarray]:
    cache: dict[int, np.ndarray] = {}
    for period in periods.tolist():
        cache[int(period)] = alma_series(close, int(period), offset, sigma)
    return cache


def compute_log_returns(close: np.ndarray) -> np.ndarray:
    lr = np.zeros(close.shape[0], dtype=np.float64)
    if close.shape[0] < 2:
        return lr
    prev = close[:-1]
    cur = close[1:]
    good = (prev > 0.0) & (cur > 0.0)
    lr[1:][good] = np.log(cur[good] / prev[good])
    return lr


def evaluate_chunk(
    log_returns: np.ndarray,
    fast_cache: dict[int, np.ndarray],
    slow_cache: dict[int, np.ndarray],
    fast_pairs: np.ndarray,
    slow_pairs: np.ndarray,
    start: int,
    end: int,
) -> tuple[float, int]:
    batch_fast = [fast_cache[int(v)] for v in fast_pairs[start:end].tolist()]
    batch_slow = [slow_cache[int(v)] for v in slow_pairs[start:end].tolist()]
    fast_mat = np.vstack(batch_fast)
    slow_mat = np.vstack(batch_slow)

    finite = np.isfinite(fast_mat) & np.isfinite(slow_mat)
    desired = np.zeros_like(fast_mat, dtype=np.int8)
    desired[finite & (fast_mat > slow_mat)] = 1

    positions = np.zeros_like(desired, dtype=np.int8)
    positions[:, 1:] = desired[:, :-1]

    step_returns = positions * np.expm1(log_returns)[None, :]
    means = step_returns.mean(axis=1)
    std = step_returns.std(axis=1, ddof=1)
    sharpe = np.divide(
        means,
        std,
        out=np.zeros_like(means),
        where=std > 0.0,
    )

    best_idx = int(np.nanargmax(sharpe))
    return float(sharpe[best_idx]), start + best_idx


def main() -> int:
    args = parse_args()

    if args.batch_size > 0:
        args.chunk_size = args.batch_size
    output_json = args.output_json or args.out

    fast_start, fast_end, fast_step = parse_range(
        args.fast_range, args.fast_start, args.fast_end, args.fast_step
    )
    slow_start, slow_end, slow_step = parse_range(
        args.slow_range, args.slow_start, args.slow_end, args.slow_step
    )

    dataset_kind = "csv"
    csv_path = ""
    if args.csv:
        csv_path = str(Path(args.csv).resolve())
        df = pd.read_csv(csv_path)
        if args.close_column not in df.columns:
            raise SystemExit(f"close column '{args.close_column}' not found in {csv_path}")
        close = df[args.close_column].astype("float64").to_numpy()
    elif args.synth_len > 0:
        dataset_kind = "synthetic_benchmark_harness"
        close = make_synth_close(args.synth_len)
    else:
        raise SystemExit("either --csv or --synth-len must be provided")

    fast_periods = expand_range(fast_start, fast_end, fast_step)
    slow_periods = expand_range(slow_start, slow_end, slow_step)
    fast_pairs, slow_pairs = build_pairs(fast_periods, slow_periods)
    pair_count = int(fast_pairs.shape[0])

    t0 = time.perf_counter()
    fast_cache = build_alma_cache(close, np.unique(fast_pairs), args.alma_offset, args.alma_sigma)
    slow_cache = build_alma_cache(close, np.unique(slow_pairs), args.alma_offset, args.alma_sigma)
    log_returns = compute_log_returns(close)
    t1 = time.perf_counter()

    best_score = -np.inf
    best_pair_idx = -1
    evaluated = 0
    for start in range(0, pair_count, args.chunk_size):
        end = min(start + args.chunk_size, pair_count)
        score, idx = evaluate_chunk(
            log_returns,
            fast_cache,
            slow_cache,
            fast_pairs,
            slow_pairs,
            start,
            end,
        )
        evaluated += end - start
        if score > best_score:
            best_score = score
            best_pair_idx = idx
    t2 = time.perf_counter()

    best_fast = int(fast_pairs[best_pair_idx])
    best_slow = int(slow_pairs[best_pair_idx])
    best_fast_series = fast_cache[best_fast]
    best_slow_series = slow_cache[best_slow]
    valid = np.isfinite(best_fast_series) & np.isfinite(best_slow_series)
    desired = np.zeros_like(close, dtype=np.int8)
    desired[valid & (best_fast_series > best_slow_series)] = 1
    positions = np.zeros_like(desired)
    positions[1:] = desired[:-1]
    step_returns = positions * np.expm1(log_returns)
    best_pnl = float(np.exp(step_returns.sum()) - 1.0)

    result = {
        "engine": "vectorbt_cpu_workflow",
        "mode": "cpu_only",
        "strategy": "double_alma_crossover",
        "objective": "sharpe",
        "objective_semantics": "app_equivalent_mean_over_sample_std_of_step_returns",
        "exact_search": True,
        "dataset": {
            "kind": dataset_kind,
            "csv": csv_path,
            "close_column": args.close_column,
            "bars": int(close.shape[0]),
        },
        "fast_range": [fast_start, fast_end, fast_step],
        "slow_range": [slow_start, slow_end, slow_step],
        "pair_count": pair_count,
        "evaluated_pairs": evaluated,
        "pair_bars": int(pair_count * close.shape[0]),
        "alma": {
            "offset": args.alma_offset,
            "sigma": args.alma_sigma,
        },
        "fees": args.fees,
        "chunk_size": args.chunk_size,
        "runtime": {
            "alma_prep_ms": (t1 - t0) * 1000.0,
            "grid_eval_ms": (t2 - t1) * 1000.0,
            "total_ms": (t2 - t0) * 1000.0,
        },
        "best": {
            "fast": best_fast,
            "slow": best_slow,
            "sharpe": best_score,
            "pnl": best_pnl,
        },
        "environment": {
            "python": sys.version.split()[0],
            "platform": platform.platform(),
            "vectorbt": getattr(vbt, "__version__", "unknown"),
            "numpy": np.__version__,
            "pandas": pd.__version__,
            "numba": getattr(nb, "__version__", "unavailable") if nb is not None else "unavailable",
        },
        "notes": [
            "CPU-only Python workflow for the exact-grid double-ALMA faceoff.",
            "ALMA generation and app-equivalent Sharpe scoring are implemented locally to match the desktop app's next-bar semantics.",
            "The script records the installed VectorBT stack version for the comparison environment.",
        ],
    }

    out = json.dumps(result, indent=2)
    if output_json:
        Path(output_json).write_text(out, encoding="utf-8")
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
