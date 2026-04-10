#include "tca_alma.h"

#include <cuda_runtime.h>
#include <cuda_fp16.h>
#include <cublasLt.h>

#include <algorithm>
#include <sstream>
#include <string>
#include <vector>
#include <type_traits>

namespace {
thread_local std::string g_last_error;

inline void set_error(const std::string& msg) {
    g_last_error = msg;
}

inline std::string cuda_err(cudaError_t err, const char* what) {
    std::ostringstream oss;
    oss << what << ": " << cudaGetErrorString(err);
    return oss.str();
}

inline std::string cublas_err(cublasStatus_t stat, const char* what) {
    std::ostringstream oss;
    oss << what << ": cublas status " << static_cast<int>(stat);
    return oss.str();
}

#define CHECK_CUDA(expr)                                                                  \
    do {                                                                                  \
        cudaError_t _err = (expr);                                                        \
        if (_err != cudaSuccess) {                                                        \
            set_error(cuda_err(_err, #expr));                                             \
            return -1;                                                                    \
        }                                                                                 \
    } while (0)

#define CHECK_CUBLAS(expr)                                                                \
    do {                                                                                  \
        cublasStatus_t _stat = (expr);                                                    \
        if (_stat != CUBLAS_STATUS_SUCCESS) {                                             \
            set_error(cublas_err(_stat, #expr));                                          \
            return -2;                                                                    \
        }                                                                                 \
    } while (0)

enum class StorageKind {
    F32,
    F16,
};

struct tca_plan {
    int n_windows = 0;
    int window = 0;
    tca_precision precision = TCA_PRECISION_TF32_FAST;
    StorageKind storage = StorageKind::F32;

    void* d_weights = nullptr;
    size_t weight_bytes = 0;

    cublasLtHandle_t lt = nullptr;
    void* workspace = nullptr;
    size_t workspace_bytes = 32ull * 1024ull * 1024ull;
};

template <typename T>
__global__ void pack_windows_kernel(const float* __restrict__ prices,
                                    T* __restrict__ out,
                                    int row_start,
                                    int rows,
                                    int window) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = rows * window;
    if (idx >= total) {
        return;
    }
    int r = idx / window;
    int c = idx - r * window;
    float v = prices[row_start + r + c];
    if constexpr (std::is_same<T, float>::value) {
        out[idx] = v;
    } else {
        out[idx] = __float2half_rn(v);
    }
}

int create_layout(cudaDataType_t type, uint64_t rows, uint64_t cols, uint64_t ld, cublasLtMatrixLayout_t* out) {
    cublasLtOrder_t order = CUBLASLT_ORDER_ROW;
    cublasStatus_t stat = cublasLtMatrixLayoutCreate(out, type, rows, cols, ld);
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatrixLayoutCreate"));
        return -2;
    }
    stat = cublasLtMatrixLayoutSetAttribute(*out, CUBLASLT_MATRIX_LAYOUT_ORDER, &order, sizeof(order));
    if (stat != CUBLAS_STATUS_SUCCESS) {
        cublasLtMatrixLayoutDestroy(*out);
        *out = nullptr;
        set_error(cublas_err(stat, "cublasLtMatrixLayoutSetAttribute(order=row)"));
        return -2;
    }
    return 0;
}

int run_matmul(tca_plan* plan,
               const void* d_a,
               int rows_this,
               float* d_out,
               int row_offset,
               cudaStream_t stream) {
    const cudaDataType_t ab_type = (plan->storage == StorageKind::F32) ? CUDA_R_32F : CUDA_R_16F;
    const cublasComputeType_t compute_type =
        (plan->precision == TCA_PRECISION_TF32_FAST) ? CUBLAS_COMPUTE_32F_FAST_TF32 : CUBLAS_COMPUTE_32F;

    cublasLtMatmulDesc_t op_desc = nullptr;
    cublasLtMatrixLayout_t a_desc = nullptr;
    cublasLtMatrixLayout_t b_desc = nullptr;
    cublasLtMatrixLayout_t c_desc = nullptr;
    cublasLtMatmulPreference_t pref = nullptr;

    float alpha = 1.0f;
    float beta = 0.0f;
    cublasOperation_t trans = CUBLAS_OP_N;
    cublasStatus_t stat = cublasLtMatmulDescCreate(&op_desc, compute_type, CUDA_R_32F);
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatmulDescCreate"));
        return -2;
    }
    stat = cublasLtMatmulDescSetAttribute(op_desc, CUBLASLT_MATMUL_DESC_TRANSA, &trans, sizeof(trans));
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatmulDescSetAttribute(TRANSA)"));
        goto fail;
    }
    stat = cublasLtMatmulDescSetAttribute(op_desc, CUBLASLT_MATMUL_DESC_TRANSB, &trans, sizeof(trans));
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatmulDescSetAttribute(TRANSB)"));
        goto fail;
    }

    if (create_layout(ab_type, static_cast<uint64_t>(rows_this), static_cast<uint64_t>(plan->window), static_cast<uint64_t>(plan->window), &a_desc) != 0) {
        goto fail;
    }
    if (create_layout(ab_type, static_cast<uint64_t>(plan->window), static_cast<uint64_t>(plan->n_windows), static_cast<uint64_t>(plan->n_windows), &b_desc) != 0) {
        goto fail;
    }
    if (create_layout(CUDA_R_32F, static_cast<uint64_t>(rows_this), static_cast<uint64_t>(plan->n_windows), static_cast<uint64_t>(plan->n_windows), &c_desc) != 0) {
        goto fail;
    }

    stat = cublasLtMatmulPreferenceCreate(&pref);
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatmulPreferenceCreate"));
        goto fail;
    }
    stat = cublasLtMatmulPreferenceSetAttribute(
        pref,
        CUBLASLT_MATMUL_PREF_MAX_WORKSPACE_BYTES,
        &plan->workspace_bytes,
        sizeof(plan->workspace_bytes));
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatmulPreferenceSetAttribute(MAX_WORKSPACE_BYTES)"));
        goto fail;
    }

    cublasLtMatmulHeuristicResult_t heuristic{};
    int returned = 0;
    stat = cublasLtMatmulAlgoGetHeuristic(
        plan->lt,
        op_desc,
        a_desc,
        b_desc,
        c_desc,
        c_desc,
        pref,
        1,
        &heuristic,
        &returned);
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatmulAlgoGetHeuristic"));
        goto fail;
    }
    if (returned <= 0) {
        set_error("cublasLtMatmulAlgoGetHeuristic returned no algorithm");
        goto fail;
    }

    float* out_ptr = d_out + static_cast<size_t>(row_offset) * static_cast<size_t>(plan->n_windows);
    stat = cublasLtMatmul(
        plan->lt,
        op_desc,
        &alpha,
        d_a,
        a_desc,
        plan->d_weights,
        b_desc,
        &beta,
        out_ptr,
        c_desc,
        out_ptr,
        c_desc,
        &heuristic.algo,
        plan->workspace,
        plan->workspace_bytes,
        stream);
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtMatmul"));
        goto fail;
    }

    cublasLtMatmulPreferenceDestroy(pref);
    cublasLtMatrixLayoutDestroy(c_desc);
    cublasLtMatrixLayoutDestroy(b_desc);
    cublasLtMatrixLayoutDestroy(a_desc);
    cublasLtMatmulDescDestroy(op_desc);
    return 0;

fail:
    if (pref) cublasLtMatmulPreferenceDestroy(pref);
    if (c_desc) cublasLtMatrixLayoutDestroy(c_desc);
    if (b_desc) cublasLtMatrixLayoutDestroy(b_desc);
    if (a_desc) cublasLtMatrixLayoutDestroy(a_desc);
    if (op_desc) cublasLtMatmulDescDestroy(op_desc);
    return -2;
}

}  // namespace

extern "C" const char* tca_last_error(void) {
    return g_last_error.c_str();
}

extern "C" int tca_create_plan(const float* host_weights_t,
                                 int n_windows,
                                 int window,
                                 int precision,
                                 tca_plan** out_plan) {
    if (!host_weights_t || !out_plan || n_windows <= 0 || window <= 0) {
        set_error("invalid arguments to tca_create_plan");
        return -3;
    }

    auto* plan = new tca_plan();
    plan->n_windows = n_windows;
    plan->window = window;
    plan->precision = static_cast<tca_precision>(precision);
    plan->storage = (plan->precision == TCA_PRECISION_F16) ? StorageKind::F16 : StorageKind::F32;

    cublasStatus_t stat = cublasLtCreate(&plan->lt);
    if (stat != CUBLAS_STATUS_SUCCESS) {
        set_error(cublas_err(stat, "cublasLtCreate"));
        delete plan;
        return -2;
    }
    cudaError_t cerr = cudaMalloc(&plan->workspace, plan->workspace_bytes);
    if (cerr != cudaSuccess) {
        set_error(cuda_err(cerr, "cudaMalloc(workspace)"));
        tca_destroy_plan(plan);
        return -1;
    }

    const size_t elems = static_cast<size_t>(window) * static_cast<size_t>(n_windows);
    if (plan->storage == StorageKind::F32) {
        plan->weight_bytes = elems * sizeof(float);
        cerr = cudaMalloc(&plan->d_weights, plan->weight_bytes);
        if (cerr != cudaSuccess) {
            set_error(cuda_err(cerr, "cudaMalloc(weights)"));
            tca_destroy_plan(plan);
            return -1;
        }
        cerr = cudaMemcpy(plan->d_weights, host_weights_t, plan->weight_bytes, cudaMemcpyHostToDevice);
        if (cerr != cudaSuccess) {
            set_error(cuda_err(cerr, "cudaMemcpy(weights_f32)"));
            tca_destroy_plan(plan);
            return -1;
        }
    } else {
        std::vector<half> tmp(elems);
        for (size_t i = 0; i < elems; ++i) {
            tmp[i] = __float2half_rn(host_weights_t[i]);
        }
        plan->weight_bytes = elems * sizeof(half);
        cerr = cudaMalloc(&plan->d_weights, plan->weight_bytes);
        if (cerr != cudaSuccess) {
            set_error(cuda_err(cerr, "cudaMalloc(weights_f16)"));
            tca_destroy_plan(plan);
            return -1;
        }
        cerr = cudaMemcpy(plan->d_weights, tmp.data(), plan->weight_bytes, cudaMemcpyHostToDevice);
        if (cerr != cudaSuccess) {
            set_error(cuda_err(cerr, "cudaMemcpy(weights_f16)"));
            tca_destroy_plan(plan);
            return -1;
        }
    }

    *out_plan = plan;
    return 0;
}

extern "C" int tca_destroy_plan(tca_plan* plan) {
    if (!plan) {
        return 0;
    }
    if (plan->d_weights) {
        cudaFree(plan->d_weights);
    }
    if (plan->workspace) {
        cudaFree(plan->workspace);
    }
    if (plan->lt) {
        cublasLtDestroy(plan->lt);
    }
    delete plan;
    return 0;
}

extern "C" int tca_run_plan(tca_plan* plan,
                              const float* device_prices,
                              int n_prices,
                              int chunk_rows,
                              float* device_out,
                              int* out_rows,
                              void* cuda_stream) {
    if (!plan || !device_prices || !device_out || !out_rows) {
        set_error("invalid arguments to tca_run_plan");
        return -3;
    }
    if (n_prices < plan->window) {
        set_error("n_prices is smaller than plan window");
        return -3;
    }

    const int rows = n_prices - plan->window + 1;
    *out_rows = rows;
    if (chunk_rows <= 0) {
        chunk_rows = 8192;
    }
    chunk_rows = std::max(16, chunk_rows);

    cudaStream_t stream = reinterpret_cast<cudaStream_t>(cuda_stream);
    const size_t scratch_elems = static_cast<size_t>(chunk_rows) * static_cast<size_t>(plan->window);
    void* d_packed = nullptr;
    if (plan->storage == StorageKind::F32) {
        CHECK_CUDA(cudaMalloc(&d_packed, scratch_elems * sizeof(float)));
    } else {
        CHECK_CUDA(cudaMalloc(&d_packed, scratch_elems * sizeof(half)));
    }

    const int threads = 256;
    for (int row_start = 0; row_start < rows; row_start += chunk_rows) {
        const int rows_this = std::min(chunk_rows, rows - row_start);
        const int total = rows_this * plan->window;
        const int blocks = (total + threads - 1) / threads;

        if (plan->storage == StorageKind::F32) {
            pack_windows_kernel<float><<<blocks, threads, 0, stream>>>(
                device_prices,
                static_cast<float*>(d_packed),
                row_start,
                rows_this,
                plan->window);
        } else {
            pack_windows_kernel<half><<<blocks, threads, 0, stream>>>(
                device_prices,
                static_cast<half*>(d_packed),
                row_start,
                rows_this,
                plan->window);
        }
        CHECK_CUDA(cudaGetLastError());

        int rc = run_matmul(plan, d_packed, rows_this, device_out, row_start, stream);
        if (rc != 0) {
            cudaFree(d_packed);
            return rc;
        }
    }

    CHECK_CUDA(cudaFree(d_packed));
    return 0;
}

extern "C" int tca_malloc_f32(size_t count, float** out_ptr) {
    if (!out_ptr) {
        set_error("invalid out_ptr in tca_malloc_f32");
        return -3;
    }
    CHECK_CUDA(cudaMalloc(reinterpret_cast<void**>(out_ptr), count * sizeof(float)));
    CHECK_CUDA(cudaMemset(*out_ptr, 0, count * sizeof(float)));
    return 0;
}

extern "C" int tca_free_ptr(void* ptr) {
    if (!ptr) {
        return 0;
    }
    CHECK_CUDA(cudaFree(ptr));
    return 0;
}

extern "C" int tca_memcpy_h2d_f32(float* dst, const float* src, size_t count) {
    if (!dst || !src) {
        set_error("invalid pointer in tca_memcpy_h2d_f32");
        return -3;
    }
    CHECK_CUDA(cudaMemcpy(dst, src, count * sizeof(float), cudaMemcpyHostToDevice));
    return 0;
}

extern "C" int tca_memcpy_d2h_f32(float* dst, const float* src, size_t count) {
    if (!dst || !src) {
        set_error("invalid pointer in tca_memcpy_d2h_f32");
        return -3;
    }
    CHECK_CUDA(cudaMemcpy(dst, src, count * sizeof(float), cudaMemcpyDeviceToHost));
    return 0;
}

extern "C" int tca_device_synchronize(void) {
    CHECK_CUDA(cudaDeviceSynchronize());
    return 0;
}
