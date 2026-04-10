#include "rtcore_barrier_api.h"
#include "ptx_path.h"

#include <cuda.h>
#include <cuda_runtime.h>

#include <optix.h>
#include <optix_stubs.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace {

struct Params {
  OptixTraversableHandle handle;
  const RtBarrierQuery* queries;
  RtBarrierResult* results;
};

template <typename T>
struct __align__(OPTIX_SBT_RECORD_ALIGNMENT) SbtRecord {
  char header[OPTIX_SBT_RECORD_HEADER_SIZE];
  T data;
};

struct EmptyData {};
using EmptyRecord = SbtRecord<EmptyData>;

struct RtBarrierScene {
  int device_ordinal = 0;
  CUcontext cuda_context = nullptr;
  cudaStream_t internal_stream = nullptr;

  OptixDeviceContext optix_context = nullptr;
  OptixModule module = nullptr;
  OptixPipeline pipeline = nullptr;
  OptixProgramGroup raygen_pg = nullptr;
  OptixProgramGroup miss_pg = nullptr;
  OptixProgramGroup hitgroup_pg = nullptr;
  OptixShaderBindingTable sbt = {};

  CUdeviceptr d_raygen_record = 0;
  CUdeviceptr d_miss_record = 0;
  CUdeviceptr d_hitgroup_record = 0;
  CUdeviceptr d_vertices = 0;
  CUdeviceptr d_indices = 0;
  CUdeviceptr d_gas = 0;
  CUdeviceptr d_scratch = 0;
  CUdeviceptr d_params = 0;
  CUdeviceptr d_queries = 0;
  CUdeviceptr d_results = 0;

  OptixTraversableHandle gas_handle = 0;
  size_t query_capacity = 0;
  int bar_count = 0;
};

void write_error(char* error_buf, int error_buf_len, const std::string& msg) {
  if (!error_buf || error_buf_len <= 0) {
    return;
  }
  const int copy_len = std::min<int>(error_buf_len - 1, static_cast<int>(msg.size()));
  std::memcpy(error_buf, msg.data(), copy_len);
  error_buf[copy_len] = '\0';
}

#define CUDA_CHECK(call)                                                            \
  do {                                                                              \
    cudaError_t _status = (call);                                                   \
    if (_status != cudaSuccess) {                                                   \
      throw std::runtime_error(std::string("CUDA error: ") + cudaGetErrorString(_status)); \
    }                                                                               \
  } while (0)

#define CU_CHECK(call)                                                              \
  do {                                                                              \
    CUresult _status = (call);                                                      \
    if (_status != CUDA_SUCCESS) {                                                  \
      const char* _name = nullptr;                                                  \
      const char* _desc = nullptr;                                                  \
      cuGetErrorName(_status, &_name);                                              \
      cuGetErrorString(_status, &_desc);                                            \
      throw std::runtime_error(std::string("CUDA driver error: ") +                \
                               (_name ? _name : "<unknown>") + " / " +             \
                               (_desc ? _desc : "<no description>"));              \
    }                                                                               \
  } while (0)

#define OPTIX_CHECK(call)                                                           \
  do {                                                                              \
    OptixResult _status = (call);                                                   \
    if (_status != OPTIX_SUCCESS) {                                                 \
      throw std::runtime_error(std::string("OptiX error: ") + std::to_string((int)_status)); \
    }                                                                               \
  } while (0)

std::string read_text_file(const char* path) {
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    throw std::runtime_error(std::string("Failed to open PTX file: ") + path);
  }
  std::ostringstream ss;
  ss << input.rdbuf();
  return ss.str();
}

void free_device_ptr(CUdeviceptr& ptr) {
  if (ptr) {
    CU_CHECK(cuMemFree(ptr));
    ptr = 0;
  }
}

void destroy_program_group(OptixProgramGroup& pg) {
  if (pg) {
    OPTIX_CHECK(optixProgramGroupDestroy(pg));
    pg = nullptr;
  }
}

void ensure_cuda_context(RtBarrierScene& scene) {
  CUDA_CHECK(cudaSetDevice(scene.device_ordinal));
  CUDA_CHECK(cudaFree(nullptr));
  CU_CHECK(cuCtxGetCurrent(&scene.cuda_context));
  if (!scene.cuda_context) {
    throw std::runtime_error("No current CUDA context after cudaFree(0)");
  }
}

void create_optix_pipeline(RtBarrierScene& scene) {
  if (scene.pipeline) {
    return;
  }

  static bool optix_initialized = false;
  if (!optix_initialized) {
    OPTIX_CHECK(optixInit());
    optix_initialized = true;
  }

  OptixDeviceContextOptions context_options = {};
  context_options.logCallbackFunction = nullptr;
  context_options.logCallbackLevel = 4;
  OPTIX_CHECK(optixDeviceContextCreate(scene.cuda_context, &context_options, &scene.optix_context));

  const std::string ptx = read_text_file(RTCORE_BARRIER_DEVICE_PTX_PATH);

  OptixModuleCompileOptions module_compile_options = {};
  module_compile_options.maxRegisterCount = OPTIX_COMPILE_DEFAULT_MAX_REGISTER_COUNT;
  module_compile_options.optLevel = OPTIX_COMPILE_OPTIMIZATION_DEFAULT;
  module_compile_options.debugLevel = OPTIX_COMPILE_DEBUG_LEVEL_LINEINFO;

  OptixPipelineCompileOptions pipeline_compile_options = {};
  pipeline_compile_options.usesMotionBlur = 0;
  pipeline_compile_options.traversableGraphFlags = OPTIX_TRAVERSABLE_GRAPH_FLAG_ALLOW_SINGLE_GAS;
  pipeline_compile_options.numPayloadValues = 1;
  pipeline_compile_options.numAttributeValues = 0;
  pipeline_compile_options.exceptionFlags = OPTIX_EXCEPTION_FLAG_NONE;
  pipeline_compile_options.pipelineLaunchParamsVariableName = "params";
#if OPTIX_VERSION >= 70600
  pipeline_compile_options.usesPrimitiveTypeFlags = OPTIX_PRIMITIVE_TYPE_FLAGS_TRIANGLE;
#endif

  char log[4096];
  size_t log_size = sizeof(log);
#if OPTIX_VERSION >= 70700
  OPTIX_CHECK(optixModuleCreate(
      scene.optix_context,
      &module_compile_options,
      &pipeline_compile_options,
      ptx.c_str(),
      ptx.size(),
      log,
      &log_size,
      &scene.module));
#else
  OPTIX_CHECK(optixModuleCreateFromPTX(
      scene.optix_context,
      &module_compile_options,
      &pipeline_compile_options,
      ptx.c_str(),
      ptx.size(),
      log,
      &log_size,
      &scene.module));
#endif

  OptixProgramGroupOptions pg_options = {};

  OptixProgramGroupDesc rg_desc = {};
  rg_desc.kind = OPTIX_PROGRAM_GROUP_KIND_RAYGEN;
  rg_desc.raygen.module = scene.module;
  rg_desc.raygen.entryFunctionName = "__raygen__barrier";
  log_size = sizeof(log);
  OPTIX_CHECK(optixProgramGroupCreate(
      scene.optix_context,
      &rg_desc,
      1,
      &pg_options,
      log,
      &log_size,
      &scene.raygen_pg));

  OptixProgramGroupDesc ms_desc = {};
  ms_desc.kind = OPTIX_PROGRAM_GROUP_KIND_MISS;
  ms_desc.miss.module = scene.module;
  ms_desc.miss.entryFunctionName = "__miss__barrier";
  log_size = sizeof(log);
  OPTIX_CHECK(optixProgramGroupCreate(
      scene.optix_context,
      &ms_desc,
      1,
      &pg_options,
      log,
      &log_size,
      &scene.miss_pg));

  OptixProgramGroupDesc hg_desc = {};
  hg_desc.kind = OPTIX_PROGRAM_GROUP_KIND_HITGROUP;
  hg_desc.hitgroup.moduleCH = scene.module;
  hg_desc.hitgroup.entryFunctionNameCH = "__closesthit__barrier";
  hg_desc.hitgroup.moduleAH = nullptr;
  hg_desc.hitgroup.entryFunctionNameAH = nullptr;
  hg_desc.hitgroup.moduleIS = nullptr;
  hg_desc.hitgroup.entryFunctionNameIS = nullptr;
  log_size = sizeof(log);
  OPTIX_CHECK(optixProgramGroupCreate(
      scene.optix_context,
      &hg_desc,
      1,
      &pg_options,
      log,
      &log_size,
      &scene.hitgroup_pg));

  OptixProgramGroup program_groups[] = {
      scene.raygen_pg,
      scene.miss_pg,
      scene.hitgroup_pg,
  };

  OptixPipelineLinkOptions link_options = {};
  link_options.maxTraceDepth = 1;
  link_options.debugLevel = OPTIX_COMPILE_DEBUG_LEVEL_LINEINFO;
  log_size = sizeof(log);
  OPTIX_CHECK(optixPipelineCreate(
      scene.optix_context,
      &pipeline_compile_options,
      &link_options,
      program_groups,
      sizeof(program_groups) / sizeof(program_groups[0]),
      log,
      &log_size,
      &scene.pipeline));

  OPTIX_CHECK(optixPipelineSetStackSize(scene.pipeline, 2048, 2048, 2048, 1));

  EmptyRecord rg_record = {};
  EmptyRecord ms_record = {};
  EmptyRecord hg_record = {};
  OPTIX_CHECK(optixSbtRecordPackHeader(scene.raygen_pg, &rg_record));
  OPTIX_CHECK(optixSbtRecordPackHeader(scene.miss_pg, &ms_record));
  OPTIX_CHECK(optixSbtRecordPackHeader(scene.hitgroup_pg, &hg_record));

  CU_CHECK(cuMemAlloc(&scene.d_raygen_record, sizeof(EmptyRecord)));
  CU_CHECK(cuMemAlloc(&scene.d_miss_record, sizeof(EmptyRecord)));
  CU_CHECK(cuMemAlloc(&scene.d_hitgroup_record, sizeof(EmptyRecord)));
  CU_CHECK(cuMemcpyHtoD(scene.d_raygen_record, &rg_record, sizeof(EmptyRecord)));
  CU_CHECK(cuMemcpyHtoD(scene.d_miss_record, &ms_record, sizeof(EmptyRecord)));
  CU_CHECK(cuMemcpyHtoD(scene.d_hitgroup_record, &hg_record, sizeof(EmptyRecord)));

  scene.sbt.raygenRecord = scene.d_raygen_record;
  scene.sbt.missRecordBase = scene.d_miss_record;
  scene.sbt.missRecordStrideInBytes = sizeof(EmptyRecord);
  scene.sbt.missRecordCount = 1;
  scene.sbt.hitgroupRecordBase = scene.d_hitgroup_record;
  scene.sbt.hitgroupRecordStrideInBytes = sizeof(EmptyRecord);
  scene.sbt.hitgroupRecordCount = 1;

  CU_CHECK(cuMemAlloc(&scene.d_params, sizeof(Params)));
  CUDA_CHECK(cudaStreamCreate(&scene.internal_stream));
}

void destroy_scene(RtBarrierScene& scene) {
  if (scene.internal_stream) {
    cudaStreamDestroy(scene.internal_stream);
    scene.internal_stream = nullptr;
  }
  free_device_ptr(scene.d_queries);
  free_device_ptr(scene.d_results);
  free_device_ptr(scene.d_params);
  free_device_ptr(scene.d_vertices);
  free_device_ptr(scene.d_indices);
  free_device_ptr(scene.d_gas);
  free_device_ptr(scene.d_scratch);
  free_device_ptr(scene.d_raygen_record);
  free_device_ptr(scene.d_miss_record);
  free_device_ptr(scene.d_hitgroup_record);

  if (scene.pipeline) {
    OPTIX_CHECK(optixPipelineDestroy(scene.pipeline));
    scene.pipeline = nullptr;
  }
  destroy_program_group(scene.raygen_pg);
  destroy_program_group(scene.miss_pg);
  destroy_program_group(scene.hitgroup_pg);

  if (scene.module) {
    OPTIX_CHECK(optixModuleDestroy(scene.module));
    scene.module = nullptr;
  }
  if (scene.optix_context) {
    OPTIX_CHECK(optixDeviceContextDestroy(scene.optix_context));
    scene.optix_context = nullptr;
  }
}

void upload_geometry_buffer(CUdeviceptr& dst, const void* src, size_t bytes) {
  if (dst) {
    CU_CHECK(cuMemFree(dst));
    dst = 0;
  }
  CU_CHECK(cuMemAlloc(&dst, bytes));
  CU_CHECK(cuMemcpyHtoD(dst, src, bytes));
}

}  // namespace

int rtbarrier_is_available(int device_ordinal) {
  try {
    RtBarrierScene scene;
    scene.device_ordinal = device_ordinal;
    ensure_cuda_context(scene);

    OPTIX_CHECK(optixInit());
    OptixDeviceContextOptions options = {};
    OPTIX_CHECK(optixDeviceContextCreate(scene.cuda_context, &options, &scene.optix_context));

    int rtcore_version = 0;
    OPTIX_CHECK(optixDeviceContextGetProperty(
        scene.optix_context,
        OPTIX_DEVICE_PROPERTY_RTCORE_VERSION,
        &rtcore_version,
        sizeof(rtcore_version)));

    OPTIX_CHECK(optixDeviceContextDestroy(scene.optix_context));
    scene.optix_context = nullptr;
    return rtcore_version > 0 ? 1 : 0;
  } catch (...) {
    return 0;
  }
}

int rtbarrier_create(int device_ordinal, RtBarrierScene** out_scene, char* error_buf, int error_buf_len) {
  try {
    if (!out_scene) {
      throw std::runtime_error("out_scene must not be null");
    }
    *out_scene = nullptr;

    RtBarrierScene* scene = new RtBarrierScene();
    scene->device_ordinal = device_ordinal;
    ensure_cuda_context(*scene);
    create_optix_pipeline(*scene);

    int rtcore_version = 0;
    OPTIX_CHECK(optixDeviceContextGetProperty(
        scene->optix_context,
        OPTIX_DEVICE_PROPERTY_RTCORE_VERSION,
        &rtcore_version,
        sizeof(rtcore_version)));
    if (rtcore_version <= 0) {
      throw std::runtime_error("Selected GPU supports OptiX but does not report RT cores");
    }

    *out_scene = scene;
    return 1;
  } catch (const std::exception& e) {
    write_error(error_buf, error_buf_len, e.what());
    return 0;
  }
}

int rtbarrier_destroy(RtBarrierScene* scene) {
  if (!scene) {
    return 1;
  }
  try {
    destroy_scene(*scene);
    delete scene;
    return 1;
  } catch (...) {
    delete scene;
    return 0;
  }
}

int rtbarrier_build(RtBarrierScene* scene, const RtBarrierBuildInput* input, char* error_buf, int error_buf_len) {
  try {
    if (!scene || !input) {
      throw std::runtime_error("scene and input must not be null");
    }
    if (!input->lows || !input->highs) {
      throw std::runtime_error("lows and highs must not be null");
    }
    if (input->count <= 0) {
      throw std::runtime_error("count must be positive");
    }

    ensure_cuda_context(*scene);

    std::vector<float3> vertices;
    std::vector<uint3> indices;
    vertices.reserve(static_cast<size_t>(input->count) * 8u);
    indices.reserve(static_cast<size_t>(input->count) * 12u);

    const float z0 = input->z_min;
    const float z1 = input->z_max > input->z_min ? input->z_max : (input->z_min + 1.0f);

    for (int i = 0; i < input->count; ++i) {
      float lo = input->lows[i];
      float hi = input->highs[i];
      if (!std::isfinite(lo) || !std::isfinite(hi)) {
        lo = 0.0f;
        hi = 0.0f;
      }
      if (hi < lo) {
        std::swap(lo, hi);
      }
      if (std::abs(hi - lo) < 1.0e-7f) {
        hi = lo + 1.0e-6f;
      }

      const float x0 = static_cast<float>(i);
      const float x1 = static_cast<float>(i + 1);
      const uint32_t base = static_cast<uint32_t>(vertices.size());

      vertices.push_back(make_float3(x0, lo, z0));
      vertices.push_back(make_float3(x1, lo, z0));
      vertices.push_back(make_float3(x1, hi, z0));
      vertices.push_back(make_float3(x0, hi, z0));
      vertices.push_back(make_float3(x0, lo, z1));
      vertices.push_back(make_float3(x1, lo, z1));
      vertices.push_back(make_float3(x1, hi, z1));
      vertices.push_back(make_float3(x0, hi, z1));

      // bottom
      indices.push_back(make_uint3(base + 0, base + 1, base + 2));
      indices.push_back(make_uint3(base + 0, base + 2, base + 3));
      // top
      indices.push_back(make_uint3(base + 4, base + 6, base + 5));
      indices.push_back(make_uint3(base + 4, base + 7, base + 6));
      // front
      indices.push_back(make_uint3(base + 0, base + 4, base + 5));
      indices.push_back(make_uint3(base + 0, base + 5, base + 1));
      // back
      indices.push_back(make_uint3(base + 3, base + 2, base + 6));
      indices.push_back(make_uint3(base + 3, base + 6, base + 7));
      // left
      indices.push_back(make_uint3(base + 0, base + 3, base + 7));
      indices.push_back(make_uint3(base + 0, base + 7, base + 4));
      // right
      indices.push_back(make_uint3(base + 1, base + 5, base + 6));
      indices.push_back(make_uint3(base + 1, base + 6, base + 2));
    }

    upload_geometry_buffer(scene->d_vertices, vertices.data(), vertices.size() * sizeof(float3));
    upload_geometry_buffer(scene->d_indices, indices.data(), indices.size() * sizeof(uint3));

    if (scene->d_gas) {
      CU_CHECK(cuMemFree(scene->d_gas));
      scene->d_gas = 0;
      scene->gas_handle = 0;
    }
    if (scene->d_scratch) {
      CU_CHECK(cuMemFree(scene->d_scratch));
      scene->d_scratch = 0;
    }

    CUdeviceptr vertex_buffers[] = {scene->d_vertices};
    unsigned int triangle_flags[] = {OPTIX_GEOMETRY_FLAG_DISABLE_ANYHIT};

    OptixBuildInput build_input = {};
    build_input.type = OPTIX_BUILD_INPUT_TYPE_TRIANGLES;
    build_input.triangleArray.vertexBuffers = vertex_buffers;
    build_input.triangleArray.numVertices = static_cast<unsigned int>(vertices.size());
    build_input.triangleArray.vertexFormat = OPTIX_VERTEX_FORMAT_FLOAT3;
    build_input.triangleArray.vertexStrideInBytes = sizeof(float3);
    build_input.triangleArray.indexBuffer = scene->d_indices;
    build_input.triangleArray.numIndexTriplets = static_cast<unsigned int>(indices.size());
    build_input.triangleArray.indexFormat = OPTIX_INDICES_FORMAT_UNSIGNED_INT3;
    build_input.triangleArray.indexStrideInBytes = sizeof(uint3);
    build_input.triangleArray.flags = triangle_flags;
    build_input.triangleArray.numSbtRecords = 1;

    OptixAccelBuildOptions accel_options = {};
    accel_options.buildFlags = OPTIX_BUILD_FLAG_PREFER_FAST_TRACE;
    accel_options.operation = OPTIX_BUILD_OPERATION_BUILD;

    OptixAccelBufferSizes sizes = {};
    OPTIX_CHECK(optixAccelComputeMemoryUsage(
        scene->optix_context,
        &accel_options,
        &build_input,
        1,
        &sizes));

    CU_CHECK(cuMemAlloc(&scene->d_scratch, sizes.tempSizeInBytes));
    CU_CHECK(cuMemAlloc(&scene->d_gas, sizes.outputSizeInBytes));

    OPTIX_CHECK(optixAccelBuild(
        scene->optix_context,
        scene->internal_stream,
        &accel_options,
        &build_input,
        1,
        scene->d_scratch,
        sizes.tempSizeInBytes,
        scene->d_gas,
        sizes.outputSizeInBytes,
        &scene->gas_handle,
        nullptr,
        0));
    CUDA_CHECK(cudaStreamSynchronize(scene->internal_stream));
    scene->bar_count = input->count;
    return 1;
  } catch (const std::exception& e) {
    write_error(error_buf, error_buf_len, e.what());
    return 0;
  }
}

int rtbarrier_query(
    RtBarrierScene* scene,
    const RtBarrierQuery* queries,
    int32_t query_count,
    RtBarrierResult* out_results,
    uint64_t cuda_stream,
    char* error_buf,
    int error_buf_len) {
  try {
    if (!scene || !queries || !out_results) {
      throw std::runtime_error("scene, queries, and out_results must not be null");
    }
    if (query_count < 0) {
      throw std::runtime_error("query_count must be non-negative");
    }
    if (query_count == 0) {
      return 1;
    }
    if (!scene->gas_handle) {
      throw std::runtime_error("Scene must be built before querying");
    }

    ensure_cuda_context(*scene);
    cudaStream_t stream = cuda_stream ? reinterpret_cast<cudaStream_t>(cuda_stream) : scene->internal_stream;

    const size_t needed = static_cast<size_t>(query_count);
    if (needed > scene->query_capacity) {
      free_device_ptr(scene->d_queries);
      free_device_ptr(scene->d_results);
      CU_CHECK(cuMemAlloc(&scene->d_queries, needed * sizeof(RtBarrierQuery)));
      CU_CHECK(cuMemAlloc(&scene->d_results, needed * sizeof(RtBarrierResult)));
      scene->query_capacity = needed;
    }

    CUDA_CHECK(cudaMemcpyAsync(
        reinterpret_cast<void*>(scene->d_queries),
        queries,
        needed * sizeof(RtBarrierQuery),
        cudaMemcpyHostToDevice,
        stream));

    Params params = {};
    params.handle = scene->gas_handle;
    params.queries = reinterpret_cast<const RtBarrierQuery*>(scene->d_queries);
    params.results = reinterpret_cast<RtBarrierResult*>(scene->d_results);

    CUDA_CHECK(cudaMemcpyAsync(
        reinterpret_cast<void*>(scene->d_params),
        &params,
        sizeof(params),
        cudaMemcpyHostToDevice,
        stream));

    OPTIX_CHECK(optixLaunch(
        scene->pipeline,
        stream,
        scene->d_params,
        sizeof(Params),
        &scene->sbt,
        static_cast<unsigned int>(query_count),
        1,
        1));

    CUDA_CHECK(cudaMemcpyAsync(
        out_results,
        reinterpret_cast<void*>(scene->d_results),
        needed * sizeof(RtBarrierResult),
        cudaMemcpyDeviceToHost,
        stream));
    CUDA_CHECK(cudaStreamSynchronize(stream));
    return 1;
  } catch (const std::exception& e) {
    write_error(error_buf, error_buf_len, e.what());
    return 0;
  }
}
