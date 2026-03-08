#include "prism_common.cuh"

using namespace prism;

extern "C" __global__ void prism_compute_set_gate_kernel(
    PrismIndicatorBankView bank,
    const PrismCandidateParams* __restrict__ params,
    PrismStageSpec stage,
    uint8_t* __restrict__ regime_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    const PrismCandidateParams p = params[cand];
    const float angle_min = p.values[P_ANGLE_MIN];
    const float er_min = p.values[P_ER_MIN];
    const float chop_max = p.values[P_CHOP_MAX];
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        const float angle = load_series(bank.linearreg_angle, t);
        const float er = load_series(bank.er, t);
        const float chop = load_series(bank.chop, t);
        regime_cache[idx2(cand, t, stage.candles)] = (angle >= angle_min && er >= er_min && chop <= chop_max) ? 1u : 0u;
    }
}

extern "C" __global__ void prism_compute_vtrs_gate_kernel(
    PrismIndicatorBankView bank,
    const PrismCandidateParams* __restrict__ params,
    PrismStageSpec stage,
    uint8_t* __restrict__ regime_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    const PrismCandidateParams p = params[cand];
    const float vol_thr = p.values[P_VOL_THRESH];
    const float kurt_thr = p.values[P_KURT_THRESH];
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        const float vol = load_series(bank.var_series, t);
        const float kurt = load_series(bank.kurtosis, t);
        const float base = load_series(bank.buff_baseline, t);
        const float close = load_series(bank.close, t);
        regime_cache[idx2(cand, t, stage.candles)] = (vol <= vol_thr && kurt <= kurt_thr && close >= base) ? 1u : 0u;
    }
}

extern "C" __global__ void prism_compute_none_gate_kernel(
    PrismStageSpec stage,
    uint8_t* __restrict__ regime_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        regime_cache[idx2(cand, t, stage.candles)] = 1u;
    }
}

extern "C" __global__ void prism_compute_vcc_entry_kernel(
    PrismIndicatorBankView bank,
    const PrismCandidateParams* __restrict__ params,
    PrismStageSpec stage,
    int8_t* __restrict__ entry_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    const PrismCandidateParams p = params[cand];
    const float entry_q = p.values[P_ENTRY_Q];
    const float exit_q = p.values[P_EXIT_Q];
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        const float sq = load_series(bank.ttm_squeeze, t);
        const float mom = load_series(bank.squeeze_momentum, t);
        const float trend = load_series(bank.ttm_trend, t);
        int8_t s = 0;
        if (sq <= entry_q && mom > 0.0f && trend > 0.0f) s = +1;
        if (sq <= entry_q && mom < 0.0f && trend < 0.0f) s = -1;
        if (sq >= exit_q) s = 0;
        entry_cache[idx2(cand, t, stage.candles)] = s;
    }
}

extern "C" __global__ void prism_compute_vemr_entry_kernel(
    PrismIndicatorBankView bank,
    const PrismCandidateParams* __restrict__ params,
    PrismStageSpec stage,
    int8_t* __restrict__ entry_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    const PrismCandidateParams p = params[cand];
    const float z_entry = p.values[P_Z_ENTRY];
    const float z_exit = p.values[P_Z_EXIT];
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        const float z = load_series(bank.zscore, t);
        const float close = load_series(bank.close, t);
        const float lo = load_series(bank.bb_lower, t);
        const float mid = load_series(bank.bb_mid, t);
        const float hi = load_series(bank.bb_upper, t);
        int8_t s = 0;
        if (z <= -z_entry && close <= lo) s = +1;
        if (z >= +z_entry && close >= hi) s = -1;
        if (fabsf(z) <= z_exit || close == mid) s = 0;
        entry_cache[idx2(cand, t, stage.candles)] = s;
    }
}

extern "C" __global__ void prism_compute_abmt_entry_kernel(
    PrismIndicatorBankView bank,
    PrismStageSpec stage,
    int8_t* __restrict__ entry_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        const float close = load_series(bank.close, t);
        const float base = load_series(bank.baseline, t);
        const float hist = load_series(bank.macz_hist, t);
        int8_t s = 0;
        if (close > base && hist > 0.0f) s = +1;
        if (close < base && hist < 0.0f) s = -1;
        entry_cache[idx2(cand, t, stage.candles)] = s;
    }
}

extern "C" __global__ void prism_compute_tdb_entry_kernel(
    PrismIndicatorBankView bank,
    const PrismCandidateParams* __restrict__ params,
    PrismStageSpec stage,
    int8_t* __restrict__ entry_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    const PrismCandidateParams p = params[cand];
    const float adxr_min = p.values[P_ADXR_MIN];
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        const float close = load_series(bank.close, t);
        const float up = load_series(bank.donchian_upper, t);
        const float dn = load_series(bank.donchian_lower, t);
        const float dip = load_series(bank.di_plus, t);
        const float dim = load_series(bank.di_minus, t);
        const float adxr = load_series(bank.adxr, t);
        int8_t s = 0;
        if (close >= up && dip > dim && adxr >= adxr_min) s = +1;
        if (close <= dn && dim > dip && adxr >= adxr_min) s = -1;
        entry_cache[idx2(cand, t, stage.candles)] = s;
    }
}

extern "C" __global__ void prism_compute_atr_risk_kernel(
    PrismIndicatorBankView bank,
    const PrismCandidateParams* __restrict__ params,
    PrismStageSpec stage,
    float* __restrict__ risk_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    const PrismCandidateParams p = params[cand];
    const float mult = p.values[P_ATR_MULT];
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        risk_cache[idx2(cand, t, stage.candles)] = load_series(bank.atr, t) * mult;
    }
}

extern "C" __global__ void prism_compute_devstop_risk_kernel(
    PrismIndicatorBankView bank,
    PrismStageSpec stage,
    float* __restrict__ risk_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        risk_cache[idx2(cand, t, stage.candles)] = load_series(bank.devstop, t);
    }
}

extern "C" __global__ void prism_compute_chandelier_risk_kernel(
    PrismIndicatorBankView bank,
    PrismStageSpec stage,
    float* __restrict__ risk_cache
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        risk_cache[idx2(cand, t, stage.candles)] = load_series(bank.chandelier_stop, t);
    }
}

extern "C" __global__ void prism_compose_signal_kernel(
    PrismStageSpec stage,
    const uint8_t* __restrict__ regime_cache,
    const int8_t* __restrict__ entry_cache,
    int8_t* __restrict__ composed_signal
) {
    const int cand = blockIdx.x;
    const int tid = threadIdx.x;
    if (cand >= stage.active_count) return;
    for (int t = tid; t < stage.candles; t += blockDim.x) {
        const int idx = idx2(cand, t, stage.candles);
        composed_signal[idx] = regime_cache[idx] ? entry_cache[idx] : 0;
    }
}

extern "C" __global__ void prism_backtest_metrics_kernel(
    PrismIndicatorBankView bank,
    PrismStageSpec stage,
    const int8_t* __restrict__ composed_signal,
    const float* __restrict__ risk_cache,
    PrismMetricsOut* __restrict__ out_metrics
) {
    const int cand = blockIdx.x;
    if (cand >= stage.active_count) return;
    if (threadIdx.x != 0) return;

    int pos = 0;
    float entry_px = 0.0f;
    float pnl = 0.0f;
    float equity = 0.0f;
    float peak = 0.0f;
    float max_dd = 0.0f;
    float turnover = 0.0f;
    int trades = 0;

    for (int t = 1; t < stage.candles; ++t) {
        const int idx = idx2(cand, t, stage.candles);
        const int8_t sig = composed_signal[idx];
        const float close = load_series(bank.close, t);
        const float prev_close = load_series(bank.close, t - 1);
        const float stop = risk_cache[idx];

        if (pos == 0 && sig != 0) {
            pos = (int)sig;
            entry_px = close;
            trades += 1;
            turnover += 1.0f;
        } else if (pos != 0) {
            const float move = (close - prev_close) * (float)pos;
            pnl += move;
            equity += move;
            peak = fmaxf(peak, equity);
            max_dd = fminf(max_dd, equity - peak);
            const bool stop_hit = (pos > 0) ? (close <= entry_px - stop) : (close >= entry_px + stop);
            const bool flip = (sig == -pos);
            const bool flat = (sig == 0);
            if (stop_hit || flip || flat) {
                pos = 0;
                entry_px = 0.0f;
                turnover += 1.0f;
            }
        }
    }

    const float obj0 = pnl;
    const float obj1 = -max_dd;
    out_metrics[cand].obj0 = obj0;
    out_metrics[cand].obj1 = obj1;
    out_metrics[cand].pnl = pnl;
    out_metrics[cand].max_drawdown = -max_dd;
    out_metrics[cand].turnover = turnover;
    out_metrics[cand].trades = trades;
}
