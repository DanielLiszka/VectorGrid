#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RtBarrierScene RtBarrierScene;

typedef struct RtBarrierBuildInput {
  const float* lows;
  const float* highs;
  int32_t count;
  float z_min;
  float z_max;
} RtBarrierBuildInput;

typedef struct RtBarrierQuery {
  int32_t entry_bar;
  float barrier_price;
} RtBarrierQuery;

typedef struct RtBarrierResult {
  int32_t hit;
  int32_t hit_bar;
  int32_t primitive_index;
} RtBarrierResult;

int rtbarrier_is_available(int device_ordinal);
int rtbarrier_create(int device_ordinal, RtBarrierScene** out_scene, char* error_buf, int error_buf_len);
int rtbarrier_destroy(RtBarrierScene* scene);
int rtbarrier_build(RtBarrierScene* scene, const RtBarrierBuildInput* input, char* error_buf, int error_buf_len);
int rtbarrier_query(
    RtBarrierScene* scene,
    const RtBarrierQuery* queries,
    int32_t query_count,
    RtBarrierResult* out_results,
    uint64_t cuda_stream,
    char* error_buf,
    int error_buf_len);

#ifdef __cplusplus
}
#endif
