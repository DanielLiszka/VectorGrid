# Fourth-Pass Review: crates/app_core/src/kqsr_gpu_kernel.rs

Review date: `2026-04-16`  
Source file: `crates/app_core/src/kqsr_gpu_kernel.rs`  
First-pass packet: `review__first_pass.md`  
Second-pass packet: `review__second_pass.md`  
Third-pass packet: `review__third_pass.md`  
Third-pass action bucket: `Small surgical refactor`  
Fourth-pass action bucket: `Small surgical refactor`  
Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally right, but it compressed the problem too aggressively. The live file is still a strategy-local GPU wrapper whose real risk lives in ownership, context, and cache contracts rather than in KQSR math. But the third pass blurred together two very different classes of follow-up work:

Small-surgical work is still available here, but it is mostly documentation and test hardening around the existing contract. By contrast, changing visibility, changing the safe/public signature, or changing context-management behavior would be crate-surface redesign work with compatibility consequences. A fourth pass was warranted to separate those and to challenge whether the earlier packet’s “narrow visibility / tighten assertions” framing was actually the safest immediate move.

## Fourth-Pass Research Log

I re-read the live source first, then checked the crate root and the shared GPU runtime file to validate what is actually part of the public surface and what verification helpers already exist.

The live source still exposes `KqsrGpuDeviceSignalMatrices`, `KqsrGpuComboBatch`, and `eval_kqsr_metrics_gpu_kernel_device_rows` publicly, still stores runtime state in a thread-local cache, still computes resident close/log-return buffers behind `ensure_resident_close_lr`, and still launches kernels from raw `u64` device pointers (`crates/app_core/src/kqsr_gpu_kernel.rs:33-56`, `:100-146`, `:302-335`, `:356-563`).

The crate root check mattered because it changes how strong the visibility recommendation can be. `kqsr_gpu_kernel` is not an isolated public export; the crate root publicly exports multiple strategy-local GPU adapter modules in the same pattern, including `asr_gpu_kernel`, `carm_gpu_kernel`, `cdmc_gpu_kernel`, `gpu_backtest_kernel`, `kvdf_gpu_kernel`, `mkcs_gpu_kernel`, `rocb_gpu_kernel`, `vcc_gpu_kernel`, `vcet_gpu_kernel`, and `vemr_gpu_kernel` (`crates/app_core/src/lib.rs:2-10`, `:29-45`, `:58-88`). That makes a one-off `pub(crate)` narrowing recommendation materially weaker than the earlier packet suggested.

I also checked the shared GPU runtime/test helper surface. `gpu_backtest_kernel.rs` already exposes serialized GPU test helpers and a `reset_test_runtime()` hook, but that reset only clears the generic runtime caches in that file; it does not clear `KQSR_GPU_RUNTIME` from this module (`crates/app_core/src/gpu_backtest_kernel.rs:65-132`; `crates/app_core/src/kqsr_gpu_kernel.rs:144-146`). That becomes important if this file gains GPU cache-behavior tests.

I then rechecked the external contract assumptions that matter most here. Rust’s `thread_local!` docs still confirm that thread-local state is per-thread rather than shared process-wide, which supports reading `KQSR_GPU_RUNTIME` as a thread-affine cache rather than a global runtime singleton. The `cust` context docs are also important: `Context::new(device)` retains the device’s primary context and repeated calls on the same device return the same internal context, while `CurrentContext::set_current` sets the current context for the calling thread. CUDA’s own context documentation still says each context has its own address space and that a host thread has only one current context at a time. Those three facts sharpen the real contract here: the sharp edge is not merely “there is unsafe code inside,” but “this safe public function accepts raw pointers, mutates the calling thread’s CUDA current-context state, and cannot prove pointer provenance from `u64` inputs.”

## What Fourth Pass Sharpened

### The current third-pass bucket should stand, but only with narrower safe-now work

`Small surgical refactor` still looks like the correct bucket for the live file. I do not see a new algorithmic bug, missing bounds check, or parity flaw that would justify escalating this to a broader redesign bucket. The implementation remains operationally solid and conservative in the places that most directly affect correctness: it validates dimensions and windows before launch, it preserves resident reuse, it keeps launch-shape selection explicit, it checks output-buffer overflow, and it avoids decoding beyond the copied metric region (`crates/app_core/src/kqsr_gpu_kernel.rs:168-255`, `:378-380`, `:437-442`, `:545-560`).

What changed in this fourth pass is not the bucket, but the staging inside that bucket. The truly safe immediate work is contract hardening: docs, a few more targeted tests, and better verification support if this file is edited. The semver-visible ideas from earlier packets are not wrong to consider, but they are not the same size of change.

### What the current third-pass packet got right

The third-pass packet was right to treat this as a contract file rather than a math file. The risky behavior is concentrated in raw device-pointer transport, current-context selection, scratch reuse, and resident-series caching. It was also right to say that thread-local runtime reuse and resident close/log-return caching should not be “cleaned up” casually. This file is small, but it sits on a benchmark-sensitive CUDA path, so apparently tidy changes can easily regress latency or parity.

It was also correct to avoid recommending broad genericization. Nothing in the live file suggests that folding KQSR into a wider shared abstraction would be a safe local cleanup. The strategy-local boundary still makes sense.

### What the current packet missed or underweighted

The largest miss is that the live public-surface problem is not only pointer lifetime. The entrypoint also changes the calling thread’s current CUDA context via `rt.set_current()` and never restores any previous current context (`crates/app_core/src/kqsr_gpu_kernel.rs:130-132`, `:387-390`). That is an observable behavior contract for any caller doing mixed GPU work on the same thread. The third pass talked about ownership and lifetime in general terms, but it did not name this side effect explicitly. For a public API, that omission matters more than another round of general “same-device ownership” wording.

The second miss is that `close_f32` is not just a host fallback convenience. Even when `close_device_ptr != 0`, the function still requires `close_f32.len() == cols` during validation and still uses the host slice together with `close_device_ptr` to key the resident close/log-return cache (`crates/app_core/src/kqsr_gpu_kernel.rs:176-179`, `:306-323`). In other words, this is not a pure device-only transport surface. The host close slice is part of identity and validity today. The third-pass packet talked about device ownership, but it underdescribed this host/device coupling.

The third miss is the intentional asymmetry around `close_device_ptr == 0`. Every signal-matrix pointer except `close_device_ptr` is rejected when null (`crates/app_core/src/kqsr_gpu_kernel.rs:180-190`), but `close_device_ptr == 0` is explicitly allowed and means “upload close from host now” (`:314-320`). That is subtle enough that it should be locked down by both docs and a focused unit test. Earlier packets noticed the behavior, but the third pass did not surface it as one of the most regression-prone local contracts.

The fourth miss is a small but real edge case in the entrypoint’s short-circuit order. The function returns `Ok(Vec::new())` on an empty combo batch before it checks whether overlays, stop-loss, or take-profit are unsupported (`crates/app_core/src/kqsr_gpu_kernel.rs:364-375`). I do not think that is necessarily wrong. Returning success for a no-op batch is defensible. But it is observable behavior, and it means a future “unsupported feature rejection” test will falsely pass if it uses an empty combo set. The packet asked for such a test without calling out this trap.

The fifth miss is verification ergonomics. This file has its own thread-local runtime cache, but unlike `gpu_backtest_kernel.rs`, it has no local test reset hook. If someone modifies runtime reuse or resident-series behavior and tries to add GPU tests here, generic runtime reset helpers will not clear `KQSR_GPU_RUNTIME`. That is not a production bug, but it is a real testability gap that becomes relevant precisely when doing the kind of cache-hardening work the earlier packet recommended.

### What the current packet overstated

The earlier packet treated visibility narrowing as a relatively clean safe-now fix if no external users exist. That is too strong on the evidence currently in hand. The live crate root exports many sibling GPU wrapper modules publicly, so `kqsr_gpu_kernel` does not look like an accidental one-off exposure (`crates/app_core/src/lib.rs:2-10`, `:29-45`, `:58-88`). A one-module `pub(crate)` change would therefore be both compatibility-visible and policy-inconsistent unless the crate is intentionally starting a coordinated public-surface reduction. That may still be the right long-term direction, but it is not a narrow local refactor anymore.

The packet also implied that “add or tighten ownership/lifetime assertions” is an immediate answer. That is weaker than it sounds. This function takes raw `u64` device pointers. It can validate sizes, nulls, and simple integer bounds, but it cannot prove pointer provenance, liveness, or context ownership from those values alone. So assertion-based hardening is limited. The small-surgical tools are documentation, better tests, and maybe narrow debug-only checks of obvious invariants. Anything stronger requires changing the transport surface itself.

There is one more nuance the third pass did not capture: the context story is bad, but not in the most naive way. `cust::Context::new(device)` uses the device’s primary context rather than creating a brand-new legacy context each time, so same-device internal callers that also use `cust` primary-context handling are more likely to interoperate than the earlier packet implied. That does not remove the raw-pointer risk, and it does not excuse the undocumented current-context side effect. But it does mean the “foreign context” concern is sharpest for mixed APIs, foreign allocations, or crate-external callers, not automatically for every same-device internal call.

## Implementation Priority

### Safe-now hardening

The first safe-now step is documentation, not signature churn. `KqsrGpuDeviceSignalMatrices`, `KqsrGpuComboBatch`, and `eval_kqsr_metrics_gpu_kernel_device_rows` should document the actual contract the live code already expects. That documentation should state, in plain terms, that the function accepts raw device pointers, sets the current CUDA context for the calling thread, expects signal pointers other than `close_device_ptr` to be non-null, treats `close_device_ptr == 0` as a host-upload fallback, still requires a host `close_f32` mirror even when a device close pointer is provided, and assumes the close series is immutable for a given host/device signature while the resident cache is live.

The second safe-now step is a small extension of local tests. At minimum, I would add one test that confirms `close_device_ptr == 0` is accepted while a genuinely required signal pointer is rejected, one test for combo-vector length mismatch, and one unsupported-feature rejection test that uses a non-empty combo batch so it actually exercises the rejection path. If the empty-combo success behavior is intentional, a tiny unit test for that is worth adding as well, because it turns a subtle ordering choice into an explicit contract.

The third safe-now step only matters if this file will get GPU runtime/cache tests, but it is worth naming now: add a file-local `#[cfg(test)]` reset helper for `KQSR_GPU_RUNTIME`, and serialize any new GPU tests with the existing GPU test lock from `gpu_backtest_kernel.rs`. Without that, cache tests here will be harder to make deterministic.

### Larger redesign still deferred

Visibility narrowing should be deferred. If the crate wants to make these strategy-local GPU wrappers internal-only, that should be a coordinated public-surface decision across the sibling modules exported from `lib.rs`, not a one-file tweak hidden inside a KQSR review. The same is true of changing the entrypoint from safe to `unsafe`: that is a plausible long-term improvement, but it is not a narrow change once the module is already part of the crate’s public surface.

A transport redesign is also deferred. Replacing raw `u64` pointers with an opaque crate-owned handle, a typed device-slice wrapper, or a more structured borrowed device-view is the real way to make provenance and context obligations enforceable. That is good debt to pay eventually, but it is not today’s small-surgical patch.

I would also defer any redesign of context restoration or stream/copy topology. The current file’s behavior around `set_current`, stream synchronization, resident reuse, and full-host decode is performance- and interop-sensitive. A future redesign may want scoped current-context restoration, reusable raw-row output, or a validation-specific output path, but those are benchmarked redesign tasks rather than immediate cleanup.

## Verification If Changed

If the change is limited to docs and additional local unit tests, the existing focused validation tests plus the new contract tests are enough.

For local coverage, keep the current file-local checks and add the missing edge cases:

- `cargo test -p ta_app_core validate_inputs_rejects_ --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core checked_i32_rejects_overflow --features cuda,cuda-backtest-kernel -- --nocapture`
- add targeted tests for `close_device_ptr == 0`, combo-length mismatch, and unsupported-feature rejection with a non-empty combo batch

If anything touches runtime behavior, caching, pointer transport, or public signature shape, then the higher-level GPU parity tests from the earlier packets are still the right bar:

- `cargo test -p ta_app_core kqsr_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kqsr_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kqsr_validation_gpu_kernel_reports_route_diagnostics --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kqsr_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`

If new GPU tests are added specifically for KQSR runtime reuse, make them deterministic by clearing `KQSR_GPU_RUNTIME` between cases and serializing the GPU test environment.

## Regression And Feature-Preservation Constraints

Preserve the kernel ABI surface exactly unless you are intentionally coordinating a broader GPU-kernel change. That means preserving argument order, metric stride, flag packing, and the current row-major pointer layout expected by `kqsr_backtest_metrics_f32`.

Preserve the `close_device_ptr == 0` fallback unless the API contract is being deliberately redesigned. The live file clearly supports that mode today, and a cleanup that starts null-checking `close_device_ptr` would silently remove behavior.

Do not casually reorder the empty-combo short-circuit and unsupported-feature checks. Either ordering is defensible, but changing it is observable behavior at the API boundary.

Do not remove thread-local runtime reuse, resident close/log-return caching, or combo-upload scratch reuse in the name of simplification. Those are exactly the kinds of apparently small cleanups that can move performance in repeated optimization or validation loops.

Do not attempt a one-file visibility or `unsafe`-surface change unless you are willing to treat it as a crate API decision. There is no IPC or persistence contract in this file, but there is a real compatibility contract at the crate boundary and a real CUDA context contract at runtime.

Finally, do not present “more assertions” as a complete answer. With raw `u64` device pointers, the wrapper cannot prove the hardest parts of the contract. If stronger enforcement becomes a priority, that is a transport redesign, not a handful of guards.

## Fourth-Pass Conclusion

The current third-pass bucket should stand: `Small surgical refactor` is still the right classification.

What changes in the fourth pass is the shape of the recommendation. The third-pass packet was right that the file’s real risk is ownership/context discipline, and right that runtime reuse should be preserved. But it underweighted the undocumented current-context side effect, the required host `close_f32` mirror even in device-pointer mode, the intentionally asymmetric `close_device_ptr == 0` fallback, the empty-combo short-circuit, and the lack of a local runtime-reset hook for future GPU tests.

So the conservative fourth-pass call is: keep the bucket, narrow the safe-now work to docs and tests, and explicitly defer visibility changes, `unsafe` signature changes, transport redesign, and performance-topology rewrites. That gives the repo a stronger, more honest contract surface without taking on a larger public-API or CUDA-runtime redesign under the label of a “small” refactor.
