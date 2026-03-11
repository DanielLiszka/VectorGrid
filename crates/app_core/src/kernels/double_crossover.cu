


#ifndef _ALLOW_COMPILER_AND_STL_VERSION_MISMATCH
#define _ALLOW_COMPILER_AND_STL_VERSION_MISMATCH
#endif

#include <cuda_runtime.h>
#include <math.h>
#include <stdint.h>
#include <cub/block/block_radix_sort.cuh>



#ifndef NEG_INF_F
#define NEG_INF_F (-1.0f / 0.0f)
#endif

#ifndef STRAT_LONG_ONLY
#define STRAT_LONG_ONLY              (1u<<0)
#endif
#ifndef STRAT_NO_FLIP
#define STRAT_NO_FLIP                (1u<<1)
#endif
#ifndef STRAT_TRADE_ON_NEXT_BAR
#define STRAT_TRADE_ON_NEXT_BAR      (1u<<2)
#endif
#ifndef STRAT_SIGNED_EXPOSURE
#define STRAT_SIGNED_EXPOSURE        (1u<<4)
#endif




static __device__ __forceinline__ int sgn_eps(float f, float s, float eps) {
    if (eps <= 0.0f) {
        return (f > s) - (f < s);
    }
    return (f > s + eps) - (f < s - eps);
}

extern "C" __global__
void rocb_backtest_metrics_f32(
    const float* __restrict__ close,
    const float* __restrict__ lr,
    int T,
    const float* __restrict__ upper_row_major,
    const float* __restrict__ lower_row_major,
    int channel_rows,
    int channel_stride,
    const float* __restrict__ atr_row_major,
    int atr_rows,
    int atr_stride,
    const int* __restrict__ combo_channel_row_idx,
    const int* __restrict__ combo_atr_row_idx,
    const int* __restrict__ combo_start_t,
    const int* __restrict__ combo_end_t,
    int combo_count,
    unsigned int flags,
    float commission,
    float atr_stop_mult,
    int max_trade_bars,
    int out_stride,
    float* __restrict__ out_metrics
) {
    const int idx = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (idx >= combo_count) return;

    const int channel_row = combo_channel_row_idx[idx];
    const int atr_row = combo_atr_row_idx[idx];
    const int start_t = combo_start_t[idx];
    const int end_t = combo_end_t[idx];
    const int long_only = (flags & STRAT_LONG_ONLY) ? 1 : 0;
    const int no_flip = (flags & STRAT_NO_FLIP) ? 1 : 0;
    const int trade_on_next_bar = (flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0;

    const int base = idx * out_stride;
    if (base < 0) return;
    if (channel_row < 0 || channel_row >= channel_rows || atr_row < 0 || atr_row >= atr_rows || start_t < 0 || start_t >= T || end_t <= start_t || end_t > T) {
        if (out_stride >= 10) {
            out_metrics[base + 0] = 0.0f;
            out_metrics[base + 1] = 0.0f;
            out_metrics[base + 2] = 0.0f;
            out_metrics[base + 3] = 0.0f;
            out_metrics[base + 4] = 0.0f;
            out_metrics[base + 5] = 0.0f;
            out_metrics[base + 6] = 0.0f;
            out_metrics[base + 7] = 0.0f;
            out_metrics[base + 8] = 0.0f;
            out_metrics[base + 9] = 0.0f;
        }
        return;
    }

    const float* __restrict__ upper = upper_row_major + ((size_t)channel_row * (size_t)channel_stride);
    const float* __restrict__ lower = lower_row_major + ((size_t)channel_row * (size_t)channel_stride);
    const float* __restrict__ atr = atr_row_major + ((size_t)atr_row * (size_t)atr_stride);

    int pos_signal = 0;
    int bars_in_trade = 0;
    float entry_price = NAN;

    int pos_exec = 0;
    float log_eq = 0.0f;
    float log_peak = 0.0f;
    float max_log_dd = 0.0f;
    int trades = 0;

    int n_obs = 0;
    float mean = 0.0f;
    float m2 = 0.0f;
    float sum_neg_sq = 0.0f;
    float sum_pos_r = 0.0f;
    float sum_neg_abs_r = 0.0f;
    int sum_abs_pos = 0;
    int sum_pos = 0;

    const float c = fminf(fmaxf(commission, 0.0f), 0.95f);
    const float log_cost = (c > 0.0f) ? logf(fmaxf(1.0e-12f, 1.0f - c)) : 0.0f;
    const int max_trade_bars_clamped = max(max_trade_bars, 1);

    for (int t = start_t; t < end_t; ++t) {
        int sig_t = t - trade_on_next_bar;
        if (sig_t < 0) sig_t = 0;
        int prev_t = sig_t - 1;

        const float close_now = close[sig_t];
        const float atr_now = atr[sig_t];
        const float upper_prev = (prev_t >= 0) ? upper[prev_t] : NAN;
        const float lower_prev = (prev_t >= 0) ? lower[prev_t] : NAN;

        const int long_breakout = isfinite(close_now) && isfinite(upper_prev) && (close_now > upper_prev);
        const int short_breakout = isfinite(close_now) && isfinite(lower_prev) && (close_now < lower_prev);

        if (pos_signal == 0) {
            bars_in_trade = 0;
            entry_price = NAN;
            if (long_breakout) {
                pos_signal = 1;
                bars_in_trade = 1;
                entry_price = close_now;
            } else if (!long_only && short_breakout) {
                pos_signal = -1;
                bars_in_trade = 1;
                entry_price = close_now;
            }
        } else if (pos_signal > 0) {
            const float stop_dist = isfinite(atr_now) ? (atr_stop_mult * atr_now) : NAN;
            const int stop_hit = isfinite(entry_price)
                && isfinite(stop_dist)
                && isfinite(close_now)
                && (close_now <= entry_price - stop_dist);
            const int channel_exit = isfinite(close_now)
                && isfinite(upper_prev)
                && (close_now <= upper_prev);
            int exit_now = stop_hit || channel_exit || (bars_in_trade >= max_trade_bars_clamped);

            if (!long_only && short_breakout) {
                if (!no_flip) {
                    pos_signal = -1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                } else {
                    exit_now = 1;
                }
            } else if (exit_now) {
                pos_signal = 0;
                bars_in_trade = 0;
                entry_price = NAN;
            } else {
                bars_in_trade += 1;
            }
        } else {
            const float stop_dist = isfinite(atr_now) ? (atr_stop_mult * atr_now) : NAN;
            const int stop_hit = isfinite(entry_price)
                && isfinite(stop_dist)
                && isfinite(close_now)
                && (close_now >= entry_price + stop_dist);
            const int channel_exit = isfinite(close_now)
                && isfinite(lower_prev)
                && (close_now >= lower_prev);
            int exit_now = stop_hit || channel_exit || (bars_in_trade >= max_trade_bars_clamped);

            if (long_breakout) {
                if (!no_flip) {
                    pos_signal = 1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                } else {
                    exit_now = 1;
                }
            } else if (exit_now) {
                pos_signal = 0;
                bars_in_trade = 0;
                entry_price = NAN;
            } else {
                bars_in_trade += 1;
            }
        }

        const int desired_exec = pos_signal;
        const int pos_prev = pos_exec;
        int trade_delta = 0;
        if (desired_exec != pos_prev) {
            if (pos_prev == 0 || desired_exec == 0) trade_delta = 1;
            else trade_delta = no_flip ? 2 : 1;
            trades += trade_delta;
        }

        float step_r = 0.0f;
        if (t > 0 && t < T) {
            step_r += (float)pos_prev * lr[t];
        }
        if (trade_delta > 0) {
            step_r += log_cost * (float)trade_delta;
        }
        log_eq += step_r;
        log_peak = fmaxf(log_peak, log_eq);
        max_log_dd = fmaxf(max_log_dd, log_peak - log_eq);

        ++n_obs;
        const float delta = step_r - mean;
        mean += delta / (float)n_obs;
        const float delta2 = step_r - mean;
        m2 += delta * delta2;
        if (step_r < 0.0f) {
            sum_neg_sq += step_r * step_r;
            sum_neg_abs_r += -step_r;
        } else {
            sum_pos_r += step_r;
        }

        sum_abs_pos += abs(pos_prev);
        sum_pos += pos_prev;
        pos_exec = desired_exec;
    }

    const float variance = (n_obs > 1) ? (m2 / (float)(n_obs - 1)) : 0.0f;
    const float volatility = sqrtf(fmaxf(variance, 0.0f));
    const float sharpe = (volatility > 0.0f) ? (mean / volatility) : 0.0f;
    const float pnl = expf(log_eq) - 1.0f;
    const float max_dd = (max_log_dd > 0.0f) ? -expm1f(-max_log_dd) : 0.0f;

    const float clamp_max = 1.0e6f;
    const float eps_denom = 1.0e-12f;
    const float denom = (float)max(n_obs, 1);
    const float downside_dev = sqrtf(fmaxf(sum_neg_sq / denom, 0.0f));
    float sortino = 0.0f;
    if (downside_dev > 0.0f) {
        sortino = mean / downside_dev;
    } else if (mean > 0.0f) {
        sortino = clamp_max;
    } else if (mean < 0.0f) {
        sortino = -clamp_max;
    }
    sortino = fminf(clamp_max, fmaxf(-clamp_max, sortino));

    float omega = 0.0f;
    if (sum_neg_abs_r > eps_denom) {
        omega = sum_pos_r / sum_neg_abs_r;
        omega = fminf(clamp_max, fmaxf(0.0f, omega));
    } else if (sum_pos_r > 0.0f) {
        omega = clamp_max;
    }

    float calmar = 0.0f;
    if (max_dd > eps_denom) {
        calmar = pnl / max_dd;
    } else if (pnl > 0.0f) {
        calmar = clamp_max;
    } else if (pnl < 0.0f) {
        calmar = -clamp_max;
    }
    calmar = fminf(clamp_max, fmaxf(-clamp_max, calmar));

    const float exposure = (n_obs > 0) ? ((float)sum_abs_pos / (float)n_obs) : 0.0f;
    const float net_exposure = (n_obs > 0) ? ((float)sum_pos / (float)n_obs) : 0.0f;

    if (out_stride >= 10) {
        out_metrics[base + 0] = pnl;
        out_metrics[base + 1] = sharpe;
        out_metrics[base + 2] = sortino;
        out_metrics[base + 3] = calmar;
        out_metrics[base + 4] = omega;
        out_metrics[base + 5] = volatility;
        out_metrics[base + 6] = max_dd;
        out_metrics[base + 7] = (float)trades;
        out_metrics[base + 8] = exposure;
        out_metrics[base + 9] = (flags & STRAT_SIGNED_EXPOSURE) ? net_exposure : 0.0f;
    }
}

extern "C" __global__
void mkcs_backtest_metrics_f32(
    const float* __restrict__ close,
    const float* __restrict__ lr,
    int T,
    const float* __restrict__ stddev_row_major,
    int stddev_rows,
    int stddev_stride,
    const float* __restrict__ upper_row_major,
    const float* __restrict__ lower_row_major,
    int breakout_rows,
    int breakout_stride,
    const int* __restrict__ combo_stddev_row_idx,
    const int* __restrict__ combo_breakout_row_idx,
    const int* __restrict__ combo_start_t,
    const int* __restrict__ combo_end_t,
    int combo_count,
    unsigned int flags,
    float commission,
    float compression_threshold,
    float reexpansion_threshold,
    int out_stride,
    float* __restrict__ out_metrics
) {
    const int idx = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (idx >= combo_count) return;

    const int stddev_row = combo_stddev_row_idx[idx];
    const int breakout_row = combo_breakout_row_idx[idx];
    const int start_t = combo_start_t[idx];
    const int end_t = combo_end_t[idx];
    const int long_only = (flags & STRAT_LONG_ONLY) ? 1 : 0;
    const int no_flip = (flags & STRAT_NO_FLIP) ? 1 : 0;
    const int trade_on_next_bar = (flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0;

    const int base = idx * out_stride;
    if (base < 0) return;
    if (stddev_row < 0 || stddev_row >= stddev_rows
        || breakout_row < 0 || breakout_row >= breakout_rows
        || start_t < 0 || start_t >= T || end_t <= start_t || end_t > T) {
        if (out_stride >= 10) {
            out_metrics[base + 0] = 0.0f;
            out_metrics[base + 1] = 0.0f;
            out_metrics[base + 2] = 0.0f;
            out_metrics[base + 3] = 0.0f;
            out_metrics[base + 4] = 0.0f;
            out_metrics[base + 5] = 0.0f;
            out_metrics[base + 6] = 0.0f;
            out_metrics[base + 7] = 0.0f;
            out_metrics[base + 8] = 0.0f;
            out_metrics[base + 9] = 0.0f;
        }
        return;
    }

    const float* __restrict__ stddev = stddev_row_major + ((size_t)stddev_row * (size_t)stddev_stride);
    const float* __restrict__ upper = upper_row_major + ((size_t)breakout_row * (size_t)breakout_stride);
    const float* __restrict__ lower = lower_row_major + ((size_t)breakout_row * (size_t)breakout_stride);

    int pos_signal = 0;
    int pos_exec = 0;
    float log_eq = 0.0f;
    float log_peak = 0.0f;
    float max_log_dd = 0.0f;
    int trades = 0;

    int n_obs = 0;
    float mean = 0.0f;
    float m2 = 0.0f;
    float sum_neg_sq = 0.0f;
    float sum_pos_r = 0.0f;
    float sum_neg_abs_r = 0.0f;
    int sum_abs_pos = 0;
    int sum_pos = 0;

    const float c = fminf(fmaxf(commission, 0.0f), 0.95f);
    const float log_cost = (c > 0.0f) ? logf(fmaxf(1.0e-12f, 1.0f - c)) : 0.0f;

    for (int t = start_t; t < end_t; ++t) {
        int sig_t = t - trade_on_next_bar;
        if (sig_t < 0) sig_t = 0;
        int prev_t = sig_t - 1;

        const float close_now = close[sig_t];
        const float close_prev = (prev_t >= 0) ? close[prev_t] : NAN;
        const float stddev_now = stddev[sig_t];
        const float stddev_prev = (prev_t >= 0) ? stddev[prev_t] : NAN;
        const float upper_prev = (prev_t >= 0) ? upper[prev_t] : NAN;
        const float lower_prev = (prev_t >= 0) ? lower[prev_t] : NAN;

        const float denom_prev = isfinite(close_prev) ? fmaxf(fabsf(close_prev), 1.0e-12f) : NAN;
        const float denom_now = isfinite(close_now) ? fmaxf(fabsf(close_now), 1.0e-12f) : NAN;
        const float compression_prev = (isfinite(stddev_prev) && isfinite(denom_prev)) ? (fabsf(stddev_prev) / denom_prev) : NAN;
        const float compression_now = (isfinite(stddev_now) && isfinite(denom_now)) ? (fabsf(stddev_now) / denom_now) : NAN;

        const int compressed_prev = isfinite(compression_prev) && (compression_prev <= compression_threshold);
        const int reexpanded_now = isfinite(compression_now) && (compression_now >= reexpansion_threshold);

        const int long_breakout = compressed_prev
            && isfinite(close_now) && isfinite(upper_prev)
            && (close_now > upper_prev);
        const int short_breakout = compressed_prev
            && isfinite(close_now) && isfinite(lower_prev)
            && (close_now < lower_prev);

        if (pos_signal == 0) {
            if (long_breakout) {
                pos_signal = 1;
            } else if (!long_only && short_breakout) {
                pos_signal = -1;
            }
        } else if (pos_signal > 0) {
            int exit_now = reexpanded_now
                || (isfinite(close_now) && isfinite(upper_prev) && (close_now <= upper_prev));

            if (!long_only && short_breakout) {
                if (!no_flip) {
                    pos_signal = -1;
                } else {
                    exit_now = 1;
                }
            } else if (exit_now) {
                pos_signal = 0;
            }
        } else {
            int exit_now = reexpanded_now
                || (isfinite(close_now) && isfinite(lower_prev) && (close_now >= lower_prev));

            if (long_breakout) {
                if (!no_flip) {
                    pos_signal = 1;
                } else {
                    exit_now = 1;
                }
            } else if (exit_now) {
                pos_signal = 0;
            }
        }

        const int desired_exec = pos_signal;
        const int pos_prev = pos_exec;
        int trade_delta = 0;
        if (desired_exec != pos_prev) {
            if (pos_prev == 0 || desired_exec == 0) trade_delta = 1;
            else trade_delta = no_flip ? 2 : 1;
            if (desired_exec != 0) ++trades;
        }

        float step_r = 0.0f;
        if (t > 0 && t < T) {
            step_r += (float)pos_prev * lr[t];
        }
        if (trade_delta > 0) {
            step_r += log_cost * (float)trade_delta;
        }
        log_eq += step_r;
        log_peak = fmaxf(log_peak, log_eq);
        max_log_dd = fmaxf(max_log_dd, log_peak - log_eq);

        ++n_obs;
        const float delta = step_r - mean;
        mean += delta / (float)n_obs;
        const float delta2 = step_r - mean;
        m2 += delta * delta2;
        if (step_r < 0.0f) {
            sum_neg_sq += step_r * step_r;
            sum_neg_abs_r += -step_r;
        } else {
            sum_pos_r += step_r;
        }

        sum_abs_pos += abs(pos_prev);
        sum_pos += pos_prev;
        pos_exec = desired_exec;
    }

    const float variance = (n_obs > 1) ? (m2 / (float)(n_obs - 1)) : 0.0f;
    const float volatility = sqrtf(fmaxf(variance, 0.0f));
    const float sharpe = (volatility > 0.0f) ? (mean / volatility) : 0.0f;
    const float pnl = expf(log_eq) - 1.0f;
    const float max_dd = (max_log_dd > 0.0f) ? -expm1f(-max_log_dd) : 0.0f;

    const float clamp_max = 1.0e6f;
    const float eps_denom = 1.0e-12f;
    const float denom = (float)max(n_obs, 1);
    const float downside_dev = sqrtf(fmaxf(sum_neg_sq / denom, 0.0f));
    float sortino = 0.0f;
    if (downside_dev > 0.0f) {
        sortino = mean / downside_dev;
    } else if (mean > 0.0f) {
        sortino = clamp_max;
    } else if (mean < 0.0f) {
        sortino = -clamp_max;
    }
    sortino = fminf(clamp_max, fmaxf(-clamp_max, sortino));

    float omega = 0.0f;
    if (sum_neg_abs_r > eps_denom) {
        omega = sum_pos_r / sum_neg_abs_r;
        omega = fminf(clamp_max, fmaxf(0.0f, omega));
    } else if (sum_pos_r > 0.0f) {
        omega = clamp_max;
    }

    float calmar = 0.0f;
    if (max_dd > eps_denom) {
        calmar = pnl / max_dd;
    } else if (pnl > 0.0f) {
        calmar = clamp_max;
    } else if (pnl < 0.0f) {
        calmar = -clamp_max;
    }
    calmar = fminf(clamp_max, fmaxf(-clamp_max, calmar));

    const float exposure = (n_obs > 0) ? ((float)sum_abs_pos / (float)n_obs) : 0.0f;
    const float net_exposure = (n_obs > 0) ? ((float)sum_pos / (float)n_obs) : 0.0f;

    if (out_stride >= 10) {
        out_metrics[base + 0] = pnl;
        out_metrics[base + 1] = sharpe;
        out_metrics[base + 2] = sortino;
        out_metrics[base + 3] = calmar;
        out_metrics[base + 4] = omega;
        out_metrics[base + 5] = volatility;
        out_metrics[base + 6] = max_dd;
        out_metrics[base + 7] = (float)trades;
        out_metrics[base + 8] = exposure;
        out_metrics[base + 9] = (flags & STRAT_SIGNED_EXPOSURE) ? net_exposure : 0.0f;
    }
}

template <int EPS_MODE>
static __device__ __forceinline__ float compute_eps_f32(float s, float eps_rel) {
    if constexpr (EPS_MODE == 0) {
        return 0.0f;
    } else if constexpr (EPS_MODE == 1) {
        return eps_rel * fmaxf(1.0f, fabsf(s));
    } else {
        return (eps_rel > 0.0f) ? (eps_rel * fmaxf(1.0f, fabsf(s))) : 0.0f;
    }
}

static __device__ __forceinline__ int desired_sign_with_regime_f32(
    float f,
    float s,
    float eps,
    unsigned int flags,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    int t_sig,
    float regime_range_entry_rel,
    float regime_range_exit_rel)
{
    const int long_only = (flags & (1u << 0)) ? 1 : 0;
    if (!use_regime || regime_filter == nullptr) {
        int sign = sgn_eps(f, s, eps);
        if (long_only) sign = max(sign, 0);
        return sign;
    }

    const float rv = regime_filter[t_sig];
    if (!isfinite(rv)) return 0;

    if (use_regime_hybrid && rv >= 1.5f && rv < 2.5f) {
        if (!isfinite(f) || !isfinite(s)) return 0;
        const float denom = fmaxf(1.0f, fabsf(s));
        const float delta = f - s;
        const float entry = regime_range_entry_rel * denom;
        int range_sign = (delta <= -entry) - (delta >= entry);
        if (long_only && range_sign < 0) {
            range_sign = 0;
        }
        return range_sign;
    }

    int sign = sgn_eps(f, s, eps);
    if (long_only) sign = max(sign, 0);
    if (rv < 0.5f) return max(sign, 0);
    return min(sign, 0);
}

static __device__ __forceinline__ int desired_sign_plain_f32(
    float f,
    float s,
    float eps,
    unsigned int flags)
{
    const int long_only = (flags & (1u << 0)) ? 1 : 0;
    int sign = sgn_eps(f, s, eps);
    if (long_only) sign = max(sign, 0);
    return sign;
}

static __device__ __forceinline__ int desired_sign_regime_only_f32(
    float f,
    float s,
    float eps,
    unsigned int flags,
    const float* __restrict__ regime_filter,
    int t_sig)
{
    const int long_only = (flags & (1u << 0)) ? 1 : 0;
    int sign = sgn_eps(f, s, eps);
    if (long_only) sign = max(sign, 0);
    if (regime_filter == nullptr) return sign;

    const float rv = regime_filter[t_sig];
    if (!isfinite(rv)) return 0;
    if (rv < 0.5f) return max(sign, 0);
    return min(sign, 0);
}

static __device__ __forceinline__ int desired_sign_regime_hybrid_f32(
    float f,
    float s,
    float eps,
    unsigned int flags,
    const float* __restrict__ regime_filter,
    int t_sig,
    float regime_range_entry_rel,
    float regime_range_exit_rel)
{
    const int long_only = (flags & (1u << 0)) ? 1 : 0;
    if (regime_filter == nullptr) {
        int sign = sgn_eps(f, s, eps);
        if (long_only) sign = max(sign, 0);
        return sign;
    }

    const float rv = regime_filter[t_sig];
    if (!isfinite(rv)) return 0;

    if (rv >= 1.5f && rv < 2.5f) {
        if (!isfinite(f) || !isfinite(s)) return 0;
        const float denom = fmaxf(1.0f, fabsf(s));
        const float delta = f - s;
        const float entry = regime_range_entry_rel * denom;
        int range_sign = (delta <= -entry) - (delta >= entry);
        if (long_only && range_sign < 0) {
            range_sign = 0;
        }
        return range_sign;
    }

    int sign = sgn_eps(f, s, eps);
    if (long_only) sign = max(sign, 0);
    if (rv < 0.5f) return max(sign, 0);
    return min(sign, 0);
}

template <int REGIME_MODE>
static __device__ __forceinline__ int desired_sign_mode_f32(
    float f,
    float s,
    float eps,
    unsigned int flags,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    int t_sig,
    float regime_range_entry_rel,
    float regime_range_exit_rel)
{
    if (REGIME_MODE == 0) {
        return desired_sign_plain_f32(f, s, eps, flags);
    }
    if (REGIME_MODE == 1) {
        return desired_sign_regime_only_f32(f, s, eps, flags, regime_filter, t_sig);
    }
    if (REGIME_MODE == 2) {
        return desired_sign_regime_hybrid_f32(
            f,
            s,
            eps,
            flags,
            regime_filter,
            t_sig,
            regime_range_entry_rel,
            regime_range_exit_rel);
    }
    return desired_sign_with_regime_f32(
        f,
        s,
        eps,
        flags,
        regime_filter,
        use_regime,
        use_regime_hybrid,
        t_sig,
        regime_range_entry_rel,
        regime_range_exit_rel);
}

static __device__ __forceinline__ float ld_tm(const float* __restrict__ a, int dim, int t, int i) {
#if __CUDA_ARCH__ >= 350
    return __ldg(a + (size_t)t * (size_t)dim + (size_t)i);
#else
    return a[(size_t)t * (size_t)dim + (size_t)i];
#endif
}

static __device__ __forceinline__ float ld_tm_fast_warp_dedup(
    const float* __restrict__ a,
    int dim,
    int t,
    int pf_local)
{
#if __CUDA_ARCH__ >= 700
    const unsigned warp_mask = __activemask();
    const unsigned same_pf_mask = __match_any_sync(warp_mask, pf_local);
    const int lane = (int)(threadIdx.x & 31);
    const int leader = __ffs((int)same_pf_mask) - 1;
    float f = 0.0f;
    if (lane == leader) {
        f = ld_tm(a, dim, t, pf_local);
    }
    return __shfl_sync(same_pf_mask, f, leader);
#else
    return ld_tm(a, dim, t, pf_local);
#endif
}






extern "C" __global__
void transpose_row_to_tm(const float* __restrict__ in,
                         int rows, int cols,
                         int out_rows,
                         float* __restrict__ out)
{
    const int tid = blockIdx.x * blockDim.x + threadIdx.x;
    const int total = rows * cols;
    for (int k = tid; k < total; k += blockDim.x * gridDim.x) {
        const int r = k / cols;
        const int c = k % cols;
        out[(size_t)c * (size_t)out_rows + (size_t)r] =
            in[(size_t)r * (size_t)cols + (size_t)c];
    }
}





extern "C" __global__
void compute_log_returns_f32(const float* __restrict__ prices,
                             int T,
                             float* __restrict__ lr_out)
{
    for (int t = blockIdx.x * blockDim.x + threadIdx.x; t < T; t += blockDim.x * gridDim.x) {
        if (t == 0) { lr_out[0] = 0.0f; continue; }
        const float p  = prices[t];
        const float pm = prices[t-1];
        lr_out[t] = (p > 0.0f && pm > 0.0f && isfinite(p) && isfinite(pm)) ? logf(p) - logf(pm) : 0.0f;
    }
}

extern "C" __global__
void build_vfct_regime_f32(const float* __restrict__ obv,
                           const float* __restrict__ obv_smooth,
                           const float* __restrict__ mfi,
                           int T,
                           int first_valid,
                           float mfi_long_min,
                           float mfi_short_max,
                           int confirm_bars,
                           float* __restrict__ out)
{
    const int stride = blockDim.x * gridDim.x;
    for (int t = blockIdx.x * blockDim.x + threadIdx.x; t < T; t += stride) {
        if (t < first_valid || confirm_bars <= 0) {
            out[t] = nanf("");
            continue;
        }

        const int start = t - (confirm_bars - 1);
        if (start < first_valid) {
            out[t] = nanf("");
            continue;
        }

        int long_active = 1;
        int short_active = 1;
        for (int j = start; j <= t; ++j) {
            const float obv_v = obv[j];
            const float obv_s_v = obv_smooth[j];
            const float mfi_v = mfi[j];
            const int finite = isfinite(obv_v) && isfinite(obv_s_v) && isfinite(mfi_v);
            const int long_ok = finite && (obv_v > obv_s_v) && (mfi_v >= mfi_long_min);
            const int short_ok = finite && (obv_v < obv_s_v) && (mfi_v <= mfi_short_max);
            long_active &= long_ok;
            short_active &= short_ok;
            if (!long_active && !short_active) {
                break;
            }
        }

        out[t] = (long_active && !short_active) ? 0.0f
               : (short_active && !long_active) ? 1.0f
               : nanf("");
    }
}

extern "C" __global__
void build_pit_regime_f32(const float* __restrict__ efi,
                          const float* __restrict__ bop,
                          const float* __restrict__ vosc,
                          const float* __restrict__ acosc,
                          const float* __restrict__ cfo,
                          int T,
                          int first_valid,
                          int confirm_bars,
                          float* __restrict__ out)
{
    const int stride = blockDim.x * gridDim.x;
    for (int t = blockIdx.x * blockDim.x + threadIdx.x; t < T; t += stride) {
        if (t < first_valid || confirm_bars <= 0) {
            out[t] = nanf("");
            continue;
        }

        const int start = t - (confirm_bars - 1);
        if (start < first_valid) {
            out[t] = nanf("");
            continue;
        }

        int long_active = 1;
        int short_active = 1;
        for (int j = start; j <= t; ++j) {
            const float efi_v = efi[j];
            const float bop_v = bop[j];
            const float vosc_v = vosc[j];
            const float acosc_v = acosc[j];
            const float cfo_v = cfo[j];
            const int finite = isfinite(efi_v) && isfinite(bop_v) && isfinite(vosc_v)
                && isfinite(acosc_v) && isfinite(cfo_v);
            const int long_votes = (efi_v > 0.0f) + (bop_v > 0.0f) + (vosc_v > 0.0f)
                + (acosc_v > 0.0f) + (cfo_v > 0.0f);
            const int short_votes = (efi_v < 0.0f) + (bop_v < 0.0f) + (vosc_v < 0.0f)
                + (acosc_v < 0.0f) + (cfo_v < 0.0f);
            const int long_ok = finite && (long_votes >= 3);
            const int short_ok = finite && (short_votes >= 3);
            long_active &= long_ok;
            short_active &= short_ok;
            if (!long_active && !short_active) {
                break;
            }
        }

        out[t] = (long_active && !short_active) ? 0.0f
               : (short_active && !long_active) ? 1.0f
               : nanf("");
    }
}

extern "C" __global__
void build_set_regime_f32(const float* __restrict__ angle,
                          const float* __restrict__ er,
                          const float* __restrict__ chop,
                          int T,
                          int first_valid,
                          float angle_min_deg,
                          float er_min,
                          float chop_max,
                          float* __restrict__ out)
{
    const int stride = blockDim.x * gridDim.x;
    for (int t = blockIdx.x * blockDim.x + threadIdx.x; t < T; t += stride) {
        if (t < first_valid) {
            out[t] = nanf("");
            continue;
        }

        const float angle_v = angle[t];
        const float er_v = er[t];
        const float chop_v = chop[t];
        const int finite = isfinite(angle_v) && isfinite(er_v) && isfinite(chop_v);
        const int long_ok = finite && (angle_v >= angle_min_deg) && (er_v >= er_min) && (chop_v <= chop_max);
        const int short_ok = finite && (angle_v <= -angle_min_deg) && (er_v >= er_min) && (chop_v <= chop_max);

        out[t] = (long_ok && !short_ok) ? 0.0f
               : (short_ok && !long_ok) ? 1.0f
               : nanf("");
    }
}

extern "C" __global__
void build_mts_regime_f32(const float* __restrict__ roc,
                          const float* __restrict__ mom,
                          const float* __restrict__ ppo,
                          int T,
                          int first_valid,
                          float* __restrict__ out)
{
    const int stride = blockDim.x * gridDim.x;
    for (int t = blockIdx.x * blockDim.x + threadIdx.x; t < T; t += stride) {
        if (t < first_valid) {
            out[t] = nanf("");
            continue;
        }

        const float roc_v = roc[t];
        const float mom_v = mom[t];
        const float ppo_v = ppo[t];
        const int finite = isfinite(roc_v) && isfinite(mom_v) && isfinite(ppo_v);
        const int long_ok = finite && (roc_v > 0.0f) && (mom_v > 0.0f) && (ppo_v > 0.0f);
        const int short_ok = finite && (roc_v < 0.0f) && (mom_v < 0.0f) && (ppo_v < 0.0f);

        out[t] = (long_ok && !short_ok) ? 0.0f
               : (short_ok && !long_ok) ? 1.0f
               : nanf("");
    }
}

extern "C" __global__
void build_orc_regime_f32(const float* __restrict__ rsi,
                          const float* __restrict__ stoch_k,
                          const float* __restrict__ willr,
                          int T,
                          int first_valid,
                          float* __restrict__ out)
{
    const int stride = blockDim.x * gridDim.x;
    for (int t = blockIdx.x * blockDim.x + threadIdx.x; t < T; t += stride) {
        if (t < first_valid) {
            out[t] = nanf("");
            continue;
        }

        const float rsi_v = rsi[t];
        const float stoch_v = stoch_k[t];
        const float willr_v = willr[t];
        const int finite = isfinite(rsi_v) && isfinite(stoch_v) && isfinite(willr_v);
        const int oversold_votes = (rsi_v <= 30.0f) + (stoch_v <= 20.0f) + (willr_v <= -80.0f);
        const int overbought_votes = (rsi_v >= 70.0f) + (stoch_v >= 80.0f) + (willr_v >= -20.0f);
        const int long_ok = finite && (oversold_votes >= 2);
        const int short_ok = finite && (overbought_votes >= 2);

        out[t] = (long_ok && !short_ok) ? 0.0f
               : (short_ok && !long_ok) ? 1.0f
               : nanf("");
    }
}




#ifndef STRAT_LONG_ONLY
#define STRAT_LONG_ONLY              (1u<<0)
#endif
#ifndef STRAT_NO_FLIP
#define STRAT_NO_FLIP                (1u<<1)
#endif
#ifndef STRAT_TRADE_ON_NEXT_BAR
#define STRAT_TRADE_ON_NEXT_BAR      (1u<<2)
#endif
#define STRAT_ENFORCE_FAST_LT_SLOW   (1u<<3)
#ifndef STRAT_SIGNED_EXPOSURE
#define STRAT_SIGNED_EXPOSURE        (1u<<4)
#endif
#define STRAT_STOP_LOSS              (1u<<5)
#define STRAT_TAKE_PROFIT            (1u<<6)





template <bool FAST_WARP_DEDUP, bool LOCKSTEP_TIME, bool FULL_M7, int REGIME_MODE = -1, int RISK_MODE = -1, int EPS_MODE = -1>
static __device__ __forceinline__ void double_cross_backtest_tm_flex_f32_body(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,

    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,

    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,

    int regime_first_valid,
    int eval_start,
    int eval_end,

    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    const int metrics_stride = (metrics_out_stride > 0) ? metrics_out_stride : (FULL_M7 ? 7 : M);
    const float log_comm = (commission > 0.0f) ? log1pf(-commission) : 0.0f;
    const int need_trades = FULL_M7 ? 1 : (M > 1);
    const int need_max_dd = FULL_M7 ? 1 : (M > 2);
    const int need_stats  = FULL_M7 ? 1 : (M > 3);
    const int need_std    = FULL_M7 ? 1 : (M > 4);
    const int need_expo   = FULL_M7 ? 1 : (M > 5);
    const int use_stop    = (flags & STRAT_STOP_LOSS) != 0;
    const int use_take    = (flags & STRAT_TAKE_PROFIT) != 0;
    const int use_risk_runtime = use_stop || use_take;
    const int use_risk =
        (RISK_MODE == 0) ? 0 : ((RISK_MODE == 1) ? 1 : use_risk_runtime);
    const int fast_stride = (fast_tm_stride > 0) ? fast_tm_stride : Pf_tile;
    const int slow_stride = (slow_tm_stride > 0) ? slow_tm_stride : Ps_tile;

    const int total_pairs = (pair_count > 0) ? pair_count : (Pf_tile * Ps_tile);
    const int dense_pairs = Pf_tile * Ps_tile;
    for (int pair_local = blockIdx.x * blockDim.x + threadIdx.x;
         pair_local < total_pairs;
         pair_local += blockDim.x * gridDim.x)
    {
        const int dense_pair =
            (pair_index_map != nullptr) ? pair_index_map[pair_local] : pair_local;
        if (dense_pair < 0 || dense_pair >= dense_pairs) {
            continue;
        }

        const int pf_local  = dense_pair / Ps_tile;
        const int ps_local  = dense_pair % Ps_tile;
        const int pf_global = f_offset + pf_local;
        const int ps_global = s_offset + ps_local;

        const int period_f = fast_periods[pf_global];
        const int period_s = slow_periods[ps_global];

        const size_t pair_global = (size_t)pf_global * (size_t)Ps_total + (size_t)ps_global;
        const size_t base = pair_global * (size_t)metrics_stride;

        if ((flags & STRAT_ENFORCE_FAST_LT_SLOW) && (period_f >= period_s)) {
            if (M > 0) metrics_out[base + 0] = 0.0f;
            if (M > 1) metrics_out[base + 1] = 0.0f;
            if (M > 2) metrics_out[base + 2] = 0.0f;
            if (M > 3) metrics_out[base + 3] = 0.0f;
            if (M > 4) metrics_out[base + 4] = 0.0f;
            if (M > 5) metrics_out[base + 5] = 0.0f;
            if (M > 6) metrics_out[base + 6] = 0.0f;
            continue;
        }

        const int pair_first_valid = first_valid + max(period_f, period_s) - 1;
        const int t_valid = max(max(pair_first_valid, regime_first_valid), eval_start);
        const int trade_offset = ((flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0);
        const int start_t = max(t_valid + trade_offset, eval_start);
        const int end_t = min(T, eval_end);

        if (start_t >= end_t) {
            if (M > 0) metrics_out[base + 0] = 0.0f;
            if (M > 1) metrics_out[base + 1] = 0.0f;
            if (M > 2) metrics_out[base + 2] = 0.0f;
            if (M > 3) metrics_out[base + 3] = 0.0f;
            if (M > 4) metrics_out[base + 4] = 0.0f;
            if (M > 5) metrics_out[base + 5] = 0.0f;
            if (M > 6) metrics_out[base + 6] = 0.0f;
            continue;
        }

        int   pos         = 0;
        float log_eq      = 0.0f;
        float log_peak    = 0.0f;
        float max_log_dd  = 0.0f;
        int   trades      = 0;
        long long sum_abs_pos = 0;
        long long sum_pos     = 0;
        float mean = 0.0f, m2 = 0.0f;
        long long n = 0;

        if (!use_risk) {
            const int loop_start = LOCKSTEP_TIME ? first_valid : start_t;
            for (int t = loop_start; t < end_t; ++t) {
                if constexpr (LOCKSTEP_TIME) {
                    if (t < start_t) {
                        continue;
                    }
                }
                const int t_sig = t - ((flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0);

                const float f = FAST_WARP_DEDUP
                    ? ld_tm_fast_warp_dedup(fast_ma_T, fast_stride, t_sig, pf_local)
                    : ld_tm(fast_ma_T, fast_stride, t_sig, pf_local);
                const float s = ld_tm(slow_ma_T, slow_stride, t_sig, ps_local);
                const float eps = compute_eps_f32<EPS_MODE>(s, eps_rel);
                int sign = desired_sign_mode_f32<REGIME_MODE>(
                    f,
                    s,
                    eps,
                    flags,
                    regime_filter,
                    use_regime,
                    use_regime_hybrid,
                    t_sig,
                    regime_range_entry_rel,
                    regime_range_exit_rel);

                if (sign != pos) {
                    if (pos == 0 || sign == 0) {
                        if (commission > 0.0f) log_eq += log_comm;
                        if (need_trades) trades += 1;
                        pos = (sign == 0) ? 0 : sign;
                    } else {
                        if (flags & STRAT_NO_FLIP) {
                            if (commission > 0.0f) log_eq += log_comm;
                            if (need_trades) trades += 1;
                            pos = 0;
                        } else {
                            if (commission > 0.0f) log_eq += 2.0f * log_comm;
                            if (need_trades) trades += 2;
                            pos = sign;
                        }
                    }
                }

                const float lr_t = lr[t];
                log_eq += (float)pos * lr_t;

                if (need_max_dd) {
                    if (log_eq > log_peak) log_peak = log_eq;
                    const float dd_log = log_peak - log_eq;
                    if (dd_log > max_log_dd) max_log_dd = dd_log;
                }

                if (need_stats) {
                    const float step_r = (pos == 0) ? 0.0f : (float)pos * expm1f(lr_t);
                    n += 1;
                    const float delta = step_r - mean;
                    mean += delta / (float)n;
                    m2   += delta * (step_r - mean);

                    if (need_expo) {
                        sum_abs_pos += llabs((long long)pos);
                        sum_pos     += (long long)pos;
                    }
                }
            }
        } else {
            float trade_log_ret = 0.0f;

            const int loop_start = LOCKSTEP_TIME ? first_valid : start_t;
            for (int t = loop_start; t < end_t; ++t) {
                if constexpr (LOCKSTEP_TIME) {
                    if (t < start_t) {
                        continue;
                    }
                }
                const int t_sig = t - ((flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0);

                const float f = FAST_WARP_DEDUP
                    ? ld_tm_fast_warp_dedup(fast_ma_T, fast_stride, t_sig, pf_local)
                    : ld_tm(fast_ma_T, fast_stride, t_sig, pf_local);
                const float s = ld_tm(slow_ma_T, slow_stride, t_sig, ps_local);
                const float eps = compute_eps_f32<EPS_MODE>(s, eps_rel);
                int sign = desired_sign_mode_f32<REGIME_MODE>(
                    f,
                    s,
                    eps,
                    flags,
                    regime_filter,
                    use_regime,
                    use_regime_hybrid,
                    t_sig,
                    regime_range_entry_rel,
                    regime_range_exit_rel);

                if (sign != pos) {
                    if (pos == 0 || sign == 0) {
                        if (commission > 0.0f) {
                            log_eq += log_comm;
                            if (pos != 0) trade_log_ret += log_comm;
                        }
                        if (need_trades) trades += 1;
                        pos = (sign == 0) ? 0 : sign;
                        trade_log_ret = (pos != 0 && commission > 0.0f) ? log_comm : 0.0f;
                    } else {
                        if (flags & STRAT_NO_FLIP) {
                            if (commission > 0.0f) {
                                log_eq += log_comm;
                                trade_log_ret += log_comm;
                            }
                            if (need_trades) trades += 1;
                            pos = 0;
                            trade_log_ret = 0.0f;
                        } else {
                            if (commission > 0.0f) {
                                log_eq += 2.0f * log_comm;
                                trade_log_ret += log_comm;
                            }
                            if (need_trades) trades += 2;
                            pos = sign;
                            trade_log_ret = (commission > 0.0f) ? log_comm : 0.0f;
                        }
                    }
                }

                const int pos_bar = pos;
                const float lr_t = lr[t];
                const float step_log = (float)pos_bar * lr_t;
                log_eq += step_log;

                if (pos_bar != 0) {
                    trade_log_ret += step_log;
                    const bool hit_stop = use_stop && (trade_log_ret <= stop_log);
                    const bool hit_take = use_take && (trade_log_ret >= take_log);
                    if (hit_stop || hit_take) {
                        if (commission > 0.0f) log_eq += log_comm;
                        if (need_trades) trades += 1;
                        pos = 0;
                        trade_log_ret = 0.0f;
                    }
                }

                if (need_max_dd) {
                    if (log_eq > log_peak) log_peak = log_eq;
                    const float dd_log = log_peak - log_eq;
                    if (dd_log > max_log_dd) max_log_dd = dd_log;
                }

                if (need_stats) {
                    const float step_r = (pos_bar == 0) ? 0.0f : (float)pos_bar * expm1f(lr_t);
                    n += 1;
                    const float delta = step_r - mean;
                    mean += delta / (float)n;
                    m2 += delta * (step_r - mean);

                    if (need_expo) {
                        sum_abs_pos += llabs((long long)pos_bar);
                        sum_pos += (long long)pos_bar;
                    }
                }
            }
        }

        const float variance = (need_std && n > 1) ? (m2 / (float)(n - 1)) : 0.0f;
        const float max_dd = (need_max_dd && max_log_dd > 0.0f) ? -expm1f(-max_log_dd) : 0.0f;
        const float exposure = (need_expo && n > 0) ? (float)((double)sum_abs_pos / (double)n) : 0.0f;
        const float net_expo = (need_expo && n > 0) ? (float)((double)sum_pos / (double)n)     : 0.0f;
        const float pnl = expf(log_eq) - 1.0f;
        const float std = sqrtf(variance);

        if constexpr (FULL_M7) {
            metrics_out[base + 0] = pnl;
            metrics_out[base + 1] = (float)trades;
            metrics_out[base + 2] = max_dd;
            metrics_out[base + 3] = mean;
            metrics_out[base + 4] = std;
            metrics_out[base + 5] = exposure;
            metrics_out[base + 6] = (flags & STRAT_SIGNED_EXPOSURE) ? net_expo : 0.0f;
        } else {
            if (M > 0) metrics_out[base + 0] = pnl;
            if (M > 1) metrics_out[base + 1] = (float)trades;
            if (M > 2) metrics_out[base + 2] = max_dd;
            if (M > 3) metrics_out[base + 3] = mean;
            if (M > 4) metrics_out[base + 4] = std;
            if (M > 5) metrics_out[base + 5] = exposure;
            if (M > 6) metrics_out[base + 6] = (flags & STRAT_SIGNED_EXPOSURE) ? net_expo : 0.0f;
        }
    }
}

template <bool FAST_WARP_DEDUP, bool LOCKSTEP_TIME, bool FULL_M7, int REGIME_MODE = -1, int RISK_MODE = -1, int EPS_MODE = -1>
static __device__ __forceinline__ void double_cross_backtest_tm_flex_f32_body(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,

    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,

    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,

    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<FAST_WARP_DEDUP, LOCKSTEP_TIME, FULL_M7, REGIME_MODE, RISK_MODE, EPS_MODE>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        0, T,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, false, false>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_windowed(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    int eval_start,
    int eval_end,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, false, false>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        eval_start, eval_end,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_fast_warp_dedup(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<true, false, false>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, false>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_m7(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, false, true>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 0>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 0, 0>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 0, 0, 0>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}


extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 1>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on_norisk(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 1, 0>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 2>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 2, 0>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}

extern "C" __global__
void double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ metrics_out
){
    double_cross_backtest_tm_flex_f32_body<false, true, true, 2, 0, 0>(
        fast_ma_T, slow_ma_T, lr,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, first_valid, regime_first_valid,
        commission, eps_rel, stop_log, take_log, flags, M, metrics_out_stride, pair_index_map, pair_count, metrics_out
    );
}




static __device__ __forceinline__ float objective_score_f32(const float* __restrict__ metrics, size_t base, int M, int objective) {
    // objective: 0 = pnl, 1 = sharpe, 2 = -max_dd, 3 = calmar, 4 = -volatility
    if (objective == 0) {
        return (M > 0) ? metrics[base + 0] : NEG_INF_F;
    }
    if (objective == 1) {
        if (M <= 4) return NEG_INF_F;
        const float mean = metrics[base + 3];
        const float std  = metrics[base + 4];
        return (std > 0.0f) ? (mean / std) : 0.0f;
    }
    if (objective == 2) {
        return (M > 2) ? -metrics[base + 2] : NEG_INF_F;
    }
    if (objective == 3) {
        if (M <= 2) return NEG_INF_F;
        const float pnl = metrics[base + 0];
        const float max_dd = metrics[base + 2];
        const float eps = 1e-12f;
        const float clamp_max = 1.0e6f;
        float raw = 0.0f;
        if (max_dd > eps) {
            raw = pnl / max_dd;
        } else if (pnl > 0.0f) {
            raw = clamp_max;
        } else if (pnl < 0.0f) {
            raw = -clamp_max;
        }
        if (raw > clamp_max) raw = clamp_max;
        if (raw < -clamp_max) raw = -clamp_max;
        return raw;
    }
    if (objective == 4) {
        if (M <= 4) return NEG_INF_F;
        const float std = metrics[base + 4];
        return -std;
    }
    return NEG_INF_F;
}

static __device__ __forceinline__ float objective_score_vemr_f32(
    const float* __restrict__ metrics,
    size_t base,
    int M,
    int objective
) {
    // objective: 0 = pnl, 1 = sharpe, 2 = sortino, 3 = calmar, 4 = omega,
    // 5 = -volatility, 6 = -max_dd
    if (objective == 0) {
        return (M > 0) ? metrics[base + 0] : NEG_INF_F;
    }
    if (objective == 1) {
        return (M > 1) ? metrics[base + 1] : NEG_INF_F;
    }
    if (objective == 2) {
        return (M > 2) ? metrics[base + 2] : NEG_INF_F;
    }
    if (objective == 3) {
        return (M > 3) ? metrics[base + 3] : NEG_INF_F;
    }
    if (objective == 4) {
        return (M > 4) ? metrics[base + 4] : NEG_INF_F;
    }
    if (objective == 5) {
        return (M > 5) ? -metrics[base + 5] : NEG_INF_F;
    }
    if (objective == 6) {
        return (M > 6) ? -metrics[base + 6] : NEG_INF_F;
    }
    return NEG_INF_F;
}

static __device__ __forceinline__ bool passes_constraints_f32(
    const float* __restrict__ metrics,
    size_t base,
    int M,
    int use_min_trades,
    float min_trades,
    int use_max_dd,
    float max_dd
) {
    if (use_min_trades && M > 1) {
        const float trades = metrics[base + 1];
        if (!(trades >= min_trades)) return false;
    }
    if (use_max_dd && M > 2) {
        const float dd = metrics[base + 2];
        if (!(dd <= max_dd)) return false;
    }
    return true;
}

static __device__ __forceinline__ void atomicMaxFloat(float* addr, float value) {
    if (!isfinite(value)) return;
    int* addr_i = (int*)addr;
    int old = *addr_i;
    while (true) {
        const float old_f = __int_as_float(old);
        if (!(value > old_f)) break;
        const int assumed = old;
        old = atomicCAS(addr_i, assumed, __float_as_int(value));
        if (old == assumed) break;
    }
}

static __device__ __forceinline__ int bin_period_i32(int v, int v_min, int v_max, int bins) {
    if (bins <= 1) return 0;
    const int denom = v_max - v_min;
    if (denom <= 0) return 0;
    const int num = v - v_min;
    long long b = ((long long)num * (long long)(bins - 1)) / (long long)denom;
    if (b < 0) b = 0;
    if (b >= bins) b = bins - 1;
    return (int)b;
}

extern "C" __global__
void fill_f32(float* __restrict__ out, int N, float value) {
    for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < N; i += blockDim.x * gridDim.x) {
        out[i] = value;
    }
}

extern "C" __global__
void heatmap_update_scores_tm_f32(
    const float* __restrict__ metrics,
    int pairs,
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int bins_fast,
    int bins_slow,
    int fast_min,
    int fast_max,
    int slow_min,
    int slow_max,
    float* __restrict__ heatmap_out
) {
    const int stride = blockDim.x * gridDim.x;
    for (int pair = blockIdx.x * blockDim.x + threadIdx.x; pair < pairs; pair += stride) {
        const int pf_local = pair / Ps_tile;
        const int ps_local = pair - pf_local * Ps_tile;
        if (pf_local < 0 || pf_local >= Pf_tile || ps_local < 0 || ps_local >= Ps_tile) continue;

        const int f = fast_periods[pf_local];
        const int s = slow_periods[ps_local];
        if (f >= s) continue;

        const size_t base = (size_t)pair * (size_t)metrics_stride;
        const float score = objective_score_f32(metrics, base, M, objective);
        if (!isfinite(score)) continue;

        const int fb = bin_period_i32(f, fast_min, fast_max, bins_fast);
        const int sb = bin_period_i32(s, slow_min, slow_max, bins_slow);
        const int idx = fb * bins_slow + sb;
        atomicMaxFloat(&heatmap_out[idx], score);
    }
}

#define TOPK_BLOCK_THREADS 256
#define TOPK_ITEMS_PER_THREAD 2
#define TOPK_BLOCK_ITEMS (TOPK_BLOCK_THREADS * TOPK_ITEMS_PER_THREAD)

extern "C" __global__
void select_topk_from_pairs_tm_f32(
    const float* __restrict__ metrics,
    int pairs,
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int topk,
    int* __restrict__ out_pair_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int idx = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        float score = NEG_INF_F;
        int val = -1;
        if (idx < pairs) {
            const int pf_local = idx / Ps_tile;
            const int ps_local = idx - pf_local * Ps_tile;
            if (pf_local >= 0 && pf_local < Pf_tile && ps_local >= 0 && ps_local < Ps_tile) {
                const int f = fast_periods[pf_local];
                const int s = slow_periods[ps_local];
                if (f < s) {
                    const size_t base = (size_t)idx * (size_t)metrics_stride;
                    score = objective_score_f32(metrics, base, M, objective);
                    if (!isfinite(score)) score = NEG_INF_F;
                }
            }
            val = idx;
        }
        keys[item] = score;
        vals[item] = val;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_pair_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_from_candidates_tm_f32(
    const float* __restrict__ metrics,
    int pairs,
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int topk,
    const int* __restrict__ in_pair_idx,
    int in_len,
    int* __restrict__ out_pair_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int in_pos = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        int pair = -1;
        float score = NEG_INF_F;
        if (in_pos < in_len) {
            pair = in_pair_idx[in_pos];
            if (pair >= 0 && pair < pairs) {
                const int pf_local = pair / Ps_tile;
                const int ps_local = pair - pf_local * Ps_tile;
                if (pf_local >= 0 && pf_local < Pf_tile && ps_local >= 0 && ps_local < Ps_tile) {
                    const int f = fast_periods[pf_local];
                    const int s = slow_periods[ps_local];
                    if (f < s) {
                        const size_t base = (size_t)pair * (size_t)metrics_stride;
                        score = objective_score_f32(metrics, base, M, objective);
                        if (!isfinite(score)) score = NEG_INF_F;
                    }
                }
            }
        }
        keys[item] = score;
        vals[item] = pair;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_pair_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_from_pairs_tm_f32_v2(
    const float* __restrict__ metrics,
    int pairs,
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int topk,
    int use_min_trades,
    float min_trades,
    int use_max_dd,
    float max_dd,
    int* __restrict__ out_pair_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int idx = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        float score = NEG_INF_F;
        int val = -1;
        if (idx < pairs) {
            const int pf_local = idx / Ps_tile;
            const int ps_local = idx - pf_local * Ps_tile;
            if (pf_local >= 0 && pf_local < Pf_tile && ps_local >= 0 && ps_local < Ps_tile) {
                const int f = fast_periods[pf_local];
                const int s = slow_periods[ps_local];
                if (f < s) {
                    const size_t base = (size_t)idx * (size_t)metrics_stride;
                    if (passes_constraints_f32(metrics, base, M, use_min_trades, min_trades, use_max_dd, max_dd)) {
                        score = objective_score_f32(metrics, base, M, objective);
                        if (!isfinite(score)) score = NEG_INF_F;
                    }
                }
            }
            val = idx;
        }
        keys[item] = score;
        vals[item] = val;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_pair_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_from_candidates_tm_f32_v2(
    const float* __restrict__ metrics,
    int pairs,
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int topk,
    const int* __restrict__ in_pair_idx,
    int in_len,
    int use_min_trades,
    float min_trades,
    int use_max_dd,
    float max_dd,
    int* __restrict__ out_pair_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int in_pos = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        int pair = -1;
        float score = NEG_INF_F;
        if (in_pos < in_len) {
            pair = in_pair_idx[in_pos];
            if (pair >= 0 && pair < pairs) {
                const int pf_local = pair / Ps_tile;
                const int ps_local = pair - pf_local * Ps_tile;
                if (pf_local >= 0 && pf_local < Pf_tile && ps_local >= 0 && ps_local < Ps_tile) {
                    const int f = fast_periods[pf_local];
                    const int s = slow_periods[ps_local];
                    if (f < s) {
                        const size_t base = (size_t)pair * (size_t)metrics_stride;
                        if (passes_constraints_f32(metrics, base, M, use_min_trades, min_trades, use_max_dd, max_dd)) {
                            score = objective_score_f32(metrics, base, M, objective);
                            if (!isfinite(score)) score = NEG_INF_F;
                        }
                    }
                }
            }
        }
        keys[item] = score;
        vals[item] = pair;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_pair_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void gather_topk_pairs_tm_f32(
    const float* __restrict__ metrics,
    int pairs,
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const int* __restrict__ top_pair_idx,
    int topk,
    int* __restrict__ out_fast_len,
    int* __restrict__ out_slow_len,
    float* __restrict__ out_metrics
) {
    for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < topk; i += blockDim.x * gridDim.x) {
        const int pair = top_pair_idx[i];
        if (pair < 0 || pair >= pairs) {
            out_fast_len[i] = 0;
            out_slow_len[i] = 0;
            for (int m = 0; m < M; ++m) out_metrics[(size_t)i * (size_t)M + (size_t)m] = NAN;
            continue;
        }
        const int pf_local = pair / Ps_tile;
        const int ps_local = pair - pf_local * Ps_tile;
        if (pf_local < 0 || pf_local >= Pf_tile || ps_local < 0 || ps_local >= Ps_tile) {
            out_fast_len[i] = 0;
            out_slow_len[i] = 0;
            for (int m = 0; m < M; ++m) out_metrics[(size_t)i * (size_t)M + (size_t)m] = NAN;
            continue;
        }
        out_fast_len[i] = fast_periods[pf_local];
        out_slow_len[i] = slow_periods[ps_local];
        const size_t base_in = (size_t)pair * (size_t)metrics_stride;
        const size_t base_out = (size_t)i * (size_t)M;
        for (int m = 0; m < M; ++m) out_metrics[base_out + (size_t)m] = metrics[base_in + (size_t)m];
    }
}

extern "C" __global__
void select_topk_metric_rows_f32(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int objective,
    int topk,
    int* __restrict__ out_row_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int idx = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        float score = NEG_INF_F;
        int val = -1;
        if (idx < rows) {
            const size_t base = (size_t)idx * (size_t)M;
            score = objective_score_f32(metrics_rows, base, M, objective);
            if (!isfinite(score)) score = NEG_INF_F;
            val = idx;
        }
        keys[item] = score;
        vals[item] = val;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_row_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_metric_rows_from_candidates_f32(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int objective,
    int topk,
    const int* __restrict__ in_row_idx,
    int in_len,
    int* __restrict__ out_row_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int in_pos = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        int row = -1;
        float score = NEG_INF_F;
        if (in_pos < in_len) {
            row = in_row_idx[in_pos];
            if (row >= 0 && row < rows) {
                const size_t base = (size_t)row * (size_t)M;
                score = objective_score_f32(metrics_rows, base, M, objective);
                if (!isfinite(score)) score = NEG_INF_F;
            }
        }
        keys[item] = score;
        vals[item] = row;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_row_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_metric_rows_f32_v2(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int objective,
    int topk,
    int use_min_trades,
    float min_trades,
    int use_max_dd,
    float max_dd,
    int* __restrict__ out_row_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int idx = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        float score = NEG_INF_F;
        int val = -1;
        if (idx < rows) {
            const size_t base = (size_t)idx * (size_t)M;
            if (passes_constraints_f32(metrics_rows, base, M, use_min_trades, min_trades, use_max_dd, max_dd)) {
                score = objective_score_f32(metrics_rows, base, M, objective);
                if (!isfinite(score)) score = NEG_INF_F;
            }
            val = idx;
        }
        keys[item] = score;
        vals[item] = val;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_row_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_metric_rows_from_candidates_f32_v2(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int objective,
    int topk,
    const int* __restrict__ in_row_idx,
    int in_len,
    int use_min_trades,
    float min_trades,
    int use_max_dd,
    float max_dd,
    int* __restrict__ out_row_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int in_pos = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        int row = -1;
        float score = NEG_INF_F;
        if (in_pos < in_len) {
            row = in_row_idx[in_pos];
            if (row >= 0 && row < rows) {
                const size_t base = (size_t)row * (size_t)M;
                if (passes_constraints_f32(metrics_rows, base, M, use_min_trades, min_trades, use_max_dd, max_dd)) {
                    score = objective_score_f32(metrics_rows, base, M, objective);
                    if (!isfinite(score)) score = NEG_INF_F;
                }
            }
        }
        keys[item] = score;
        vals[item] = row;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_row_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_vemr_metric_rows_f32(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int objective,
    int topk,
    int* __restrict__ out_row_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int idx = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        float score = NEG_INF_F;
        int val = -1;
        if (idx < rows) {
            const size_t base = (size_t)idx * (size_t)M;
            score = objective_score_vemr_f32(metrics_rows, base, M, objective);
            if (!isfinite(score)) score = NEG_INF_F;
            val = idx;
        }
        keys[item] = score;
        vals[item] = val;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_row_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void select_topk_vemr_metric_rows_from_candidates_f32(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int objective,
    int topk,
    const int* __restrict__ in_row_idx,
    int in_len,
    int* __restrict__ out_row_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int in_pos = block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        int row = -1;
        float score = NEG_INF_F;
        if (in_pos < in_len) {
            row = in_row_idx[in_pos];
            if (row >= 0 && row < rows) {
                const size_t base = (size_t)row * (size_t)M;
                score = objective_score_vemr_f32(metrics_rows, base, M, objective);
                if (!isfinite(score)) score = NEG_INF_F;
            }
        }
        keys[item] = score;
        vals[item] = row;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_row_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}

extern "C" __global__
void gather_topk_metric_rows_f32(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    const int* __restrict__ fast_rows,
    const int* __restrict__ slow_rows,
    const int* __restrict__ top_row_idx,
    int topk,
    int* __restrict__ out_fast_len,
    int* __restrict__ out_slow_len,
    float* __restrict__ out_metrics
) {
    for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < topk; i += blockDim.x * gridDim.x) {
        const int row = top_row_idx[i];
        if (row < 0 || row >= rows) {
            out_fast_len[i] = 0;
            out_slow_len[i] = 0;
            for (int m = 0; m < M; ++m) out_metrics[(size_t)i * (size_t)M + (size_t)m] = NAN;
            continue;
        }
        out_fast_len[i] = fast_rows[row];
        out_slow_len[i] = slow_rows[row];
        const size_t base_in = (size_t)row * (size_t)M;
        const size_t base_out = (size_t)i * (size_t)M;
        for (int m = 0; m < M; ++m) out_metrics[base_out + (size_t)m] = metrics_rows[base_in + (size_t)m];
    }
}

static __device__ __forceinline__ uint64_t splitmix64_step(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}

static __device__ __forceinline__ float u64_unit_f32(uint64_t x) {
    const double denom = 9007199254740992.0; // 2^53
    return (float)((double)((x >> 11) & 0x1fffffffffffffULL) / denom);
}

static __device__ __forceinline__ int nearest_period_i32(
    const int* __restrict__ periods,
    int len,
    int value
) {
    if (len <= 0) return 0;
    if (value <= periods[0]) return periods[0];
    if (value >= periods[len - 1]) return periods[len - 1];

    int lo = 0;
    int hi = len;
    while (lo < hi) {
        const int mid = lo + ((hi - lo) >> 1);
        if (periods[mid] < value) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    const int idx = lo;
    if (idx <= 0) return periods[0];
    if (idx >= len) return periods[len - 1];
    const int p_lo = periods[idx - 1];
    const int p_hi = periods[idx];
    const int d_lo = (value >= p_lo) ? (value - p_lo) : (p_lo - value);
    const int d_hi = (value >= p_hi) ? (value - p_hi) : (p_hi - value);
    return (d_lo <= d_hi) ? p_lo : p_hi;
}

extern "C" __global__
void vdps_generate_seed_pairs_tm_i32(
    const int* __restrict__ elite_fast,
    const int* __restrict__ elite_slow,
    int elite_len,
    const int* __restrict__ fast_periods,
    int fast_len,
    const int* __restrict__ slow_periods,
    int slow_len,
    int elite_count,
    int diff_count,
    int random_count,
    int fast_step,
    int slow_step,
    unsigned long long seed,
    int out_len,
    int* __restrict__ out_fast,
    int* __restrict__ out_slow
) {
    const int stride = blockDim.x * gridDim.x;
    const int diff_start = elite_count;
    const int rand_start = elite_count + diff_count;
    const int f_step = max(fast_step, 1);
    const int s_step = max(slow_step, 1);
    const int f_delta = f_step * 2;
    const int s_delta = s_step * 2;

    for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < out_len; i += stride) {
        if (fast_len <= 0 || slow_len <= 0 || (i >= (elite_count + diff_count + random_count))) {
            out_fast[i] = 0;
            out_slow[i] = 0;
            continue;
        }

        uint64_t h0 = splitmix64_step((uint64_t)seed ^ ((uint64_t)(i + 1) * 0x9e3779b97f4a7c15ULL));
        uint64_t h1 = splitmix64_step(h0 ^ 0x243f6a8885a308d3ULL);
        uint64_t h2 = splitmix64_step(h1 ^ 0x13198a2e03707344ULL);
        uint64_t h3 = splitmix64_step(h2 ^ 0xa4093822299f31d0ULL);
        uint64_t h4 = splitmix64_step(h3 ^ 0x082efa98ec4e6c89ULL);

        int cand_f = 0;
        int cand_s = 0;

        const bool can_diff = elite_len >= 3;
        const bool use_elite = (i < diff_start) && elite_len > 0;
        const bool use_diff = (i >= diff_start && i < rand_start) && can_diff;

        if (use_elite) {
            const int e_idx = (int)(h0 % (uint64_t)elite_len);
            const int base_f = elite_fast[e_idx];
            const int base_s = elite_slow[e_idx];
            const int df = (int)(h1 % (uint64_t)(2 * f_delta + 1)) - f_delta;
            const int ds = (int)(h2 % (uint64_t)(2 * s_delta + 1)) - s_delta;
            cand_f = base_f + df;
            cand_s = base_s + ds;
        } else if (use_diff) {
            int ia = (int)(h0 % (uint64_t)elite_len);
            int ib = (int)(h1 % (uint64_t)elite_len);
            int ic = (int)(h2 % (uint64_t)elite_len);

            if (ib == ia) ib = (ib + 1) % elite_len;
            if (ic == ia || ic == ib) ic = (ic + 1) % elite_len;
            if (ic == ia || ic == ib) ic = (ic + 1) % elite_len;

            const float f_coeff = 0.55f + 0.35f * u64_unit_f32(h3);
            const float s_coeff = 0.55f + 0.35f * u64_unit_f32(h4);

            const float a_f = (float)elite_fast[ia];
            const float b_f = (float)elite_fast[ib];
            const float c_f = (float)elite_fast[ic];
            const float a_s = (float)elite_slow[ia];
            const float b_s = (float)elite_slow[ib];
            const float c_s = (float)elite_slow[ic];

            cand_f = (int)nearbyintf(a_f + (b_f - c_f) * f_coeff);
            cand_s = (int)nearbyintf(a_s + (b_s - c_s) * s_coeff);
        } else {
            const int f_idx = (int)(h0 % (uint64_t)fast_len);
            const int s_idx = (int)(h1 % (uint64_t)slow_len);
            cand_f = fast_periods[f_idx];
            cand_s = slow_periods[s_idx];
        }

        const int f = nearest_period_i32(fast_periods, fast_len, cand_f);
        const int s = nearest_period_i32(slow_periods, slow_len, cand_s);
        if (f <= 0 || s <= 0 || f >= s) {
            out_fast[i] = 0;
            out_slow[i] = 0;
        } else {
            out_fast[i] = f;
            out_slow[i] = s;
        }
    }
}

extern "C" __global__
void pack_rows_u64_ptr_f32(
    const unsigned long long* __restrict__ row_ptrs,
    int rows,
    int cols,
    float* __restrict__ out_row_major
) {
    if (rows <= 0 || cols <= 0) return;
    const int total = rows * cols;
    const int stride = blockDim.x * gridDim.x;
    for (int idx = blockIdx.x * blockDim.x + threadIdx.x; idx < total; idx += stride) {
        const int row = idx / cols;
        const int col = idx - row * cols;
        const uintptr_t raw = (uintptr_t)row_ptrs[row];
        const float* __restrict__ src = reinterpret_cast<const float*>(raw);
        out_row_major[idx] = src ? src[col] : NAN;
    }
}

extern "C" __global__
void asr_backtest_metrics_f32(
    const float* __restrict__ close,
    const float* __restrict__ lr,
    int T,
    const float* __restrict__ fast_squeeze_row_major,
    const float* __restrict__ fast_momentum_row_major,
    int fast_rows,
    int fast_stride,
    const float* __restrict__ slow_angle_row_major,
    const float* __restrict__ slow_er_row_major,
    const float* __restrict__ slow_z_row_major,
    int slow_rows,
    int slow_stride,
    const float* __restrict__ kaufman_long,
    const float* __restrict__ kaufman_short,
    const int* __restrict__ combo_fast_row_idx,
    const int* __restrict__ combo_slow_row_idx,
    const int* __restrict__ combo_start_t,
    const int* __restrict__ combo_end_t,
    int combo_count,
    unsigned int flags,
    float commission,
    float angle_deg_min,
    float angle_deg_mr,
    int use_er_filter,
    float er_min,
    float er_mr,
    float z_entry,
    float z_exit,
    int confirm_bars,
    int mom_fade_bars,
    int max_trade_bars,
    int max_mr_trade_bars,
    int out_stride,
    float* __restrict__ out_metrics
) {
    const int idx = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (idx >= combo_count) return;

    const int fast_row = combo_fast_row_idx[idx];
    const int slow_row = combo_slow_row_idx[idx];
    const int start_t = combo_start_t[idx];
    const int end_t = combo_end_t[idx];
    const int long_only = (flags & STRAT_LONG_ONLY) ? 1 : 0;
    const int no_flip = (flags & STRAT_NO_FLIP) ? 1 : 0;
    const int trade_on_next_bar = (flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0;

    const int base = idx * out_stride;
    if (base < 0) return;
    if (out_stride >= 10) {
        out_metrics[base + 0] = 0.0f; // pnl
        out_metrics[base + 1] = 0.0f; // sharpe
        out_metrics[base + 2] = 0.0f; // sortino
        out_metrics[base + 3] = 0.0f; // calmar
        out_metrics[base + 4] = 0.0f; // omega
        out_metrics[base + 5] = 0.0f; // volatility
        out_metrics[base + 6] = 0.0f; // max_dd
        out_metrics[base + 7] = 0.0f; // trades
        out_metrics[base + 8] = 0.0f; // exposure
        out_metrics[base + 9] = 0.0f; // net_exposure
    }
    if (out_stride >= 12) {
        out_metrics[base + 10] = 0.0f; // slice_step_return_sum
        out_metrics[base + 11] = 0.0f; // slice_step_return_sumsq
    }
    if (out_stride >= 12) {
        out_metrics[base + 10] = 0.0f; // step_return_sum
        out_metrics[base + 11] = 0.0f; // step_return_sumsq
    }
    if (out_stride >= 12) {
        out_metrics[base + 10] = 0.0f; // step_return_sum
        out_metrics[base + 11] = 0.0f; // step_return_sumsq
    }
    if (out_stride >= 12) {
        out_metrics[base + 10] = 0.0f; // step_return_sum
        out_metrics[base + 11] = 0.0f; // step_return_sumsq
    }
    if (fast_row < 0 || fast_row >= fast_rows || slow_row < 0 || slow_row >= slow_rows) return;
    if (start_t < 0 || start_t >= T || end_t <= start_t || end_t > T) return;

    const float* __restrict__ squeeze = fast_squeeze_row_major + ((size_t)fast_row * (size_t)fast_stride);
    const float* __restrict__ momentum = fast_momentum_row_major + ((size_t)fast_row * (size_t)fast_stride);
    const float* __restrict__ angle = slow_angle_row_major + ((size_t)slow_row * (size_t)slow_stride);
    const float* __restrict__ er = slow_er_row_major + ((size_t)slow_row * (size_t)slow_stride);
    const float* __restrict__ zscore = slow_z_row_major + ((size_t)slow_row * (size_t)slow_stride);

    int pos_signal = 0;
    int entry_mode = 0; // 0=None, 1=Trend, 2=MeanReversion
    int bars_in_trade = 0;
    int mom_fade_count = 0;
    int release_confirm_left = 0;

    int pos_exec = 0;
    float log_eq = 0.0f;
    float log_peak = 0.0f;
    float max_log_dd = 0.0f;
    int trades = 0;

    int n_obs = 0;
    float mean = 0.0f;
    float m2 = 0.0f;
    float sum_step_r = 0.0f;
    float sumsq_step_r = 0.0f;
    float sum_neg_sq = 0.0f;
    float sum_pos_r = 0.0f;
    float sum_neg_abs_r = 0.0f;
    int sum_abs_pos = 0;
    int sum_pos = 0;

    const float c = fminf(fmaxf(commission, 0.0f), 0.95f);
    const float log_cost = (c > 0.0f) ? logf(fmaxf(1.0e-12f, 1.0f - c)) : 0.0f;
    const int confirm_bars_clamped = max(confirm_bars, 0);
    const int mom_fade_bars_clamped = max(mom_fade_bars, 1);
    const int max_trade_bars_clamped = max(max_trade_bars, 1);
    const int max_mr_trade_bars_clamped = max(max_mr_trade_bars, 1);

    for (int t = start_t; t < end_t; ++t) {
        const int sig_t = max(0, t - trade_on_next_bar);
        const int prev_t = max(0, sig_t - 1);

        const float squeeze_now = squeeze[sig_t];
        const float squeeze_prev = squeeze[prev_t];
        const float mom_now = momentum[sig_t];
        const float mom_prev = momentum[prev_t];
        const float mom_delta = (isfinite(mom_now) && isfinite(mom_prev)) ? (mom_now - mom_prev) : NAN;
        const float ang_now = angle[sig_t];
        const float er_now = er[sig_t];
        const float z_now = zscore[sig_t];

        const int squeeze_on_now = isfinite(squeeze_now) && squeeze_now > 0.5f;
        const int squeeze_on_prev = isfinite(squeeze_prev) && squeeze_prev > 0.5f;
        const int squeeze_release = squeeze_on_prev && !squeeze_on_now;

        if (squeeze_release) {
            release_confirm_left = confirm_bars_clamped;
        } else if (release_confirm_left > 0 && !squeeze_on_now) {
            release_confirm_left -= 1;
        }
        const int release_confirmed = squeeze_release || (confirm_bars_clamped > 0 && release_confirm_left == 0 && !squeeze_on_now);

        const int trend_ok_long = isfinite(ang_now)
            && (ang_now >= angle_deg_min)
            && (!use_er_filter || (isfinite(er_now) && er_now >= er_min));
        const int trend_ok_short = isfinite(ang_now)
            && (ang_now <= -angle_deg_min)
            && (!use_er_filter || (isfinite(er_now) && er_now >= er_min));
        const int trend_weak = isfinite(ang_now)
            && (fabsf(ang_now) <= angle_deg_mr)
            && (!use_er_filter || (isfinite(er_now) && er_now <= er_mr));

        if (pos_signal == 0) {
            bars_in_trade = 0;
            mom_fade_count = 0;

            int next_pos = 0;
            int next_mode = 0;

            if (release_confirmed) {
                if (trend_ok_long && isfinite(mom_now) && mom_now > 0.0f && isfinite(mom_delta) && mom_delta > 0.0f) {
                    next_pos = 1;
                    next_mode = 1;
                } else if (!long_only && trend_ok_short && isfinite(mom_now) && mom_now < 0.0f && isfinite(mom_delta) && mom_delta < 0.0f) {
                    next_pos = -1;
                    next_mode = 1;
                }
            }

            if (next_pos == 0 && trend_weak && isfinite(z_now)) {
                if (z_now <= -z_entry) {
                    next_pos = 1;
                    next_mode = 2;
                } else if (!long_only && z_now >= z_entry) {
                    next_pos = -1;
                    next_mode = 2;
                }
            }

            if (long_only && next_pos < 0) {
                next_pos = 0;
                next_mode = 0;
            }

            pos_signal = next_pos;
            entry_mode = next_mode;
            if (pos_signal != 0) {
                bars_in_trade = 1;
            }
        } else {
            int exit_now = 0;
            const float close_now = close[t];
            if (pos_signal > 0) {
                const float stop = kaufman_long[sig_t];
                if (isfinite(close_now) && isfinite(stop) && close_now <= stop) {
                    exit_now = 1;
                }
            } else {
                const float stop = kaufman_short[sig_t];
                if (isfinite(close_now) && isfinite(stop) && close_now >= stop) {
                    exit_now = 1;
                }
            }

            if (!exit_now) {
                if (entry_mode == 1) {
                    int fading = 0;
                    if (pos_signal > 0) {
                        fading = isfinite(mom_delta) && (mom_delta < 0.0f);
                    } else {
                        fading = isfinite(mom_delta) && (mom_delta > 0.0f);
                    }
                    if (fading) {
                        mom_fade_count += 1;
                    } else {
                        mom_fade_count = 0;
                    }
                    if (mom_fade_count >= mom_fade_bars_clamped) {
                        exit_now = 1;
                    }
                    if (bars_in_trade >= max_trade_bars_clamped) {
                        exit_now = 1;
                    }
                } else if (entry_mode == 2) {
                    if (isfinite(z_now) && fabsf(z_now) <= z_exit) {
                        exit_now = 1;
                    }
                    if (bars_in_trade >= max_mr_trade_bars_clamped) {
                        exit_now = 1;
                    }
                }
            }

            if (exit_now) {
                pos_signal = 0;
                entry_mode = 0;
                bars_in_trade = 0;
                mom_fade_count = 0;
            } else {
                bars_in_trade += 1;
            }
        }

        int desired = pos_signal;
        if (long_only && desired < 0) {
            desired = 0;
        }
        const float slice_step_r = (desired == 0) ? 0.0f : ((float)desired) * expm1f(lr[t]);
        sum_step_r += slice_step_r;
        sumsq_step_r += slice_step_r * slice_step_r;

        if (desired != pos_exec) {
            if (pos_exec == 0 || desired == 0) {
                if (log_cost != 0.0f) log_eq += log_cost;
                trades += 1;
                pos_exec = desired;
            } else if (no_flip) {
                if (log_cost != 0.0f) log_eq += log_cost;
                trades += 1;
                pos_exec = 0;
            } else {
                if (log_cost != 0.0f) log_eq += 2.0f * log_cost;
                trades += 2;
                pos_exec = desired;
            }
        }

        const float step_log = (pos_exec == 0) ? 0.0f : ((float)pos_exec) * lr[t];
        log_eq += step_log;

        if (log_eq > log_peak) {
            log_peak = log_eq;
        }
        const float dd_log = log_peak - log_eq;
        if (dd_log > max_log_dd) {
            max_log_dd = dd_log;
        }

        const float step_r = expf(step_log) - 1.0f;
        n_obs += 1;
        const float delta = step_r - mean;
        mean += delta / (float)n_obs;
        const float delta2 = step_r - mean;
        m2 += delta * delta2;

        if (step_r < 0.0f) {
            sum_neg_sq += step_r * step_r;
            sum_neg_abs_r += -step_r;
        } else {
            sum_pos_r += step_r;
        }

        sum_abs_pos += abs(pos_exec);
        sum_pos += pos_exec;
    }

    const float variance = (n_obs > 1) ? (m2 / (float)(n_obs - 1)) : 0.0f;
    const float volatility = sqrtf(fmaxf(variance, 0.0f));
    const float sharpe = (volatility > 0.0f) ? (mean / volatility) : 0.0f;
    const float pnl = expf(log_eq) - 1.0f;
    const float max_dd = (max_log_dd > 0.0f) ? -expm1f(-max_log_dd) : 0.0f;
    const float obs_f = (float)n_obs;
    const float step_return_sum = (n_obs > 0) ? (mean * obs_f) : 0.0f;
    const float step_return_sumsq = (n_obs > 0) ? (m2 + mean * mean * obs_f) : 0.0f;

    const float clamp_max = 1.0e6f;
    const float eps_denom = 1.0e-12f;
    const float denom = (float)max(n_obs, 1);
    const float downside_dev = sqrtf(fmaxf(sum_neg_sq / denom, 0.0f));
    float sortino = 0.0f;
    if (downside_dev > 0.0f) {
        sortino = mean / downside_dev;
    } else if (mean > 0.0f) {
        sortino = clamp_max;
    } else if (mean < 0.0f) {
        sortino = -clamp_max;
    }
    sortino = fminf(clamp_max, fmaxf(-clamp_max, sortino));

    float omega = 0.0f;
    if (sum_neg_abs_r > eps_denom) {
        omega = sum_pos_r / sum_neg_abs_r;
        omega = fminf(clamp_max, fmaxf(0.0f, omega));
    } else if (sum_pos_r > 0.0f) {
        omega = clamp_max;
    }

    float calmar = 0.0f;
    if (max_dd > eps_denom) {
        calmar = pnl / max_dd;
    } else if (pnl > 0.0f) {
        calmar = clamp_max;
    } else if (pnl < 0.0f) {
        calmar = -clamp_max;
    }
    calmar = fminf(clamp_max, fmaxf(-clamp_max, calmar));

    const float exposure = (n_obs > 0) ? ((float)sum_abs_pos / (float)n_obs) : 0.0f;
    const float net_exposure = (n_obs > 0) ? ((float)sum_pos / (float)n_obs) : 0.0f;

    if (out_stride >= 10) {
        out_metrics[base + 0] = pnl;
        out_metrics[base + 1] = sharpe;
        out_metrics[base + 2] = sortino;
        out_metrics[base + 3] = calmar;
        out_metrics[base + 4] = omega;
        out_metrics[base + 5] = volatility;
        out_metrics[base + 6] = max_dd;
        out_metrics[base + 7] = (float)trades;
        out_metrics[base + 8] = exposure;
        out_metrics[base + 9] = (flags & STRAT_SIGNED_EXPOSURE) ? net_exposure : 0.0f;
    }
    if (out_stride >= 12) {
        out_metrics[base + 10] = step_return_sum;
        out_metrics[base + 11] = step_return_sumsq;
    }
}

extern "C" __global__
void vcc_backtest_metrics_f32(
    const float* __restrict__ close,
    const float* __restrict__ lr,
    const double* __restrict__ lr64,
    int T,
    const float* __restrict__ fast_squeeze_row_major,
    const float* __restrict__ fast_momentum_row_major,
    int fast_rows,
    int fast_stride,
    const float* __restrict__ trend_row_major,
    int trend_rows,
    int trend_stride,
    const int* __restrict__ combo_fast_row_idx,
    const int* __restrict__ combo_trend_row_idx,
    const int* __restrict__ combo_start_t,
    const int* __restrict__ combo_metric_start_t,
    const int* __restrict__ combo_end_t,
    int combo_count,
    unsigned int flags,
    float commission,
    int require_momentum_accel,
    int confirm_bars,
    int max_trade_bars,
    int out_stride,
    float* __restrict__ out_metrics,
    double* __restrict__ out_slice_stats
) {
    const int idx = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (idx >= combo_count) return;

    const int fast_row = combo_fast_row_idx[idx];
    const int trend_row = combo_trend_row_idx[idx];
    const int start_t = combo_start_t[idx];
    const int metric_start_t = combo_metric_start_t[idx];
    const int end_t = combo_end_t[idx];
    const int long_only = (flags & STRAT_LONG_ONLY) ? 1 : 0;
    const int no_flip = (flags & STRAT_NO_FLIP) ? 1 : 0;
    const int trade_on_next_bar = (flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0;

    const int base = idx * out_stride;
    const int slice_base = idx * 2;
    if (base < 0) return;
    if (out_stride >= 10) {
        out_metrics[base + 0] = 0.0f; // pnl
        out_metrics[base + 1] = 0.0f; // sharpe
        out_metrics[base + 2] = 0.0f; // sortino
        out_metrics[base + 3] = 0.0f; // calmar
        out_metrics[base + 4] = 0.0f; // omega
        out_metrics[base + 5] = 0.0f; // volatility
        out_metrics[base + 6] = 0.0f; // max_dd
        out_metrics[base + 7] = 0.0f; // trades
        out_metrics[base + 8] = 0.0f; // exposure
        out_metrics[base + 9] = 0.0f; // net_exposure
    }
    if (out_slice_stats != nullptr) {
        out_slice_stats[slice_base + 0] = 0.0;
        out_slice_stats[slice_base + 1] = 0.0;
    }
    if (fast_row < 0 || fast_row >= fast_rows || trend_row < 0 || trend_row >= trend_rows) return;
    if (start_t < 0 || start_t >= T || end_t <= start_t || end_t > T) return;
    if (metric_start_t < start_t || metric_start_t > end_t) return;

    const float* __restrict__ squeeze = fast_squeeze_row_major + ((size_t)fast_row * (size_t)fast_stride);
    const float* __restrict__ momentum = fast_momentum_row_major + ((size_t)fast_row * (size_t)fast_stride);
    const float* __restrict__ trend = trend_row_major + ((size_t)trend_row * (size_t)trend_stride);

    int pos_signal = 0;
    int bars_in_trade = 0;
    int release_confirm_left = 0;

    int pos_exec = 0;
    float log_eq = 0.0f;
    float log_peak = 0.0f;
    float max_log_dd = 0.0f;
    int trades = 0;

    int n_obs = 0;
    float mean = 0.0f;
    float m2 = 0.0f;
    double sum_step_r = 0.0;
    double sumsq_step_r = 0.0;
    float sum_neg_sq = 0.0f;
    float sum_pos_r = 0.0f;
    float sum_neg_abs_r = 0.0f;
    int sum_abs_pos = 0;
    int sum_pos = 0;

    const float c = fminf(fmaxf(commission, 0.0f), 0.95f);
    const float log_cost = (c > 0.0f) ? logf(fmaxf(1.0e-12f, 1.0f - c)) : 0.0f;
    const int confirm_bars_clamped = max(confirm_bars, 0);
    const int max_trade_bars_clamped = max(max_trade_bars, 1);

    for (int t = start_t; t < metric_start_t; ++t) {
        const int sig_t = max(0, t - trade_on_next_bar);
        const int prev_t = max(0, sig_t - 1);

        const float squeeze_now = squeeze[sig_t];
        const float squeeze_prev = squeeze[prev_t];
        const float mom_now = momentum[sig_t];
        const float mom_prev = momentum[prev_t];
        const float mom_delta = (isfinite(mom_now) && isfinite(mom_prev)) ? (mom_now - mom_prev) : NAN;
        const float trend_now = trend[sig_t];

        const int squeeze_on_now = isfinite(squeeze_now) && squeeze_now > 0.5f;
        const int squeeze_on_prev = isfinite(squeeze_prev) && squeeze_prev > 0.5f;
        const int squeeze_release = squeeze_on_prev && !squeeze_on_now;

        if (squeeze_release) {
            release_confirm_left = confirm_bars_clamped;
        } else if (release_confirm_left > 0) {
            release_confirm_left -= 1;
        }
        const int release_confirmed = squeeze_release || (release_confirm_left == 0);

        const int bullish_trend = isfinite(trend_now) && trend_now > 0.5f;
        const int bearish_trend = isfinite(trend_now) && trend_now <= 0.5f;

        if (pos_signal == 0) {
            bars_in_trade = 0;
            if (release_confirmed) {
                int long_mom_ok = 0;
                int short_mom_ok = 0;
                if (require_momentum_accel) {
                    long_mom_ok = isfinite(mom_now) && mom_now > 0.0f && isfinite(mom_delta) && mom_delta > 0.0f;
                    short_mom_ok = isfinite(mom_now) && mom_now < 0.0f && isfinite(mom_delta) && mom_delta < 0.0f;
                } else {
                    long_mom_ok = isfinite(mom_now) && mom_now > 0.0f;
                    short_mom_ok = isfinite(mom_now) && mom_now < 0.0f;
                }

                if (bullish_trend && long_mom_ok) {
                    pos_signal = 1;
                    bars_in_trade = 1;
                } else if (!long_only && bearish_trend && short_mom_ok) {
                    pos_signal = -1;
                    bars_in_trade = 1;
                }
            }
        } else {
            int exit_now = 0;
            if (pos_signal > 0) {
                if (!isfinite(mom_now) || mom_now <= 0.0f || !bullish_trend) {
                    exit_now = 1;
                }
            } else {
                if (!isfinite(mom_now) || mom_now >= 0.0f || !bearish_trend) {
                    exit_now = 1;
                }
            }

            if (bars_in_trade >= max_trade_bars_clamped) {
                exit_now = 1;
            }

            if (exit_now) {
                pos_signal = 0;
                bars_in_trade = 0;
            } else {
                bars_in_trade += 1;
            }
        }
    }

    pos_exec = 0;
    log_eq = 0.0f;
    log_peak = 0.0f;
    max_log_dd = 0.0f;
    trades = 0;
    n_obs = 0;
    mean = 0.0f;
    m2 = 0.0f;
    sum_step_r = 0.0f;
    sumsq_step_r = 0.0f;
    sum_neg_sq = 0.0f;
    sum_pos_r = 0.0f;
    sum_neg_abs_r = 0.0f;
    sum_abs_pos = 0;
    sum_pos = 0;

    for (int t = metric_start_t; t < end_t; ++t) {
        const int sig_t = max(0, t - trade_on_next_bar);
        const int prev_t = max(0, sig_t - 1);

        const float squeeze_now = squeeze[sig_t];
        const float squeeze_prev = squeeze[prev_t];
        const float mom_now = momentum[sig_t];
        const float mom_prev = momentum[prev_t];
        const float mom_delta = (isfinite(mom_now) && isfinite(mom_prev)) ? (mom_now - mom_prev) : NAN;
        const float trend_now = trend[sig_t];

        const int squeeze_on_now = isfinite(squeeze_now) && squeeze_now > 0.5f;
        const int squeeze_on_prev = isfinite(squeeze_prev) && squeeze_prev > 0.5f;
        const int squeeze_release = squeeze_on_prev && !squeeze_on_now;

        if (squeeze_release) {
            release_confirm_left = confirm_bars_clamped;
        } else if (release_confirm_left > 0) {
            release_confirm_left -= 1;
        }
        const int release_confirmed = squeeze_release || (release_confirm_left == 0);

        const int bullish_trend = isfinite(trend_now) && trend_now > 0.5f;
        const int bearish_trend = isfinite(trend_now) && trend_now <= 0.5f;

        if (pos_signal == 0) {
            bars_in_trade = 0;
            if (release_confirmed) {
                int long_mom_ok = 0;
                int short_mom_ok = 0;
                if (require_momentum_accel) {
                    long_mom_ok = isfinite(mom_now) && mom_now > 0.0f && isfinite(mom_delta) && mom_delta > 0.0f;
                    short_mom_ok = isfinite(mom_now) && mom_now < 0.0f && isfinite(mom_delta) && mom_delta < 0.0f;
                } else {
                    long_mom_ok = isfinite(mom_now) && mom_now > 0.0f;
                    short_mom_ok = isfinite(mom_now) && mom_now < 0.0f;
                }

                if (bullish_trend && long_mom_ok) {
                    pos_signal = 1;
                    bars_in_trade = 1;
                } else if (!long_only && bearish_trend && short_mom_ok) {
                    pos_signal = -1;
                    bars_in_trade = 1;
                }
            }
        } else {
            int exit_now = 0;
            if (pos_signal > 0) {
                if (!isfinite(mom_now) || mom_now <= 0.0f || !bullish_trend) {
                    exit_now = 1;
                }
            } else {
                if (!isfinite(mom_now) || mom_now >= 0.0f || !bearish_trend) {
                    exit_now = 1;
                }
            }

            if (bars_in_trade >= max_trade_bars_clamped) {
                exit_now = 1;
            }

            if (exit_now) {
                pos_signal = 0;
                bars_in_trade = 0;
            } else {
                bars_in_trade += 1;
            }
        }

        int desired = pos_signal;
        if (long_only && desired < 0) {
            desired = 0;
        }
        const double slice_step_r = (desired == 0) ? 0.0 : ((double)desired) * expm1(lr64[t]);
        sum_step_r += slice_step_r;
        sumsq_step_r += slice_step_r * slice_step_r;

        if (desired != pos_exec) {
            if (pos_exec == 0 || desired == 0) {
                if (log_cost != 0.0f) log_eq += log_cost;
                trades += 1;
                pos_exec = desired;
            } else if (no_flip) {
                if (log_cost != 0.0f) log_eq += log_cost;
                trades += 1;
                pos_exec = 0;
            } else {
                if (log_cost != 0.0f) log_eq += 2.0f * log_cost;
                trades += 2;
                pos_exec = desired;
            }
        }

        const float step_log = (pos_exec == 0) ? 0.0f : ((float)pos_exec) * lr[t];
        log_eq += step_log;

        if (log_eq > log_peak) {
            log_peak = log_eq;
        }
        const float dd_log = log_peak - log_eq;
        if (dd_log > max_log_dd) {
            max_log_dd = dd_log;
        }

        const float step_r = expf(step_log) - 1.0f;
        n_obs += 1;
        const float delta = step_r - mean;
        mean += delta / (float)n_obs;
        const float delta2 = step_r - mean;
        m2 += delta * delta2;

        if (step_r < 0.0f) {
            sum_neg_sq += step_r * step_r;
            sum_neg_abs_r += -step_r;
        } else {
            sum_pos_r += step_r;
        }

        sum_abs_pos += abs(pos_exec);
        sum_pos += pos_exec;
    }

    const float variance = (n_obs > 1) ? (m2 / (float)(n_obs - 1)) : 0.0f;
    const float volatility = sqrtf(fmaxf(variance, 0.0f));
    const float sharpe = (volatility > 0.0f) ? (mean / volatility) : 0.0f;
    const float pnl = expf(log_eq) - 1.0f;
    const float max_dd = (max_log_dd > 0.0f) ? -expm1f(-max_log_dd) : 0.0f;

    const float clamp_max = 1.0e6f;
    const float eps_denom = 1.0e-12f;
    const float denom = (float)max(n_obs, 1);
    const float downside_dev = sqrtf(fmaxf(sum_neg_sq / denom, 0.0f));
    float sortino = 0.0f;
    if (downside_dev > 0.0f) {
        sortino = mean / downside_dev;
    } else if (mean > 0.0f) {
        sortino = clamp_max;
    } else if (mean < 0.0f) {
        sortino = -clamp_max;
    }
    sortino = fminf(clamp_max, fmaxf(-clamp_max, sortino));

    float omega = 0.0f;
    if (sum_neg_abs_r > eps_denom) {
        omega = sum_pos_r / sum_neg_abs_r;
        omega = fminf(clamp_max, fmaxf(0.0f, omega));
    } else if (sum_pos_r > 0.0f) {
        omega = clamp_max;
    }

    float calmar = 0.0f;
    if (max_dd > eps_denom) {
        calmar = pnl / max_dd;
    } else if (pnl > 0.0f) {
        calmar = clamp_max;
    } else if (pnl < 0.0f) {
        calmar = -clamp_max;
    }
    calmar = fminf(clamp_max, fmaxf(-clamp_max, calmar));

    const float exposure = (n_obs > 0) ? ((float)sum_abs_pos / (float)n_obs) : 0.0f;
    const float net_exposure = (n_obs > 0) ? ((float)sum_pos / (float)n_obs) : 0.0f;

    if (out_stride >= 10) {
        out_metrics[base + 0] = pnl;
        out_metrics[base + 1] = sharpe;
        out_metrics[base + 2] = sortino;
        out_metrics[base + 3] = calmar;
        out_metrics[base + 4] = omega;
        out_metrics[base + 5] = volatility;
        out_metrics[base + 6] = max_dd;
        out_metrics[base + 7] = (float)trades;
        out_metrics[base + 8] = exposure;
        out_metrics[base + 9] = (flags & STRAT_SIGNED_EXPOSURE) ? net_exposure : 0.0f;
    }
    if (out_slice_stats != nullptr) {
        out_slice_stats[slice_base + 0] = sum_step_r;
        out_slice_stats[slice_base + 1] = sumsq_step_r;
    }
}

extern "C" __global__
void vemr_backtest_metrics_f32(
    const float* __restrict__ close,
    const float* __restrict__ lr,
    int T,
    const float* __restrict__ zscore_row_major,
    int z_rows,
    int z_stride,
    const float* __restrict__ bb_upper_row_major,
    const float* __restrict__ bb_middle_row_major,
    const float* __restrict__ bb_lower_row_major,
    int bb_rows,
    int bb_stride,
    const float* __restrict__ atr_row_major,
    int atr_rows,
    int atr_stride,
    const int* __restrict__ combo_z_row_idx,
    const int* __restrict__ combo_bb_row_idx,
    const int* __restrict__ combo_atr_row_idx,
    const int* __restrict__ combo_start_t,
    const int* __restrict__ combo_end_t,
    int combo_count,
    unsigned int flags,
    float commission,
    float z_entry,
    float z_exit,
    float atr_stop_mult,
    int require_band_touch,
    int confirm_bars,
    int max_trade_bars,
    int out_stride,
    float* __restrict__ out_metrics
) {
    const int idx = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (idx >= combo_count) return;

    const int z_row = combo_z_row_idx[idx];
    const int bb_row = combo_bb_row_idx[idx];
    const int atr_row = combo_atr_row_idx[idx];
    const int start_t = combo_start_t[idx];
    const int end_t = combo_end_t[idx];
    const int long_only = (flags & STRAT_LONG_ONLY) ? 1 : 0;
    const int no_flip = (flags & STRAT_NO_FLIP) ? 1 : 0;
    const int trade_on_next_bar = (flags & STRAT_TRADE_ON_NEXT_BAR) ? 1 : 0;

    const int base = idx * out_stride;
    if (base < 0) return;
    if (z_row < 0 || z_row >= z_rows || bb_row < 0 || bb_row >= bb_rows || atr_row < 0 || atr_row >= atr_rows || start_t < 0 || start_t >= T || end_t <= start_t || end_t > T) {
        if (out_stride >= 10) {
            out_metrics[base + 0] = 0.0f;
            out_metrics[base + 1] = 0.0f;
            out_metrics[base + 2] = 0.0f;
            out_metrics[base + 3] = 0.0f;
            out_metrics[base + 4] = 0.0f;
            out_metrics[base + 5] = 0.0f;
            out_metrics[base + 6] = 0.0f;
            out_metrics[base + 7] = 0.0f;
            out_metrics[base + 8] = 0.0f;
            out_metrics[base + 9] = 0.0f;
        }
        return;
    }

    const float* __restrict__ zscore = zscore_row_major + ((size_t)z_row * (size_t)z_stride);
    const float* __restrict__ bb_upper = bb_upper_row_major + ((size_t)bb_row * (size_t)bb_stride);
    const float* __restrict__ bb_middle = bb_middle_row_major + ((size_t)bb_row * (size_t)bb_stride);
    const float* __restrict__ bb_lower = bb_lower_row_major + ((size_t)bb_row * (size_t)bb_stride);
    const float* __restrict__ atr = atr_row_major + ((size_t)atr_row * (size_t)atr_stride);

    int pos_signal = 0;
    int bars_in_trade = 0;
    float entry_price = NAN;
    int long_setup_count = 0;
    int short_setup_count = 0;

    int pos_exec = 0;
    float log_eq = 0.0f;
    float log_peak = 0.0f;
    float max_log_dd = 0.0f;
    int trades = 0;

    int n_obs = 0;
    float mean = 0.0f;
    float m2 = 0.0f;
    float sum_neg_sq = 0.0f;
    float sum_pos_r = 0.0f;
    float sum_neg_abs_r = 0.0f;
    int sum_abs_pos = 0;
    int sum_pos = 0;

    const float c = fminf(fmaxf(commission, 0.0f), 0.95f);
    const float log_cost = (c > 0.0f) ? logf(fmaxf(1.0e-12f, 1.0f - c)) : 0.0f;
    const int confirm_bars_clamped = max(confirm_bars, 0);
    const int need_confirm = confirm_bars_clamped + 1;
    const int max_trade_bars_clamped = max(max_trade_bars, 1);
    const int require_band_touch_i = (require_band_touch != 0) ? 1 : 0;

    for (int t = start_t; t < end_t; ++t) {
        int sig_t = t - trade_on_next_bar;
        if (sig_t < 0) sig_t = 0;

        const float z_now = zscore[sig_t];
        const int z_valid = isfinite(z_now);
        float close_now = 0.0f;
        int have_close = 0;
        int long_seed = 0;
        int short_seed = 0;

        if (pos_signal == 0) {
            if (z_valid && (z_now <= -z_entry)) {
                if (!require_band_touch_i) {
                    long_seed = 1;
                } else {
                    close_now = close[sig_t];
                    have_close = 1;
                    const float lower_now = bb_lower[sig_t];
                    long_seed = isfinite(close_now) && isfinite(lower_now) && (close_now <= lower_now);
                }
            }
            if (!long_only && z_valid && (z_now >= z_entry)) {
                if (!require_band_touch_i) {
                    short_seed = 1;
                } else {
                    if (!have_close) {
                        close_now = close[sig_t];
                        have_close = 1;
                    }
                    const float upper_now = bb_upper[sig_t];
                    short_seed = isfinite(close_now) && isfinite(upper_now) && (close_now >= upper_now);
                }
            }

            bars_in_trade = 0;
            entry_price = NAN;

            long_setup_count = long_seed ? (long_setup_count + 1) : 0;
            short_setup_count = short_seed ? (short_setup_count + 1) : 0;

            if (long_setup_count >= need_confirm) {
                if (!have_close) {
                    close_now = close[sig_t];
                    have_close = 1;
                }
                pos_signal = 1;
                bars_in_trade = 1;
                entry_price = close_now;
                long_setup_count = 0;
                short_setup_count = 0;
            } else if (!long_only && short_setup_count >= need_confirm) {
                if (!have_close) {
                    close_now = close[sig_t];
                    have_close = 1;
                }
                pos_signal = -1;
                bars_in_trade = 1;
                entry_price = close_now;
                long_setup_count = 0;
                short_setup_count = 0;
            }
        } else if (pos_signal > 0) {
            close_now = close[sig_t];
            const float atr_now = atr[sig_t];
            const float stop_dist = isfinite(atr_now) ? (atr_stop_mult * atr_now) : NAN;
            const int long_stop_hit = isfinite(entry_price)
                && isfinite(stop_dist)
                && isfinite(close_now)
                && (close_now <= entry_price - stop_dist);
            const float middle_now = bb_middle[sig_t];
            const int mean_exit = (z_valid && (z_now >= -z_exit))
                || (isfinite(close_now) && isfinite(middle_now) && (close_now >= middle_now));
            int exit_now = long_stop_hit || mean_exit;

            if (!long_only && z_valid && (z_now >= z_entry)) {
                if (!require_band_touch_i) {
                    short_seed = 1;
                } else {
                    const float upper_now = bb_upper[sig_t];
                    short_seed = isfinite(close_now) && isfinite(upper_now) && (close_now >= upper_now);
                }
            }

            if (!long_only && short_seed) {
                if (!no_flip) {
                    pos_signal = -1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    long_setup_count = 0;
                    short_setup_count = 0;
                } else {
                    exit_now = 1;
                }
            }
            if (bars_in_trade >= max_trade_bars_clamped) {
                exit_now = 1;
            }

            if (exit_now) {
                pos_signal = 0;
                bars_in_trade = 0;
                entry_price = NAN;
            } else if (pos_signal > 0) {
                bars_in_trade += 1;
            }
        } else {
            close_now = close[sig_t];
            const float atr_now = atr[sig_t];
            const float stop_dist = isfinite(atr_now) ? (atr_stop_mult * atr_now) : NAN;
            const int short_stop_hit = isfinite(entry_price)
                && isfinite(stop_dist)
                && isfinite(close_now)
                && (close_now >= entry_price + stop_dist);
            const float middle_now = bb_middle[sig_t];
            const int mean_exit = (z_valid && (z_now <= z_exit))
                || (isfinite(close_now) && isfinite(middle_now) && (close_now <= middle_now));
            int exit_now = short_stop_hit || mean_exit;

            if (z_valid && (z_now <= -z_entry)) {
                if (!require_band_touch_i) {
                    long_seed = 1;
                } else {
                    const float lower_now = bb_lower[sig_t];
                    long_seed = isfinite(close_now) && isfinite(lower_now) && (close_now <= lower_now);
                }
            }

            if (long_seed) {
                if (!no_flip) {
                    pos_signal = 1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    long_setup_count = 0;
                    short_setup_count = 0;
                } else {
                    exit_now = 1;
                }
            }
            if (bars_in_trade >= max_trade_bars_clamped) {
                exit_now = 1;
            }

            if (exit_now) {
                pos_signal = 0;
                bars_in_trade = 0;
                entry_price = NAN;
            } else if (pos_signal < 0) {
                bars_in_trade += 1;
            }
        }

        int desired = pos_signal;
        if (long_only && desired < 0) {
            desired = 0;
        }

        if (desired != pos_exec) {
            if (pos_exec == 0 || desired == 0) {
                if (log_cost != 0.0f) log_eq += log_cost;
                trades += 1;
                pos_exec = desired;
            } else if (no_flip) {
                if (log_cost != 0.0f) log_eq += log_cost;
                trades += 1;
                pos_exec = 0;
            } else {
                if (log_cost != 0.0f) log_eq += 2.0f * log_cost;
                trades += 2;
                pos_exec = desired;
            }
        }

        const float step_log = (pos_exec == 0) ? 0.0f : ((float)pos_exec) * lr[t];
        log_eq += step_log;

        if (log_eq > log_peak) {
            log_peak = log_eq;
        }
        const float dd_log = log_peak - log_eq;
        if (dd_log > max_log_dd) {
            max_log_dd = dd_log;
        }

        const float step_r = expf(step_log) - 1.0f;
        n_obs += 1;
        const float delta = step_r - mean;
        mean += delta / (float)n_obs;
        const float delta2 = step_r - mean;
        m2 += delta * delta2;

        if (step_r < 0.0f) {
            sum_neg_sq += step_r * step_r;
            sum_neg_abs_r += -step_r;
        } else {
            sum_pos_r += step_r;
        }

        sum_abs_pos += abs(pos_exec);
        sum_pos += pos_exec;
    }

    const float variance = (n_obs > 1) ? (m2 / (float)(n_obs - 1)) : 0.0f;
    const float volatility = sqrtf(fmaxf(variance, 0.0f));
    const float sharpe = (volatility > 0.0f) ? (mean / volatility) : 0.0f;
    const float pnl = expf(log_eq) - 1.0f;
    const float max_dd = (max_log_dd > 0.0f) ? -expm1f(-max_log_dd) : 0.0f;

    const float clamp_max = 1.0e6f;
    const float eps_denom = 1.0e-12f;
    const float denom = (float)max(n_obs, 1);
    const float downside_dev = sqrtf(fmaxf(sum_neg_sq / denom, 0.0f));
    float sortino = 0.0f;
    if (downside_dev > 0.0f) {
        sortino = mean / downside_dev;
    } else if (mean > 0.0f) {
        sortino = clamp_max;
    } else if (mean < 0.0f) {
        sortino = -clamp_max;
    }
    sortino = fminf(clamp_max, fmaxf(-clamp_max, sortino));

    float omega = 0.0f;
    if (sum_neg_abs_r > eps_denom) {
        omega = sum_pos_r / sum_neg_abs_r;
        omega = fminf(clamp_max, fmaxf(0.0f, omega));
    } else if (sum_pos_r > 0.0f) {
        omega = clamp_max;
    }

    float calmar = 0.0f;
    if (max_dd > eps_denom) {
        calmar = pnl / max_dd;
    } else if (pnl > 0.0f) {
        calmar = clamp_max;
    } else if (pnl < 0.0f) {
        calmar = -clamp_max;
    }
    calmar = fminf(clamp_max, fmaxf(-clamp_max, calmar));

    const float exposure = (n_obs > 0) ? ((float)sum_abs_pos / (float)n_obs) : 0.0f;
    const float net_exposure = (n_obs > 0) ? ((float)sum_pos / (float)n_obs) : 0.0f;

    if (out_stride >= 10) {
        out_metrics[base + 0] = pnl;
        out_metrics[base + 1] = sharpe;
        out_metrics[base + 2] = sortino;
        out_metrics[base + 3] = calmar;
        out_metrics[base + 4] = omega;
        out_metrics[base + 5] = volatility;
        out_metrics[base + 6] = max_dd;
        out_metrics[base + 7] = (float)trades;
        out_metrics[base + 8] = exposure;
        out_metrics[base + 9] = (flags & STRAT_SIGNED_EXPOSURE) ? net_exposure : 0.0f;
    }
}




