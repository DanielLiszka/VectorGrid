# Fourth-Pass Review: crates/app_core/src/options_expression_gpu.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/options_expression_gpu.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally useful, but it still left two important things insufficiently challenged.

First, it treated cache operability and poison handling as the strongest remaining issues without re-checking whether the live source had a more consequential local mismatch. After re-reading the file against the current callers in `options_expression.rs`, the sharper remaining issue is not the cache-function name by itself. It is that the feasibility inspector and the live scorer do not enforce exactly the same preconditions, and that the combined-path chunk planner models more per-launch memory than the current runtime actually uses in the current pipeline.

Second, the earlier packet chain continued to carry forward the claim that this file is a distinct CUDA-context owner in the strong NVIDIA sense. That needed a re-check against current `cust` semantics before it stayed in the review story. That re-check materially changes how strong that claim should be.

So this fourth pass is not here to replace the third-pass bucket. It is here to keep the bucket, but correct the emphasis.

## Fourth-Pass Research Log

I reviewed the live source first, then the third-pass packet, then earlier packet history, and only then pulled in the local context files that actually constrain safe recommendations:

- `source__crates__app_core__src__options_expression_gpu.rs`
- `review__third_pass.md`
- `review__second_pass.md`
- `review__first_pass.md`
- `context__03__crates__app_core__src__options_expression.rs`
- `context__04__crates__app_core__src__gpu_backtest_kernel.rs`
- `context__02__crates__app_core__src__options_types.rs`
- `context__AGENTS.md`
- `context__REPO_MAP.md`

External research used to challenge or narrow the packet claims:

1. `cust` context docs/source.
   - Current `cust` docs say primary contexts are the default, only a single context exists per device, `Context::new(device)` on the same device returns the same internal context, and `CurrentContext::set_current` is the per-thread current-context binding point.
   - Relevant source/docs:
     - `cust/context/mod.rs`
     - <https://docs.rs/cust/latest/src/cust/context/mod.rs.html>

2. NVIDIA CUDA Programming Guide.
   - I re-checked two specific constraints:
     - kernels from different CUDA contexts cannot execute concurrently;
     - host-involved asynchronous copies require page-locked host memory to overlap.
   - Relevant docs:
     - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/>
     - <https://docs.nvidia.com/cuda/cuda-programming-guide/02-basics/asynchronous-execution.html>

3. `cust::memory::DeviceBuffer` safety docs.
   - I re-checked the exact requirement around `DeviceBuffer::uninitialized(...)`: caller must initialize before reading.
   - Relevant docs:
     - <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>

4. Rust `Mutex` poisoning docs.
   - I re-checked whether “recover from poison” is actually the conservative recommendation for this runtime.
   - Relevant docs:
     - <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
     - <https://doc.rust-lang.org/src/std/sync/poison/mutex.rs.html>

## What Fourth Pass Sharpened

### 1. The third-pass bucket should stand, but the rationale should change

`Small surgical refactor` is still the right bucket.

The strongest remaining work is still local to this file and its immediate caller. Nothing I found requires a broad architectural rewrite, a new IPC contract, or a GPU ABI break. The live code still points to bounded hardening: validation parity, tighter accounting truthfulness, safer poison handling, and documentation cleanup.

What changes in the fourth pass is priority. The third-pass packet over-centered cache naming and panic-on-poison. Those are real, but they are not the best next intervention.

### 2. What the current third-pass packet got right

The current third-pass packet was right about three things.

It was right that `evict_oldest_if_needed()` is misnamed relative to its actual behavior. The function removes `cache.keys().next()`, which is lexical `BTreeMap` order, not insertion order or recency order (`options_expression_gpu.rs:1092-1098`). That is a truthfulness issue worth cleaning up.

It was right that `.expect(...)` on the runtime-store lock and the per-device runtime lock is an operationally harsh choice for a surface that otherwise returns `Result<_, String>` (`options_expression_gpu.rs:1108-1136`). A panic here can turn later work into repeat panics instead of a recoverable caller-visible error.

It was also right to preserve the current device-resident reuse posture and the current correctness-first sync/copy choreography unless profiling justifies change. This file caches uploaded tensors and reusable output buffers (`options_expression_gpu.rs:568-572`, `617-700`), and NVIDIA still documents that overlap with host memory requires page-locked buffers. That keeps the current serialized launch/sync/copy posture reasonable until a larger pinned-memory path exists.

### 3. The stronger missed issue is inspector/scorer contract drift

The higher-value missed issue is that the feasibility inspector does not fully mirror the live scorer’s preconditions.

In the live scorer paths:

- single-lane scoring checks tuple-offset parity before launch (`options_expression_gpu.rs:710-712`) and outcome-tensor parity when uploading outcomes (`435-451`, `736`);
- combined scoring checks per-lane tuple-offset parity and identical tuple counts across lanes (`823-837`) and then validates per-lane outcome tensor parity through `ensure_outcome_inputs(...)` (`870-872`, `643-661`).

But the inspector paths are weaker:

- `inspect_plan(...)` checks only tuple-offset parity and then proceeds directly to chunk-plan construction (`1015-1047`); it does **not** validate `outcome_valid.len() == outcome_pnl.len() == outcome_premium.len()`.
- `inspect_combined_plan(...)` checks only cross-lane tuple counts (`1050-1088`); it does **not** validate per-lane tuple-offset parity, and it does **not** validate per-lane outcome-tensor parity.

That matters because the inspector is not a dead helper. `options_expression.rs` uses it in `inspect_gpu_plan_for_feasibility(...)` as part of GPU-feasibility probing and auto-backend decision flow (`options_expression.rs:102-132`). So the current code can report a viable GPU plan in a case where the live scorer would later reject the same inputs during upload or launch preparation.

This is not a claim that the current prepared-path inputs are malformed today. In the shipped path, `GpuFlatScoreInputs` are built from structured caches in `build_gpu_flat_score_inputs(...)` (`options_expression.rs:224-287`), so the current pipeline probably keeps these lengths aligned. But that is exactly why this is a good small fix: it is local, low-risk, and makes the inspector truthful instead of “probably okay because upstream is disciplined.”

### 4. The combined chunk planner is more pessimistic than the current combined runtime

This is the most concrete missed performance/telemetry issue.

The current combined planner assumes three shared-input lanes:

- `for_combined_inputs(...)` calls `for_layout(..., shared_input_lane_count = 3, aggregate_lane_count = 4, extra_output_bytes_per_tuple = size_of::<f64>())` (`options_expression_gpu.rs:369-389`).

But the current runtime does not necessarily materialize three independent shared-input uploads.

`GpuFlatScoreInputs.shared_key` is derived only from offset layout and strides, not from stress-mode-specific outcome tensors (`options_expression.rs:224-287`). In the current shipped pipeline, baseline, conservative, and harsher lanes are built from the same compiled tuple grid and the same offset-layout logic, so they share the same layout key. Then `ensure_shared_inputs_chunk(...)` caches chunk uploads by `inputs.shared_key` plus tuple range (`options_expression_gpu.rs:617-640`). In the combined scorer, the file calls that helper three times per chunk (`912-929`), which means the second and third calls become cache hits when the keys match.

So the live combined runtime, in the current pipeline, reuses one shared-input chunk upload across the three stress lanes. The planner still prices it as three.

That is not just a naming nit. It changes the working-set model:

- current per-tuple planner model for combined path:
  - shared inputs: `8 * 3 = 24` bytes
  - aggregate outputs: `24 * 4 = 96` bytes
  - robust score: `8` bytes
  - total: `128` bytes/tuple
- current runtime when shared layout is deduplicated across lanes:
  - shared inputs: `8` bytes
  - aggregate outputs: `96` bytes
  - robust score: `8` bytes
  - total: `112` bytes/tuple

That is roughly a 14% overstatement on the combined per-tuple working-set estimate before even discussing warm-cache reuse.

The impact is not only theoretical:

- it can shrink `tuple_chunk_size` unnecessarily;
- it inflates `gpu_shared_input_bytes_per_chunk` and `gpu_total_device_bytes_per_chunk` in cache/feasibility stats (`options_expression.rs:5478-5510`);
- and if the pessimistic combined plan errors, `options_expression.rs` silently falls back to the legacy multi-launch path (`options_expression.rs:3499-3520`).

That last point matters. A too-pessimistic planner here does not just make a statistic ugly. It can silently turn one combined launch regime into the slower legacy route without surfacing a real reason to the operator.

This is the strongest safe-now hardening target I found.

### 5. There is a larger, still-deferred planner/accounting issue on warm caches

The current planner computes free memory before launch (`options_expression_gpu.rs:714-720`, `843-851`) and then budgets outcome tensors, shared inputs, and reusable outputs as if they still need to fit into that free-memory window for the current call.

That is reasonable for cold-start planning.

It is not exact for warm-cache planning, because the same runtime also retains outcome tensors, shared chunks, and reusable outputs across calls (`568-572`, `617-700`). On a warmed runtime, `mem_get_info()` already sees lower free memory because prior allocations are resident, but the planner then charges some of those bytes again as if they are incremental for the current launch.

I am not classifying that as a safe-now bug fix in the same way as the combined shared-input overcount. The reason is semantic: the current fields may be intentionally read as a conservative “cold or worst-case per-launch budget” rather than “incremental new allocation budget.” Changing that interpretation without a schema note would make artifacts and diagnostics harder to compare over time.

So this is real, but still deferred. The safe-now move is to fix the obvious combined-path overcount and document that the remaining stats are conservative. A fuller warm-cache-aware planner is a larger redesign.

### 6. The “second CUDA-context owner” claim is too strong in the current packet chain

This is the biggest overstatement I found.

The earlier packets repeatedly treated this file and `gpu_backtest_kernel.rs` as separate CUDA-context owners because both call `Context::new(device)` (`options_expression_gpu.rs:576-584`; `gpu_backtest_kernel.rs:1840-1847`). The review chain then imported NVIDIA’s rule that kernels from different CUDA contexts cannot execute concurrently.

That would be a strong concern **if** the code were creating distinct CUDA contexts for the same GPU.

But current `cust` documentation says the opposite of what the earlier packet chain implied:

- primary contexts are the default;
- only a single context exists per device;
- and multiple `Context::new(device)` calls on the same device return the same internal context.

That does not make context handling irrelevant here. The file still owns its own runtime wrapper, its own stream, its own thread-local `set_current()` calls, and its own residency/caching policy. But it does mean the existing packet chain overstates the specific “multiple independent contexts per GPU” debt. The safer wording is: **separate runtime wrappers and current-context choreography remain coordination debt; proven multi-context time-slicing debt is not established by this code as written.**

That is an important correction for a public-facing review packet.

### 7. The poison-handling recommendation also needs narrowing

The third-pass packet was right that repeated panic-on-poison is not desirable here. But “recover from poison” is too broad for a runtime that owns mutable GPU-side caches and reusable buffers.

Rust’s own docs make two points relevant here:

- poisoning is advisory, not a soundness boundary;
- and blindly continuing from a poisoned mutex is not automatically the conservative choice.

In this file, the per-device runtime owns cached uploaded inputs, reusable output buffers, a loaded CUDA module, and a stream (`options_expression_gpu.rs:562-572`). If a panic happens while mutating those structures, the safest bounded response is **not** “just call `into_inner()` and keep using the same runtime.” The safer small fix is one of these two:

- convert poison into a surfaced `Err(...)`; or
- drop and rebuild the per-device runtime entry.

That is narrower, more conservative, and much less regression-prone.

### 8. Range/order guardrails are still real, but they are guardrails, not a current bucket-changing bug

The first-pass packet raised launch-bound range validation, and the third-pass packet downweighted it. The right fourth-pass position is in between.

The live score entry points still do not validate that:

- `event_start <= event_end <= event_count` for range scoring;
- `search_end <= holdout_end <= event_count` for combined scoring.

That is true in `options_expression_gpu.rs` (`749-753`, `839-842`).

But the current shipped caller path derives these values from `split_event_range(...)` in `options_expression.rs` (`3252-3262`) and passes them straight into the GPU scorer (`3499-3502`). So this is not strong evidence of a current product bug. It is a bounded local guardrail that should be added because the GPU file should reject malformed launch bounds early, not because the current caller is obviously broken.

## Implementation Priority

### Safe-now hardening

The current third-pass bucket should **stand**.

But the safe-now order should be:

1. **Make the inspector mirror the scorer.**
   - Add outcome-tensor parity validation to `inspect_plan(...)`.
   - Add per-lane tuple-offset parity and per-lane outcome-tensor parity validation to `inspect_combined_plan(...)`.
   - This is the cleanest truthfulness fix because the feasibility path in `options_expression.rs` relies on these helpers.

2. **Add local launch-bound guardrails.**
   - Reject invalid `event_start/event_end` and `search_end/holdout_end` ordering locally in this file.
   - Keep this as hardening, not as a “current bug” narrative.

3. **Fix combined-plan shared-input accounting for the current deduplicated layout case.**
   - The safe version is not “always use one shared lane.”
   - The safe version is: detect whether the three lanes share the same shared layout (for example via `shared_key` or an explicit equality check on layout inputs), and only then use one shared-input lane in the combined planner.
   - Add a unit test so this remains true if future work ever lets combined lanes diverge structurally.

4. **Narrow poison handling.**
   - Replace `.expect(...)` with either surfaced `Err(...)` or a bounded runtime-entry rebuild.
   - Do **not** recommend `PoisonError::into_inner()` as the default fix here.

5. **Correct the review/story documentation.**
   - Rename or comment `evict_oldest_if_needed()` so it stops implying recency.
   - Add rustdoc to `prewarm_options_expression_gpu_runtime(...)`.
   - Stop describing this file as proven multi-context-per-GPU debt unless the repo moves away from `cust` primary contexts or proves otherwise.

### Larger redesign still deferred

These remain deferred, and should stay deferred:

1. **Warm-cache-aware VRAM accounting.**
   - That is a real improvement target, but it changes the meaning of currently exported cache/feasibility metrics.

2. **Pinned host buffers and overlapping copy/compute.**
   - NVIDIA still requires page-locked host memory for the overlap story. This is bigger than a local cleanup.

3. **Byte-budgeted or recency-aware cache eviction.**
   - Still reasonable future debt, but the combined-plan accounting mismatch is the higher-value local fix.

4. **Any broader cross-runtime architecture change justified by the old context story.**
   - That story is now materially weaker. Any future coordination work should be framed around stream ownership, residency interactions, and instrumentation, not assumed multi-context time-slicing.

## Verification If Changed

At minimum, if this file changes, I would run:

- `cargo test -p ta_app_core options_expression_gpu::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core options_expression::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`

If chunk-plan accounting changes, I would also run the repo’s CUDA perf gate when GPU hardware is available:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

And I would add targeted tests for the exact places the current packets missed:

- inspector rejects mismatched outcome tensor lengths;
- combined inspector rejects per-lane tuple-offset mismatches;
- combined chunk planning uses the deduplicated shared-layout path only when the three lanes actually share layout identity;
- existing cache-usage fields continue to serialize/populate correctly after the accounting fix.

## Regression And Feature-Preservation Constraints

Preserve the current device-resident reuse posture. The caches and reusable output buffers are part of the intended performance design, not accidental clutter.

Preserve the current `shared_key` / `outcome_key` identity behavior unless a coordinated caller-side change is made. The combined-path accounting fix should adapt to those identities, not rewrite them casually.

Do not silently change the interpretation of `gpu_shared_input_bytes_per_chunk`, `gpu_output_bytes_per_chunk`, or `gpu_total_device_bytes_per_chunk` without noting whether they mean conservative cold-start bytes or incremental warm-run bytes. Those values are copied into higher-level feasibility/cache stats in `options_expression.rs` (`5478-5510`), so they already have report/diagnostic consequences.

Do not “fix” poison handling by continuing with a tainted runtime in place. For this file, conservative behavior is erroring out or rebuilding the per-device runtime entry.

Do not keep repeating the earlier “independent CUDA context owner” story as if it were established fact. With current `cust` primary-context semantics, that wording is too strong.

Do not change launch/sync/copy choreography unless profiling and pinned-memory design are in scope. The current synchronous behavior is still defensible in the absence of a larger overlap path.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `Small surgical refactor` is still the correct action bucket.

But the fourth pass changes what deserves attention first.

The current third-pass packet got the cache-name truthfulness issue right, got the panic-on-poison concern right, and was correct to avoid casual performance rewrites. What it missed or underweighted is the more important local mismatch between feasibility inspection and live scoring, and the fact that the combined-path chunk planner currently overstates shared-input memory in the shipped pipeline.

It also carried forward two claims too strongly:

- the code does **not** currently prove that options-expression and Double-MA runtimes own distinct CUDA contexts on the same GPU;
- and “recover from poison” is too loose a recommendation for a runtime that owns mutable GPU-side state.

So the real fourth-pass recommendation is:

- keep the bucket;
- move the top safe-now work to validation parity and combined-plan accounting truthfulness;
- keep larger cache redesign and overlap work deferred;
- and tighten the wording so the public-facing review does not overclaim architectural debt that the live `cust` semantics do not actually establish.
