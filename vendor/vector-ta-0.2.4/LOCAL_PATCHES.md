# Local Patches

This vendored `vector-ta` package is based on the `vector-ta 0.2.4` crate
metadata in `Cargo.toml.orig` and upstream git revision
`59dc31af77632c5d0ce0b9c89821296e98a5cdb5`, as recorded in
`.cargo_vcs_info.json`.

Most differences from the published crate archive are line-ending changes. The
repository carries one substantive local patch: a VWMA CUDA time-major batch
path used by VectorGrid's resident GPU moving-average runtime.

## VWMA CUDA Time-Major Batch Patch

Reviewed local patch files:

- `kernels/cuda/moving_averages/vwma_kernel.cu`
- `kernels/ptx/compute_89/vwma_kernel.ptx`
- `kernels/cubin/sm_89/vwma_kernel_sm89.cubin`
- `src/cuda/moving_averages/vram_ma.rs`
- `src/cuda/moving_averages/vwma_wrapper.rs`

Patch summary:

- Adds `vwma_batch_tm_f32` to produce time-major VWMA output on the device.
- Adds `CudaVwma::vwma_batch_device_tm` host-side launch and chunking support.
- Routes `vram_ma` VWMA time-major requests through the device output path.
- Regenerates the `compute_89` PTX and `sm_89` CUBIN artifacts for the patched
  kernel.

Release requirements:

- Preserve the upstream Apache-2.0 license, `NOTICE`, and `COPYRIGHT` files.
- Treat the five files above as local first-party modifications during review.
- Regenerate PTX/CUBIN with the CUDA/NVCC toolchain before changing the CUDA
  source, and record the verification command in the repository review tracker.
- Do not describe this vendored subtree as byte-identical to the published
  `vector-ta 0.2.4` crate while this patch is present.
