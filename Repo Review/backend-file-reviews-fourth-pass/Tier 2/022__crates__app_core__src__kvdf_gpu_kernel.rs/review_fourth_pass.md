# Fourth-Pass Review: `crates/app_core/src/kvdf_gpu_kernel.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/kvdf_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kvdf_gpu_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kvdf_gpu_kernel.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kvdf_gpu_kernel.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The live file still reads as a small, strategy-local CUDA wrapper rather than a candidate for broader redesign. The current third-pass packet therefore got the bucket right, but it also got too abstract. It compressed earlier concrete findings into generic “ownership/lifetime” language and weakened the verification guidance, even though the live source still exposes a safe public entrypoint over raw `u64` device pointers (`kvdf_gpu_kernel.rs:33-60`, `348-607`), still keys resident reuse from both host and device close-series identity (`295-327`), and still sits behind CUDA-only feature gates.

So this fourth pass is not reopening the algorithm or the strategy math. It is sharpening what the next move inside the existing bucket actually is: contract hardening, focused tests, and corrected verification, while keeping broader API and performance redesign deferred.

## Fourth-Pass Research Log

This pass re-read the live source first, then checked how the packet evolved across the first, second, and third review files. It also rechecked the surrounding repo guidance so the recommendation would stay aligned with local architecture rules and validation expectations.

Repo-local materials reviewed:

- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/lib.rs`
- `review__first_pass.md`
- `review__second_pass.md`
- `review__third_pass.md`
- `AGENTS.md`
- `REPO_MAP.md`

External standards rechecked:

1. Rust API Guidelines (`about`, `documentation`, `checklist`)
   - Relevance here:
     - the module is publicly exported from `lib.rs:37-39`
     - the wrapper exposes public structs and a public function whose real contract is much sharper than their surface types suggest
   - Constraint informed:
     - if the surface remains public, documentation is not optional polish; it is part of the safety story

2. Rustonomicon (`meet-safe-and-unsafe`, `working-with-unsafe`)
   - Relevance here:
     - the file is a safe wrapper over unsafe CUDA launches (`275-289`, `545-587`)
   - Constraint informed:
     - the wrapper should either honestly document what safe callers must guarantee, or the boundary should be redesigned in a later coordinated pass

3. Rust `thread_local!` / `LocalKey` docs
   - Relevance here:
     - the runtime cache is thread-local (`150-152`)
   - Constraint informed:
     - current reuse is per-thread, per-device state; it should not be described or “cleaned up” as though it were a generic process-global cache

4. NVIDIA CUDA Programming Guide and CUDA Best Practices Guide
   - Relevance here:
     - this file owns stream launch, explicit synchronization, and resident device reuse (`250-327`, `467-599`)
   - Constraint informed:
     - upload/cache/launch changes remain benchmark-sensitive and should not be folded into a nominal cleanup

None of those sources changed the bucket. They did reinforce that the sharpest live issue is the public contract surface, not the kernel math.

## What Fourth Pass Sharpened

### The current third-pass bucket should stand

`Small surgical refactor` still looks correct.

Nothing in the live source shows a clear kernel-ABI bug, metric-decode bug, or obvious parity defect. The file is small, strategy-local, and already structured around reuse: per-thread runtime caching (`106-152`), resident close/log-return reuse (`295-327`), launch-shape reuse (`260-289`, `467-476`), and scratch-buffer reuse (`62-98`, `390-395`). Promoting this to a larger redesign bucket would overstate what the live file proves.

### What the current third-pass packet got right

The third-pass packet was right to keep the focus on contract sensitivity rather than algorithm refactor. That still matches the source. The main risks are not KVDF math mistakes in this wrapper; they are:

- raw pointer ownership/provenance crossing a safe public boundary (`33-60`, `348-607`)
- cache validity assumptions across repeated calls (`295-327`)
- performance-sensitive runtime reuse and explicit sync/copy behavior (`467-599`)

It was also right to defend strategy-local organization. That matches repo guidance in `AGENTS.md`, which explicitly prefers strategy-local backend organization and warns against turning strategy-specific backends into generic layers without strong evidence.

### What it missed or underweighted

#### 1. The public safe raw-pointer contract is still the sharpest concrete issue

The third-pass packet flattened this into “ownership and lifetime discipline,” but the live source is more specific than that. `KvdfGpuDeviceSignalMatrices` exposes nine raw row-major signal pointers plus `close_device_ptr` as plain `u64` fields (`33-52`), and `eval_kvdf_metrics_gpu_kernel_device_rows` remains a safe `pub fn` (`348-607`). The implementation then passes those values into unsafe CUDA launches (`275-289`, `545-587`).

That does not prove Rust UB in every failure mode, and prior wording should not overclaim that. But it does prove that the safe API overstates what it can verify: `validate_inputs` can reject nulls, bounds, and shape mismatches (`154-248`), but it cannot establish CUDA-context ownership, pointer provenance, allocation lifetime, or row-major allocation size behind those raw addresses.

The safe-now implication is narrower than “add assertions.” The live code needs honest contract docs and focused tests first. Making the surface `pub(crate)`, turning the function `unsafe`, or replacing raw `u64` with an opaque typed transport may still be the right end-state, but those are coordinated compatibility changes, not the first safe-now move.

#### 2. The hidden cache contract is more specific than “immutability”

The second-pass packet was more concrete here than the third pass.

Two details matter together:

- `validate_inputs` requires `close_f32.len() == cols` even when `close_device_ptr` is non-zero (`162-165`)
- resident reuse is keyed by `CloseSeriesKey::from_host_and_device(signals.close_f32, signals.close_device_ptr)` (`299-304`)

That means `close_f32` is not just a fallback payload for the `close_device_ptr == 0` case. It is also part of the resident cache identity even when the device pointer is provided. On top of that, when `close_device_ptr != 0`, the cache stores no owner for that device allocation (`306-320`), so the runtime is explicitly trusting the caller to keep that external device memory alive and semantically matched to the close series used for cache identity.

That is a sharper contract than the third-pass packet described. The real rule is not just “keep data immutable.” It is “keep the caller-owned device close allocation alive, on the same CUDA device/context, and semantically aligned with the `close_f32` series used as part of the cache key.”

That contract should be documented directly if the API stays public.

#### 3. Verification guidance regressed in the third-pass packet

This is the clearest concrete miss in the current packet.

`kvdf_gpu_kernel.rs` is feature-gated by `#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]` (`1`). But the third-pass verification section reduced the check to:

- `cargo test -p ta_app_core kvdf_gpu_kernel -- --nocapture`

without the required feature flags (`review__third_pass.md:58-61`).

That is materially weaker than the first-pass packet, which preserved the actual CUDA-featured commands plus the follow-on parity tests. For a CUDA-only module, dropping `--features cuda,cuda-backtest-kernel` is not a harmless simplification. It makes it easy to under-test or effectively skip the exact module under review.

#### 4. Focused tests are still missing, and the current packet did not keep them explicit

The third-pass packet mentions “performance-sensitive tests” but does not keep the earlier targeted test asks alive. The live source still has only three file-local tests, all for `validate_inputs` edge cases (`609-717`). That is thin coverage for the concrete contract surface.

The narrowest missing tests remain:

- unsupported strategy-feature rejection for overlays / stop-loss / take-profit (`360-367`)
- null signal-pointer rejection for one or more row-major signal buffers (`184-194`)
- if GPU-backed integration coverage is already available, a parity check for the documented `close_device_ptr == 0` host-upload path (`306-315`)

Those are much more actionable than generic “ownership/lifetime assertions.”

### What the current packet overstated or left too vague

#### 1. Visibility narrowing is not safe-now unless semver posture is confirmed

The first-pass packet treated `pub(crate)` narrowing as a low-churn option. The fourth pass is more conservative than that.

Yes, the module is publicly exported from `lib.rs:37-39`. Yes, the packet history only showed internal repo call sites. But the source alone does not prove that no crate consumer relies on this surface. Changing `pub` to `pub(crate)` may be the right move if `ta_app_core` is intentionally workspace-internal, but it is still a compatibility-sensitive change and should stay in the coordinated bucket, not the default safe-now list.

#### 2. “Add or tighten ownership/lifetime assertions” is weaker than it sounds

The current third-pass implementation priority (`review__third_pass.md:53-56`) is too vague to be a good next action.

There is no local assertion that can prove a foreign raw CUDA pointer belongs to `device_id`, is valid for the active context, and still names live memory of the expected length. Assertions can help document intent, but they do not solve the actual contract gap. Without redesigning the API surface, docs and focused tests do more real work here than assertion churn.

#### 3. The sync/copy/decode cost remains deferred, not urgent

The first-pass packet was directionally right that repeated validation loops can pay for explicit synchronization and full host decode (`589-604`). But the current third-pass packet was also right not to escalate that into the primary action item.

The live file alone does not justify promoting a raw-row output path, reusable host-buffer transport, or async result design over the simpler contract/documentation work. Those are still redesign items for a later pass because they touch validation throughput, route parity, and possibly the effective public surface.

## Implementation Priority

### Safe-now hardening

1. Add real rustdoc to the public surface:
   - `KvdfGpuDeviceSignalMatrices`
   - `KvdfGpuComboBatch`
   - `eval_kvdf_metrics_gpu_kernel_device_rows`

   The docs should state, explicitly, that:
   - all non-zero device pointers must belong to the same active CUDA device/context selected by `device_id`
   - `close_device_ptr == 0` means the function will upload `close_f32` and compute resident log returns from that host series (`306-315`)
   - `close_f32` is still part of the resident-cache identity even when `close_device_ptr` is non-zero (`299-304`)
   - externally owned device pointers must remain valid for the full call and for any later cache hit that reuses the same identity
   - overlays / stop-loss / take-profit are intentionally unsupported on this path (`360-367`)

2. Add narrow tests before any code-shape refactor:
   - one local unit test for null row-major signal-pointer rejection
   - one local test for unsupported strategy-feature rejection
   - keep or extend existing repo-level GPU parity coverage for host-upload fallback if GPU-backed test infrastructure already exists

3. Restore precise verification guidance instead of generic CUDA wording:
   - keep the CUDA feature flags in every command
   - keep the known optimization / validation parity tests visible in the review packet

### Larger redesign still deferred

1. Public-surface redesign:
   - narrowing visibility to `pub(crate)`
   - changing the entrypoint to `unsafe fn`
   - replacing raw `u64` transports with an opaque validated handle or typed device-row wrapper

   Any of those can be defensible, but they are compatibility-sensitive and should not be smuggled in as “surgical cleanup.”

2. Validation-path transport redesign:
   - reusable host output buffers
   - raw metric-row return path
   - more asynchronous or lower-copy result handling for repeated validation windows

   Those are plausible performance wins, but they affect validation routes, parity surfaces, and maybe caller ergonomics.

3. Cross-strategy wrapper consolidation:
   - not supported by this file alone
   - specifically discouraged unless measured benefits justify risking perf and route stability

## Verification If Changed

Use the stronger first-pass verification posture, not the weakened third-pass shorthand:

- `cargo test -p ta_app_core kvdf_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kvdf_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kvdf_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`

If transport/runtime behavior changes, also run:

- `cargo test -p ta_app_core kvdf_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kvdf_validation_gpu_kernel_reports_route_diagnostics --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kvdf_validation_gpu_kernel_cscv_matches_cpu --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kvdf_validation_gpu_kernel_reopt_walk_forward_matches_cpu --features cuda,cuda-backtest-kernel -- --nocapture`

If cache, launch, copy, or result-transport behavior changes in a nontrivial way, this is also a candidate for the repo’s CUDA perf gate rather than test-only validation.

## Regression And Feature-Preservation Constraints

Preserve the current strategy-local organization. That is not cosmetic here; it matches repo guidance and keeps the change surface understandable.

Preserve thread-local per-device runtime reuse (`106-152`) and resident close/log-return reuse (`295-327`) unless there is measured evidence that a different design is better. “Simplifying” those paths is a likely regression.

Do not change kernel argument ordering, row-major pointer semantics, `KVDF_METRICS_STRIDE`, or the `decode_non_double_ma_gpu_metrics_row` decode contract without matching kernel-side and parity revalidation. The real low-level cross-boundary risk in this file is CUDA launch ABI and metric-row interpretation, not persistence or IPC.

Treat any visibility narrowing, `unsafe fn` conversion, or opaque-handle redesign as compatibility-sensitive. Treat any raw-row / reusable-buffer validation optimization as validation-workflow-sensitive.

## Fourth-Pass Conclusion

The third-pass action bucket should stand. `kvdf_gpu_kernel.rs` is still a `Small surgical refactor`, not a larger redesign.

But the current third-pass packet is too abstract to be the best next-step guide. It got the bucket right and correctly defended strategy-local, benchmark-sensitive behavior. What it missed was that the live file still has a concrete public-contract problem, a very specific cache-identity contract around `close_f32` plus `close_device_ptr`, and a verification section that regressed from the stronger first-pass commands.

So the safest next move is not “assertions first.” It is:

- document the real public contract
- add one or two precise contract tests
- restore exact CUDA-featured verification guidance
- keep visibility narrowing, `unsafe` conversion, and validation-path transport redesign deferred until compatibility and parity scope are deliberately reviewed
