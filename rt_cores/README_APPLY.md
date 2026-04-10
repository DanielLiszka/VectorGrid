# RT-core exit-scan patch bundle

This bundle adds one practical RT-core feature to the GPU backtest project:

**RT-core-accelerated earliest barrier-hit search** for batched stop-loss / take-profit queries.

It is designed for the project's **validation / scenario / Monte Carlo lane**, where you often need to evaluate huge batches of candidate exits against the same OHLC history. The implementation builds one reusable OptiX GAS over candle-range prisms and launches one ray per query to find the earliest future bar whose `[low, high]` range contains the barrier price.

## Why this fits the project

- It uses **RT cores for BVH traversal + triangle intersection**, which is the part RTX hardware accelerates best when you stick to OptiX built-in triangle geometry.
- It does **not** require rewriting the existing CUDA backtest kernel.
- It slots naturally beside the existing GPU-native backtest / validation architecture.
- It is especially useful when you run **many queries against the same history**, which amortizes BVH build cost.

## Files in this bundle

### New files

- `new_files/crates/app_core/src/rtcore_exit_scan.rs`
- `new_files/crates/app_core/native/rtcore_barrier/CMakeLists.txt`
- `new_files/crates/app_core/native/rtcore_barrier/ptx_path.h.in`
- `new_files/crates/app_core/native/rtcore_barrier/rtcore_barrier_api.h`
- `new_files/crates/app_core/native/rtcore_barrier/rtcore_barrier_device.cu`
- `new_files/crates/app_core/native/rtcore_barrier/rtcore_barrier_host.cpp`
- `new_files/src-tauri/src/bin/ta_rtcore_exit_scan_bench.rs`

### Small edits to existing files

- `patch_snippets/build_rs_rtcore_addendum.rs`
- `patch_snippets/lib_rs_addendum.rs`
- `patch_snippets/integration_snippet_gpu_backtest_kernel.rs`

## Apply order

1. Copy the new native files into `crates/app_core/native/rtcore_barrier/`.
2. Copy `rtcore_exit_scan.rs` into `crates/app_core/src/`.
3. Export the module from `crates/app_core/src/lib.rs` using `patch_snippets/lib_rs_addendum.rs`.
4. Add the `build_rtcore_barrier_if_requested()` function from `patch_snippets/build_rs_rtcore_addendum.rs` into the existing `crates/app_core/build.rs`, then call it from `main()`.
5. Optionally add the benchmark binary to `src-tauri/src/bin/ta_rtcore_exit_scan_bench.rs`.
6. Optionally wire the helper into the validation / scenario path using `patch_snippets/integration_snippet_gpu_backtest_kernel.rs`.

## Build-time requirements

- NVIDIA RTX-class GPU
- CUDA toolkit
- OptiX SDK headers available via `OPTIX_HOME`
- `cmake` present on PATH

## How to build it

Set:

```bash
export OPTIX_HOME=/path/to/NVIDIA-OptiX-SDK
export VECTORBT_ENABLE_RTCORE_BARRIER=1
cargo build
```

If `OPTIX_HOME` is not set, the rest of the Rust project still compiles, but the RT-core helper stays disabled.

## How to run the benchmark

```bash
export VECTORBT_RTCORE_BARRIER=1
cargo run --bin ta_rtcore_exit_scan_bench
```

Optional knobs:

```bash
RTCORE_BARS=200000 RTCORE_QUERIES=1000000 cargo run --bin ta_rtcore_exit_scan_bench
```

## Practical semantics

This implementation returns the earliest future bar where the barrier price lies inside the candle's `[low, high]` range.

That means:

- It is a **broad-phase / earliest-hit** accelerator.
- It works very well for batched stop/target lookups.
- If both stop and target land in the **same bar**, you should still use your existing intrabar tie-break rule.

## Best place to cache it

The reusable object is `RtCoreBarrierScene` in `rtcore_exit_scan.rs`.

A good production integration is to cache one scene per retained dataset / resample signature, alongside the existing GPU-resident market-series caches in your app state. That way you pay the OptiX GAS build once per active dataset and reuse it across scenario batches.

## Notes

- The code is written to be as drop-in as possible given only the linked repo structure, but I could not compile it against the private working tree here.
- The native path uses **built-in triangles**, intentionally, because that is the reliable RT-core-accelerated path across RTX generations.
