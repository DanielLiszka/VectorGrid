# Fourth-Pass Review: `crates/app_core/src/dynamic_indicator_adapter.rs`

**Review date:** 2026-04-16  
**Source file:** `crates/app_core/src/dynamic_indicator_adapter.rs`  
**First-pass packet:** `review__first_pass.md`  
**Second-pass packet:** `review__second_pass.md`  
**Third-pass packet:** `review__third_pass.md`  
**Third-pass action bucket:** `Small surgical refactor`  
**Fourth-pass action bucket:** `Small surgical refactor`  
**Packet status:** `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet correctly resisted a broad rewrite, but it compressed the remaining risk too far toward naming and cache-policy framing. Reopening the live source shows one sharper issue that was still underweighted: the prepared CUDA cache is invalidated only by a pointer-and-length signature, while repo context explicitly says loaded datasets can mutate in memory and that live bars mutate loaded data by `data_id` (`dynamic_indicator_adapter.rs:253-260`, `478-487`, `689-709`; `REPO_MAP.md:1537-1562`). That is not just a documentation nuance. It is a plausible stale-buffer correctness hazard.

This pass also rechecked whether earlier public-surface warnings were perfectly calibrated. The unchecked/direct CUDA helper is still too sharp for casual use, but its blast radius is narrower than the earlier packet implied because it is not on the crate’s flattened re-export surface in `lib.rs`; it remains reachable through the public module path, so the issue is real, just more bounded (`lib.rs:28`, `165-173`, `288-289`).

My conclusion is that the third-pass bucket should **stand**, but only for the immediate next patch set. The file still wants contract hardening, tests, and diagnostics first. A robust fix for cache invalidation is larger than that and should stay explicitly deferred rather than being smuggled into a “small” patch and overclaimed as solved.

## Fourth-Pass Research Log

I reread the live source with attention to four surfaces: cache identity and rebuild policy, CUDA fallback/error behavior, public versus merely module-public API exposure, and tests. I also rechecked `crates/app_core/src/lib.rs` to see what is actually re-exported, and `REPO_MAP.md` to validate whether in-memory candle mutation is part of the repository’s real runtime model.

For standards and framing, I revisited the Rust API Guidelines sections on documentation, error/panic disclosure, and using narrower visibility or hidden docs for implementation-detail surfaces; the Rust standard-library docs for `thread_local!` and `OnceLock`; and NVIDIA’s CUDA best-practices guidance around systematic error checking and realistic profiling. Those sources reinforce the same practical bar here: public or quasi-public CUDA helpers should make failure, panic, ownership, and scope semantics legible, and performance-sensitive fallback behavior should not silently destroy route observability.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep this file out of broad refactor territory. The selective-upload prepared-input design is still a real strength, the device-slice row helper still does the right basic bounds/overflow checks before the unsafe projection, and the file still reads like an adapter whose immediate problems are contract clarity and policy sharp edges rather than algorithmic chaos (`dynamic_indicator_adapter.rs:533-583`, `1119-1149`, `1554-1680`).

It was also right that thread-local cache and stats behavior should not be described as durable global runtime truth. That concern remains valid because both the cache and dispatch stats are thread-local, and the exported `take_prepared_cuda_dispatch_stats()` helper drains only the current thread’s counters since the last read (`dynamic_indicator_adapter.rs:407-438`).

### What the current packet missed or underweighted

#### 1. The prepared CUDA cache key is a correctness hazard, not just a heuristic naming issue

The strongest missed finding is the cache invalidation story. `PreparedCudaDatasetSignature` contains only `len` plus the current vector pointers for timestamp/open/high/low/close/volume, and rebuild is driven only by inequality on that signature (`dynamic_indicator_adapter.rs:253-260`, `478-487`, `689-709`). If candle values change in place without reallocating the backing vectors or changing the length, the cache reuses old host/device buffers and old derived-source overrides.

That matters more here than it would in a purely immutable data lane, because repo guidance explicitly says base datasets are cached in memory, base changes invalidate derived descendants, and live bars mutate the loaded dataset by `data_id` (`REPO_MAP.md:1537-1562`). Some live updates will be caught because they change `len` or trigger reallocation. But last-bar edits, in-place repairs, or any mutation that preserves both length and allocation can slip through.

The third-pass packet described hash/cache identity as “local heuristics,” which is directionally right, but the real sharpened point is stronger: this is a plausible stale-data bug surface, not only a truth-in-diagnostics issue. I would not call the file fully public-review ready while that remains unresolved.

I would also not “fix” this with a casual full-series content hash on every call. That would be easy to justify rhetorically and easy to regret in a benchmark-sensitive path. The robust fix is more likely an explicit dataset revision/invalidation contract from the owning runtime layer, or a consciously scoped cache-reset hook for mutation-heavy lanes.

#### 2. The public CUDA helper family has inconsistent failure contracts

The earlier packets focused mainly on the unchecked/direct path, but the more important framing is that the file’s CUDA helpers do not all behave the same way:

`compute_dynamic_indicator_cuda_from_device_matrix_row()` is effectively strict device-path execution and surfaces device-path failures directly (`dynamic_indicator_adapter.rs:1152-1217`). `compute_dynamic_indicator_cuda_with_inputs()` tries the device path first, then falls back to host-backed CUDA on any device-data or device-compute error, and records thread-local stats (`1220-1301`). `compute_dynamic_indicator_cuda_direct_with_inputs()` also falls back, but drops both typed validation and fallback accounting (`1304-1369`).

That means the public CUDA surface is not merely “validated” versus “unchecked.” It is also “strict device-only” versus “fallback-permissive,” and that distinction is not obvious from the names or docs because there are currently no docs. The third-pass packet underweighted this inconsistency.

This is important for both correctness interpretation and performance interpretation. Two functions whose names both begin with `compute_dynamic_indicator_cuda...` can disagree about whether a device failure is surfaced, silently rerouted, or rerouted without accounting.

#### 3. The fallback diagnostics are weaker than the earlier packet implied

The first packet correctly called out silent fallback, but the fourth pass sharpens two details.

First, `take_prepared_cuda_dispatch_stats()` is exported at the crate root, but it only reports the current thread’s prepared-input stats and resets them on read (`dynamic_indicator_adapter.rs:432-438`; `lib.rs:288-289`). That is a narrower contract than the name alone suggests.

Second, once fallback happens, the original device error is mostly lost. In the validated prepared-input path, the device error is only preserved through the optional env-var log path and the incremented counter. If the host-backed `compute_cuda(...)` fallback then also fails, the returned `ComputeFailure` contains only the host-path error text, not the original device-path failure that caused the reroute (`dynamic_indicator_adapter.rs:1271-1284`). The unchecked/direct path is even weaker because it discards the device error entirely (`1341-1353`).

That is a narrower, safer place to improve than rewriting fallback policy itself: preserve both error contexts before touching route semantics.

#### 4. The unchecked entrypoint warning was directionally right but slightly overstated

The earlier review was right that `compute_dynamic_indicator_cuda_direct_with_inputs()` bypasses catalog validation and is a sharp surface (`dynamic_indicator_adapter.rs:1304-1369`). But it overstated the external blast radius a little by treating it like a first-class top-level crate API. In `lib.rs`, the crate re-exports `compute_dynamic_indicator_cuda`, `compute_dynamic_indicator_cuda_with_inputs`, `PreparedCudaIndicatorInputs`, and the stats helper, but not the unchecked/direct helper (`lib.rs:165-173`, `288-289`).

That does **not** make the concern disappear, because the module itself is public (`lib.rs:28`) and module-path users can still call it. It does mean the safer recommendation is narrower than “make it `pub(crate)` immediately or rename it now.” A documentation-only containment step such as `#[doc(hidden)]` plus blunt rustdoc may be the more conservative first move if compatibility risk matters.

#### 5. There is still an unproven but real edge-case test gap around `first_valid`

I would not call this a proven bug yet, but it deserves a sharper note than the earlier packets gave it. `first_valid_for_input_kind()` uses OHLC-only readiness for both `Candles` and `Ohlc` inputs (`dynamic_indicator_adapter.rs:935-980`), while the CUDA data refs for those input kinds also carry a `source` series (`1019-1034`). If the repo allows a `ma_source` whose finite-prefix behavior can differ from OHLC readiness in those lanes, the injected `first_valid` can be optimistic.

I do **not** recommend changing this logic on speculation alone. I do recommend adding a targeted test around any candle-kind indicator that accepts a non-close source with a different validity pattern before treating the current rule as complete.

### Claims in the current packet that are too weak or too strong

The weakest claim in the current packet is the framing of cache identity as mainly a diagnostics/truthfulness issue. On the live source plus current repo runtime model, it is stronger than that.

The strongest claim in the current packet is the implied urgency of visibility-changing the unchecked helper. That surface is sharp, but because it is not on the top-level re-export list, the narrowest safe move is probably documentation/de-emphasis first, not an immediate semver-visible restriction.

## Implementation Priority

### Safe-now hardening

The immediate work should stay small and explicit.

First, add rustdoc to every exported or module-public sharp edge that currently has none: `PreparedCudaIndicatorInputs`, `compute_dynamic_indicator_cuda_with_inputs`, `compute_dynamic_indicator_cuda`, `compute_dynamic_indicator_cuda_from_device_matrix_row`, `compute_dynamic_indicator_cuda_direct_with_inputs`, and `take_prepared_cuda_dispatch_stats()`. Those docs should say, plainly, that `from_candles()` and `compute_dynamic_indicator_cuda()` default to device `0`; that dispatch stats are current-thread and reset-on-read; that the prepared-input path may fall back to host-backed CUDA; that the device-matrix-row path is strict device execution and requires the owning matrix to stay alive; and that the direct helper bypasses catalog validation.

Second, preserve more diagnostic truth without changing route policy. The lowest-risk code improvement here is to keep the original device-path error text when fallback also fails, instead of returning only the fallback error. That is a small change with a high debugging payoff and much less regression risk than changing fallback into a hard error.

Third, tighten tests around public contract edges. The prior packets already asked for unsupported-indicator, invalid-output, and out-of-range row tests; those are still warranted. Fourth pass adds two more targeted gaps: a test that the stats helper is thread-local/reset-on-read by contract, and a targeted `first_valid` regression test for any candle-kind indicator/source combination whose validity can differ from plain OHLC readiness.

Fourth, if the team wants to reduce casual discovery of the unchecked helper without taking a semver-visible action, `#[doc(hidden)]` is a narrower move than changing visibility outright. I would choose that over a `pub(crate)` change unless there is clear evidence that no module-path consumer can exist.

### Larger redesign still deferred

The most important deferred item is the prepared-input invalidation contract. If the repository wants cached CUDA indicator inputs to be correct across mutable datasets, this file needs a stronger invalidation signal than pointer equality and length. The clean version of that likely comes from outside the file: a dataset revision counter, a content-generation token, or an explicit cache-reset/invalidation surface owned by the dataset/runtime layer.

I would **not** recommend a hurried full-series fingerprint or an ad hoc mixture of sentinel values and pointers as the “real fix.” Those can be useful heuristics, but they are still heuristics, and they risk adding hot-path cost while leaving the review text falsely confident.

The second deferred item is CUDA API contract unification. Right now the module mixes strict-device and permissive-fallback helpers under very similar names. A cleaner long-term shape would split those contracts explicitly, either by naming or by a result type that reports whether fallback occurred. That is a broader change because higher-level strategy code and profiling flows may already depend on the current behavior split.

The third deferred item is public-surface trimming. If this module is meant to remain a public adapter, it probably wants a thinner public face and a more obviously internal sublayer for prepared cache plumbing and unchecked execution. That is worthwhile, but not the next patch.

## Verification If Changed

If this file changes, the basic verification set should still include the existing in-file CPU and CUDA parity tests, especially the dynamic-indicator adapter test module and representative prepared-input CUDA parity cases.

Beyond that, the most important new verification depends on which class of change lands.

If the patch is documentation/diagnostics only, run the adapter-focused unit tests and any doctests added for the new public contract text. If the patch changes fallback error reporting, add assertions that a fallback failure preserves both the device-path and host-path error context. If the patch touches stats semantics, add a test that demonstrates current-thread scope and reset-on-read. If the patch touches cache invalidation at all, add a regression case that mutates candle contents without changing the allocation shape and proves the cache no longer reuses stale buffers. I would not accept an invalidation change here without that exact kind of regression coverage.

## Regression And Feature-Preservation Constraints

Do not disturb the selective-upload prepared-input behavior as part of generic cleanup. That design is one of the file’s real strengths and is already pinned by tests.

Do not convert the validated prepared-input helper from soft fallback to hard error unless the downstream strategy/runtime consumers are re-audited on purpose. That may be the right direction later, but it is semantics work, not “cleanup.”

Do not claim the stale-cache problem is solved unless the invalidation signal is actually stronger than pointer-and-length identity. Cheap heuristics can be useful, but they should be described honestly as mitigations, not proofs.

Do not make the unchecked/direct helper less reachable in a way that silently breaks module-path users unless the compatibility posture is intentional. Documentation de-emphasis is the safer immediate move.

Do not add heavyweight content hashing or whole-series scans to the hot path without measurement. This file sits on a benchmark-sensitive CUDA staging path, and an apparently tidy invalidation fix can become the performance regression that future reviewers have to unwind.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**. The next responsible patch is still a small surgical refactor: better rustdoc, sharper tests, and better fallback diagnostics.

What changed in fourth pass is not the bucket, but the priority map inside it. The most important thing the current packet missed is that the prepared CUDA cache key is not just a local heuristic worth documenting; in the repo’s stated mutable-dataset runtime model, it is a plausible stale-buffer correctness hazard. That issue is too important to ignore, but too cross-cutting to “solve” casually in the same small patch.

So the right fourth-pass read is: keep the immediate bucket, keep the conservative stance, but stop treating this file’s remaining risk as mostly naming and public-surface polish. Safe-now hardening is still small. The cache invalidation redesign is still deferred. And any review that says this module is fully hardened without addressing that split would be overstating the state of the file.
