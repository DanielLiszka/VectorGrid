#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import platform
import sys
import time
from collections import OrderedDict
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


LEGACY_MODE = "search_only_legacy"
REAL_WORKFLOW_MODE = "real_workflow_10row"
METRIC_NAMES_10ROW = [
    "pnl",
    "trades",
    "max_dd",
    "mean",
    "std",
    "exposure",
    "net_exposure",
    "downside_deviation",
    "ulcer_index",
    "omega",
]
EPS_DENOM = 1.0e-12
OMEGA_CLAMP_MAX = 1.0e6


def optional_njit(*args, **kwargs):
    if nb is None:
        def decorator(fn):
            return fn
        return decorator
    return nb.njit(*args, **kwargs)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="VectorBT vs VectorGrid double-ALMA faceoff harness."
    )
    parser.add_argument("--csv", default="", help="Path to CSV with at least a close column.")
    parser.add_argument("--synth-len", type=int, default=0)
    parser.add_argument("--close-column", default="close")
    parser.add_argument(
        "--benchmark-mode",
        choices=(LEGACY_MODE, REAL_WORKFLOW_MODE),
        default=LEGACY_MODE,
    )
    parser.add_argument("--fast-range", default="")
    parser.add_argument("--slow-range", default="")
    parser.add_argument("--fast-start", type=int, default=5)
    parser.add_argument("--fast-end", type=int, default=200)
    parser.add_argument("--fast-step", type=int, default=1)
    parser.add_argument("--slow-start", type=int, default=10)
    parser.add_argument("--slow-end", type=int, default=400)
    parser.add_argument("--slow-step", type=int, default=1)
    parser.add_argument("--alma-offset", type=float, default=0.85)
    parser.add_argument("--alma-sigma", type=float, default=6.0)
    parser.add_argument("--fees", type=float, default=0.0)
    parser.add_argument("--hurdle-tau", type=float, default=0.0)
    parser.add_argument("--chunk-size", type=int, default=256)
    parser.add_argument("--batch-size", type=int, default=0)
    parser.add_argument("--cache-max-entries", type=int, default=1024)
    parser.add_argument("--long-only", default="true", choices=("true", "false"))
    parser.add_argument("--allow-flip", default="true", choices=("true", "false"))
    parser.add_argument("--trade-on-next-bar", default="true", choices=("true", "false"))
    parser.add_argument("--output-json", default="")
    parser.add_argument("--out", default="")
    return parser.parse_args()


def parse_bool(text: str) -> bool:
    return text.strip().lower() == "true"


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
    if end < start:
        raise ValueError("end must be >= start")
    return np.arange(start, end + 1, step, dtype=np.int32)


def count_valid_pairs(fast_periods: np.ndarray, slow_periods: np.ndarray) -> int:
    total = 0
    for fast_period in fast_periods.tolist():
        start_idx = int(np.searchsorted(slow_periods, fast_period + 1, side="left"))
        total += int(slow_periods.shape[0] - start_idx)
    return total


def make_synth_close(length: int) -> np.ndarray:
    close = np.empty(length, dtype=np.float64)
    price = 100.0
    for idx in range(length):
        drift = 0.00005
        noise = math.sin(idx * 0.017) * 0.001
        price *= 1.0 + drift + noise
        close[idx] = price
    return close


@optional_njit(cache=True)
def alma_series(close: np.ndarray, period: int, offset: float, sigma: float) -> np.ndarray:
    out = np.empty(close.shape[0], dtype=np.float64)
    out[:] = np.nan
    if period <= 0 or close.shape[0] < period:
        return out
    m = offset * (period - 1)
    s = period / sigma
    weights = np.empty(period, dtype=np.float64)
    weight_sum = 0.0
    for idx in range(period):
        weight = math.exp(-((idx - m) * (idx - m)) / (2.0 * s * s))
        weights[idx] = weight
        weight_sum += weight
    for idx in range(period):
        weights[idx] /= weight_sum
    for t in range(period - 1, close.shape[0]):
        acc = 0.0
        base = t - period + 1
        for idx in range(period):
            acc += close[base + idx] * weights[idx]
        out[t] = acc
    return out


def compute_log_returns(close: np.ndarray) -> np.ndarray:
    out = np.zeros(close.shape[0], dtype=np.float64)
    prev = close[:-1]
    cur = close[1:]
    good = (prev > 0.0) & (cur > 0.0)
    out[1:][good] = np.log(cur[good] / prev[good])
    return out


class AlmaSeriesCache:
    def __init__(self, close: np.ndarray, offset: float, sigma: float, max_entries: int) -> None:
        self.close = close
        self.offset = offset
        self.sigma = sigma
        self.max_entries = max(1, int(max_entries))
        self._cache: OrderedDict[int, np.ndarray] = OrderedDict()
        self.hits = 0
        self.misses = 0
        self.evictions = 0
        self.compute_ms = 0.0

    def get(self, period: int) -> np.ndarray:
        key = int(period)
        cached = self._cache.get(key)
        if cached is not None:
            self.hits += 1
            self._cache.move_to_end(key)
            return cached
        self.misses += 1
        started = time.perf_counter()
        series = alma_series(self.close, key, self.offset, self.sigma)
        self.compute_ms += (time.perf_counter() - started) * 1000.0
        self._cache[key] = series
        while len(self._cache) > self.max_entries:
            self._cache.popitem(last=False)
            self.evictions += 1
        return series

    def stack(self, periods: np.ndarray) -> np.ndarray:
        rows = [self.get(int(period)) for period in periods.tolist()]
        return np.stack(rows, axis=0)

    def stats(self) -> dict[str, object]:
        return {
            "max_entries": int(self.max_entries),
            "live_entries": len(self._cache),
            "hits": self.hits,
            "misses": self.misses,
            "evictions": self.evictions,
            "compute_ms": self.compute_ms,
        }


def iter_fast_major_blocks(
    fast_periods: np.ndarray,
    slow_periods: np.ndarray,
    chunk_size: int,
):
    if chunk_size <= 0:
        raise ValueError("chunk_size must be > 0")
    for fast_period in fast_periods.tolist():
        start_idx = int(np.searchsorted(slow_periods, fast_period + 1, side="left"))
        valid_slow = slow_periods[start_idx:]
        for block_start in range(0, valid_slow.shape[0], chunk_size):
            slow_block = valid_slow[block_start : block_start + chunk_size]
            if slow_block.shape[0] > 0:
                yield int(fast_period), slow_block


def evaluate_legacy_block_impl(
    log_returns: np.ndarray,
    fast_series: np.ndarray,
    slow_matrix: np.ndarray,
) -> tuple[int, float, float]:
    rows = slow_matrix.shape[0]
    cols = fast_series.shape[0]
    best_idx = 0
    best_sharpe = -1.0e300
    best_pnl = 0.0

    for row in range(rows):
        count = 1
        mean = 0.0
        m2 = 0.0
        step_sum = 0.0
        for t in range(1, cols):
            step_r = 0.0
            fast_sig = fast_series[t - 1]
            slow_sig = slow_matrix[row, t - 1]
            if math.isfinite(fast_sig) and math.isfinite(slow_sig) and fast_sig > slow_sig:
                step_r = math.expm1(log_returns[t])
            step_sum += step_r
            count += 1
            delta = step_r - mean
            mean += delta / count
            delta2 = step_r - mean
            m2 += delta * delta2
        variance = m2 / (count - 1) if count > 1 else 0.0
        std = math.sqrt(variance)
        sharpe = mean / std if std > 0.0 else 0.0
        if sharpe > best_sharpe:
            best_idx = row
            best_sharpe = sharpe
            best_pnl = math.exp(step_sum) - 1.0

    return best_idx, best_sharpe, best_pnl


def evaluate_real_block_impl(
    log_returns: np.ndarray,
    fast_series: np.ndarray,
    slow_matrix: np.ndarray,
    slow_periods: np.ndarray,
    fast_period: int,
    long_only: bool,
    allow_flip: bool,
    trade_on_next_bar: bool,
    commission: float,
    hurdle_tau: float,
) -> tuple[int, float, float, float, float, float, float, float, float, float, float, float]:
    rows = slow_matrix.shape[0]
    cols = fast_series.shape[0]
    trade_offset = 1 if trade_on_next_bar else 0
    log_comm = math.log(1.0 - commission) if commission > 0.0 else 0.0
    best_idx = 0
    best_sharpe = -1.0e300
    best_metrics = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)

    for row in range(rows):
        slow_period = int(slow_periods[row])
        start_t = max(fast_period, slow_period) - 1 + trade_offset
        if start_t >= cols:
            sharpe = 0.0
            if sharpe > best_sharpe:
                best_idx = row
                best_sharpe = sharpe
                best_metrics = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, sharpe)
            continue

        pos = 0
        log_eq = 0.0
        log_peak = 0.0
        max_log_dd = 0.0
        ulcer_sum_sq = 0.0
        trades = 0
        count = 0
        mean = 0.0
        m2 = 0.0
        sum_neg_sq = 0.0
        sum_pos_r = 0.0
        sum_neg_abs_r = 0.0
        sum_abs_pos = 0.0
        sum_pos = 0.0

        for t in range(start_t, cols):
            t_sig = t - trade_offset if trade_on_next_bar else t
            fast_sig = fast_series[t_sig]
            slow_sig = slow_matrix[row, t_sig]
            if math.isfinite(fast_sig) and math.isfinite(slow_sig):
                if fast_sig > slow_sig:
                    sign = 1
                elif fast_sig < slow_sig:
                    sign = -1
                else:
                    sign = 0
            else:
                sign = 0
            if long_only and sign < 0:
                sign = 0

            if sign != pos:
                if pos == 0 or sign == 0:
                    if commission > 0.0:
                        log_eq += log_comm
                    trades += 1
                    pos = sign
                elif not allow_flip:
                    if commission > 0.0:
                        log_eq += log_comm
                    trades += 1
                    pos = 0
                else:
                    if commission > 0.0:
                        log_eq += 2.0 * log_comm
                    trades += 2
                    pos = sign

            pos_bar = pos
            lr_t = log_returns[t]
            log_eq += pos_bar * lr_t

            if log_eq > log_peak:
                log_peak = log_eq
            dd_log = log_peak - log_eq
            if dd_log > max_log_dd:
                max_log_dd = dd_log
            dd = -math.expm1(-dd_log) if dd_log > 0.0 else 0.0
            ulcer_sum_sq += dd * dd

            step_r = 0.0 if pos_bar == 0 else pos_bar * math.expm1(lr_t)
            count += 1
            delta = step_r - mean
            mean += delta / count
            delta2 = step_r - mean
            m2 += delta * delta2

            excess_r = step_r - hurdle_tau
            if excess_r < 0.0:
                sum_neg_sq += excess_r * excess_r
                sum_neg_abs_r += -excess_r
            else:
                sum_pos_r += excess_r

            sum_abs_pos += abs(pos_bar)
            sum_pos += pos_bar

        variance = m2 / (count - 1) if count > 1 else 0.0
        std = math.sqrt(variance)
        sharpe = mean / std if std > 0.0 else 0.0
        downside_deviation = math.sqrt(sum_neg_sq / count) if count > 0 else 0.0
        omega_raw = (sum_pos_r + EPS_DENOM) / (sum_neg_abs_r + EPS_DENOM)
        omega = min(max(omega_raw, 0.0), OMEGA_CLAMP_MAX)
        max_dd = -math.expm1(-max_log_dd) if max_log_dd > 0.0 else 0.0
        ulcer_index = math.sqrt(ulcer_sum_sq / count) if count > 0 else 0.0
        exposure = sum_abs_pos / count if count > 0 else 0.0
        net_exposure = sum_pos / count if count > 0 else 0.0
        pnl = math.exp(log_eq) - 1.0

        if sharpe > best_sharpe:
            best_idx = row
            best_sharpe = sharpe
            best_metrics = (
                pnl,
                float(trades),
                max_dd,
                mean,
                std,
                exposure,
                net_exposure,
                downside_deviation,
                ulcer_index,
                omega,
                sharpe,
            )

    return (best_idx,) + best_metrics


if nb is not None:
    evaluate_legacy_block = nb.njit(cache=True)(evaluate_legacy_block_impl)
    evaluate_real_block = nb.njit(cache=True)(evaluate_real_block_impl)
else:
    evaluate_legacy_block = evaluate_legacy_block_impl
    evaluate_real_block = evaluate_real_block_impl


def build_metric_payload(mode: str) -> dict[str, object]:
    if mode == LEGACY_MODE:
        return {
            "mode": mode,
            "count": 1,
            "names": ["sharpe"],
            "notes": [
                "Legacy search-only lane.",
                "Timed candidate evaluation computes step_returns, mean, std, and sharpe.",
                "pnl is derived only for the winning candidate.",
            ],
        }
    return {
        "mode": mode,
        "count": len(METRIC_NAMES_10ROW),
        "names": METRIC_NAMES_10ROW,
        "semantics": METRIC_SEMANTICS_10ROW,
        "notes": [
            "Real-workflow 10-row lane.",
            "Timed candidate evaluation computes the default VectorGrid 10-row metric payload.",
            "VectorBT is imported for environment attribution; the timed path uses local metric aggregation.",
        ],
    }


def run_search_only_legacy(
    log_returns: np.ndarray,
    cache: AlmaSeriesCache,
    fast_periods: np.ndarray,
    slow_periods: np.ndarray,
    chunk_size: int,
) -> dict[str, object]:
    simple_returns = np.expm1(log_returns)
    eval_started = time.perf_counter()
    best_sharpe = -math.inf
    best = None
    evaluated_pairs = 0

    for fast_period, slow_block in iter_fast_major_blocks(fast_periods, slow_periods, chunk_size):
        fast_series = cache.get(fast_period)
        slow_matrix = cache.stack(slow_block)
        evaluated_pairs += int(slow_block.shape[0])
        best_idx, block_sharpe = evaluate_legacy_block(
            log_returns,
            simple_returns,
            fast_series,
            slow_matrix,
            slow_block.astype(np.int32, copy=False),
            int(fast_period),
        )
        if best_idx >= 0 and block_sharpe > best_sharpe:
            best_sharpe = block_sharpe
            best_slow = int(slow_block[int(best_idx)])
            best = {
                "fast": int(fast_period),
                "slow": best_slow,
                "sharpe": float(block_sharpe),
            }

    if best is None:
        raise SystemExit("legacy evaluation found no valid candidate")

    best_fast_series = cache.get(int(best["fast"]))
    best_slow_series = cache.get(int(best["slow"]))
    positions = np.zeros_like(log_returns, dtype=np.int8)
    start_t = max(int(best["fast"]), int(best["slow"]))
    for t in range(start_t, log_returns.shape[0]):
        t_sig = t - 1
        if best_fast_series[t_sig] > best_slow_series[t_sig]:
            positions[t] = 1
    best["pnl"] = float(np.exp((positions.astype(np.float64) * log_returns).sum()) - 1.0)

    return {
        "best": best,
        "evaluated_pairs": evaluated_pairs,
        "grid_eval_ms": (time.perf_counter() - eval_started) * 1000.0,
    }


def run_real_workflow_10row(
    log_returns: np.ndarray,
    cache: AlmaSeriesCache,
    fast_periods: np.ndarray,
    slow_periods: np.ndarray,
    chunk_size: int,
    long_only: bool,
    allow_flip: bool,
    trade_on_next_bar: bool,
    commission: float,
    hurdle_tau: float,
) -> dict[str, object]:
    eval_started = time.perf_counter()
    best_sharpe = -math.inf
    best = None
    evaluated_pairs = 0

    for fast_period, slow_block in iter_fast_major_blocks(fast_periods, slow_periods, chunk_size):
        fast_series = cache.get(fast_period)
        slow_matrix = cache.stack(slow_block)
        evaluated_pairs += int(slow_block.shape[0])
        (
            best_idx,
            pnl,
            trades,
            max_dd,
            mean,
            std,
            exposure,
            net_exposure,
            downside_deviation,
            ulcer_index,
            omega,
            sharpe,
        ) = evaluate_real_block(
            log_returns,
            fast_series,
            slow_matrix,
            slow_block.astype(np.int32, copy=False),
            int(fast_period),
            long_only,
            allow_flip,
            trade_on_next_bar,
            commission,
            hurdle_tau,
        )
        if best_idx >= 0 and sharpe > best_sharpe:
            best_sharpe = sharpe
            best = {
                "fast": int(fast_period),
                "slow": int(slow_block[int(best_idx)]),
                "sharpe": float(sharpe),
                "pnl": float(pnl),
                "metrics": {
                    "pnl": float(pnl),
                    "trades": int(trades),
                    "max_dd": float(max_dd),
                    "mean": float(mean),
                    "std": float(std),
                    "exposure": float(exposure),
                    "net_exposure": float(net_exposure),
                    "downside_deviation": float(downside_deviation),
                    "ulcer_index": float(ulcer_index),
                    "omega": float(omega),
                },
            }

    if best is None:
        raise SystemExit("real_workflow_10row evaluation found no valid candidate")

    return {
        "best": best,
        "evaluated_pairs": evaluated_pairs,
        "grid_eval_ms": (time.perf_counter() - eval_started) * 1000.0,
    }


def load_close(args: argparse.Namespace) -> tuple[np.ndarray, str, str]:
    dataset_kind = "csv"
    csv_path = ""
    if args.csv:
        csv_path = str(Path(args.csv).resolve())
        frame = pd.read_csv(csv_path)
        if args.close_column not in frame.columns:
            raise SystemExit(f"close column '{args.close_column}' not found in {csv_path}")
        close = frame[args.close_column].astype("float64").to_numpy()
    elif args.synth_len > 0:
        dataset_kind = "synthetic_benchmark_harness"
        close = make_synth_close(args.synth_len)
    else:
        raise SystemExit("either --csv or --synth-len must be provided")
    if close.shape[0] < 2:
        raise SystemExit("close series must contain at least 2 bars")
    return close, dataset_kind, csv_path


def build_metric_payload(mode: str) -> dict[str, object]:
    if mode == LEGACY_MODE:
        return {
            "count": 1,
            "names": ["sharpe"],
            "winner_only_metrics": ["pnl"],
        }
    return {
        "count": len(METRIC_NAMES_10ROW),
        "names": METRIC_NAMES_10ROW,
        "objective": "sharpe",
        "objective_from_metrics": ["mean", "std"],
    }


def build_metric_semantics(
    mode: str,
    long_only: bool,
    allow_flip: bool,
    trade_on_next_bar: bool,
    hurdle_tau: float,
) -> dict[str, object]:
    if mode == LEGACY_MODE:
        return {
            "objective": "mean_over_sample_std_of_full_length_step_returns",
            "timed_metrics_per_candidate": ["mean", "std", "sharpe"],
            "winner_only_metrics": ["pnl"],
            "matches_legacy_faceoff": True,
        }
    return {
        "source": "local_python_implementation_matching_repo_double_ma_defaults",
        "timed_metrics_per_candidate": METRIC_NAMES_10ROW + ["sharpe"],
        "trade_on_next_bar": trade_on_next_bar,
        "long_only": long_only,
        "allow_flip": allow_flip,
        "commission_applied_in_log_space": True,
        "hurdle_tau": hurdle_tau,
    }


def run_faceoff(args: argparse.Namespace) -> dict[str, object]:
    if args.batch_size > 0:
        args.chunk_size = args.batch_size
    if args.chunk_size <= 0:
        raise SystemExit("chunk-size must be > 0")
    if args.cache_max_entries <= 0:
        raise SystemExit("cache-max-entries must be > 0")
    if not (0.0 <= args.fees < 1.0):
        raise SystemExit("fees must be in [0, 1)")

    long_only = parse_bool(args.long_only)
    allow_flip = parse_bool(args.allow_flip)
    trade_on_next_bar = parse_bool(args.trade_on_next_bar)

    fast_start, fast_end, fast_step = parse_range(
        args.fast_range, args.fast_start, args.fast_end, args.fast_step
    )
    slow_start, slow_end, slow_step = parse_range(
        args.slow_range, args.slow_start, args.slow_end, args.slow_step
    )
    fast_periods = expand_range(fast_start, fast_end, fast_step)
    slow_periods = expand_range(slow_start, slow_end, slow_step)
    pair_count = count_valid_pairs(fast_periods, slow_periods)
    if pair_count <= 0:
        raise SystemExit("no valid fast < slow pairs in the requested ranges")

    close, dataset_kind, csv_path = load_close(args)
    log_returns = compute_log_returns(close)
    cache = AlmaSeriesCache(close, args.alma_offset, args.alma_sigma, args.cache_max_entries)

    started = time.perf_counter()
    evaluated_pairs = 0
    best = {
        "fast": int(fast_periods[0]),
        "slow": int(slow_periods[-1]),
        "sharpe": -math.inf,
        "pnl": 0.0,
    }

    for fast_period, slow_block in iter_fast_major_blocks(fast_periods, slow_periods, args.chunk_size):
        fast_series = cache.get(fast_period)
        slow_matrix = cache.stack(slow_block)
        if args.benchmark_mode == LEGACY_MODE:
            block_idx, block_sharpe, block_pnl = evaluate_legacy_block(
                log_returns,
                fast_series,
                slow_matrix,
            )
            evaluated_pairs += int(slow_block.shape[0])
            if block_sharpe > best["sharpe"]:
                best = {
                    "fast": fast_period,
                    "slow": int(slow_block[int(block_idx)]),
                    "sharpe": float(block_sharpe),
                    "pnl": float(block_pnl),
                }
        else:
            block = evaluate_real_block(
                log_returns,
                fast_series,
                slow_matrix,
                slow_block,
                fast_period,
                long_only,
                allow_flip,
                trade_on_next_bar,
                args.fees,
                args.hurdle_tau,
            )
            block_idx = int(block[0])
            block_metrics = {
                "pnl": float(block[1]),
                "trades": int(round(float(block[2]))),
                "max_dd": float(block[3]),
                "mean": float(block[4]),
                "std": float(block[5]),
                "exposure": float(block[6]),
                "net_exposure": float(block[7]),
                "downside_deviation": float(block[8]),
                "ulcer_index": float(block[9]),
                "omega": float(block[10]),
            }
            block_sharpe = float(block[11])
            evaluated_pairs += int(slow_block.shape[0])
            if block_sharpe > best["sharpe"]:
                best = {
                    "fast": fast_period,
                    "slow": int(slow_block[block_idx]),
                    "sharpe": block_sharpe,
                    "pnl": block_metrics["pnl"],
                    "metrics": block_metrics,
                }

    runtime_ms = (time.perf_counter() - started) * 1000.0

    return {
        "status": "completed",
        "engine": "vectorbt_cpu_workflow",
        "mode": "cpu_only",
        "strategy": "double_alma_crossover",
        "objective": "sharpe",
        "benchmark_mode": args.benchmark_mode,
        "workflow_impl": (
            "vectorbt_environment_plus_local_search_metric_aggregation"
            if args.benchmark_mode == LEGACY_MODE
            else "vectorbt_environment_plus_local_10row_metric_aggregation"
        ),
        "exact_search": True,
        "metric_payload": build_metric_payload(args.benchmark_mode),
        "metric_semantics": build_metric_semantics(
            args.benchmark_mode,
            long_only,
            allow_flip,
            trade_on_next_bar,
            args.hurdle_tau,
        ),
        "dataset": {
            "kind": dataset_kind,
            "csv": csv_path,
            "close_column": args.close_column,
            "bars": int(close.shape[0]),
        },
        "fast_range": [fast_start, fast_end, fast_step],
        "slow_range": [slow_start, slow_end, slow_step],
        "pair_count": int(pair_count),
        "evaluated_pairs": int(evaluated_pairs),
        "pair_bars": int(pair_count) * int(close.shape[0]),
        "alma": {
            "offset": args.alma_offset,
            "sigma": args.alma_sigma,
        },
        "costs": {
            "explicit_fees_fraction": args.fees,
            "effective_commission": args.fees,
            "hurdle_tau": args.hurdle_tau,
        },
        "strategy_config": {
            "long_only": long_only,
            "allow_flip": allow_flip,
            "trade_on_next_bar": trade_on_next_bar,
        },
        "chunk_size": args.chunk_size,
        "runtime": {
            "grid_eval_ms": runtime_ms,
            "total_ms": runtime_ms,
        },
        "cache": cache.stats(),
        "best": best,
        "environment": {
            "python": sys.version.split()[0],
            "executable": sys.executable,
            "platform": platform.platform(),
            "vectorbt": getattr(vbt, "__version__", "unknown"),
            "numpy": np.__version__,
            "pandas": pd.__version__,
            "numba": getattr(nb, "__version__", "unavailable") if nb is not None else "unavailable",
        },
        "notes": [
            "VectorBT is imported for environment capture; timed scoring is local Python/Numba logic.",
            "The real-workflow lane computes the default 10-row VectorGrid metric payload per candidate.",
            "The max preset is supported through streaming fast-major blocks instead of pre-materializing the full grid.",
        ],
    }


def main() -> int:
    args = parse_args()
    result = run_faceoff(args)
    encoded = json.dumps(result, indent=2)
    output_json = args.output_json or args.out
    if output_json:
        Path(output_json).write_text(encoded, encoding="utf-8")
    print(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
