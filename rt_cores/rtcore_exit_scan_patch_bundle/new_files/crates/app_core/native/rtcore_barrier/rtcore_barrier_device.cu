#include <optix.h>
#include <optix_device.h>

#include <stdint.h>

struct RtBarrierQuery {
  int32_t entry_bar;
  float barrier_price;
};

struct RtBarrierResult {
  int32_t hit;
  int32_t hit_bar;
  int32_t primitive_index;
};

struct Params {
  OptixTraversableHandle handle;
  const RtBarrierQuery* queries;
  RtBarrierResult* results;
};

extern "C" __constant__ Params params;

extern "C" __global__ void __raygen__barrier() {
  const uint32_t idx = optixGetLaunchIndex().x;
  const RtBarrierQuery q = params.queries[idx];

  unsigned int primitive = 0xFFFFFFFFu;
  const float origin_x = static_cast<float>(q.entry_bar + 1) + 1.0e-4f;
  const float3 origin = make_float3(origin_x, q.barrier_price, 0.5f);
  const float3 direction = make_float3(1.0f, 0.0f, 0.0f);

  optixTrace(
      params.handle,
      origin,
      direction,
      0.0f,
      1.0e20f,
      0.0f,
      OptixVisibilityMask(255),
      OPTIX_RAY_FLAG_DISABLE_ANYHIT,
      0,
      1,
      0,
      primitive);

  RtBarrierResult out{};
  out.hit = primitive != 0xFFFFFFFFu;
  out.primitive_index = out.hit ? static_cast<int32_t>(primitive) : -1;
  out.hit_bar = out.hit ? static_cast<int32_t>(primitive / 12u) : -1;
  params.results[idx] = out;
}

extern "C" __global__ void __miss__barrier() {
  optixSetPayload_0(0xFFFFFFFFu);
}

extern "C" __global__ void __closesthit__barrier() {
  optixSetPayload_0(optixGetPrimitiveIndex());
}
