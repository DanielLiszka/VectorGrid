# Fourth-Pass Review: crates/app_core/src/strategy_gpu_runtime.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/strategy_gpu_runtime.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because it is a public, stateful GPU-runtime surface that sits directly on the compiled-program, upload, and executable-cache boundary. It owns `StrategyHandle` materialization for GPU-capable plans, the process-global compiled-bundle cache, the process-global executable cache, and the current-thread uploaded-program cache (`strategy_gpu_runtime.rs:46-340`). That is already enough surface area to make “mostly fine, just add docs” an unsafe default conclusion.

The third-pass packet was directionally right to keep the bucket small. It was also right that reset/stat scope is flatter than the implementation really is, especially once current-thread upload state enters the picture (`strategy_gpu_runtime.rs:154-176`, `223-266`). But after reopening the live source with the adjacent compiler/runtime files in hand, the sharper remaining issues are not primarily the poisoned-lock path. The more concrete gaps are ABI/fingerprint intake hardening and the semantics of the diagnostic counters under races.

So the current third-pass bucket should **stand**, but the reason it stands is narrower than the third-pass packet implied: this still looks like a cache-owner hardening pass, not a redesign, yet the best safe-now hardening targets are different from the ones the third-pass packet prioritized.

## Fourth-Pass Research Log

I re-read the live `strategy_gpu_runtime.rs` rather than treating the earlier packets as authoritative. The core ownership remains unchanged: `StrategyHandle::from_plan()` eagerly materializes GPU program/ABI data for `GpuUnified` + `gpu_dynamic_supported` plans, and the runtime then reuses process-global compiled bundles plus process-global executable metadata plus a current-thread upload cache (`strategy_gpu_runtime.rs:46-76`, `154-340`).

I also re-opened `strategy_gpu_program.rs` because this runtime file trusts fields that are produced there. That neighboring file matters more than the earlier packets acknowledged. In particular, the compiler path guarantees that freshly compiled programs populate both `compiled_plan_fingerprint_hex` and `executable_fingerprint_hex`, but the serializable structs also mark those fields with `#[serde(default)]`, which means the runtime can still receive deserialized or manually constructed payloads with those fields empty unless this file rejects them (`strategy_gpu_program.rs:63-132`, `184-227`, `229-283`, `335-427`).

I re-opened `strategy_plan.rs` as well, because the compiled-plan fingerprint that seeds this cache story is itself produced there. That file still fingerprints the serialized plan using `DefaultHasher` (`strategy_plan.rs:187-193`), which is relevant when deciding what belongs in safe-now hardening versus a broader redesign.

Finally, I re-opened `gpu_backtest_kernel.rs` to validate what the upload layer actually owns. The uploaded GPU program carries indicator slots, conditions, groups, group-condition indexes, signals, ABI version, fingerprints, and device id, and the probe helper returns a 7-word summary (`gpu_backtest_kernel.rs:2413-2501`). That matters because `strategy_gpu_runtime.rs` currently treats the probe as a useful sanity check, but it only verifies part of the shape (`strategy_gpu_runtime.rs:269-302`).

As background standards, I re-checked the Rust std docs on `Mutex`, `thread_local!`, `OnceLock`, and `DefaultHasher`. Those docs support the prior packets’ general framing about poisoning, thread-local scope, process-lifetime lazy globals, and unstable default hashing across releases. The fourth-pass shift is not that those facts are wrong. It is that, in this specific file, they are not the most important remaining hardening targets.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet got three important things right.

First, it was right to keep the bucket at `Small surgical refactor`. Nothing in the live source justifies a broad redesign inside this file alone. The cache keys are not obviously wrong for fresh current-version handles, and the upload ownership model is intentionally current-thread-local under CUDA (`strategy_gpu_runtime.rs:154-176`, `304-351`).

Second, it was right that reset/stat scope is easy to overread. `reset_gpu_strategy_executable_cache()` clears process-global caches when their mutexes can be acquired, but it only clears the current thread’s upload cache, not every thread’s upload cache (`strategy_gpu_runtime.rs:223-242`). `gpu_strategy_executable_cache_stats()` similarly reports process-global entry counts plus only the current thread’s uploaded entry count (`strategy_gpu_runtime.rs:244-266`). That should stay explicit.

Third, it was right that any recommendation which changes cache-key semantics, upload timing, or ownership boundaries is regression-sensitive. This runtime is coupled to the GPU upload path, and the repo guidance still treats that upload/runtime plumbing as shared infrastructure, not a place for casual policy changes (`gpu_backtest_kernel.rs:2413-2501`; `AGENTS.md` / `REPO_MAP.md` context).

### What the third-pass packet missed or underweighted

#### 1. ABI and fingerprint intake validation is the sharper safe-now gap

The biggest underweighted issue is not poisoned-lock reporting. It is that this file trusts incoming ABI payloads more than its own cache invariants justify.

`StrategyHandle` is a public serializable container with public `gpu_program` and `gpu_program_abi` fields (`strategy_gpu_runtime.rs:46-53`). Fresh handles produced by `StrategyHandle::from_plan()` are safe because they come from `get_or_build_compiled_gpu_predicate_bundle()` and therefore from the current compiler path (`strategy_gpu_runtime.rs:56-76`, `188-220`). But this file does not only operate on freshly built handles. Its public API accepts any `StrategyHandle` with a populated `gpu_program_abi`.

That matters because the producer-side types in `strategy_gpu_program.rs` default the very fingerprint fields that this runtime uses as cache identity. Both `CompiledGpuPredicateProgram` and `CompiledGpuPredicateProgramAbi` mark `compiled_plan_fingerprint_hex` and `executable_fingerprint_hex` with `#[serde(default)]` (`strategy_gpu_program.rs:63-71`, `119-126`). Fresh compilation fills those fields (`strategy_gpu_program.rs:413-427`), but deserialized or manually constructed handles can still reach this file with those fields empty.

`get_or_build_gpu_strategy_executable()` then builds the executable-cache key from `handle.gpu_program_abi.executable_fingerprint_hex` plus `device_id`, and `GpuStrategyExecutableCacheKey::new()` stamps the **current** `STRATEGY_GPU_EXECUTABLE_ABI_VERSION` into the key regardless of the incoming ABI payload’s own `abi_version` (`strategy_gpu_runtime.rs:104-119`, `304-315`). `GpuStrategyExecutable::from_handle()` and the upload path still trust the incoming ABI payload’s version and shape (`strategy_gpu_runtime.rs:121-152`, `269-302`).

That combination is a real contract weakness. It means the key’s “ABI version” field is a current-build tag, not a validated fact about the payload being inserted or uploaded. For fresh compiler-produced handles that is fine because the producer and consumer are aligned. For stale, deserialized, or manually assembled handles it is not fine. The likely failure modes are not guaranteed production bugs in today’s bundle, but they are concrete enough to justify safe-now hardening:

- empty/default fingerprint strings can collapse unrelated payloads onto the same `(fingerprint, device)` key;
- a non-current `abi_version` payload can still be admitted under a key that advertises the current ABI version;
- nothing in this file rejects “ABI present, but not plausibly compiler-produced by this build.”

That is a compatibility / persistence / GPU ABI boundary problem. The earlier packets barely touched it, and the third-pass packet did not elevate it at all.

The bounded fix is still small: add an explicit validation gate before cache lookup/upload that rejects ABI payloads with empty fingerprint fields and rejects non-current ABI versions. The important constraint is what **not** to do: do not silently auto-upgrade or auto-recompute those fields inside this file, because that would turn a cache owner into an artifact-migration layer.

#### 2. The miss counters are not “true misses” under races

The second underweighted issue is the semantics of the cache counters.

Both `get_or_build_compiled_gpu_predicate_bundle()` and `get_or_build_gpu_strategy_executable()` use the same double-checked pattern: check the map under the mutex, drop the lock, do expensive work, reacquire the mutex, insert with `entry(...).or_insert_with(...)`, then unconditionally increment the miss counter (`strategy_gpu_runtime.rs:188-220`, `304-340`).

That pattern preserves correctness of the stored entry, but it does **not** preserve a “one miss equals one inserted cache entry” meaning. Two threads racing the same cold key can both go down the build path, only one will actually populate the map, and both will still increment the `*_MISSES` counter. So the current counters mean something closer to “cold-path traversals” or “build attempts,” not real misses in the usual cache-analysis sense.

This is not a theoretical nit. The file’s tests explicitly serialize themselves behind `STRATEGY_EXECUTABLE_CACHE_TEST_GUARD` (`strategy_gpu_runtime.rs:361-367`) and therefore can never falsify the single-thread miss accounting assumptions baked into the current assertions (`strategy_gpu_runtime.rs:419-447`, `449-485`, `560-593`).

The third-pass packet did not call this out. It should have, because it is a cleaner and more common diagnostics problem than the poisoned-lock case. The safe-now fix is again small: either document that these counters are cold-path/build-attempt counters, or make them count only actual vacant insertions after the second lock acquisition. What should be avoided is moving compilation under the mutex just to improve counter purity, because that would create unnecessary contention on a path that is supposed to stay mostly lock-light.

#### 3. The upload probe check is only a partial shape assertion

The runtime’s upload sanity check is also weaker than it first reads.

`ensure_uploaded_gpu_strategy_executable_for_current_thread()` uploads the ABI and then probes it, but it only checks three fields: ABI version, indicator-slot count, and condition count (`strategy_gpu_runtime.rs:283-291`). Meanwhile, the uploaded object itself also carries groups, group-condition indexes, signals, fingerprints, and device id (`gpu_backtest_kernel.rs:2413-2445`), and the probe helper returns a 7-word result (`gpu_backtest_kernel.rs:2449-2501`).

That means the current probe is not a full end-to-end integrity check. It is a useful partial assertion. The difference matters because the review packet should not overstate this as “upload metadata verified” in a broad sense. Right now the code does not prove that group topology or signal-binding shape survived upload in the same way it proves slot and condition counts.

This is **not** a safe-now mandate to expand the check blindly. The broader probe contract lives on the kernel side, and widening the assertion without confirming what the remaining words mean could create false negatives. The right fourth-pass conclusion is simply that the current packet under-described this as a partial check.

### What the third-pass packet likely overstated

The third-pass packet overweights poisoned-lock truthfulness as the first implementation priority.

That concern is real. `reset_gpu_strategy_executable_cache()` clears counters even when it could not clear a poisoned map, and `gpu_strategy_executable_cache_stats()` maps “could not lock” to `0` entries (`strategy_gpu_runtime.rs:223-266`). Later build paths, by contrast, return explicit poisoned-lock errors (`strategy_gpu_runtime.rs:195-197`, `216-217`, `318-320`, `335-336`). So the reporting path really can look cleaner than the build path is.

But this is still not the sharpest remaining hardening target. In this file the mutex-protected critical sections are small `HashMap` operations and clones. By comparison, stale/default ABI intake and race-loose miss counters are normal operational edge cases that can happen without any panic path at all. So poisoned-lock reporting should remain on the list, but it should be demoted below the two issues above.

### Safe-now hardening vs larger redesign still deferred

The safe-now hardening work is narrower than a redesign.

Safe now:

1. Add a small validation gate for incoming `gpu_program_abi` payloads before executable-cache lookup and upload. At minimum, reject empty `compiled_plan_fingerprint_hex` / `executable_fingerprint_hex` and reject non-current `abi_version` values. If both `gpu_program` and `gpu_program_abi` are present, an additional consistency check is reasonable, but even that is secondary to rejecting empty/defaulted identity.
2. Make miss-counter semantics honest. Either count only real insertions after the second lock acquisition, or document/rename the fields so they are clearly “cold-path attempts.” Do not serialize compilation under the mutex just to make the counters cleaner.
3. If the stats/reset surface is touched, make any poisoned-state reporting additive rather than silently changing the meaning of existing fields. The third-pass packet was right that hiding poisoned state behind `0` is misleading, but the least risky fix is an explicit signal, not a breaking semantic rewrite.

Still deferred:

1. Stable cross-release fingerprinting. `strategy_plan.rs` and `strategy_gpu_program.rs` both fingerprint serialized payloads through `DefaultHasher` (`strategy_plan.rs:187-193`; `strategy_gpu_program.rs:184-227`). If the project wants fingerprints that remain stable across compiler/runtime releases or serialized artifact exchange, that is a coordinated producer/consumer redesign, not a one-file patch here.
2. Public-handle ownership and duplication policy. `StrategyHandle::from_plan()` still clones the compiled GPU program and ABI into each handle even though a process-global compiled-bundle cache also exists (`strategy_gpu_runtime.rs:56-76`, `154-220`). That may be acceptable today because the handle is a public serializable surface. Reworking it would be larger than this pass should recommend.
3. Broadening reset semantics across threads or devices. The current upload cache is thread-local by construction (`strategy_gpu_runtime.rs:167-176`). Any attempt to make `reset_gpu_strategy_executable_cache()` mean “clear every uploaded program everywhere” would need deliberate cross-thread coordination and device-lifetime review.
4. Expanding upload-probe validation beyond the first three words before the kernel-side probe contract is explicitly reviewed.

## Implementation Priority

1. **First priority: validate incoming ABI identity before cache use.**
   Add a local guard that rejects empty/defaulted fingerprint fields and rejects non-current ABI versions before building a `GpuStrategyExecutableCacheKey` or uploading to the current-thread cache. This is the highest-value bounded change because it hardens a real public/serde boundary without altering fresh-handle behavior.

2. **Second priority: fix or relabel miss-counter semantics.**
   Keep the compile/upload work outside the mutex, but only increment misses on actual vacant insertion after the second lock acquisition, or explicitly rename/document the counters as build-attempt counters. This preserves performance while making diagnostics trustworthy.

3. **Third priority: only then address poisoned-lock reporting clarity.**
   If the team wants the stats/reset path to surface poisoned state honestly, prefer additive reporting over silent semantic changes. The packet should not encourage a breaking reset/stat API churn just to fix an edge-case observability issue.

4. **Deferred redesign: stable digest and artifact-compatibility policy.**
   If this runtime is expected to ingest serialized GPU handles or artifacts across releases, the fingerprint story needs a broader design review in `strategy_plan.rs` and `strategy_gpu_program.rs`, not a local patch here.

## Verification If Changed

Run the existing targeted tests first:

- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`

If CUDA is available, also rerun the upload-specific runtime test on the real feature surface:

- `cargo test -p ta_app_core strategy_gpu_runtime::tests::uploaded_executable_is_available_after_cache_build --features cuda,cuda-backtest-kernel -- --nocapture --test-threads=1`

Add targeted new coverage if the file changes:

- a runtime test that rejects a `StrategyHandle` carrying a `gpu_program_abi` with empty/defaulted fingerprint fields;
- a runtime test that rejects a non-current `abi_version` payload rather than keying it as if it were current;
- a multithreaded same-key race test proving `entries == 1` and clarifying whether miss counters represent actual insertions or build attempts;
- if poisoned-state reporting is changed, a targeted test that verifies the stats/reset surface does not silently claim a clean reset when the cache lock could not be acquired.

## Regression And Feature-Preservation Constraints

Preserve the current behavior for fresh, compiler-produced handles. The earlier packets were right to be conservative here. Do not change cache-key shape for current-version handles, and do not disturb the current-thread upload ownership model unless the paired runtime path is reviewed end to end.

Do not implement a “helpful” in-place upgrade path for stale or partially populated ABI payloads inside this cache owner. That would silently expand this module’s job from cache/runtime orchestration into persistence/compatibility repair, and it would make failures harder to reason about.

Do not move program compilation or upload work under the cache mutex merely to make counters look nicer. The correct fix for the counter issue is post-lock insertion accounting or clearer naming, not extra serialization.

Do not change `reset_gpu_strategy_executable_cache()` to imply or attempt cross-thread upload-cache invalidation unless the design is explicitly widened. Right now the name already reads broader than the implementation. Widening the implementation without a larger lifecycle review would be riskier than documenting the current scope.

Do not change fingerprint generation in `strategy_gpu_runtime.rs` alone. If stable digests are required for persistence or IPC, that change belongs with the fingerprint producers in `strategy_plan.rs` and `strategy_gpu_program.rs` so the whole pipeline moves together.

If the upload-probe validation is expanded, keep it aligned with the kernel-side probe ABI. The current check is partial, but a wrong “stronger” check is worse than an explicitly partial one.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `strategy_gpu_runtime.rs` is still a `Small surgical refactor`, not a redesign.

What the third-pass packet got right is the overall caution level. It was right to preserve current cache-key and upload ownership behavior, and it was right that reset/stat scope is less global than the file can make it sound.

What it missed is the sharper contract weakness: this runtime accepts incoming ABI identity too trustingly for a public serializable handle surface, and its miss counters are looser than their names imply once races are introduced. Those are better fourth-pass targets than making poisoned-lock reporting the headline issue.

So the fourth-pass recommendation is: keep the bucket, but change the priority order. Harden ABI/fingerprint intake first. Make counter semantics honest second. Treat poisoned-lock reporting as a smaller additive cleanup after those two are addressed. Defer stable fingerprinting, ownership reshaping, and broader reset/invalidation semantics until they are reviewed as cross-file design work rather than local cleanup.
