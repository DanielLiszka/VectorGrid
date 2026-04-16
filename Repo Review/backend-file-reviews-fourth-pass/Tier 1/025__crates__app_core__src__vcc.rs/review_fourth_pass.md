# Fourth-Pass Review: crates/app_core/src/vcc.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/vcc.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Safe-now hardening; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third pass was directionally right that `vcc.rs` is not a safe candidate for broad cleanup-first refactoring, but it left too much value on the table by collapsing the next step to “document more and defer the rest.” Reopening the live source shows a narrower and safer tranche of work that the third pass underweighted: strengthen the always-on guardrails around current semantics before any larger redesign.

The bucket should change. The larger redesign is still deferred, but `Larger refactor but defer` is now too coarse because there is concrete, bounded hardening available immediately.

That matters more here than the third pass made explicit because the whole module is public (`pub mod vcc;` in `lib.rs` at line 78), while the crate root only re-exports `VccStrategyParams` directly (`pub use vcc::VccStrategyParams;` at line 127). In practice, that means helper-level semantics such as `compute_vcc_start_t`, `build_vcc_desired_positions`, and `VccPeriodSweepDevice` remain externally reachable through the public module path and should not stay mostly implicit.

## Fourth-Pass Research Log

This pass re-read the live `vcc.rs` source from top to bottom, compared it against all three prior packets, and re-checked the crate export surface in `lib.rs` to confirm what is actually public. It also pressure-tested the earlier packet’s claims against the code that exists today rather than the code the packet seemed to assume.

On standards research, this pass revisited the official Serde field/container attribute docs because `VccStrategyParams` relies on per-field defaults across nearly the entire struct (`33-61`) and does not opt into unknown-field rejection. That combination means omission-driven defaults and tolerant parsing are active IPC and persistence behavior, not passive convenience.

It also revisited Rust’s `thread_local!` and `LocalKey` documentation because the route diagnostics and forced-fallback toggle are TLS-scoped (`99-155`), so any recommendation about observability or test controls had to preserve thread-local semantics.

Finally, it used the Rust API Guidelines as a tie-breaker for what can be hardened now without semantic churn. That is the main reason this pass favors deterministic tests, debug-only assertions, and targeted contract docs over another round of “defer everything structural.”

## What Fourth Pass Sharpened

### Bucket assessment

The third-pass bucket should **change**, not stand as-is.

The correct fourth-pass bucket is `Safe-now hardening; larger redesign still deferred`.

The reason is not that the third pass misread the redesign risk. It is that it overstated how much routine protection the live file already has, and understated how much bounded hardening can be done without changing behavior.

### What the third pass got right

The third pass was right that `vcc.rs` is still a strategy-owner policy surface, not a cleanup-only implementation detail. Config defaults (`33-97`), indicator dispatch (`440-507`), CUDA sweep materialization (`563-783`), start-index math (`899-931`), and desired-position state transitions (`933-1038`) are coupled enough that broad extraction remains risky.

It was also right that config parsing changes are compatibility-sensitive. Because `VccStrategyParams` is omission-driven and field-defaulted, tightening this surface now would change how frontends, saved payloads, and reopened artifacts are interpreted.

It was further right to defer sweeping GPU-path cleanup. The file contains an explicit device-pointer keepalive pattern: `VccOwnedDeviceMatrix` stores both the raw device pointer and the owning allocation wrapper (`210-243`), and `row_ptr_for_period` deliberately touches the owner before returning a derived row pointer (`269-276`). That is a real GPU ABI/lifetime constraint, not cosmetic structure.

### What the third pass missed, underweighted, or overstated

The largest miss is the test-story overstatement. The earlier packet described strong parity-oriented file-local tests, but the live file has twelve tests total, and nine of them are `#[ignore = "diagnostic"]` (`1157-2224`). Only three tests run by default (`1053-1155`). The active suite does **not** routinely lock down the most semantics-sensitive behavior in this file: start-index computation, release gating, fp32 roundtrip helpers, or device-sweep warmup semantics.

The second miss is that the current fail-open entry behavior is not vague; it is concentrated in one exact piece of code. `release_confirm_left` starts at `0` (`954`), and `release_confirmed` is computed as `squeeze_release || release_confirm_left == 0` (`975-980`). In flat-position mode (`982-1005`), that means entry gating is open whenever the countdown is not active. The third pass mentioned fail-open behavior abstractly, but it did not make clear that this behavior is central, concrete, and easy to change accidentally.

The third miss is a public-helper warmup mismatch. `VccPeriodSweepDevice::first_valid_for_period` returns `period - 1` (`279-281`), while `compute_vcc_fast_first_valid` uses `max(momentum_bb_period, squeeze_period, momentum_period) - 1` (`919-930`). Those are not the same contract. The simpler helper may be acceptable as a row-local lower bound for some sweeps, but it is not the full VCC fast-signal warmup rule. Leaving that undocumented is riskier than the third pass acknowledged.

The fourth miss is route-diagnostic coverage. The TLS diagnostics only capture attempted CUDA-helper fallbacks (`453-489`). They do not explain the equally important “no GPU attempt” cases, such as missing prepared inputs or a descriptor that is not currently GPU-preferred by catalog metadata. So the third pass was too generous in treating these diagnostics as route-truthful observability. They are useful, but much narrower than that.

The fifth miss is parity-fragility evidence hiding inside the ignored diagnostics. The file-local diagnostic flow at `2149-2203` explicitly constructs a CPU-derived momentum host buffer and feeds that into the GPU-metric path. That is not production behavior, but it is evidence that fp32 momentum handling is sensitive enough that broad GPU-path cleanup still needs caution. The third pass was right to defer redesign, but it underexplained *why*.

### Safe-now hardening

There is meaningful, low-risk work available now.

The highest-value step is to promote a small subset of current semantics into routine, non-ignored tests. The best candidates are CPU-only and deterministic: one test that pins current flat-entry behavior when no squeeze release has happened yet, one test that pins the current `confirm_bars` timing around a release, one test that locks the relationship between `compute_vcc_fast_first_valid` and `compute_vcc_start_t`, and one test that exercises `vcc_momentum_roundtrip_parity` on a tiny hand-built fixture. Those guardrails are more valuable right now than another round of prose about parity risk.

The second safe-now step is debug-only misuse detection inside `build_vcc_desired_positions`. Today the function uses `close.len()` as the authoritative series length (`941-945`) and then silently degrades mismatched signal lengths into `NaN`-driven no-trade behavior through `.get(...).unwrap_or(f64::NAN)` (`960-969`). A few `debug_assert_eq!` checks on `close`, `fast.squeeze_state`, `fast.momentum`, and `trend.trend_state` would catch misuse in debug/test builds without changing release semantics.

The third safe-now step is to add narrow rustdoc exactly where the public traps are. `VccPeriodSweepDevice::first_valid_for_period` should say plainly that it is *not* the full fast-signal warmup rule for VCC, and `VccOwnedDeviceMatrix` / `row_ptr_for_period` should say that the returned row pointers are only valid while the owning matrix remains alive.

The fourth safe-now step is to document route-diagnostic scope honestly. Preserve the thread-local mechanism, but say explicitly that it records helper fallback events, not the full reason a call ended up on CPU.

### Larger redesign still deferred

The structural refactor is still deferred.

This file still mixes config defaults, deserialization behavior, indicator dispatch, device sweep materialization, public warmup helpers, start-index computation, and the desired-position state machine. That is real technical debt, but the fourth pass does not support jumping straight to extraction because the current semantics are still only weakly locked down.

The highest-risk redesigns remain the same, but they now have a clearer reason to wait. Changing `release_confirmed` semantics would alter optimizer and validation outputs. Tightening Serde parsing with `deny_unknown_fields` or removing the `.max(1)` clamps would change IPC and persistence behavior. Reworking `VccOwnedDeviceMatrix` or the row-pointer API could break device-pointer lifetime guarantees. And collapsing the generic dynamic-indicator path together with the direct CUDA sweep helpers would be difficult to validate while most parity evidence is still diagnostic-only.

## Implementation Priority

1. Add a handful of deterministic, non-ignored tests that pin the current semantics the file actually depends on.
2. Add debug-only shape assertions and targeted rustdoc on the public helper traps.
3. Preserve current behavior, current deserialization tolerance, and current device-pointer ownership shape.
4. Revisit extraction or API cleanup only after the newly explicit semantics are locked by routine coverage.

## Verification If Changed

At minimum, rerun the file-local tests with:

- `cargo test -p ta_app_core vcc::tests:: -- --nocapture`

Then rerun the adjacent composite surfaces that consume or exercise this strategy path:

- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

If the change touches CUDA sweeps, row-pointer ownership, route diagnostics, or any helper that influences GPU metric evaluation, also rerun the CUDA-enabled VCC tests under the project’s normal CUDA feature set. The currently ignored diagnostics are still valuable local evidence for parity-sensitive changes, but they should be treated as supplemental, not as the primary safety net.

This fourth pass did **not** re-run those commands from the bundle-only review context, so the guidance here is source-driven rather than execution-backed.

## Regression And Feature-Preservation Constraints

Do not “fix” the current squeeze-release confirmation logic casually. Even if the naming suggests a stricter gate than the implementation enforces, changing it would clearly alter strategy semantics and downstream optimization/validation outputs.

Do not add `#[serde(deny_unknown_fields)]` or otherwise tighten deserialization in this file without treating it as an IPC and persistence compatibility decision. The current omission-driven defaults and unknown-field tolerance are live behavior.

Do not flatten `VccOwnedDeviceMatrix` into bare device pointers or split the row-pointer API away from the owner unless the existing keepalive guarantee is preserved explicitly. That is a GPU ABI/lifetime constraint.

Do not market the current thread-local diagnostics as full route truth. Broadening them may be worthwhile later, but it can change tests, logging surfaces, and route-explanation behavior.

Do not assume the ignored diagnostic tests are enough coverage to carry a structural refactor safely. They are useful diagnostics, not a substitute for always-on contract tests.

## Fourth-Pass Conclusion

The third-pass packet was mostly right about the *redesign* question and too weak on the *next step* question.

`crates/app_core/src/vcc.rs` should **not** jump straight into a larger refactor. But it also should not stay parked in a pure defer bucket, because there is a real safe-now hardening tranche that the third pass missed.

So the bucket should change. The right fourth-pass call is `Safe-now hardening; larger redesign still deferred`.

What the third pass got right: the file is still semantics-sensitive, the GPU path still has real lifetime/parity constraints, and tightening config behavior would be compatibility-sensitive.

What it missed or overstated: the routine safety net is much thinner than described, the fail-open release logic is more concrete and more central than the packet made explicit, the public warmup helpers are easier to misuse than the packet acknowledged, and the existing route diagnostics are only partial observability.

The conservative path forward is therefore clear: add a small number of real guardrails now, leave behavior unchanged, and postpone the larger redesign until those guardrails exist.
