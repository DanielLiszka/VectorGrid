# Fourth-Pass Review: crates/app_core/src/carm_gpu_kernel.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/carm_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/carm_gpu_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/carm_gpu_kernel.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/carm_gpu_kernel.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file warranted a fourth pass because the third-pass packet was directionally correct but too abstract. It correctly treated `carm_gpu_kernel.rs` as a strategy-local, reuse-sensitive CUDA wrapper, but it did not materially challenge the live source beyond repeating that ownership and lifetime matter.

Reopening the live file shows that the main unresolved work is still concrete and bounded: one kernel-shape invariant remains unenforced in `validate_inputs` (`carm_gpu_kernel.rs:168-255`), the public CUDA-facing input carriers are still undocumented (`carm_gpu_kernel.rs:33-56`), and the resident close/log-return cache still depends on a subtle host/device logical-series contract that callers cannot infer from the current public surface (`carm_gpu_kernel.rs:303-337`).

The point of the fourth pass, then, was not to force a larger bucket. It was to separate the real safe-now hardening from the more speculative redesign pressure, and to check whether the third pass overstated any runtime concern. After that recheck, the current bucket should stand.

## Fourth-Pass Research Log

I reread the live source file first, then the third-pass packet, then the earlier packets only where the live source still left an open question. I also checked the surrounding contract surface in `lib.rs` and `gpu_backtest_kernel.rs` to confirm public visibility, feature gating, and the repo’s broader CUDA wrapper posture.

On the external side, I rechecked four things.

First, the Rust API Guidelines still support the first-pass concern that a safe public API should make its sharp invariants obvious, especially when the types are just bags of raw pointer-like values.

Second, the current `cust` context model materially sharpens the thread-local discussion. The current `cust` source/docs show that primary contexts are now the default, that there is one primary context per device, and that repeated `Context::new(device)` calls for the same device retain that same underlying primary context rather than minting unrelated legacy contexts. That matters because it narrows what thread-local runtime caching can plausibly break.

Third, I rechecked `cust` stream and buffer semantics. The existing use of `DeviceBuffer::uninitialized`, followed by kernel launch, stream synchronization, and host decode, still fits the overwrite-before-read assumption for this file. I did not find evidence that this wrapper needs a defensive zero-fill change.

Fourth, I rechecked NVIDIA’s current-device and stream rules. They still support the importance of `set_current()` before launch and the need to keep launch/device/stream association stable, but they do not create a new safe-now change beyond preserving the existing current-device discipline.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep the action bucket at `Small surgical refactor`. The live file still does not justify a broader refactor campaign. Its risk profile is dominated by contract clarity and cache correctness, not by evidence of broken strategy math.

It was also right to avoid pushing genericization. Repo guidance favors strategy-local ownership for strategy-local GPU wrappers, and this file’s reuse structure is clearly part of the performance story: thread-local per-device runtime reuse, resident close/log-return reuse, uploaded combo signature reuse, and output scratch reuse are all visible in the live source (`carm_gpu_kernel.rs:58-166`, `303-337`, `384-560`). Flattening that into per-call allocation or a premature generic wrapper would be more likely to regress throughput than to improve maintainability.

Finally, the third pass was right that the sensitive part of this file is the runtime contract, not a missing algorithmic optimization. The existing kernel launch path, stream synchronization, and metric-row decode are structurally coherent enough that the remaining issues are about what the wrapper allows and documents, not about an obviously wrong backtest formula.

### What it missed or underweighted

The biggest miss is that the most actionable live defect is still the same one identified earlier: `validate_inputs` never rejects `combo_count > mixed_rows`. The function checks row bounds, window bounds, stride bounds, and vector-length agreement, but it never compares the combo cardinality to `signals.mixed_rows` (`carm_gpu_kernel.rs:209-255`). For this wrapper, that is not an abstract ownership concern. It is the one missing Rust-side shape invariant most likely to turn an invalid call into plausible-looking weak metrics instead of a rejected call.

The third pass also underweighted the public-contract debt. `CarmGpuDeviceSignalMatrices` and `CarmGpuComboBatch` remain public field bags with raw `u64` device-pointer fields and no rustdoc (`carm_gpu_kernel.rs:33-56`), and `lib.rs` still exports the module publicly under CUDA feature gates (`lib.rs:5-7`). The safe-now issue here is not that the API must be redesigned immediately. It is that callers still are not told, in the public API itself, what must be same-device, what is row-major, what each stride means, and what the wrapper currently refuses to support.

There is also a subtler live contract that the third pass did not state clearly enough: `close_f32` is mandatory even when `close_device_ptr` is nonzero. On the resident-device path, the kernel consumes the device pointer, while the host slice still participates in length validation and cache identity (`carm_gpu_kernel.rs:172-179`, `303-337`, `394-399`). That means the public contract is not simply “either host or device close data.” It is “host length plus optional host upload ownership plus device identity plus logical-series alignment.” That deserves explicit documentation now, because it is easy for a caller to assume otherwise.

The current local tests are also still too narrow for the real wrapper contract. The file has good targeted checks for stride smaller than cols, out-of-range combo rows, invalid windows, and `i32` overflow (`carm_gpu_kernel.rs:588-647`), but it still has no direct test for the mixed-row/cardinality mismatch that matters most here.

### What prior claims were too strong or too weak

The third-pass packet’s thread-local warning needs narrowing. Read literally, it leans toward a “same device, different thread, different runtime” hazard frame that sounds closer to distinct-context pointer invalidity than the current evidence supports. Under current `cust`, `Context::new(device)` retains the device primary context, so repeated same-device calls do not automatically create unrelated legacy contexts. That means thread-local runtime caching is more accurately a reuse-scope and duplication story than a newly discovered same-device context-separation bug.

At the same time, the third-pass recommendation to “add or tighten ownership/lifetime assertions before refactoring” is too weak to drive a concrete change list. This file does not mainly need a new assertion framework. It needs one missing shape check, one missing unit test, and clear public documentation. Anything more ambitious should wait for compatibility and benchmark review.

I would also keep the first-pass documentation recommendation narrower than a doc-test or example push. For this raw-pointer CUDA wrapper, concise Rustdoc describing invariants is the safe-now win. Feature-gated compile examples for this surface are lower priority and more brittle.

### Safe-now hardening

The safe-now hardening set is still small and clear.

First, add a Rust-side validation that `combos.fast_row_index.len() <= signals.mixed_rows` inside `validate_inputs`. That is the highest-value change because it blocks the one live kernel-shape mismatch the wrapper still permits.

Second, add a focused unit test for that exact mismatch. This should be a narrow local validation test, not an end-to-end GPU test, because the problem is the Rust-side contract.

Third, add rustdoc on `CarmGpuDeviceSignalMatrices`, `CarmGpuComboBatch`, and `eval_carm_metrics_gpu_kernel_device_rows` (`carm_gpu_kernel.rs:33-56`, `358-560`). The docs should explicitly cover:

- raw pointer validity and same-device expectations for `device_id`
- row-major layout and the meaning of `mixed_rows`, `fast_rows`, `otto_rows`, and their strides
- that `close_f32.len()` must equal `cols`
- that when `close_device_ptr != 0`, `close_f32` must still describe the same logical series because the host slice participates in validation and cache identity
- that repeated calls rely on the logical series remaining immutable for the resident close/log-return cache lifetime on that runtime/thread
- that overlays, stop-loss, and take-profit are rejected by this entry point today (`carm_gpu_kernel.rs:370-377`)

A fourth, clearly secondary, improvement would be to derive `Debug` on the two public input carriers. That is modest public-surface hardening, but it is not the main reason to touch this file.

### Larger redesign still deferred

The bigger redesigns are real, but they should remain deferred.

The strongest one is cache identity. `ensure_resident_close_lr` currently keys reuse through `CloseSeriesKey::from_host_and_device(...)` and therefore inherits a pointer-identity-style immutability assumption (`carm_gpu_kernel.rs:303-337`). A stronger logical-series key, caller-provided version, or explicit generation counter would be more correct, but it would also be benchmark-sensitive in a hot path. That is not a safe-now change.

The second deferred redesign is the public type surface. A safe public API that accepts raw device pointers as plain `u64` values is weaker than ideal. Better long-term shapes exist: narrowed visibility after compatibility review, validated internal wrapper types, or at least explicit nullable/non-nullable pointer modeling rather than plain integers. But all of those are compatibility-sensitive because the module is publicly exported today.

The third deferred redesign is the close-series contract itself. Requiring a host slice even on a resident-device path is surprising and could be redesigned, but doing so changes the call contract and the cache-identity story at the same time. That is larger than the current bucket.

Finally, I would keep any cross-thread or process-wide runtime-cache redesign deferred. The fourth-pass research did not uncover evidence that the present thread-local scope is the source of a concrete correctness bug. It is a reuse-scope choice with performance and complexity tradeoffs, not a proven broken design in this wave.

### Recommendations that would risk regressions if implemented casually

Changing the resident close/log-return cache key without measurement is benchmark-sensitive. A stronger key may be more correct, but content hashing or deeper identity plumbing in this wrapper could become measurable on repeated optimization and validation calls.

Changing public field types or narrowing visibility is compatibility-sensitive. Because `lib.rs` exports this module publicly under CUDA feature gates, even an apparently cleaner raw-pointer type change could break out-of-crate consumers.

Changing launch argument order, metrics stride handling, or current-device/stream sequencing is a GPU ABI/runtime risk. This wrapper is the Rust half of a fixed kernel contract. Casual cleanup in that area is not “just refactoring.”

Adding a device-side sentinel/error-decoding policy only for this wrapper would also be regression-prone. The right immediate fix is to reject bad Rust-side inputs earlier, not to invent a one-off kernel error protocol here.

## Implementation Priority

The current third-pass bucket should stand.

If this file is touched now, the implementation priority should be:

1. Add the missing `combo_count <= mixed_rows` validation.
2. Add the direct unit test for that invariant.
3. Add concise public Rustdoc and one inline cache-path note explaining the immutability and logical-series assumptions.
4. Stop there unless the change budget explicitly includes compatibility review or benchmark work.

That is still a `Small surgical refactor`. It is not a hidden runtime rewrite.

## Verification If Changed

For the safe-now slice, verification should stay narrow and explicit.

Run the file’s local validation tests with the CUDA features enabled, not the misleading no-feature filtered command. At minimum:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_stride_smaller_than_cols -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_out_of_range_combo_rows -- --nocapture`
- add and run a new direct test for combo count exceeding `mixed_rows`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_gpu_kernel -- --nocapture`

If anything beyond validation/docs/cache comments changes—especially cache identity, launch arguments, or public type shape—also run representative CARM GPU parity coverage through the owning composite optimization/validation routes, and treat the repo’s GPU perf gate as the authority surface for performance-sensitive behavior.

## Regression And Feature-Preservation Constraints

Preserve the current reuse posture. That includes thread-local per-device runtime reuse, resident close/log-return reuse, uploaded combo signature reuse, and scratch-buffer reuse. Nothing in this fourth pass supports flattening those into per-call allocations.

Preserve the current overwrite-before-read assumptions. The present use of uninitialized device buffers is acceptable here because the wrapper launches kernels, synchronizes the stream, and only then copies results back for decode. Replacing that with defensive full-buffer initialization would add cost without proving a correctness gain.

Preserve the current strategy-local organization. Repo guidance already favors strategy-local ownership for strategy-local GPU wrappers, and this file still fits that pattern.

Preserve the current GPU contract boundaries. There is no direct IPC or persistence surface here, but there is a real GPU runtime/ABI surface: launch argument ordering, metrics stride, same-device stream usage, and the unsupported overlay/stop/take-profit behavior. Those should not move without coordinated kernel and parity review.

Preserve validation fidelity by rejecting bad calls earlier rather than by silently broadening fallback behavior. The right regression posture here is stronger Rust-side contract checking, not more permissive invalid-input handling.

## Fourth-Pass Conclusion

`Bucket unchanged.`

The current third-pass bucket should stand. `crates/app_core/src/carm_gpu_kernel.rs` is still a `Small surgical refactor` case, not a redesign case.

The third-pass packet got the high-level posture right: keep the wrapper strategy-local, preserve reuse-heavy CUDA plumbing, and avoid speculative genericization. What it missed or underweighted is that the live file still has one concrete missing kernel-shape validation, one still-undocumented public raw-pointer contract, and one surprisingly subtle close-series/cache identity contract that callers cannot infer from the current API.

The main overstatement to correct is the thread-local context framing. Under current `cust`, `Context::new(device)` retains the device primary context, so the sharper risk is not “same device, different thread, different hidden context.” It is narrower: duplicated per-thread runtime reuse, non-process-global cache scope, and still-underdocumented caller obligations.

So the fourth-pass recommendation is conservative and evidence-backed: keep the bucket, add the mixed-row/cardinality validation, add the missing unit test, document the public and cache invariants, and defer cache-key or public-API redesign until compatibility and benchmark review are explicitly in scope.
