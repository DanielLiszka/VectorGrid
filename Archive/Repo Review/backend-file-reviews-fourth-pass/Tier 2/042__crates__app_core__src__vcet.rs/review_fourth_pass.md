# Fourth-Pass Review: crates/app_core/src/vcet.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/vcet.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vcet.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcet.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcet.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the third-pass packet was directionally right on bucket size, but still too passive about what can safely be improved now.

A fresh read of the live source shows that `vcet.rs` is not just a strategy-local signal builder. It is also a public serialized config surface (`VcetStrategyParams` is `Serialize`/`Deserialize` and publicly re-exported from `lib.rs`), a CPU/CUDA route-bridge surface, and a validation-sensitive strategy owner with a GPU companion module. That means small mistakes here can spill into request compatibility, persisted artifacts, route observability, and CPU/GPU parity rather than staying file-local.

The earlier packets correctly treated broad cleanup as risky, but they underweighted a narrower class of safe-now hardening: a few additive tests, one small defensive guard in CUDA pointer math, and a modest observability improvement around fallback behavior. Those are still firmly inside the current bucket. They do not justify redesign, but they do justify a sharper packet than “mainly documentation.”

## Fourth-Pass Research Log

This fourth pass used the live `vcet.rs` source as the primary authority and checked only current repo guidance and current standard-library / Serde behavior where contract interpretation mattered.

First, I re-read the live source rather than trusting the prior packets. The main contract surfaces are: omission-driven config defaults in `VcetStrategyParams` (`vcet.rs:32-84`, `161-177`), thread-local route diagnostics and test-only CUDA fallback toggles (`vcet.rs:86-159`), public validation and period-resolution helpers (`vcet.rs:285-339`), heuristic indicator-id resolution cached through `OnceLock<Result<...>>` (`vcet.rs:367-486`), CPU/CUDA indicator dispatch and fallback behavior (`vcet.rs:488-751`), and desired-position semantics (`vcet.rs:754-875`).

Second, I checked the current crate export surface. `vcet` is a public module and `VcetStrategyParams` is publicly re-exported from `crates/app_core/src/lib.rs`, which raises the compatibility bar for any changes to field defaults, field names, or validation behavior (`lib.rs:81-83`, `128`).

Third, I checked the current repo map to avoid making review claims that conflict with the live repo posture. The current routing docs describe `Vcet` as a standalone composite-family strategy with CPU and `GpuKernel` routes, and they explicitly note broader runtime-tested `GpuKernel` validation coverage than earlier generic “GPU caution” wording would suggest. That makes route and desired-position changes validation-sensitive, but it also means this file is not a speculative prototype that can be freely normalized or simplified.

Fourth, I rechecked the relevant standard behavior behind a few implementation choices. Official Serde default-field behavior reinforces that omission-driven fields are a real deserialization contract, not just convenience defaults. Official `thread_local!` behavior confirms that these diagnostics and test knobs are thread-local controls, not process-global state. Official `OnceLock` behavior matters here because `vcet_indicator_ids()` caches a `Result`; in practice that means both successful binding and binding failure are sticky for the lifetime of the process unless the process restarts. That is not automatically wrong, but it is stronger than the earlier packets acknowledged.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass bucket should stand. This file still does not justify a redesign packet.

The third-pass packet was also right to treat VCET as a compatibility-sensitive policy surface instead of a cleanup playground. That framing still holds. `VcetStrategyParams` is a real contract, not an internal helper struct. The route diagnostics and test-only fallback toggles are not noise. The desired-position logic is directly tied to optimization and validation behavior. And because `vcet.rs` sits beside a `vcet_gpu_kernel.rs` companion and the repo map describes meaningful current `GpuKernel` validation coverage for VCET, any behavioral “cleanup” has to be parity-aware.

### What it missed or underweighted

The main miss is that the safe-now lane is wider than the packet implied.

The earlier packet focused heavily on documentation, but the live file now supports a few code-level hardenings that are still low-risk.

The first underweighted area is the **normalization contract** in `resolve_vcet_periods()` (`vcet.rs:313-339`). The real contract is not just “defaults map fast/slow into VCET periods.” It is “deserialized values are later normalized again.” `validate_vcet_params()` allows certain combinations, while `resolve_vcet_periods()` then clamps mapped or explicit periods upward with `max(1)` and usually forces the long VIDYA period above the short one with `raw_long.max(vidya_short.saturating_add(1))`. That is more than a documentation concern. It deserves explicit tests, because callers can observe the effective periods even when raw serialized fields looked different.

There is also a very narrow edge case here that the earlier packets did not call out: the “long is strictly greater than short” rule is only practically true while `vidya_short < u16::MAX`. At `u16::MAX`, `saturating_add(1)` saturates, so the code can no longer force a strictly larger `vidya_long`. That does not mean the file is buggy today, because the reachable MA range may already exclude that case, but it does mean any future documentation or tests should describe the actual normalization precisely rather than overstating it.

The second underweighted area is **incomplete fallback observability**. In `compute_indicator_f64()` (`vcet.rs:488-543`), CUDA-capable backends record a diagnostic when the CUDA helper is forced off for tests or when a CUDA helper call errors. But if the backend is not `CpuOnly` and `prepared_cuda_inputs` is simply `None`, the function silently falls through to CPU with no diagnostic. That makes route notes weaker than they need to be and is exactly the kind of additive hardening that fits this bucket.

The third underweighted area is the **indicator binding contract**. `resolve_output_id()` and `resolve_param_key()` use exact-match first, then substring heuristics, and `vcet_indicator_ids()` caches the resulting binding through `OnceLock<Result<VcetIndicatorIds, String>>` (`vcet.rs:367-486`). That means this file is relying on two nontrivial assumptions: indicator descriptors are stable enough for heuristic binding to be safe, and the dynamic indicator catalog is static enough that caching failure forever is acceptable. Those assumptions may be completely valid in this repo, but the earlier packets treated `OnceLock` mostly as an anti-overhead detail rather than as a contract choice.

The fourth underweighted area is a small **CUDA pointer guard** issue. In `VcetOwnedDeviceMatrix::from_shared_row()` the code checks row and byte-offset multiplication for overflow, but then uses `saturating_add` when adding the byte offset to the base device pointer (`vcet.rs:251-272`). Saturating arithmetic is not a great fit for raw pointer derivation because overflow should be reported, not rounded to a clamped address. This is still a tiny hardening change, but it is a real one.

The fifth underweighted area is **test coverage depth**. The current file-local tests cover only one period-mapping case and one desired-position scenario (`vcet.rs:877-939`). Missing coverage is still meaningful here because the control flow includes `trade_on_next_bar`, `allow_flip`, `long_only`, `require_vidya_slope`, `max_trade_bars`, `compute_vcet_start_t()`, and mixed map-flag combinations. That is enough branch surface that a small battery of additional unit tests is justified.

### Claims in the current packet that are too weak or too strong

The earlier packet was too weak in treating the safe-now path as mostly documentation. That is no longer the best recommendation. The live file supports a small hardening packet with tests and one or two tiny defensive code changes.

It was also slightly too strong, at least by implication, in acting as though there is no meaningful local performance debt beyond comments. There is a real deferred optimization target in `compute_vcet_chandelier_signals()` (`vcet.rs:609-665`), which computes `chandelier_exit` twice through the generic helper path, once for `long_stop` and once for `short_stop`. That does not mean it should be changed now. On the contrary, changing it casually would be regression-prone because it probably wants adapter-level multi-output support rather than a local trick. But the file does contain a real deferred efficiency topic, so the correct message is “defer it,” not “there is none.”

### Safe-now hardening

The safe-now work is still small and surgical.

First, expand the file-local tests. The highest-value additions are:

- mixed map-flag cases for `resolve_vcet_periods()` so the current normalization rules are explicit rather than inferred;
- `compute_vcet_start_t()` coverage for `trade_on_next_bar`; and
- desired-position tests for `allow_flip = false`, `long_only = true`, and `max_trade_bars` expiry, because those are all semantics-bearing branches in `build_vcet_desired_positions()`.

Second, add an additive route diagnostic in `compute_indicator_f64()` when a non-CPU backend reaches the CPU path because `prepared_cuda_inputs` is absent. That preserves behavior while improving route truthfulness.

Third, change `VcetOwnedDeviceMatrix::from_shared_row()` from `saturating_add` to `checked_add` plus a descriptive error. That is a classic defensive fix: it tightens impossible-state handling without changing intended successful behavior.

Fourth, document or test the indicator-binding assumptions around `resolve_param_key()`, `resolve_output_id()`, and `vcet_indicator_ids()`. The safest version is probably tests, not extra abstraction. The goal is to pin the current expected binding behavior, not redesign it.

### Larger redesign still deferred

The larger redesign items are still correctly deferred.

Do not turn the current normalization into stricter hard failures yet. Because `VcetStrategyParams` is serialized, public, and omission-driven, tightening deserialization or validation would be a compatibility decision, not a cleanup.

Do not rewrite the `OnceLock<Result<...>>` pattern yet. If the indicator catalog is effectively static, the current sticky behavior may be exactly right. Replacing it now would risk changing startup and error-handling semantics without a proven need.

Do not try to “optimize away” the double generic `chandelier_exit` dispatch inside `compute_vcet_chandelier_signals()` as a local refactor. That likely wants an adapter-level multi-output contract or a validated shared helper. Done locally, it is more likely to create parity drift than meaningful value.

Do not symmetrize the VIDYA and Chandelier CUDA-device fallback stories without tracing the whole caller contract. VIDYA has a host-staged fallback path in this file (`vcet.rs:668-721`), while Chandelier uses a dedicated CUDA batch call and returns errors directly (`vcet.rs:724-751`). That asymmetry may be intentional. It is not a safe-now cleanup target.

## Implementation Priority

1. **Keep the bucket unchanged.** The third-pass action bucket should stand as `Small surgical refactor`.
2. **Use the bucket for hardening, not beautification.** The next change set should be tests plus tiny guardrails, not structural extraction.
3. **Preferred safe-now packet:**
   - add branch-covering local tests for normalization and desired-position semantics;
   - add the missing “prepared CUDA inputs absent” route diagnostic; and
   - replace `saturating_add` with `checked_add` in shared-row device pointer derivation.
4. **Explicitly defer bigger ideas.** Multi-output chandelier consolidation, indicator-binding abstraction, and validation-tightening should all stay deferred.

## Verification If Changed

At minimum:

- `cargo test -p ta_app_core vcet::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

If any change touches CUDA fallback behavior, `VcetOwnedDeviceMatrix`, or route diagnostics, also rerun representative VCET CUDA-path coverage under `--features cuda,cuda-backtest-kernel`. The exact selector depends on the local CUDA-enabled test surface, but this is not an area where CPU-only green tests are enough.

If any change touches desired-position semantics or effective-period normalization, treat it as validation-sensitive and recheck the downstream VCET validation/report surfaces rather than stopping at file-local tests.

## Regression And Feature-Preservation Constraints

Preserve the serialized config contract. `VcetStrategyParams` is public, serde-driven, and re-exported, so changing defaults, field names, or turning current normalization into new hard errors can break IPC between layers and any persisted request or artifact flows that rely on omission-driven behavior.

Preserve the strategy’s current semantic posture. `resolve_vcet_periods()`, `compute_vcet_start_t()`, and `build_vcet_desired_positions()` collectively define effective VCET behavior. Changes here are not cosmetic. They can alter optimization results, fixed-candidate reruns, and validation verdicts.

Preserve the current GPU lifetime contract. `VcetOwnedDeviceMatrix` carries both a raw device pointer and an owner enum specifically so the backing allocation stays alive. Do not simplify away the owner-keepalive model, and do not treat raw device-pointer math as a harmless detail.

Preserve test and debug observability. The thread-local diagnostics and test-only fallback toggles are deliberate observability aids. Do not remove them, and be cautious even with additive diagnostic strings if any tests or snapshots assert exact messages.

Avoid premature route-policy changes. The repo’s current posture already treats VCET as having meaningful `GpuKernel` validation coverage. That is exactly why route fallback behavior must be adjusted cautiously: not because GPU support is fragile in general, but because this file now sits on a real validated route surface.

Finally, do not oversell unreachable edge cases. The `u16::MAX` saturation nuance in period normalization is worth recording, but it is not enough evidence by itself to justify changing behavior unless upstream range legality confirms the case is reachable in practice.

## Fourth-Pass Conclusion

The third-pass action bucket should **stand**.

`vcet.rs` still belongs in `Small surgical refactor`, not in a redesign bucket. The current packet got the central classification right: this is a public, semantics-sensitive, CPU/CUDA-adjacent strategy file where compatibility and parity matter more than abstraction cleanliness.

What the current packet missed is that the correct next step is not merely more documentation. The live file now supports a better small-surgical packet: lock down normalization and desired-position semantics with additional local tests, improve one silent CPU fallback diagnostic, and harden one CUDA pointer-addition guard. Those are real improvements, they are evidence-backed, and they stay well within the current bucket.

The larger ideas remain deferred. Do not tighten config semantics, rework the indicator-binding model, or try to locally optimize the chandelier multi-output path without broader route and parity review.
