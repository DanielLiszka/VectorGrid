# Fourth-Pass Review: `crates/app_core/src/cdmc_gpu_kernel.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/cdmc_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/cdmc_gpu_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cdmc_gpu_kernel.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cdmc_gpu_kernel.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet kept the bucket steady, but it flattened the file’s concrete risks into a generic “ownership and lifetime” framing. That is directionally correct, but too abstract for a public, feature-gated CUDA wrapper whose live source still exposes raw device-pointer inputs, manually mirrors a kernel ABI, mutates the current CUDA context on the calling thread, and relies on mixed host/device cache identity (`cdmc_gpu_kernel.rs:33-49`, `133-135`, `316-349`, `370-595`).

This fourth pass re-read the live source first, then checked the earlier packets only to see what had been lost in the simplification. It also reopened `crates/app_core/src/lib.rs` to confirm the public export and `crates/app_core/src/gpu_backtest_kernel.rs` to see whether the context-management pattern here is strategy-local or repo-wide. The result is that the third-pass bucket should **stand**, but for narrower reasons than the third-pass packet gave.

## Fourth-Pass Research Log

1. **Live source re-read of `cdmc_gpu_kernel.rs`.**
   The main focus was the public input types (`33-57`), the validation boundary (`171-269`), resident close/log-return reuse (`316-349`), and the launch/marshal block (`459-576`). That confirmed the real hardening opportunities are still contract clarity and targeted validation coverage, not algorithm reshaping.

2. **`crates/app_core/src/lib.rs` export check.**
   `cdmc_gpu_kernel` is still exported as `pub mod` behind `cuda` + `cuda-backtest-kernel`. That keeps visibility changes compatibility-sensitive and means even “small” signature hardening ideas are not automatically low-risk.

3. **`crates/app_core/src/gpu_backtest_kernel.rs` pattern check.**
   The shared runtime uses the same `Context::new(device)` plus `CurrentContext::set_current(...)` pattern (`gpu_backtest_kernel.rs:1840-1867`). That matters because the current-context side effect in this file is not an isolated quirk; it is consistent with the repo’s broader CUDA-runtime posture. That makes local documentation safer than a one-file behavior change.

4. **`cust::context::CurrentContext` and NVIDIA CUDA context documentation.**
   The external docs confirm that `set_current` sets the context for the current thread, and CUDA’s driver model ties current-context state to the calling host thread. That supports treating thread-context mutation as a real API side effect, not an implementation detail.

5. **`cust::memory::DeviceBuffer` and `cust::stream` documentation.**
   Those docs still support the first-pass conclusion that uninitialized device buffers and non-blocking stream launches are acceptable only because this wrapper preserves stream ordering and performs an explicit synchronization before host-side metric decode.

6. **Rust safe-abstraction guidance.**
   The Rust book and API-guideline material remain relevant here: safe wrappers around unsafe internals need their caller obligations made explicit. That is especially important because this file’s public surface is safe Rust while still depending on raw `u64` device-pointer and extent invariants that the type system does not encode.

## What Fourth Pass Sharpened

### The third-pass bucket should stand

`Small surgical refactor` is still the right action bucket. The live file does **not** justify a broader redesign on current evidence. Its strategy-local structure, per-device runtime reuse, resident close/log-return caching, combo upload reuse, and single-stream launch/synchronize flow all look intentional and consistent with the surrounding GPU runtime design. The third-pass instinct to avoid generic cleanup was right.

### What the current third-pass packet got right

The third-pass packet correctly resisted two tempting but regression-prone moves: broad cross-strategy abstraction and casual cache/upload churn. It also correctly recognized that the wrapper’s risk is contract-heavy and that performance-sensitive behavior is easier to break than the line count suggests.

### What it missed or underweighted

The live file’s problem is not just “ownership/lifetime discipline.” It is a more concrete public-contract surface:

First, the public API still exposes raw device-pointer contracts without any Rustdoc on the public structs or entry point (`33-57`, `370-376`). The third-pass packet did not carry forward the first-pass specificity here. The file still needs explicit caller-facing contract text for at least these invariants:

- every non-close pointer must reference valid device memory for `device_id`;
- `mama_ptr` and `fama_ptr` must each address at least `cols` `f32` elements;
- DPO / cycle / decycler matrices must match their declared rows and strides;
- `close_f32` and `close_device_ptr` must represent the same logical close series.

Second, the close-series contract is narrower than the third-pass packet suggested. `close_f32` is not merely fallback host data. `ensure_resident_close_lr` always builds cache identity from the host slice plus the optional device pointer (`320`), and when `close_device_ptr != 0` the host slice is still part of the cache story even though the kernel path uses the device pointer (`327-337`). That means the public contract is really **host/device pair coherence**, not just raw device ownership. If callers mutate the logical close series in place or swap what the device pointer means without changing the identity inputs the cache sees, the wrapper can reuse stale log returns. The first pass already identified the broader invalidation risk; the fourth pass sharpens that the immediate safe-now fix is documentation, not a cache rewrite.

Third, the current-context side effect is real and presently undocumented. `CdmcCudaRuntime::new` creates a context and `set_current` makes it current on the calling thread (`114-120`, `133-135`, `404`). Because this file never restores the prior context, callers do not get a “pure function” in CUDA-runtime terms. That is probably acceptable in-repo because the shared GPU runtime follows the same pattern, but it is still a public-surface side effect that should be documented if the API remains public.

Fourth, the third-pass verification section is materially too weak. Its example command, `cargo test -p ta_app_core cdmc_gpu_kernel -- --nocapture`, does not account for the file’s top-level feature gate (`#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]`) or the module gate in `lib.rs`. Without the feature flags, that command is not a trustworthy validation instruction for this file.

Fifth, “add or tighten ownership/lifetime assertions” is too optimistic as an immediate recommendation. The wrapper can cheaply reject null pointers, bad row indices, and invalid windows, and it already does some of that (`183-265`). What it **cannot** do from safe Rust is prove same-device provenance or the true allocation extent behind raw `u64` inputs. So the immediate hardening path is not “more assertions” in the abstract; it is documentation plus a few targeted negative tests for the invariants the file can actually check.

Finally, the third-pass packet dropped a practical maintenance hazard that the first pass had correctly called out: the kernel-launch argument list is still manually mirrored in Rust (`537-566`). The safest local hardening here is still a short ABI-sensitivity comment pinned to `cdmc_backtest_metrics_f32`, not a launch-path rewrite.

### What looks overstated or regression-prone if implemented naively

The risky temptation after reading the third-pass packet would be to force a more assert-heavy or genericized cleanup. That would overshoot what the live evidence supports.

- Making this a one-file “ownership assertion” project is too strong because the unsafe boundary is carried by opaque device pointers; most of the important invariants are not machine-checkable here.
- Converting the public function to `unsafe fn` as a quick fix would be a public-contract change, would not by itself solve cache-identity or context-side-effect issues, and could break downstream use for limited practical gain.
- Restoring the previous CUDA context on return might look cleaner in isolation, but this file follows a broader runtime pattern also used by `gpu_backtest_kernel.rs`; changing it here alone risks creating divergent CUDA-context behavior across sibling wrappers.

## Implementation Priority

### Safe-now hardening

1. **Document the public contract immediately.**
   Add Rustdoc to `CdmcGpuDeviceSignalMatrices`, `CdmcGpuComboBatch`, and `eval_cdmc_metrics_gpu_kernel_device_rows`. The docs should explicitly say that raw pointers must be valid device allocations for the selected device, that `mama_ptr` and `fama_ptr` each cover `cols` samples, that row-major matrices must match their declared rows/strides, that `close_f32` and `close_device_ptr` must describe the same logical close series, and that cache reuse assumes that logical close series remains stable for the cache key’s lifetime.

2. **Document the current-context side effect.**
   If this module remains public, callers should be told that the function sets the selected CUDA context current on the calling thread as part of runtime reuse. That is a real behavior contract even if it is acceptable for the repo’s current CUDA model.

3. **Add narrow negative tests for what the wrapper can actually validate.**
   The existing tests cover stride bounds, row bounds, invalid windows, and `i32` overflow. Add cheap local tests for null-pointer rejection and combo-length mismatch. Also add a pre-launch rejection test for unsupported strategy overlays / stop-loss / take-profit, because that branch is local and does not require a live CUDA runtime.

4. **Add a local ABI warning comment above the launch-argument block.**
   The argument order in `537-566` is hand-maintained and must match `cdmc_backtest_metrics_f32`. A short comment here is low-risk and high-value.

5. **Fix the packet’s verification guidance.**
   All suggested commands for this file should include `--features cuda,cuda-backtest-kernel`. That is not optional for credible local validation.

### Larger redesign still deferred

1. **Public-surface narrowing or redesign.**
   The first-pass concern about a public raw-pointer API still stands, but this pass cannot prove whether out-of-crate consumers exist. Narrowing to `pub(crate)` therefore remains deferred pending compatibility review.

2. **Typed device-input views instead of raw `u64` fields.**
   A stronger design would wrap device pointers, shapes, and perhaps device identity in validated internal types. That would better match the actual invariants, especially for MAMA/FAMA and the row-major matrix inputs. It is not a safe-now change for a public module.

3. **Explicit close-series generation/keying.**
   The resident close/log-return cache would be more robust with a caller-supplied generation key, content signature, or stronger validated handle instead of inferring identity from `close_f32` plus an optional device pointer. That is correctness-attractive but benchmark-sensitive and should stay deferred.

4. **Repo-wide CUDA-context policy review.**
   If the project eventually wants “restore previous context on return” semantics, that should be a repo-wide CUDA-runtime decision across wrappers, not a local edit to this one strategy file.

## Verification If Changed

For doc-only or local validation-test changes, the minimum credible commands are:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_stride_smaller_than_cols -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_out_of_range_combo_rows -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_invalid_windows -- --nocapture`
- plus any newly added null-pointer or overlay-rejection tests.

For launch/cache/contract behavior changes, re-run the existing strategy-family GPU parity coverage with the same feature flags, including:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_gpu_kernel_major_parity_vs_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_validation_gpu_kernel_start_t_matches_cpu_series -- --nocapture`
- `cargo test -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel cross_ticker_cdmc_gpu_kernel_smoke_applies_stats -- --nocapture`

The main thing the fourth pass changes here is the verification **framing**: do not present non-feature-gated cargo commands as evidence for a file that is entirely behind `cuda` + `cuda-backtest-kernel`.

## Regression And Feature-Preservation Constraints

- Preserve the current strategy-local wrapper shape. Nothing in the live source justifies genericizing this into shared cross-strategy infrastructure yet.
- Preserve scratch-buffer reuse, resident close/log-return reuse, combo upload reuse, and the current single post-launch synchronization pattern unless performance evidence says otherwise.
- Preserve CPU/GPU semantics coupling for `phase_threshold.abs()`; this file is not the place for unilateral CDMC math changes.
- Preserve the metrics-row decode contract and kernel-argument order unless parity tests are rerun.
- Do **not** treat `unsafe fn`, `pub(crate)`, stronger cache identity, or context-restoration changes as safe-now one-file cleanups. Those are compatibility-sensitive or benchmark-sensitive redesigns.
- Keep verification commands feature-gated. A green run without `cuda,cuda-backtest-kernel` is not evidence for this wrapper.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `cdmc_gpu_kernel.rs` remains a `Small surgical refactor`, not a redesign candidate.

What the third pass got right was the instinct to avoid broad refactoring and to treat runtime reuse as fragile. What it missed was specificity. The live file’s actionable hardening work is not generic “ownership/lifetime assertions”; it is tighter documentation of the public raw-pointer contract, documentation of the current-context side effect, a few additional local negative tests, an ABI warning comment at the launch site, and corrected feature-gated verification instructions.

The larger ideas remain deferred: visibility narrowing, typed device views, stronger close-series cache identity, and any repo-wide reconsideration of CUDA context behavior. Those changes may be worthwhile later, but they are the ones most likely to create compatibility or performance regressions if implemented as a one-file cleanup.
