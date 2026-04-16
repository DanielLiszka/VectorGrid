# Fourth-Pass Review: `crates/app_core/src/validation_core.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/validation_core.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_core.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_core.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_core.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the current third-pass bucket is still directionally right, but the packet itself drifted away from the live source. `validation_core.rs` is a very small orchestrator, yet it is not the serde/default/DTO-facing compatibility seam that the third-pass write-up describes. The live source is a crate-private helper that bridges two internal validation stages: parallel candidate processing and shared probability/CSCV finalization.

That distinction matters. The real risk here is not wire shape, persistence, or GPU ABI breakage. The real risk is quieter: mis-stating the internal contract around the two returned `usize` values, weakening deterministic replay, or changing progress/CSCV sequencing without enough tests. The fourth pass therefore re-centered the review on what the file actually owns, what the current tests do and do not prove, and which changes would harden the code without creating churn.

The third-pass action bucket should **stand** as `Small surgical refactor`, but the rationale needs correction. This file is best treated as a crate-private validation seam that wants sharper invariant coverage and clearer intent, not a representation-compatibility cleanup target.

## Fourth-Pass Research Log

I re-read the live source first, especially `run_validation_candidates` (`validation_core.rs:7-58`) and the three file-local tests (`validation_core.rs:185-382`). I then checked the module boundary in `lib.rs`, where `validation_core` is declared as `pub(crate) mod validation_core` (`lib.rs:74`), which materially changes the public-surface analysis.

I also re-opened the validation pipeline functions that this wrapper depends on:

1. `parallel_collect_validation_candidates` (`validation_pipeline.rs:581-616`) to verify cancellation, deterministic per-index seeding, and progress emission during the parallel stage.
2. `compute_probability_metrics` (`validation_pipeline.rs:268-298`) to verify how `sample_obs` is consumed and to check whether the local observation floor in `validation_core.rs` is the sole policy owner.
3. `finalize_with_probability_and_optional_cscv` (`validation_pipeline.rs:1367-1446`) to verify how `common_start_t`, CSCV gating, and CSCV-prep progress are actually used downstream.

For external guidance, I checked the official Rust API Guidelines on type safety and dependability. The relevant guidance is that types should carry semantic meaning instead of relying on ambiguous primitive values, and that APIs should validate inputs when practical. I also checked the Rayon `IndexedParallelIterator` documentation because this helper relies on indexed parallel iteration with `enumerate()` and collection semantics, which makes reproducibility and ordered behavior worth testing explicitly rather than merely assuming.

## What Fourth Pass Sharpened

The current third-pass packet got two things right. First, it correctly kept the bucket small. Second, it correctly recognized that broad refactor churn would be lower value than clarifying and preserving the validation contract. Those are still the right instincts.

What it missed, underweighted, or overstated is more important.

The biggest miss is visibility. This is **not** a public crate surface in the way the earlier packets imply. `validation_core` is crate-private in `lib.rs:74`, so the main compatibility story here is internal caller coupling and deterministic validation behavior, not external semver or serde-visible behavior. The third-pass framing around “defaults, enum labels, omission behavior, or exported shape semantics” is therefore mis-scoped for the live file.

The second miss is that the two returned `usize` values are not just a vague tuple-contract problem. They drive two *different* downstream statistical paths. The `start_t` value is reduced across all candidates via `max` in `validation_core.rs:38-41`, and that aggregated `common_start_t` is then used only for CSCV slice alignment and `slice_bars` calculation in `validation_pipeline.rs:1389-1403`. The `sample_obs` value is handled per candidate and feeds PSR/DSR plus track-record fields in `validation_pipeline.rs:284-297`. The earlier packets were right to flag tuple ambiguity, but they did not sharpen why the ambiguity matters: these two numbers are both `usize`, yet they are semantically unrelated enough that swapping them would distort different parts of the validation result.

The third miss is test coverage. The current tests are useful, but they do not lock down the file’s most important behavior. All three tests return `start_t = 0` and `sample_obs = 64`. That means there is no direct file-local test proving that `common_start_t` is the maximum returned start, no test proving the minimum-observation floor behavior, no test proving that CSCV prep is skipped without invoking the expensive slice-builder closure when CSCV is disabled, and no test proving deterministic replay under a fixed seed when the closure actually consumes RNG output.

The fourth sharpened point is that the third-pass “do not casually remove `sample_obs.max(2)`” warning is directionally understandable but too absolute. The local floor in `validation_core.rs:41` is **currently redundant** with the downstream floor in `compute_probability_metrics`, which does `sample_obs.get(i).copied().unwrap_or(2).max(2)` in `validation_pipeline.rs:288`. So the observation floor is absolutely part of the behavior, but this line is not the only owner of that behavior. A review packet should be precise about that. The safer conclusion is: do not centralize or remove the floor without tests and a small caller audit, not “this exact line is the only place protecting the invariant.”

The fifth sharpened point is argument validation. Today, an impossible `start_t > total_bars` returned by `process_one` would not fail loudly here. Instead, `common_start_t` would flow into `total_bars.saturating_sub(common_start_t)` in downstream CSCV prep, which can silently collapse CSCV into the skipped/zero-combination path. That is not evidence of a known live bug, so I would not recommend a behavior-changing clamp or unconditional new error path yet. But it *is* a real invariant gap that the earlier packets missed, and it is a good place for a debug assertion or a narrowly scoped validation follow-up.

The sixth sharpened point is reproducibility. This wrapper passes the same seed into both the parallel collection stage and the later CSCV finalization stage (`validation_core.rs:26-33` and `44-55`). The parallel collector then derives per-index RNGs from that seed in `validation_pipeline.rs:605-606`. That makes reproducibility a more concrete contract than the third-pass packet acknowledged. If someone “cleans up” the seed plumbing, changes when sorting occurs, or alters how the parallel stage is collected, they can change validation results without touching business logic. That deserves a deterministic replay test.

The last sharpened point is about progress phases. The concern about stringly typed phase names is real, but the third-pass packet overstates how much a local enum would help. This wrapper accepts caller-provided `validate_phase` and `cscv_prep_phase`, but the existing test already shows a third phase, `composite_validate_cscv`, arriving from deeper in the stack (`validation_core.rs:366-381`). So a local enum in this file would be incomplete unless the downstream CSCV emitter moved with it. That makes phase-typing a larger coordinated cleanup, not a safe isolated refactor.

## Implementation Priority

### Safe-now hardening

The highest-value next step is **tests, not representation churn**.

First, add a focused file-local test with varied `start_t` values and CSCV enabled, and assert that `build_slice_stats_for_candidate` receives the maximum start across all candidates. That is the single most direct way to lock down the file’s core normalization behavior.

Second, add a file-local test with `sample_obs` values of `0` and `1`, then assert that returned candidates end up with `track_record_bars >= 2`. That locks down the probability-track-record floor behavior without pretending that line 41 is the only owner.

Third, add a test that counts `build_slice_stats_for_candidate` invocations and proves the closure is not called when `cscv_cfg` is `None`. That is both a correctness and a performance hardening win, because this closure can be the expensive side of the wrapper.

Fourth, add a deterministic replay test in which `process_one` actually consumes the provided RNG and two runs with the same seed produce the same post-sort result. This is the best low-cost protection against accidental seed-plumbing regressions.

Fifth, add a small rustdoc or inline contract comment above `run_validation_candidates` clarifying the two-stage execution model and the meaning of the tuple returned by `process_one`: `(candidate, candidate_start_t_for_common_cscv_alignment, sample_observations_for_probability_metrics)`. That is more useful than a generic note about “tuple semantics.”

Sixth, consider tightening the function signature from `pub fn` to `pub(crate) fn`. Because the enclosing module is already `pub(crate)`, this is not a behavior change, but it does more honestly describe the intended visibility of the helper.

Seventh, consider a `debug_assert!(st <= total_bars)` in the normalization loop if caller audits confirm that this is an internal invariant rather than an intentionally soft contract. That is a safer immediate move than runtime clamping.

### Larger redesign still deferred

The obvious redesign is to replace `(C, usize, usize)` with a small internal struct or newtyped fields that name the two semantic integers explicitly. That would be an improvement, and the official Rust API guidance on using types to convey meaning supports it. But it is still a cross-file signature change touching this helper, its pipeline dependency, and its callers. That is larger than the current bucket needs.

Likewise, a typed progress-phase model should be deferred unless it is done across the full progress path, including the downstream CSCV phase emitter. Doing it only in `validation_core.rs` would create the appearance of safety without actually eliminating the string vocabulary.

I would also defer any attempt to parallelize CSCV slice-stat construction. The current split between `process_one: Fn + Sync` and `build_slice_stats_for_candidate: FnMut` is telling: this API shape strongly suggests that the first stage is intentionally parallel and the second stage is intentionally allowed to reuse mutable scratch or state. Parallelizing the latter as a cleanup would carry reproducibility and state-safety risk for little guaranteed benefit.

## Verification If Changed

If only documentation, visibility tightening, debug assertions, or file-local tests are changed, the minimum verification should be:

1. `cargo test -p ta_app_core validation_core::tests:: -- --nocapture`

If any contract behavior, seed plumbing, progress phase handling, or tuple/struct representation is changed, run the broader caller-facing suites as well:

2. `cargo test -p ta_app_core double_ma_validate -- --nocapture`
3. `cargo test -p ta_app_core composite_validate -- --nocapture`

If a deterministic replay test is added, run it multiple times locally in debug mode before handoff. This is one of the rare places where “passes once” is not the whole story.

## Regression And Feature-Preservation Constraints

Preserve the distinction between the two returned integers. `start_t` is a cross-candidate alignment input for CSCV, while `sample_obs` is a per-candidate probability input. Any change that collapses or derives one from the other risks changing validation math.

Preserve the current seed plumbing unless reproducibility is deliberately re-baselined. This file currently hands one seed through both candidate processing and optional CSCV finalization, and the parallel stage derives deterministic per-index RNG streams from it.

Do not replace the current `common_start_t = max(start_t)` rule with clamping or per-candidate CSCV windows as a “small cleanup.” That would change the comparability model of CSCV slices.

Do not runtime-clamp impossible `start_t` values just to make the helper more forgiving. If the invariant is worth enforcing, prefer an explicit checked failure after caller audit or a debug assertion first. Silent clamping would mask upstream bugs.

Do not treat the local `sample_obs.max(2)` as the lone owner of the floor policy, but also do not remove or centralize it casually. The correct approach is to lock behavior down with tests before deciding where the invariant should live.

Do not convert only the local phase strings to an enum while leaving downstream CSCV progress phases string-based. That would create an incomplete abstraction and likely force awkward conversions at the boundary.

Do not parallelize `build_slice_stats_for_candidate` without proving that its captured state, scratch usage, and replay behavior remain correct under `Sync` execution.

There is no direct persistence contract, wire serialization contract, or GPU ABI surface in this file. The real regression risks are internal validation semantics, progress-label compatibility, deterministic replay, and accidental extra work in CSCV prep.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand** as `Small surgical refactor`, but the packet itself is not sharp enough. It correctly preferred conservative hardening over large refactor churn, yet it over-framed this file as a public serde/default compatibility seam and under-framed the more relevant risks around internal invariants, deterministic replay, and untested normalization behavior.

The best fourth-pass conclusion is therefore narrower and stronger:

- keep the bucket unchanged;
- correct the visibility/compatibility framing;
- prioritize invariant tests over representation cleanup;
- treat tuple replacement and phase typing as deferred coordinated work, not isolated cleanup;
- and avoid performance or concurrency “improvements” unless they preserve the current seed, progress, and CSCV sequencing contract.

In other words, this file does not need a redesign to look stellar. It needs a more accurate packet, a few sharper tests, and one or two intent-signaling hardening moves.
