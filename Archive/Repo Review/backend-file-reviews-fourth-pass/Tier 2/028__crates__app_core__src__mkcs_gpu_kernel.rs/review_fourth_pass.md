# Fourth-Pass Review: `crates/app_core/src/mkcs_gpu_kernel.rs`

- **Review date:** `2026-04-16`
- **Source file:** `crates/app_core/src/mkcs_gpu_kernel.rs`
- **First-pass packet:** `Repo Review/backend-file-reviews/crates/app_core/src/mkcs_gpu_kernel.rs.review.md`
- **Second-pass packet:** `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/mkcs_gpu_kernel.rs.review.md`
- **Third-pass packet:** `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/mkcs_gpu_kernel.rs.review.md`
- **Third-pass action bucket:** `Small surgical refactor`
- **Fourth-pass action bucket:** `Small surgical refactor`
- **Packet status:** `Reviewed`

## Why This File Got A Fourth Pass

The live file still looks like a strategy-local CUDA wrapper whose main risk is contract sharpness, not algorithmic correctness. The third-pass packet was directionally right to keep the bucket small and to warn against genericizing the wrapper casually. The reason to reopen it was not that the bucket looked obviously wrong; it was that the third pass stayed too abstract for a public, CUDA-runtime-critical surface that still exports raw-handle transport structs from `lib.rs` and still relies on caller discipline for device affinity, pointer lifetime, row-major layout, and cache coherence.

After re-reading the live source first, then checking the crate export in `lib.rs` and the shared runtime helper patterns in `gpu_backtest_kernel.rs`, I do **not** think this file needs a larger redesign now. I do think the third-pass packet underweighted the most concrete safe-now work: documenting the actual contract of the public evaluator, freezing a couple of edge semantics with tests, and being more careful about which “hardening” ideas are actually safe versus semver- or performance-sensitive.

So the current third-pass bucket should **stand**. What changes in the fourth pass is the priority order inside that bucket: contract documentation and a few precise tests are justified now; visibility changes, cache-key redesign, and runtime pooling still belong in the deferred lane.

## Fourth-Pass Research Log

I re-read the live `mkcs_gpu_kernel.rs` first, especially the public transport structs at `33-51`, input validation at `163-245`, resident close/log-return caching at `292-326`, the public evaluator at `347-549`, and the existing file-local tests at `551-630`. I then checked `crates/app_core/src/lib.rs` to confirm that this module is still publicly exported at `45`, and reviewed the shared runtime helper patterns in `crates/app_core/src/gpu_backtest_kernel.rs`, especially the matching context/module/stream setup and shared launch-cap caching around `1838-1895`.

I also rechecked the standards side against the exact kinds of claims the third pass was making. The Rust API Guidelines remain relevant here for two reasons: public fields are a strong compatibility commitment, and public items should document failure conditions and caller obligations clearly. The standard-library `thread_local!` docs remain relevant because this file’s runtime map is thread-local, not process-global. The `cust` docs matter because `Context::new` retains the primary context for a device and makes it current, `CurrentContext::set_current` sets the current context for the calling thread, and stream work is ordered within a stream. Those points are exactly what determine whether cache reuse, current-context setup, and “small cleanup” suggestions are actually safe.

## What Fourth Pass Sharpened

### What the third-pass packet got right

The third pass was right about the big picture. This wrapper is still primarily about ownership, lifetime, and launch-contract discipline rather than about MKCS strategy math. It was also right to preserve strategy-local organization, to treat upload/cache/sync behavior as benchmark-sensitive, and to avoid suggesting broad cleanup without parity and runtime evidence. Nothing in the live file contradicts that framing.

### What it missed or underweighted

The first thing the third pass underweighted is that the **public contract problem is still the clearest safe-now hardening target**. The live module is publicly exported from `lib.rs` at `45`. The two transport structs are public with public fields at `33-51`. The main evaluator is a safe public function at `347-353`. But the actual invariants are much stricter than the type system expresses: the `u64` fields must be valid device pointers for the active device/context, the pointed-to rows must have the advertised row-major shape and stride, the buffers must outlive the launch, and the caller must not rely on host validation to prove any of that. `validate_inputs()` at `163-245` does useful bounds and shape checks, but it cannot make this surface truly safe. The third pass talked about ownership/lifetime generally; the fourth pass says more plainly that missing rustdoc on the public surface is now a higher-priority fix than vague calls for “more assertions.”

The second underweighted point is the **dual-source close-series contract**. When `close_device_ptr == 0`, `ensure_resident_close_lr()` uploads `close_f32` and owns the resulting device buffer at `305-310`. When `close_device_ptr != 0`, it uses the caller-supplied device pointer directly at `305-306`. But in both cases, cache identity is still built from `CloseSeriesKey::from_host_and_device(close_f32, close_device_ptr)` at `297`. That means the host slice and the device pointer are both part of the contract even when the GPU path only computes from the device pointer. The live file does not document which input is authoritative when both are present, and the conservative reading is that callers must keep them coherent and immutable for the lifetime of cache reuse. The third pass did not surface this nuance sharply enough.

The third missed point is that **reuse here is thread-local, not merely “lifetime-sensitive.”** The runtime map is a `thread_local!` `HashMap<u32, MkcsCudaRuntime>` at `139-141`. Each runtime owns its own stream, scratch buffers, and resident-series cache at `95-103`. Calls also reassert the current CUDA context via `set_current()` at `125-126` and `381`. With the current `cust` primary-context model, that is a legitimate design, but it means wrapper-local reuse does not automatically cross threads. A caller that shifts equivalent MKCS GPU work between threads may hit separate Rust-side scratch/cache instances even on the same device. That is not a reason to redesign the file now; it is a reason not to oversell current reuse behavior and to document that cache locality is thread-scoped.

The fourth missed point is a small but real **empty-batch semantic quirk**. `eval_mkcs_metrics_gpu_kernel_device_rows()` returns early with `Ok(Vec::new())` when `combo_count == 0` at `355-357`, and only afterwards rejects overlays, stop-loss, or take-profit at `359-366`. So unsupported execution features are rejected for non-empty work, but not for an empty batch. That may be perfectly acceptable as a no-op behavior, and I would not change it casually. But it is a real observable contract detail that the third pass did not call out.

The fifth underweighted point is testing. The existing file-local tests cover host-side shape and bounds validation well enough for their size, but the third pass was too vague about what to add next. The most justified additive test here is not an “invalid nonzero device pointer” test; that would depend on CUDA-driver failure behavior and would not make the API safer. The better additions are stable host-side semantics: an explicit unit test that unsupported overlays/SL/TP are rejected before any CUDA work, and, if maintainers want to preserve the current no-op semantics, a tiny test that an empty combo batch with unsupported execution settings still returns `Ok(Vec::new())`.

### What the third-pass packet overstated or left too vague

The biggest overstatement is the suggestion to “add or tighten ownership/lifetime assertions before refactoring.” For this file, that sounds stronger than it is. The hard invariants are not the sort that ordinary host-side assertions can prove. A safe Rust assertion can check lengths, ranges, nonzero handles, and maybe a few cheap invariants; it cannot prove that a raw `u64` is valid for the active context, that the pointed-to device allocation has the right element type and pitch, or that the pointed-to rows remain alive until the kernel completes. So the safe-now move is not “more assertions” in the abstract. It is clearer documentation of caller obligations and, only after an API audit, possibly a different surface shape.

The other point that needs downgrading is any implication that **visibility narrowing is routine hardening**. The earlier packet history floated `pub(crate)` as the smallest justified fix if there are no external consumers. That may turn out to be the right end state, but it is not a casual small refactor when `lib.rs` still exports the module publicly. In a public-facing repository, changing that visibility is a compatibility decision. The fourth pass therefore moves visibility narrowing out of the safe-now lane and into the deferred/API-audit lane.

The third pass also kept verification too generic. For a CUDA wrapper like this, “representative GPU upload/probe/parity coverage” is directionally fine but not operationally sharp enough. The earlier packet history already had more concrete verification commands, and those are the better fit for this file.

### Safe-now hardening

The safe-now lane is still modest, but it is more specific than the third-pass writeup made it sound.

The highest-value change is rustdoc on `MkcsGpuDeviceSignalMatrices`, `MkcsGpuComboBatch`, and `eval_mkcs_metrics_gpu_kernel_device_rows()`. That documentation should state, explicitly, that the signal pointers and any nonzero `close_device_ptr` must be valid device-resident buffers for the selected device/context; that the matrices must be row-major with the advertised row counts and strides; that `close_f32` must describe the same close series as `close_device_ptr` when both are supplied; that the close series and signal matrices are assumed immutable for cache reuse; that `close_device_ptr == 0` triggers host upload; that overlays, stop-loss, and take-profit are unsupported on this route; and that runtime reuse is thread-local rather than a process-global guarantee.

The next justified hardening is additive tests that freeze stable host-side behavior. The cleanest one is a unit test for unsupported overlay/SL/TP rejection because that branch happens before any CUDA runtime setup. A second optional test can freeze the current empty-combo semantics if maintainers decide that the no-op behavior is intentional. Those tests are narrower and safer than trying to “test invalid pointers.”

### Larger redesign still deferred

The larger redesign lane is unchanged, but the fourth pass makes the boundaries clearer.

Changing the public surface shape—by making the evaluator `unsafe`, hiding fields behind constructors/newtypes, or narrowing visibility to `pub(crate)`—is still deferred until maintainers do an explicit API audit. Those changes may be correct, but they carry compatibility cost.

Changing `CloseSeriesKey` behavior or introducing explicit cache invalidation is also deferred. The current cache is clearly performance-motivated, and any redesign has to account for repeated-window validation cost, possible host/device coherence rules, and the fact that the current key is deliberately derived from both host and device inputs.

Any process-wide or cross-thread runtime pooling is deferred as well. The current thread-local runtime map is crude but understandable. Replacing it with shared runtime state would require a clearer model for thread affinity, current-context management, stream ownership, and memory-lifetime discipline across callers.

Finally, async output paths, staging changes, sync removal, or launch-ABI reshaping remain deferred. This function currently returns host-side `Vec<Metrics>`, synchronizes the stream at `531`, copies the full metrics buffer back at `533-541`, and launches kernels via positional argument arrays at `272-287` and `495-529`. Those are precisely the kinds of changes that can look like cleanup while silently perturbing parity or performance.

## Implementation Priority

The priority order should remain conservative.

First, document the public contract and add one or two narrow, host-side tests that lock down behavior already present in the file. That is the highest-confidence way to improve code quality without disturbing runtime behavior.

Second, if maintainers want to reduce the public blast radius, do an explicit crate-API audit before changing visibility or surface shape. That is not a same-pass cleanup.

Third, leave cache identity, runtime topology, synchronization, and ABI work for a later pass that includes measured GPU evidence and coordinated review with the CUDA source.

## Verification If Changed

If the safe-now lane changes, the minimum verification set should stay specific:

`cargo test -p ta_app_core mkcs_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`

`cargo test -p ta_app_core mkcs_gpu_kernel_request_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture`

`cargo test -p ta_app_core mkcs_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`

`cargo test -p ta_app_core mkcs_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`

If anything touches cache semantics, context/runtime reuse, stream synchronization, launch argument order, or pointer-contract shape, add the broader smoke and perf checks:

`cargo test -p ta_desktop_demo_app cross_ticker_mkcs_gpu_kernel_smoke_applies_stats --features cuda,cuda-backtest-kernel -- --nocapture`

`powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression And Feature-Preservation Constraints

Do not remove or silently change host-upload fallback. The branch at `305-310` is active behavior, not dead compatibility code.

Do not weaken or silently broaden the explicit unsupported-feature rejection at `359-366`. If the route still does not support overlays, stop-loss, or take-profit, the file should continue to fail honestly.

Do not change the order of the empty-combo return and unsupported-feature rejection without deciding whether empty-batch calls are meant to be a strict capability check or just a no-op evaluation path.

Do not “optimize away” per-call `set_current()` unless current-context ownership is re-proven across all callers. In this wrapper, redundant-looking current-context setup is part of correctness posture, not obviously dead work.

Do not change cache-key semantics, stream synchronization, host-copy shape, or launch argument order without coordinated review of the surrounding runtime and CUDA entrypoints. Those are regression-prone surfaces even when edits look small.

Do not treat visibility narrowing or API-surface reshaping as a harmless cleanup. Because the module is currently exported publicly, that belongs behind an intentional compatibility decision.

## Fourth-Pass Conclusion

The third-pass action bucket should **stand**: `mkcs_gpu_kernel.rs` remains a `Small surgical refactor`, not a redesign candidate.

What the third-pass packet got right was the overall posture: this file is primarily an ownership, lifetime, and benchmark-sensitive wrapper, and it should not be generalized or performance-tuned casually. What it missed was where the safest value actually sits: the public evaluator still has an underdocumented raw-handle contract; the close-series input is really a dual-source host/device coherence contract; runtime reuse is thread-local, not global; and there is a small empty-batch semantic quirk worth either documenting or freezing with a test.

The fourth-pass recommendation is therefore narrower and more evidence-backed than a generic “tighten assertions” story. Do the docs. Add the small stable tests. Keep the bucket. Defer visibility changes, cache redesign, runtime pooling, and stream/ABI work until there is an explicit API audit or measured GPU motivation.
