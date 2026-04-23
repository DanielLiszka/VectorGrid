# Fourth-Pass Review: crates/app_core/src/vcet_gpu_kernel.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/vcet_gpu_kernel.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Fourth-pass action bucket: `Targeted safe-now hardening is justified; larger benchmark-sensitive redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally right about the file’s risk profile, but it stopped too early. `vcet_gpu_kernel.rs` is still a hot, CUDA-facing, strategy-local wrapper, so broad cleanup remains dangerous. But “do not touch yet” is too absolute for the live file as written.

A fourth pass was justified because the public wrapper surface still permits at least one concrete contract failure that is narrower than a refactor and safer than leaving implicit: the batch is prepared on one device in `PreparedVcetGpuKernelBatch::prepare()` (`lines 155-185`), but `PreparedVcetGpuKernelBatch::eval_metrics()` later accepts an arbitrary `device_id` and forwards that device choice into the runtime without proving it matches the device that owns the prepared CUDA inputs and row pointers (`lines 334-355`). That is not a style issue. It is a missing invariant on a public GPU-prepared batch type.

So the right fourth-pass question is not “should this file be refactored?” It is “are there any conservative, evidence-backed hardenings that should happen now while leaving benchmark-facing structure alone?” For this file, the answer is yes.

## Fourth-Pass Research Log

This pass re-read the live `vcet_gpu_kernel.rs` source first, then the first-, second-, and third-pass packets, then the local routing and policy context in `lib.rs`, `REPO_MAP.md`, `AGENTS.md`, and the shared `gpu_backtest_kernel.rs` context file.

The live-source review focused on five surfaces:

1. owned-device lifetime and raw row-pointer export in `VcetGpuOwnedSignalTables` (`lines 54-142`)
2. batch preparation and combo shaping in `PreparedVcetGpuKernelBatch::prepare()` (`lines 155-323`)
3. the prepared-batch execution entrypoint in `PreparedVcetGpuKernelBatch::eval_metrics()` (`lines 334-355`)
4. runtime cache scope and resident close/log-return reuse in `VcetCudaRuntime`, `VCET_GPU_RUNTIME`, and `ensure_resident_close_lr()` (`lines 418-625`)
5. the shared-kernel ABI handoff in `eval_vcet_metrics_gpu_kernel_device_rows()` (`lines 631-938`)

I also rechecked the external primitives that matter for avoiding false positives here:

- Rust `thread_local!` semantics, to confirm that the runtime cache is per-thread rather than process-global
- `cust` context documentation, to confirm that `Context::new(device)` retains the device’s primary context rather than creating an unrelated legacy context
- CUDA driver current-context semantics, to confirm that “current context” remains thread-scoped

That external research matters because it narrows the correct criticism. The problem is duplicated per-thread scratch, stream, and resident-series state, not “this file is secretly creating incompatible device contexts per thread.” The earlier packet family was correct to be cautious about cache scope, but any review that implies a stronger context-isolation bug would be overstating the evidence.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right that the dominant risk in this file is contract drift around ownership, lifetime, cache scope, and ABI assumptions, not algorithm math. The strategy-local organization should stand. `REPO_MAP.md` explicitly tells contributors to start with the strategy-local owner and GPU companion for `Vcet`, not to genericize first, and it also reiterates the repo-wide rule that downstream kernels consuming rows derived from device matrices must keep the owning matrices alive. That high-level framing still fits this file.

The third-pass packet was also right that cache and launch changes here are benchmark-sensitive. The file owns a thread-local runtime map (`lines 462-464`), resident close/log-return reuse (`lines 593-625`), signature-gated scratch uploads (`lines 694-789`), and launch-shape selection for both the close/log-return kernel and the VCET metrics kernel (`lines 558-567`, `796-800`). Those are exactly the sorts of small wrapper decisions that can perturb timing without changing visible behavior.

Finally, the packet was right not to push a generic shared-runtime extraction. `AGENTS.md` explicitly prefers strategy-local organization, while keeping shared CUDA/runtime plumbing intentionally shared in places like `gpu_backtest_kernel.rs`. Nothing in the live VCET file justifies crossing that boundary yet.

### What it missed or underweighted

#### 1. The strongest live issue is not the keepalive comment gap. It is the missing prepare/eval device lockstep.

`PreparedVcetGpuKernelBatch::prepare()` uploads and prepares CUDA inputs on the caller-specified `device_id` through `PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(..., device_id as usize, ...)` (`lines 178-185`). The resulting batch stores those prepared device inputs and owned device matrices (`lines 144-152`, `315-323`).

But `PreparedVcetGpuKernelBatch::eval_metrics()` accepts a fresh `device_id` parameter (`lines 334-338`) and forwards that device choice into `eval_vcet_metrics_gpu_kernel_device_rows()` (`line 354`) while also passing raw device pointers obtained from the already-prepared batch (`lines 340-347`). The struct does not store the preparation device, and `eval_metrics()` does not verify that the execution device matches the device that owns the prepared CUDA inputs and row-pointer tables.

That means the public API currently permits “prepare on device A, evaluate on device B” without an explicit guard. I did not find evidence in the provided local context that current callers are doing that incorrectly, but the contract hole is real and the cost of hardening it is low.

This is the most important fourth-pass finding because it is both conservative and concrete: storing `prepared_device_id: u32` on the batch and rejecting a mismatched `eval_metrics(device_id, ...)` call adds a deterministic prelaunch error path without changing the hot kernel body, scratch policy, module loading, or host/device copy shape.

#### 2. The public raw-pointer entrypoint still lacks an explicit same-device/current-context contract.

The third-pass packet correctly focused on lifetime, but it did not go far enough on provenance. `VcetGpuDeviceSignalMatrices` is a public struct carrying raw device pointers and raw row-pointer tables (`lines 36-45`), and `eval_vcet_metrics_gpu_kernel_device_rows()` is a public entrypoint that accepts those raw pointers plus a `device_id` (`lines 631-637`). `validate_inputs()` checks shape, null-ness, and index/range bounds (`lines 466-546`), but it does not and cannot prove that the pointers belong to the same CUDA device as the selected runtime.

That does not mean the function is wrong. It means the public contract is underspecified. The same-device rule is implicit in the surrounding architecture, and `REPO_MAP.md` explicitly calls out that device rows derived from returned matrices must keep their owners alive and be treated as device-pointer handoffs with care. For a public raw-pointer surface, this should be written down, not left as repo folklore.

This is a safe-now documentation hardening, not a redesign request. The low-level API can stay public for now. But it should explicitly say that all device pointers and row-pointer tables must originate from the same CUDA device/context that will be selected by the `device_id` argument, and that the owning matrices must remain alive for the whole call.

#### 3. The shared kernel ABI coupling is more implicit than the third-pass packet acknowledged.

This wrapper does not own a strategy-local cubin/PTX. `VcetCudaRuntime::new()` loads the shared double-crossover module through `load_double_crossover_module(device)` (`line 434`), and `eval_vcet_metrics_gpu_kernel_device_rows()` pulls the `vcet_backtest_metrics_f32` symbol from that shared module (`lines 792-795`). It then constructs the flag word with raw bit literals (`lines 803-813`):

- `1 << 0` for long-only
- `1 << 1` for no-flip
- `1 << 2` for trade-on-next-bar
- `1 << 4` unconditionally

That last bit is especially opaque inside this file. In the shared `gpu_backtest_kernel.rs` context, the same bit is named `STRAT_SIGNED_EXPOSURE`, and the flag layout is expressed with named constants rather than literals. This file may intentionally use the same ABI, but it does not say so.

This is a real GPU ABI maintenance risk, not a performance problem. A future editor can preserve behavior here much more safely if the bits are named and documented as aligned with the shared kernel flag layout. That is a tiny code change with effectively zero runtime cost.

#### 4. The prior packets slightly overstated what the no-op binding in `as_signal_matrices()` is doing.

The keepalive model itself is real. `VcetGpuOwnedSignalTables` owns the `VcetOwnedDeviceMatrix` keepalive vectors (`lines 59-61`), and `as_signal_matrices()` returns pointer slices borrowed from `&self` (`lines 121-141`). That borrow is the real lifetime mechanism.

The no-op binding:

```rust
let _ = (
    &self.vidya_rows_keepalive,
    &self.ce_long_rows_keepalive,
    &self.ce_short_rows_keepalive,
);
```

(`lines 126-130`)

is not what “keeps them live.” Those fields are already live because the struct owns them and the returned view borrows `self`. At most, that binding is a readability hint to humans and a defense against future “why do these fields exist?” confusion. It is not the semantic mechanism.

That matters because the earlier packet’s recommended fix was to comment that line. That is too weak and slightly mis-aimed. The useful documentation should live on the owner fields or on `as_signal_matrices()` itself, explaining that the raw row-pointer slices are only valid because the struct retains ownership of the underlying device matrices for the duration of the borrow.

#### 5. The packet family’s local “uninitialized allocation” evidence was weaker than it looked.

The first-pass packet cited local line numbers `573`, `611`, and `886` as examples of “uninitialized device buffers” in the evaluation path. On the live file, `611` is indeed the explicit `unsafe { DeviceBuffer::<f32>::uninitialized(signals.cols) }` used for the resident close/log-return cache in `ensure_resident_close_lr()` (`lines 611-613`). But `573` and `886` are kernel-launch `unsafe` blocks, not uninitialized allocations.

So the broader caution about overwrite-before-read discipline remains valid in repo terms, especially because `AGENTS.md` explicitly says uninitialized allocation is acceptable only when every element is written before any read. But the third-pass packet inherited a claim that was stronger than the local evidence it quoted. The precise local warning here is much narrower: this file has one explicit uninitialized `DeviceBuffer<f32>` allocation for `d_lr`, while the broader scratch allocation behavior is delegated to shared helpers such as `ensure_device_buffer_len(...)`.

That correction matters because it changes the recommended tone. The safe-now action is not “scatter more comments near every unsafe block.” It is “comment the actual overwrite-before-read assumption where this file truly depends on it, and avoid overstating that this module is doing ad hoc unsafe allocation in multiple places.”

### Bucket assessment

The current third-pass bucket should **change**.

`Benchmark-sensitive; do not touch yet` is too broad for the live file. It is still the right bucket for launch-policy edits, cache-scope redesign, or structural cleanup. But it is the wrong bucket for a narrow device-lockstep guard, named ABI flags, and explicit public-contract documentation.

The better fourth-pass bucket is:

**`Targeted safe-now hardening is justified; larger benchmark-sensitive redesign still deferred`**

That change is warranted because the highest-confidence findings are conservative hardenings that do not require retuning the benchmark-sensitive parts of the file.

## Implementation Priority

### Safe-now hardening

1. **Store the preparation device on `PreparedVcetGpuKernelBatch` and reject mismatched evaluation devices.**  
   Add a private `prepared_device_id: u32` field, set it in `prepare()`, and return a deterministic error from `eval_metrics()` if the caller passes a different `device_id`. This is the strongest safe-now recommendation in the file because it closes a public contract hole without touching the kernel body or cache policy.

2. **Replace the raw strategy-flag bit literals with named local constants and document that they must stay aligned with the shared CUDA-kernel ABI.**  
   The current `flags |= 1u32 << 4;` is especially brittle because it encodes a shared convention without naming it. Converting those literals to named constants is a readability and ABI-preservation hardening, not a behavioral change.

3. **Add explicit contract documentation to the public raw-pointer surface.**  
   `VcetGpuDeviceSignalMatrices`, `VcetGpuComboBatch`, and `eval_vcet_metrics_gpu_kernel_device_rows()` should explicitly state:
   - all device pointers and row-pointer tables must belong to the same CUDA device/context selected by `device_id`
   - the owning device matrices must remain alive for the whole call
   - `close_device_ptr == 0` triggers host upload fallback, but non-zero device pointers are trusted as already-device-resident inputs

4. **If documenting lifetime, document the owner fields and the returned borrow, not the no-op binding.**  
   I would not spend change budget “fixing” `let _ = (...)` by itself. If the line stays, fine. If documentation is added, it should explain the struct-level ownership model rather than imply that the no-op binding is the mechanism.

5. **Tighten the overwrite-before-read note only where it is actually local.**  
   A short comment on the explicit `d_lr` uninitialized allocation in `ensure_resident_close_lr()` is reasonable because this file does rely on `launch_compute_log_returns_f32()` fully overwriting the device buffer before any read.

### Larger redesign still deferred

1. **Do not redesign the runtime cache scope in this pass.**  
   The runtime map is thread-local (`lines 462-464`), which means scratch buffers, streams, and resident close/log-return caches are duplicated per thread. That is a real architectural property, but changing it would cross into shared-runtime design, synchronization, and measurement work. It is not a safe-now review fix.

2. **Do not genericize this strategy-local wrapper into shared infrastructure yet.**  
   `REPO_MAP.md` and `AGENTS.md` both point in the opposite direction unless the shared abstraction is clearly cross-cutting and measured. Nothing in the live evidence justifies collapsing `vcet_gpu_kernel.rs` into broader generic infrastructure.

3. **Do not retune launch-shape candidates or scratch-allocation policy here.**  
   `choose_launch_shape_1d_from_candidates(...)` and the signature-gated scratch uploads are part of the benchmark-sensitive path. The current evidence does not show a correctness bug there, only sensitivity.

4. **Do not narrow the public visibility of the low-level structs/functions in a cleanup-only pass.**  
   That may be a good future API-shaping discussion, but it is a public-surface change and should not be smuggled in as a review cleanup.

## Verification If Changed

For **docs-only** hardening, compile-time and existing CUDA-gated tests are enough.

For **code** hardening, especially the prepared/eval device-lockstep guard, the minimum verification should be:

1. file-local guard tests  
   - `cargo test -p ta_app_core vcet_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`

2. representative VCET GPU orchestration/parity coverage  
   - `cargo test -p ta_app_core composite_cpu::tests::vcet_gpu_kernel_request_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture`
   - `cargo test -p ta_app_core composite_cpu::tests::vcet_gpu_kernel_major_parity_vs_cpu --features cuda,cuda-backtest-kernel -- --nocapture`

3. if the change touches runtime-path code rather than comments only, use the repo’s broader CUDA verification posture  
   - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
   - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

I would also add one new targeted regression test if code changes are made: a CUDA-gated test that prepares a batch on one device and verifies that `eval_metrics()` rejects a different device id with a stable error string. If the local machine only has one CUDA device, that test can still be structured conditionally so it runs only when two devices are available.

## Regression And Feature-Preservation Constraints

Preserve the current strategy-local ownership model. The raw row-pointer tables are valid because `VcetGpuOwnedSignalTables` retains the underlying `VcetOwnedDeviceMatrix` owners. Any rewrite that changes that arrangement must preserve borrowed-lifetime correctness, not just pointer values.

Preserve the current shared-kernel ABI values. Renaming the flag bits is safe. Changing their numeric layout is not. The shared module loader and symbol lookup (`load_double_crossover_module(...)`, `vcet_backtest_metrics_f32`) mean this file is coupled to a broader CUDA ABI surface even though the wrapper is strategy-local.

Do not change the thread-local runtime shape casually. It is fair to document that the cache is per-thread and not process-global, but unifying it across threads is a larger synchronization and measurement decision.

Do not treat public low-level API cleanup as free. `VcetGpuDeviceSignalMatrices`, `VcetGpuComboBatch`, and `eval_vcet_metrics_gpu_kernel_device_rows()` are public today. Tightening behavior with a prelaunch device-id guard is justified because it turns an implicit invalid state into an explicit error. Narrowing visibility or reshaping the API is a separate compatibility discussion.

Keep the repo’s broader VCET validation posture in view. `REPO_MAP.md` describes `Vcet` as having runtime-tested `GpuKernel` coverage on the fixed-candidate path and broader validation/reporting expectations. That is another reason to avoid speculative restructuring while still allowing small contract hardenings.

## Fourth-Pass Conclusion

The current third-pass packet got the broad posture mostly right: this file is still a hot, CUDA-facing, strategy-local wrapper where broad cleanup would be easy to regret. It was right to defend ownership/lifetime discipline and to defer genericization, cache redesign, and launch-policy edits.

But the bucket should **not** remain `Benchmark-sensitive; do not touch yet`.

That bucket is too strong because it lumps together two different classes of work:

- **safe-now contract hardening** that can be done without perturbing the benchmark-sensitive body
- **larger redesign** that absolutely should remain deferred

The live file still has at least three conservative improvements that are justified now:

1. enforce prepare/eval device-id lockstep on `PreparedVcetGpuKernelBatch`
2. name and document the shared kernel ABI flag bits instead of relying on raw literals
3. write the same-device / owner-lifetime contract directly on the public raw-pointer surface

Those are not “cleanup by taste.” They are narrow hardenings that reduce misuse and maintenance risk while preserving current behavior, hot-path structure, and benchmark posture.

So the correct fourth-pass outcome is:

**Keep broad runtime/cache/layout changes deferred, but allow targeted safe-now hardening.**
