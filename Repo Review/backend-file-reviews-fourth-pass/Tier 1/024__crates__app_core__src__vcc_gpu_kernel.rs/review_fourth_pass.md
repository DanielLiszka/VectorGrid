# Fourth-Pass Review: crates/app_core/src/vcc_gpu_kernel.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/vcc_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vcc_gpu_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcc_gpu_kernel.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcc_gpu_kernel.rs.review.md`
- Third-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Fourth-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally correct on the bucket, but it was too generic for what the live file actually is.

`vcc_gpu_kernel.rs` is not just an internal scratch wrapper. In CUDA builds it is exported as a public module from `crates/app_core/src/lib.rs`, and it exposes public structs and entrypoints that accept raw `u64` device pointers and row-pointer arrays (`crates/app_core/src/lib.rs:78-80`; `crates/app_core/src/vcc_gpu_kernel.rs:28-56`, `950-982`). That makes its compatibility and GPU-ABI surface more concrete than the earlier packet suggested.

The live source also carries more contract detail than “ownership and lifetime” alone: it has two input modes with explicit precedence rules, a resident close/return cache keyed from only part of the caller-provided state, and a detailed path that deliberately keeps a separate `f64` return buffer alive alongside the main `f32` route (`crates/app_core/src/vcc_gpu_kernel.rs:432-450`, `544-665`, `903-945`). Those are the details the fourth pass needed to sharpen.

## Fourth-Pass Research Log

I re-read the live source and the third-pass packet first, then checked the earlier packets only to see where the framing drifted. I used the local repo guidance only where it changed the recommendation boundary:

- `crates/app_core/src/lib.rs` to confirm whether this module is actually public in CUDA builds (`crates/app_core/src/lib.rs:78-80`).
- `AGENTS.md` to re-check the repo’s bias toward strategy-local GPU code and measured verification before refactoring (`AGENTS.md:44-49`, `82-96`).
- `crates/app_core/src/gpu_backtest_kernel.rs` to compare this file’s raw-pointer input ABI with nearby typed device-slice patterns that carry length/device ownership information (`crates/app_core/src/gpu_backtest_kernel.rs:2191-2233`, `2312-2338`).

I also rechecked primary external references on 2026-04-15:

- NVIDIA CUDA C++ Programming Guide (docs.nvidia.com): for stream launch semantics and the fact that wrapper-level launch/order assumptions are real runtime contracts, not stylistic details.
- NVIDIA CUDA C++ Best Practices Guide (docs.nvidia.com): for the general constraint that transfer/layout/launch choices in hot CUDA paths are performance-sensitive and should not be “cleaned up” casually.
- Rust standard-library `thread_local!` docs (doc.rust-lang.org): to keep the cache-scope discussion accurate; this runtime is per-thread, not process-global.
- `cust` docs/source on device slices and raw pointers (docs.rs): to ground the pointer-lifetime critique in the actual Rust CUDA wrapper surface, especially the requirement that device pointers remain valid for the lifetime of downstream use.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**.

This file still belongs in `Benchmark-sensitive; do not touch yet` because the live implementation couples together all of the things that are easy to regress in a CUDA wrapper: per-thread runtime caching, resident device buffers, raw-pointer kernel arguments, reused scratch allocations, and downstream metric decode/report semantics (`crates/app_core/src/vcc_gpu_kernel.rs:65-91`, `143-188`, `428-463`, `748-945`).

### What the third-pass packet got right

The third-pass packet was right about three things.

First, it correctly resisted cleanup-driven refactors. The live code is absolutely hot-path runtime code, and the repack/upload/cache behavior is not accidental.

Second, it correctly kept strategy-locality in place. Repo guidance still says strategy-specific GPU code should stay strategy-local unless a shared abstraction is truly cross-cutting (`AGENTS.md:82-96`; `REPO_MAP.md` strategy-local owners section).

Third, it correctly treated thread-local runtime state as part of the contract. `VCC_GPU_RUNTIME` is thread-local, and the cache story here is therefore “per device, per thread,” not some stronger global guarantee (`crates/app_core/src/vcc_gpu_kernel.rs:187-189`).

### What the third-pass packet missed or underweighted

#### 1. The public surface is stronger, and riskier, than the earlier packets implied

The first-pass packet called the module “effectively yes” on public surface. The live code is stronger than that: the module is publicly exported in CUDA builds, and it exposes public input/output structs and public entrypoints (`crates/app_core/src/lib.rs:78-80`; `crates/app_core/src/vcc_gpu_kernel.rs:28-56`, `950-982`).

That matters because the raw-pointer fields in `VccGpuDeviceSignalMatrices` are not just internal implementation detail. They are part of a feature-gated public Rust API and an implicit GPU ABI boundary:

- `close_device_ptr: u64`
- `fast_squeeze_rows: &'a [u64]`
- `fast_momentum_rows: &'a [u64]`
- `trend_rows_ptr: &'a [u64]`

The biggest risk here is not the presence of `unsafe` blocks by itself. It is that pointer provenance, same-device ownership, and keepalive lifetime are largely supplied by the caller and cannot be fully validated locally.

#### 2. The real contract is not just “ownership and lifetime”; it is also precedence and coherence

The third-pass packet flattened the file into “ownership/lifetime discipline.” The live source is stricter and more specific than that.

There are at least three precedence/coherence rules that the packet did not spell out:

- If host row-major data is present, it wins over the row-pointer arrays for that signal family. The validator and the upload path both follow `if let Some(host_rows) { ... } else { row-pointer path }` (`crates/app_core/src/vcc_gpu_kernel.rs:246-277`, `544-665`).
- If `close_device_ptr != 0`, the runtime trusts that pointer and does **not** upload `close_f32`; otherwise it uploads host `close_f32` and owns the resulting `DeviceBuffer` (`crates/app_core/src/vcc_gpu_kernel.rs:439-445`).
- The resident cache key is built from `close_f32` plus `close_device_ptr`, but the cached `d_lr64` buffer is built from `close_f64` (`crates/app_core/src/vcc_gpu_kernel.rs:432-450`). So the detailed path relies on the caller keeping `close_f64`, `close_f32`, and `close_device_ptr` coherent as one logical price series.

That last point is especially important. I am not calling it a proven bug in the current repo, but it is a real hidden invariant the third-pass packet missed.

#### 3. “Upload-signature caching” is only part of the story

The earlier packets repeatedly described signature-based upload skipping as central behavior. That is true for host-backed row-major matrices and combo vectors (`crates/app_core/src/vcc_gpu_kernel.rs:212-229`, `544-556`, `585-598`, `626-639`, `667-730`).

But it is incomplete for the device-row-pointer mode.

When callers provide device row pointers instead of host row-major matrices, this file only signature-caches the **list of row pointers**. It still launches `pack_rows_u64_ptr_f32` every call to repack the pointed-to device rows into contiguous scratch (`crates/app_core/src/vcc_gpu_kernel.rs:560-581`, `601-623`, `642-664`). That is a crucial distinction.

Any recommendation to “extend caching” here is dangerous unless it first proves that row-pointer identity implies row-content immutability. The current code does **not** assume that, and that restraint looks correct.

#### 4. Test coverage is much thinner than the current packet suggests

The earlier reviews noted that file-local tests exist, but they underweighted what those tests actually cover.

The current test module is entirely host-side. It exercises:

- input validation failures
- `checked_i32` overflow rejection
- host `compute_log_returns_f64`

It does **not** exercise:

- a positive CUDA launch path
- scratch reuse across repeated invocations
- host-row-major versus row-pointer equivalence
- the `Detailed` path and `d_slice_out` readback
- resident cache refresh behavior

That gap is visible directly in the test block (`crates/app_core/src/vcc_gpu_kernel.rs:985-1135`). For a Tier 1 file, that missing positive-path coverage is more important than the third-pass packet let on.

#### 5. The metrics-only path still carries detailed-path baggage

`eval_vcc_metrics_gpu_kernel_device_rows_impl` always sizes `d_slice_out`, always passes `slice_out_ptr` into the kernel, and always maintains the resident `d_lr64` buffer, even when the caller requested `MetricsOnly` and no slice readback occurs (`crates/app_core/src/vcc_gpu_kernel.rs:489-496`, `541-542`, `828-878`, `903-917`).

I am **not** recommending changing that now. It is exactly the sort of kernel-ABI and benchmark-sensitive simplification that should stay deferred. But the third-pass packet missed that the file also owns a deliberate metrics-only versus detailed tradeoff, not just cache/lifetime discipline.

### What the current packet overstated or stated too weakly

The main overstatement is in the third-pass implementation advice: “Add or tighten ownership/lifetime assertions before refactoring.” That is easy to say, but the file’s hardest ownership contract sits in public raw `u64` device pointers. Without changing the public API shape, this module cannot cheaply or robustly assert same-device provenance or pointed-to lifetime for those inputs. The safer near-term work is documentation and test coverage, not pretending that a few local assertions materially solve the problem.

The main weak statement is the third-pass verification guidance. Its first command was:

`cargo test -p ta_app_core vcc_gpu_kernel -- --nocapture`

That is incomplete for this file. The module itself is gated by `#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]` (`crates/app_core/src/vcc_gpu_kernel.rs:1`; `crates/app_core/src/lib.rs:79-80`). Without the CUDA features, that command is not a reliable verification instruction for this source at all.

The packet also undercalled the public API risk. The file is not just a local wrapper where comments around `unsafe` will do most of the work. It is a feature-gated public module whose caller-facing structs encode raw device-pointer conventions.

## Implementation Priority

### Safe-now hardening

1. **Document the caller contract on the public input structs and entrypoints.**
   Add targeted doc comments on `VccGpuDeviceSignalMatrices`, `VccGpuComboBatch`, `eval_vcc_metrics_gpu_kernel_device_rows`, and `eval_vcc_metrics_gpu_kernel_device_rows_detailed` that make four rules explicit:
   - all raw device pointers must belong to the selected CUDA device/context and stay alive until the stream completes;
   - host row-major inputs take precedence over row-pointer arrays when both are present;
   - non-zero `close_device_ptr` takes precedence over host close upload;
   - `close_f64`, `close_f32`, and `close_device_ptr` must describe the same logical close series if cache reuse is to stay correct.

   This is safe now because it is commentary only. It strengthens the real contract without perturbing hot-path behavior.

2. **Add host-only unit tests that lock the hidden API rules in place.**
   The file already has pure host tests. Extend them with tests that lock down current semantics without requiring a GPU:
   - a precedence test showing that valid host row-major input is accepted even if the corresponding row-pointer slice is empty or otherwise unusable;
   - a bit-pattern test for `f32_upload_signature`, so future cleanup does not silently weaken the current bitwise cache semantics around `+0.0/-0.0` or NaN payloads.

   This is safe now because it changes only tests, not runtime behavior.

3. **Add an in-code note explaining why the row-pointer mode repacks every call.**
   The `pack_rows_u64_ptr_f32` launch in the row-pointer branches is easy for a future reviewer to “optimize away.” A short comment there would be unusually valuable because it explains a benchmark-sensitive non-optimization: pointer-list stability is not enough to prove pointed-to row contents are unchanged.

   This is safe now because it documents an existing design choice instead of changing it.

### Larger redesign still deferred

1. **Replace raw `u64` device pointers with typed device-row descriptors.**
   The nearby `gpu_backtest_kernel.rs` context already shows stronger typed patterns that carry device id, length, and keepalive ownership. A future redesign could move VCC toward a similar shape. That would materially improve local validation.

   It should stay deferred because it is a public API change, it will touch callers, and it may force broader runtime plumbing changes.

2. **Split the metrics-only and detailed paths at the kernel/output-buffer level.**
   Right now metrics-only still pays for `d_slice_out` plumbing and `d_lr64` residency. A future redesign could separate those contracts so metrics-only can stay narrower.

   That should stay deferred because it changes kernel ABI assumptions, launch arguments, and potentially benchmark posture.

3. **Rework the resident-series cache key so detailed-mode provenance is explicit.**
   A future redesign could either key `d_lr64` with explicit `close_f64` provenance or collapse the API toward a single authoritative close source.

   That should stay deferred because it can change cache-hit behavior, host hashing/copy costs, and detailed-path semantics in ways that require parity and performance evidence.

## Verification If Changed

The third-pass packet’s verification section was too weak for this file because it dropped the required feature flags.

For any actual code change here, the minimum credible local verification set is:

1. `cargo test -p ta_app_core vcc_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
2. `cargo test -p ta_app_core composite_validate --features cuda,cuda-backtest-kernel -- --nocapture`
3. one representative VCC CUDA path that exercises both public entrypoints if available in the local harness, especially if the change touches cache keys, detailed-mode output, or input-shape validation
4. `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

If the change is strictly comments or host-only tests, the perf risk is lower, but the repo guidance still treats CUDA backtest-kernel work as performance-sensitive enough to justify the GPU perf gate when hardware is available (`AGENTS.md:44-49`).

## Regression And Feature-Preservation Constraints

- Preserve the current third-pass bucket. The safe-now work here is contract hardening, not behavior cleanup.
- Preserve the feature-gated public Rust API unless an explicit compatibility break is being taken.
- Preserve the precedence rules already encoded in the source: host row-major beats row-pointer slices; non-zero `close_device_ptr` beats host close upload (`crates/app_core/src/vcc_gpu_kernel.rs:246-277`, `439-445`, `544-665`).
- Preserve bitwise `f32` upload-signature behavior. Replacing it with approximate or semantic float comparison would be a silent cache-contract change (`crates/app_core/src/vcc_gpu_kernel.rs:207-229`).
- Preserve the current row-pointer repack-on-every-call behavior unless a stronger immutability contract is added. Caching packed matrices from pointer identity alone is not justified by the current API (`crates/app_core/src/vcc_gpu_kernel.rs:560-581`, `601-623`, `642-664`).
- Preserve the dual-precision split between the main `f32` metrics route and the detailed `f64` slice-return path unless the detailed contract is being deliberately redesigned (`crates/app_core/src/vcc_gpu_kernel.rs:432-450`, `489-496`, `903-945`).
- Preserve strategy-local ownership. `AGENTS.md` explicitly pushes against premature genericization of strategy-specific GPU code (`AGENTS.md:82-96`).

There is no meaningful IPC or persistence surface visible in this file. The real regression axes are performance, public compatibility, GPU ABI, and downstream validation/report semantics.

## Fourth-Pass Conclusion

The third-pass bucket should **remain unchanged**: `Benchmark-sensitive; do not touch yet`.

What the third-pass packet got right was the broad caution: this is strategy-local CUDA runtime code, and casual cleanup would be risky. What it missed was the shape of the live contract. The actual sharp edges are not just generic “ownership and lifetime” concerns. They are:

- a feature-gated **public** module surface,
- raw `u64` device-pointer inputs that the file cannot fully validate,
- hidden precedence/coherence rules across host row-major input, row-pointer input, and resident close-return caching,
- a row-pointer path that repacks every call by design,
- and tests that do not currently exercise the positive CUDA path.

That combination is exactly why the bucket still stands. The safe-now work is narrow and boring on purpose: document the caller contract better, add host-only tests that lock the hidden semantics in place, and avoid implying that local assertions or superficial refactors would make this file materially safer. The larger redesign ideas are real, but they are still redesigns, and they should stay deferred until there is explicit parity and benchmark evidence.
