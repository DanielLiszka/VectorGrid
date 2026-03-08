#pragma once

#include <cuda_fp16.h>
#include <cuda_bf16.h>
#include <cuda_runtime.h>
#include <stdint.h>

namespace prism {

enum PrismRegimeGate : int32_t {
    PRISM_REGIME_NONE = 0,
    PRISM_REGIME_SET = 1,
    PRISM_REGIME_VTRS = 2,
};

enum PrismEntryModule : int32_t {
    PRISM_ENTRY_VCC = 0,
    PRISM_ENTRY_VEMR = 1,
    PRISM_ENTRY_ABMT = 2,
    PRISM_ENTRY_TDB = 3,
};

enum PrismRiskOverlay : int32_t {
    PRISM_RISK_ATR = 0,
    PRISM_RISK_DEVSTOP = 1,
    PRISM_RISK_CHANDELIER = 2,
};

enum PrismChangedGroupMask : uint32_t {
    PRISM_CHANGED_NONE   = 0u,
    PRISM_CHANGED_REGIME = 1u << 0,
    PRISM_CHANGED_ENTRY  = 1u << 1,
    PRISM_CHANGED_RISK   = 1u << 2,
    PRISM_CHANGED_FULL   = PRISM_CHANGED_REGIME | PRISM_CHANGED_ENTRY | PRISM_CHANGED_RISK,
};

enum PrismParamSlot : int32_t {
    P_LINREG_LEN = 0,
    P_ANGLE_MIN,
    P_ER_LEN,
    P_ER_MIN,
    P_CHOP_LEN,
    P_CHOP_MAX,
    P_VOL_THRESH,
    P_KURT_THRESH,
    P_BUFF_LEN,
    P_BB_LEN,
    P_KC_LEN,
    P_MOM_LEN,
    P_FAST_LEN,
    P_SLOW_LEN,
    P_ENTRY_Q,
    P_EXIT_Q,
    P_Z_LEN,
    P_Z_ENTRY,
    P_Z_EXIT,
    P_ATR_LEN,
    P_ATR_MULT,
    P_DEV_LEN,
    P_DEV_MULT,
    P_CHAND_PERIOD,
    P_CHAND_MULT,
    P_DONCHIAN_LEN,
    P_DI_LEN,
    P_ADXR_LEN,
    P_ADXR_MIN,
    P_BASELINE_LEN,
    P_MACZ_FAST,
    P_MACZ_SLOW,
    P_MACZ_SIGNAL,
    P_PARAM_COUNT
};

struct PrismSignatureSpec {
    int32_t regime_gate;
    int32_t entry_module;
    int32_t risk_overlay;
    int32_t batch_shape;
    int32_t active_count;
    int32_t candles;
    int32_t candle_stride;
    uint32_t changed_group_mask;
};

struct PrismStageSpec {
    int32_t candles;
    int32_t batch_shape;
    int32_t active_count;
};

struct PrismCandidateParams {
    float values[P_PARAM_COUNT];
};

struct PrismIndicatorBankView {
    const float* close;
    const float* logret;

    const float* linearreg_angle;
    const float* er;
    const float* chop;

    const float* var_series;
    const float* kurtosis;
    const float* buff_baseline;

    const float* ttm_squeeze;
    const float* squeeze_momentum;
    const float* ttm_trend;

    const float* zscore;
    const float* bb_upper;
    const float* bb_mid;
    const float* bb_lower;

    const float* atr;
    const float* devstop;
    const float* chandelier_stop;

    const float* donchian_upper;
    const float* donchian_lower;
    const float* di_plus;
    const float* di_minus;
    const float* adxr;

    const float* baseline;
    const float* macz_hist;
};

struct PrismModuleCacheView {
    uint8_t* regime_mask;       // [batch_shape, candles]
    int8_t* entry_signal;       // [batch_shape, candles]
    float* risk_stop;           // [batch_shape, candles]
    int8_t* composed_signal;    // [batch_shape, candles]
};

struct PrismMetricsOut {
    float obj0;
    float obj1;
    float pnl;
    float max_drawdown;
    float turnover;
    int32_t trades;
};

struct PrismScreenModelView {
    const half* beta;               // [phi_dim_padded, out_cols_padded]
    const float* half_width_base;   // [2]
    int32_t latent_dim;
    int32_t phi_dim;
    int32_t phi_dim_padded;
    int32_t out_cols_padded;
    int32_t group_count;
    const int32_t* group_offsets;   // [group_count+1]
    const int32_t* group_indices;   // flattened feature indices for group mean/std
};

__device__ __forceinline__ int idx2(int row, int col, int stride) {
    return row * stride + col;
}

__device__ __forceinline__ float load_series(const float* ptr, int t) {
    return ptr ? ptr[t] : 0.0f;
}

__device__ __forceinline__ int8_t sign_i8(float x) {
    return (x > 0.0f) - (x < 0.0f);
}

__device__ __forceinline__ float warp_sum(float v) {
    for (int offset = 16; offset > 0; offset >>= 1) {
        v += __shfl_down_sync(0xffffffffu, v, offset);
    }
    return v;
}

} // namespace prism
