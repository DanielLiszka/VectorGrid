# Fourth-Pass Review: `crates/app_core/src/rocb_gpu_kernel.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/rocb_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/rocb_gpu_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/rocb_gpu_kernel.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/rocb_gpu_kernel.rs.review.md`
- Third-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Fourth-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The live source is small enough to look mechanically simple, but the third-pass packet stayed too high-level. It correctly identified this as a strategy-local CUDA wrapper where casual cleanup is dangerous, but it underdescribed the actual contracts that make it dangerous.

On reread, the main reason to keep this file in the current bucket is not only scratch-buffer reuse or launch-shape sensitivity. The file is also carrying a quiet GPU ABI contract in three places at once: shared module loading (`load_double_crossover_module`), shared metric-row decoding (`NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE` plus `decode_non_double_ma_gpu_metrics_row`), and local kernel argument/flag packing (`flags`, `commission`, `atr_stop_mult`, `max_trade_bars`, `hurdle_tau`) across lines `27`, `100-106`, `295-296`, and `399-500` of the live file, together with the shared loader in `gpu_backtest_kernel.rs:1879-1901`.

That is the part the third-pass packet did not sharpen enough. The bucket still stands, but for stronger reasons than the packet captured.

## Fourth-Pass Research Log

I re-checked the live source first, then validated the CUDA/runtime assumptions against the shared local context and primary-source docs.

1. **Live ROCB wrapper**
   - `rocb_gpu_kernel.rs:29-47`, `89-135`, `137-213`, `261-529`
   - Why it mattered:
     - these lines define the real public-ish CUDA contract for this strategy: raw device pointers, row/stride assumptions, resident close/LR reuse, scratch reuse, and host decode.

2. **Shared module loader**
   - `gpu_backtest_kernel.rs:1879-1901`
   - Why it mattered:
     - ROCB does not load its own module; it depends on the shared double-crossover PTX/CUBIN packaging. That is a compatibility and release-coupling fact, not just an implementation detail.

3. **`cust::context::Context::new` and `CurrentContext::set_current`**
   - `cust` docs say `Context::new(device)` retains the **primary context** for that device, and repeated calls on the same device return the same internal context. `CurrentContext::set_current` sets the given context as current on the calling thread.
   - Why it mattered:
     - this weakens any claim that the wrapper is creating fully isolated per-thread legacy contexts, but it also confirms that `set_current()` is not optional boilerplate. The thread must still have a valid current context before launches.

4. **CUDA Programming Guide, §3.3.1 Context**
   - NVIDIA states that each CUDA context has its own distinct address space, `CUdeviceptr` values from different contexts reference different memory locations, and most CUDA work requires a valid current context on the calling thread.
   - Why it mattered:
     - the wrapper’s raw-pointer ABI is only safe when callers and callee agree on pointer provenance. The primary-context behavior from `cust` helps, but it does not erase the provenance contract.

5. **`cust::memory::DeviceBuffer::uninitialized`**
   - `cust` documents that the caller must ensure the buffer is initialized before any read.
   - Why it mattered:
     - the uninitialized `d_lr` allocation at `279-280` is acceptable only because the compute-log-returns kernel fully writes it before later reads on the same stream.

6. **Rust `thread_local!` and `std::thread` docs**
   - Rust documents that thread-local values are per-thread, internally accessed through shared references plus interior mutability, and destroyed when the thread exits.
   - Why it mattered:
     - this cache is not process-global forever; it is thread-scoped retained state. That nuance matters when discussing cleanup and resource-lifetime risk.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**.

`rocb_gpu_kernel.rs` is still a `Benchmark-sensitive; do not touch yet` file. The difference is that the strongest reason is not merely “GPU hot path” in the abstract. It is that this wrapper is simultaneously:

- performance-sensitive,
- GPU-ABI-sensitive,
- validation-parity-sensitive, and
- shared-module-packaging-sensitive.

That combination makes broad cleanup riskier than the current packet conveyed.

### What the third-pass packet got right

The third-pass packet was directionally correct on four important points.

First, it was right to resist genericization. The file is strategy-local on purpose, and the current structure keeps the device-facing invariants legible enough to audit.

Second, it was right that scratch reuse and resident-series reuse are load-bearing. `RocbKernelScratch` and `ensure_resident_close_lr()` are exactly the kinds of places where “cleaner” abstractions can accidentally add alloc/free churn, extra uploads, or stale-ordering bugs.

Third, it was right that the current code is still safer to improve through contract clarification than through behavioral refactor. The unsafe launch sites at `241-256` and `477-510`, and the uninitialized allocation at `279-280`, do deserve clearer comments.

Fourth, it was right to keep cross-strategy runtime unification deferred. Nothing in the live file newly proves that a shared abstraction would be cheap or low-risk.

### What the third-pass packet missed or underweighted

The biggest miss is that the file is carrying more ABI than the packet acknowledged.

The row layout is not ROCB-local. `ROCB_METRICS_STRIDE` is an alias of `NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE` at `27`, and host decode is delegated to `decode_non_double_ma_gpu_metrics_row()` at `295-296`. That means a shared metric decoder or stride change can silently break ROCB even if this file’s Rust code still compiles cleanly.

The module boundary is also not ROCB-local. `RocbCudaRuntime::new()` calls `load_double_crossover_module(device)` at `105`, and that loader pulls from the shared `double_crossover` CUBIN/PTX packaging in `gpu_backtest_kernel.rs:1879-1901`. So this wrapper is coupled to symbol packaging outside its own file. The third-pass packet described ownership/lifetime risk, but not this release/packaging risk.

The API is not actually device-only. `validate_inputs()` requires `close_f32.len() == cols` even when `close_device_ptr != 0` at `146-148`, and `ensure_resident_close_lr()` keys cache identity from `CloseSeriesKey::from_host_and_device(signals.close_f32, signals.close_device_ptr)` at `265`. That means callers still need a host mirror of close prices even on the device-pointer path. The current packet never called out that contract, and it matters because it weakens any “pure VRAM” interpretation of the wrapper surface.

The third-pass packet also underweighted local semantic normalization. The wrapper silently bakes kernel-facing normalization into host argument packing: it clamps commission to `[0.0, 0.95]` at `422`, forces `max_trade_bars` to at least `1` at `466`, and packs strategy behavior into raw flag bits at `410-420`. Those are not presentation details. They are part of the ROCB GPU semantics and must be preserved for parity.

Finally, the third-pass verification advice regressed from the first-pass packet. The file is cfg-gated behind `#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]` at line `1`, but the third-pass packet suggested `cargo test -p ta_app_core rocb_gpu_kernel -- --nocapture` without the required features. That is too weak for a practical handoff note.

### Claims in the current packet that are too strong or too weak

The strongest overstatement is the implied claim that “the real contract here is ownership and lifetime discipline.” That is only partially true. The live file is also carrying a shared module-loading contract, a shared metrics-row ABI, and local numeric/flag normalization. Those are just as load-bearing as lifetime.

The weakest part is the suggestion that safe-now hardening is basically documentation only. Documentation is the lowest-risk improvement, but it is not the only one. `validate_inputs()` is pure host-side logic, and expanding its unit-test surface would not perturb hot-path behavior, kernel launch shape, or device ABI.

Another weak recommendation is the generic call to “tighten assertions.” At this layer, raw pointer validity is not something you can cheaply or robustly assert away. Adding CUDA pointer-attribute checks here would introduce extra driver calls, new failure modes, and still would not solve all provenance problems. If stronger guarantees are wanted, that is a typed-input redesign, not a small assertion patch.

### Safe-now hardening

There are safe-now improvements, but they should stay narrow.

The first is **contract documentation on the public structs and exported function**. `RocbGpuDeviceSignalMatrices` should explicitly say that:
- `upper_row_major_ptr`, `lower_row_major_ptr`, and `atr_row_major_ptr` are expected to point at row-major `f32` device matrices,
- `channel_stride` and `atr_stride` are element strides, not byte strides,
- all raw device pointers must be valid in the same device/primary-context regime used by this runtime,
- `close_f32` is still required even when `close_device_ptr` is non-zero because it participates in cache identity and host-upload fallback.

That is safe because it changes no behavior and simply makes the existing contract auditable.

The second is **targeted safety comments**. The current packet already pointed at the `unsafe` sites, but it should also call out the local semantic packers that are easy to “simplify” incorrectly: the shared metrics-stride alias at `27`, the kernel flag packing at `410-420`, the commission clamp at `422`, and the `max_trade_bars.max(1)` normalization at `466`. Those comments would reduce refactor risk without changing generated code in any meaningful way.

The third is **more file-local host-only tests for `validate_inputs()`**. Safe additions would cover:
- null signal pointers,
- close-length mismatch,
- combo vector length mismatch,
- invalid stride smaller than `cols`,
- zero-row / zero-stride shape rejection.

Those tests do not require a GPU, do not change hot-path dispatch, and directly harden the most brittle host-side preflight logic in the file.

### Larger redesign still deferred

The larger changes remain deferred.

The first deferred redesign is replacing raw `u64` device pointers with typed device handles that carry device provenance and shape. That would be a real improvement, but it would change caller contracts and likely ripple into composite optimization/validation code.

The second deferred redesign is decoupling ROCB from the shared `double_crossover` module loader and the shared non-double-MA metrics decoder. Today that coupling is real. Breaking it apart could improve reviewability and release isolation, but it also touches PTX/CUBIN packaging, symbol availability, and validation parity.

The third deferred redesign is making the close-series contract truly device-native by removing the mandatory host mirror on the `close_device_ptr != 0` path. That would improve API clarity and reduce host coupling, but it changes cache-key semantics and caller expectations, so it should not be done opportunistically.

The fourth deferred redesign is any attempt to make this path more asynchronous or pipelined. The unconditional `stream.synchronize()` at `513` and subsequent host decode at `514-528` are semantically simple and predictable. Turning that into overlapped execution would require a broader ownership and lifecycle redesign, not a small cleanup.

## Implementation Priority

### 1. Safe-now hardening

Keep this limited to documentation and host-only test expansion.

If anything is changed now, the priority order should be:
1. rustdoc and inline comments for the raw-pointer, stride, and host-mirror contracts,
2. safety comments for the `unsafe` launch and uninitialized-allocation sites,
3. small pure-host test additions around `validate_inputs()`.

Those changes are conservative and do not threaten hot-path behavior, GPU ABI stability, or benchmark posture.

### 2. Larger redesign still deferred

Defer:
- typed device-matrix handles,
- shared loader/ABI decoupling,
- device-only close input redesign,
- async/pipelined output work,
- any launch-shape retuning,
- any cache-key or resident-series policy change.

Every one of those items can regress either performance, compatibility, or parity unless measured and revalidated.

## Verification If Changed

If only documentation or host-only validation tests change, the minimum useful check is:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel rocb_gpu_kernel::tests:: -- --nocapture`

If the public contract, loader coupling, or ROCB caller surface changes, also run:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

If anything touches:
- kernel argument packing,
- metrics stride / decoder coupling,
- resident-series reuse,
- launch-shape candidates, or
- shared module loading,

then this is no longer “doc-only” work and it should also go through the repo’s GPU performance gate (`scripts/run_gpu_perf_gate.ps1 -Mode candidate`) on a representative GPU-enabled setup.

The third-pass packet’s simplified `cargo test -p ta_app_core rocb_gpu_kernel -- --nocapture` command is not strong enough for this cfg-gated file.

## Regression And Feature-Preservation Constraints

Do not remove or weaken `CurrentContext::set_current()` on the theory that the runtime is already thread-local. CUDA still requires a valid current context on the calling thread, and `cust`’s primary-context model makes that call semantically important, not ornamental.

Do not change the raw kernel flag packing, commission clamp, hurdle-tau packing, or `max_trade_bars.max(1)` normalization unless CPU/GPU ROCB parity is explicitly revalidated. Those are local semantics, not cleanup noise.

Do not treat `ROCB_METRICS_STRIDE` as a private constant. It is coupled to a shared non-ROCB decoder contract. Any change there is effectively a GPU ABI change for this wrapper.

Do not decouple or rename ROCB kernel symbols casually. The file currently depends on the shared `double_crossover` module packaging, so compatibility and release risk live outside the file itself.

Do not assume the thread-local runtime is a process-global leak. Rust thread-local storage is per-thread and is destroyed when the thread exits. The real risk is retained CUDA/module/stream state on long-lived worker threads, not immortal global state.

Do not replace the required local unsupported-feature guard (`overlays`, `stop-loss`, `take-profit`) with only an upstream preflight. Duplicating the guard upstream may improve UX, but this wrapper still needs its own authoritative rejection unless kernel semantics are extended.

IPC and persistence are not meaningful concerns here. The material risks are GPU ABI drift, CUDA context/provenance assumptions, performance regression, and validation parity.

## Fourth-Pass Conclusion

The current third-pass bucket should **remain unchanged**: `Benchmark-sensitive; do not touch yet`.

The third-pass packet got the broad posture right, but it was too generic. The live file is not merely a lifetime-sensitive CUDA wrapper. It is also a shared-module, shared-decoder, semantics-packing boundary. That makes false-positive refactors more likely than the earlier packet suggested.

The safest near-term work is still narrow:
- document the raw-pointer and host-mirror contract,
- document the unsafe and normalization invariants,
- add a few more host-only validation tests.

Everything more ambitious should stay deferred until someone is prepared to revalidate GPU ABI compatibility, ROCB CPU/GPU parity, and benchmark posture together.
