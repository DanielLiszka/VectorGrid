#include "prism_common.cuh"
#include <mma.h>

using namespace nvcuda;
using prism::PrismScreenModelView;

extern "C" __global__ void prism_encode_latent_features_kernel(
    const float* __restrict__ latent,      // [batch, latent_dim]
    PrismScreenModelView model,
    half* __restrict__ phi_out             // [batch, phi_dim_padded]
) {
    const int batch_idx = blockIdx.x;
    const int tid = threadIdx.x;
    if (batch_idx >= blockDim.y * gridDim.x) {
        return;
    }

    const float* u = latent + batch_idx * model.latent_dim;
    half* phi = phi_out + batch_idx * model.phi_dim_padded;

    for (int j = tid; j < model.phi_dim_padded; j += blockDim.x) {
        phi[j] = __float2half(0.0f);
    }
    __syncthreads();

    if (tid == 0) {
        int c = 0;
        phi[c++] = __float2half(1.0f);
        for (int j = 0; j < model.latent_dim; ++j) phi[c++] = __float2half(u[j]);
        for (int j = 0; j < model.latent_dim; ++j) phi[c++] = __float2half(u[j] * u[j]);
        for (int j = 0; j < model.latent_dim; ++j) phi[c++] = __float2half(__sinf(3.14159265358979323846f * u[j]));

        for (int g = 0; g < model.group_count; ++g) {
            const int start = model.group_offsets[g];
            const int end = model.group_offsets[g + 1];
            float mean = 0.0f;
            for (int k = start; k < end; ++k) {
                mean += u[model.group_indices[k]];
            }
            const int count = max(1, end - start);
            mean /= (float)count;
            float var = 0.0f;
            for (int k = start; k < end; ++k) {
                float d = u[model.group_indices[k]] - mean;
                var += d * d;
            }
            var = sqrtf(var / (float)count);
            phi[c++] = __float2half(mean);
            phi[c++] = __float2half(var);
        }
    }
}

#if (__CUDA_ARCH__ >= 700)
extern "C" __global__ void prism_tc_linear_predict_kernel(
    const half* __restrict__ phi,          // [batch, phi_dim_padded]
    PrismScreenModelView model,
    float* __restrict__ out_mean           // [batch, out_cols_padded]
) {
    const int warp_id = (threadIdx.x / warpSize);
    const int lane = threadIdx.x % warpSize;
    const int warps_per_block = blockDim.x / warpSize;
    const int batch_tile = blockIdx.x * warps_per_block + warp_id;
    if (batch_tile * 16 >= gridDim.y * 16) {
        return;
    }

    constexpr int M = 16;
    constexpr int N = 16;
    constexpr int K = 16;

    const int row0 = batch_tile * M;
    const int phi_rows = gridDim.y * M;
    if (row0 >= phi_rows) return;

    wmma::fragment<wmma::matrix_a, M, N, K, half, wmma::row_major> a_frag;
    wmma::fragment<wmma::matrix_b, M, N, K, half, wmma::row_major> b_frag;
    wmma::fragment<wmma::accumulator, M, N, K, float> c_frag;
    wmma::fill_fragment(c_frag, 0.0f);

    const int k_tiles = model.phi_dim_padded / K;
    for (int kt = 0; kt < k_tiles; ++kt) {
        const half* a_ptr = phi + row0 * model.phi_dim_padded + kt * K;
        const half* b_ptr = model.beta + (kt * K) * model.out_cols_padded;
        wmma::load_matrix_sync(a_frag, a_ptr, model.phi_dim_padded);
        wmma::load_matrix_sync(b_frag, b_ptr, model.out_cols_padded);
        wmma::mma_sync(c_frag, a_frag, b_frag, c_frag);
    }
    float* out_ptr = out_mean + row0 * model.out_cols_padded;
    wmma::store_matrix_sync(out_ptr, c_frag, model.out_cols_padded, wmma::mem_row_major);
}
#else
extern "C" __global__ void prism_tc_linear_predict_kernel(
    const half* __restrict__ phi,
    PrismScreenModelView model,
    float* __restrict__ out_mean
) {
    const int row = blockIdx.x;
    const int col = threadIdx.x;
    if (row >= gridDim.x || col >= model.out_cols_padded) return;
    float acc = 0.0f;
    for (int k = 0; k < model.phi_dim_padded; ++k) {
        acc += __half2float(phi[row * model.phi_dim_padded + k]) * __half2float(model.beta[k * model.out_cols_padded + col]);
    }
    out_mean[row * model.out_cols_padded + col] = acc;
}
#endif

extern "C" __global__ void prism_conservative_bounds_kernel(
    const float* __restrict__ mean,        // [batch, out_cols_padded]
    PrismScreenModelView model,
    float cell_radius,
    int batch,
    float* __restrict__ out_half,          // [batch, 2]
    float* __restrict__ out_optimistic,    // [batch, 2]
    float* __restrict__ out_pessimistic    // [batch, 2]
) {
    const int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= batch) return;
    const float h0 = max(model.half_width_base[0] + 0.20f * cell_radius, 1e-4f);
    const float h1 = max(model.half_width_base[1] + 0.20f * cell_radius, 1e-4f);
    const float m0 = mean[idx * model.out_cols_padded + 0];
    const float m1 = mean[idx * model.out_cols_padded + 1];
    out_half[idx * 2 + 0] = h0;
    out_half[idx * 2 + 1] = h1;
    out_optimistic[idx * 2 + 0] = fminf(1.0f, m0 + h0);
    out_optimistic[idx * 2 + 1] = fminf(1.0f, m1 + h1);
    out_pessimistic[idx * 2 + 0] = fmaxf(0.0f, m0 - h0);
    out_pessimistic[idx * 2 + 1] = fmaxf(0.0f, m1 - h1);
}
