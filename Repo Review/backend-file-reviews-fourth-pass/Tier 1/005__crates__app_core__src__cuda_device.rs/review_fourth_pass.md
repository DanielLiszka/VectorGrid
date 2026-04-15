# Fourth-Pass Review: crates/app_core/src/cuda_device.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/cuda_device.rs`
- First-pass packet: `review__first_pass.md` (`Repo Review/backend-file-reviews/crates/app_core/src/cuda_device.rs.review.md`)
- Second-pass packet: `review__second_pass.md` (`Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cuda_device.rs.review.md`)
- Third-pass packet: `review__third_pass.md` (`Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cuda_device.rs.review.md`)
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

`cuda_device.rs` is tiny, but it is not low-leverage. The helper trio is publicly re-exported from `ta_app_core` via `crates/app_core/src/lib.rs`, so its semantics are crate-surface behavior, not a private implementation detail. In the local runtime context, selected device IDs feed explicit CUDA context creation and module loading in `crates/app_core/src/gpu_backtest_kernel.rs`, which means device-selection changes can alter real kernel-runtime behavior, not just status text.

That is why the third-pass packet needed to be challenged rather than merely accepted. Its core instinct was good: this file is mostly about operational truthfulness, not throughput. But its framing drifted toward behavior-changing hardening without fully accounting for what the live source actually does, what stable `OnceLock` can and cannot do cleanly, and how much downstream GPU-runtime behavior is coupled to a chosen `device_id`.

## Fourth-Pass Research Log

1. I re-read the live source first. The important concrete behaviors are all visible in `cuda_device.rs`: one `OnceLock<Result<Option<u32>, String>>` for the preferred device ID at lines 12-40, a second independent `OnceLock<Result<Option<PreferredCudaDevice>, String>>` for metadata at lines 49-69, and a lossy convenience wrapper at lines 81-82.

2. I re-checked the local contract surface. `crates/app_core/src/lib.rs` re-exports `preferred_cuda_device`, `preferred_cuda_device_id`, `preferred_cuda_device_id_result`, and `PreferredCudaDevice`, confirming that this is public crate API rather than an internal-only helper.

3. I re-opened the nearby GPU runtime owner. In `crates/app_core/src/gpu_backtest_kernel.rs`, `CudaKernelRuntime::new(device_id)` calls `cust::init`, resolves the concrete `Device`, creates a `Context`, and loads the double-crossover module for that device. The same file also switches between an embedded `sm_89` cubin and PTX/JIT fallback based on the selected device’s compute capability. That raises the blast radius of any device-selection change.

4. I checked the current Rust `OnceLock` docs. `get_or_init` permanently stores the returned value once initialization succeeds, while `get_or_try_init` remains nightly-only experimental, and `take` requires `&mut self`.[^1] That matters because the earlier packet family implicitly treated retry-on-error and reset-like behavior as if they were a trivial `OnceLock` tweak.

5. I checked the current `cust::init` docs and the CUDA driver docs. Both confirm that CUDA driver initialization must happen before other driver/device work in the current process.[^2][^3]

6. I checked NVIDIA’s current multi-GPU guidance. CUDA’s own programming guide says that selecting among devices is an application policy that may need to consider hardware properties, CPU affinity, and peer connectivity. That supports documenting the current “largest total memory wins” rule as product policy, but it does not support marketing it as universally optimal.[^4]

### References consulted

- Rust `OnceLock` documentation: `get_or_init`, `get_or_try_init`, and `take`.[^1]
- `cust::init` documentation.[^2]
- NVIDIA CUDA Driver API initialization and driver-model overview.[^3][^5]
- NVIDIA multi-GPU device-selection guidance.[^4]

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**, but only in a narrower sense than the earlier packets implied. `Small surgical refactor` is still the right size classification for behavior-preserving hardening such as rustdoc, comments, and narrowly targeted tests. It is **not** the right framing for retryable cache semantics, runtime reset behavior, or broader device-selection redesign. Those are larger coordination items.

### What the current third-pass packet got right

The third-pass packet was right on three important points.

First, the file really does have process-lifetime sticky semantics today. Because both caches store `Result<...>` inside `OnceLock` and use `get_or_init`, the first successful initialization of the cell stores either success or failure and keeps it for the rest of the process under the current design.

Second, the selection heuristic should be treated as explicit product policy, not as a secretly “smart” scheduler. The live code chooses the device with the largest `total_memory()` and keeps the first enumerated device on ties (`cuda_device.rs:28-38`).

Third, the packet was right to keep major heuristic redesign deferred. Once a `device_id` is chosen, nearby runtime code turns that into a real CUDA context and module load path, so heuristic changes are not cosmetic.

### What it missed or underweighted

The biggest missed point is not generic “sticky bad early state.” It is the **existence of two independently sticky truths**.

`preferred_cuda_device_id_result()` caches only the chosen ID. `preferred_cuda_device()` then performs a second, separate metadata hydration pass under a different `OnceLock`, repeating `cust::init`, `Device::get_device(device_id)`, `device.name()`, and `device.total_memory()` (`cuda_device.rs:49-69`). That means the two public helpers can permanently diverge inside one process if ID resolution succeeds but metadata hydration fails. The second-pass packet noticed this. The third-pass packet largely collapsed it into a more abstract “stale snapshot” discussion, which is weaker than the actual risk.

That divergence matters more than the packet admitted because callers can observe conflicting truths: one helper can say “a preferred device ID exists,” while the metadata helper can stay stuck in `Err(...)`. For a public runtime helper, that is a more concrete contract issue than the broader idea that an early snapshot is sticky.

The third-pass packet also underweighted the GPU-runtime consequences of changing selection semantics. In local context, `gpu_backtest_kernel.rs` uses the selected `device_id` to create a device-bound `Context`, construct `VramMaComputer::new(device_id)`, and choose between an `sm_89` cubin fast path and PTX/JIT fallback based on the concrete device. On a multi-GPU host, a selection-policy change can therefore alter startup cost, JIT behavior, benchmark posture, and even which compiled artifact path is exercised.

Finally, the third-pass packet underweighted how little of the downstream caller graph was actually reopened in this bundle. Earlier packets point to startup prewarm, GPU-status reporting, options-expression flows, and benchmark/tooling consumers. Those may all be valid concerns, but they were not all live-reopened here. That means aggressive call-site rewiring should stay out of the safe-now set.

### What it overstated

The strongest overstatement is the implied ease of making discovery retryable or resettable.

On stable Rust, the obvious `OnceLock` API that would avoid storing failed initialization results, `get_or_try_init`, is still nightly-only experimental.[^1] Likewise, `OnceLock::take` requires `&mut self`, which is not available for the current `static` caches.[^1] So a real retry-on-error or resettable-cache design is not a tiny local patch. It requires a different synchronization/state model, such as a mutex-protected state machine or another explicit cache wrapper. That is beyond what the third-pass packet’s tone suggested.

The packet also leaned too hard on “transient enumeration failure” language. Such failures can happen, but the current evidence base does not show that they are common or that implicit retry is safer than stable process-lifetime diagnostics. CUDA initialization and device-discovery errors are often environment or driver state issues, not harmless one-off blips.[^2][^3] Without a deliberate refresh boundary, automatic retry can make diagnostics and optional GPU routing flap.

One smaller overstatement: the suggested verification command `cargo test -p ta_app_core cuda_device -- --nocapture` reads like an existing direct test target, but the live file currently has no direct tests. That is not a blocker, but the fourth-pass packet should be honest about it.

## Implementation Priority

### Safe-now hardening

The safe-now set is narrow and behavior-preserving.

Add rustdoc to every public item in this file. The docs should explicitly state that CPU-only builds return `Ok(None)`, that CUDA builds cache process-lifetime results under the current design, that multi-device auto-selection currently means “largest total VRAM wins, first enumerated device wins ties,” and that `preferred_cuda_device_id()` is the lossy convenience helper that converts `Err` to `None`.

Add a small amount of local structure only if it directly improves testability without changing behavior. The best candidate is extracting the “pick best device by total memory, first tie wins” rule into a tiny pure helper so it can be unit-tested without a CUDA environment. In the same pass, add CPU-build tests for the non-CUDA stubs and for the lossy convenience wrapper. That locks the documented contract without inventing a fake CUDA runtime.

If a comment-level clarification is added, make it explicit that `preferred_cuda_device()` is not just formatting the cached ID; it is a second metadata-discovery phase with its own cache.

### Larger redesign still deferred

Any change that makes cache behavior retryable, refreshable, or resettable should be deferred. That is not a trivial `OnceLock` cleanup; it requires a different state-holder design and a fresh review of startup prewarm, GPU-status reporting, and any optional-vs-required GPU call sites.

Any attempt to unify the ID cache and metadata cache should also be treated as redesign work, even if it stays small in line count. Conceptually it is the right future direction, because the current file really does expose two independently sticky truths. But once those truths are unified, the error model and user-visible diagnostics can change, so the exact downstream callers need to be reopened first.

Any heuristic change beyond documenting the current rule remains deferred. That includes switching from total memory to free memory, incorporating compute capability or peer topology, or adding a user-configurable override. Those may be valuable later, but on this repo they affect benchmark comparability, runtime context creation, and which GPU-kernel artifact path gets exercised.

If later work does reopen behavior, the first design target should be **one authoritative preferred-device state model**, not “retry everything by default.”

## Verification If Changed

If the file only receives docs, comments, or new behavior-preserving unit tests, `cargo test -p ta_app_core` is the honest minimum verification lane. For a broader non-options hardening pass, the repo-quality gate from the routing docs is appropriate.

If any behavior changes are made to cache semantics, helper naming, or device-selection policy, verification has to widen. At minimum that means re-running `cargo test -p ta_app_core`, checking the Tauri-side GPU-status surface because `get_gpu_status` reports current device-discovery status, and doing a manual smoke on both a CPU-only build and a CUDA-enabled host.

If device-selection behavior changes on the CUDA path, add a GPU-specific validation step. In this repo that means a candidate-mode GPU perf verification lane and a targeted manual check on a multi-GPU machine, because selection changes can move work onto a different device and can switch the kernel runtime between the `sm_89` cubin path and PTX/JIT fallback.

Finally, keep the repo’s benchmark hygiene intact: run one workflow/perf/profile script at a time. A helper that can redirect workloads between GPUs is benchmark-sensitive even if it is not hot-path code.

## Regression And Feature-Preservation Constraints

Preserve the current CPU-only fallback contract. `preferred_cuda_device()` and `preferred_cuda_device_id_result()` returning `Ok(None)` without CUDA is a stable and useful crate-surface behavior.

Preserve current public API names unless there is an explicit compatibility review. The plain-name lossy helper is arguably too implicit, but renaming it is public-surface churn, not casual cleanup.

Do not silently change process-lifetime cache semantics under the same helper names. Stable status may be part of the current product contract even if it is imperfect.

Do not change auto-selection policy casually on multi-GPU hosts. In this codebase, the chosen `device_id` does not just affect status output; it affects CUDA context ownership, module loading, and potentially whether the runtime uses architecture-specific cubin or PTX/JIT fallback.

Do not promise early-failure retry or refresh behavior without designing an explicit state/reset model first. The current `static OnceLock` shape does not provide that for free.

If later work narrows lossy-helper usage at call sites, reopen those exact live files before acting. Optional paths, required-GPU paths, and benchmark skip/report logic do not all want the same failure semantics.

## Fourth-Pass Conclusion

The third-pass bucket should remain `Small surgical refactor`, but the safe interpretation of that bucket is narrower than the prior packets implied.

What the current third-pass packet got right is the broad shape: this file is a small but public GPU-runtime helper, sticky cache semantics are real, and heuristic redesign should stay deferred. What it still missed is the sharper contract weakness: the live source maintains two independently sticky caches, one for the preferred device ID and one for preferred-device metadata, and those helpers can disagree permanently inside one process.

What it overstated is the ease and safety of making the cache retryable or resettable. On the current stable `OnceLock` surface, that is not a one-line cleanup; it is a different synchronization design with user-visible consequences. Because the selected device also feeds real CUDA context creation and module loading nearby, any behavior change here carries runtime, diagnostics, and benchmark risk.

So the fourth-pass recommendation is conservative. Keep the bucket. Narrow the safe-now work to documentation, comments, and low-risk tests. Defer retry/reset/cache-unification and heuristic redesign until the exact downstream callers are reopened and the repo is ready to treat this file as an explicit device-selection policy owner rather than a tiny convenience helper.

[^1]: Rust standard library documentation for `std::sync::OnceLock`, including `get_or_init`, `get_or_try_init`, and `take`: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>.
[^2]: `cust::init` documentation: <https://docs.rs/cust/latest/cust/fn.init.html>.
[^3]: NVIDIA CUDA Driver API, Initialization (`cuInit`): <https://docs.nvidia.com/cuda/archive/12.9.1/cuda-driver-api/group__CUDA__INITIALIZE.html>.
[^4]: NVIDIA CUDA Programming Guide, multi-GPU systems and device-selection considerations: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/multi-gpu-systems.html>.
[^5]: NVIDIA CUDA Programming Guide, Driver API and context/module model: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/driver-api.html>.
