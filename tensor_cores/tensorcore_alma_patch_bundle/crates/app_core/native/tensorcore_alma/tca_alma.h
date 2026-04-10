#pragma once

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct tca_plan tca_plan;

typedef enum tca_precision {
    TCA_PRECISION_TF32_FAST = 0,
    TCA_PRECISION_F16 = 1,
} tca_precision;

int tca_create_plan(
    const float* host_weights_t,
    int n_windows,
    int window,
    int precision,
    tca_plan** out_plan);

int tca_destroy_plan(tca_plan* plan);

int tca_run_plan(
    tca_plan* plan,
    const float* device_prices,
    int n_prices,
    int chunk_rows,
    float* device_out,
    int* out_rows,
    void* cuda_stream);

const char* tca_last_error(void);

/* Lightweight helpers for the included Rust benchmark binary. */
int tca_malloc_f32(size_t count, float** out_ptr);
int tca_free_ptr(void* ptr);
int tca_memcpy_h2d_f32(float* dst, const float* src, size_t count);
int tca_memcpy_d2h_f32(float* dst, const float* src, size_t count);
int tca_device_synchronize(void);

#ifdef __cplusplus
}
#endif
