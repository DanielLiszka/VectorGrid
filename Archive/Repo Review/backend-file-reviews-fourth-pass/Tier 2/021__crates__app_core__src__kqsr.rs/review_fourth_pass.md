# Fourth-Pass Review: crates/app_core/src/kqsr.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/kqsr.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kqsr.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kqsr.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kqsr.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

`kqsr.rs` is still a hot, strategy-local policy surface rather than generic plumbing. It owns the KQSR config DTO, period resolution, dynamic-indicator output selection, CPU/CUDA helper dispatch, start-index logic, and final desired-position assembly. That makes small-looking cleanup proposals deceptively risky: this file sits directly on correctness, route-truthfulness, and CPU/GPU parity boundaries.

The third-pass bucket should stand. I do **not** see evidence here for a redesign bucket. But the current packet’s safe-now list needs tightening. A few of its concerns are real, while some of its proposed fixes are broader than the live source justifies.

One important scope note: this bundle includes the live `kqsr.rs`, the prior packets, repo routing docs, and `lib.rs`, but not `composite_cpu.rs` or `composite_validate.rs`. So I re-checked the direct-file claims from the live source, but I did **not** independently re-prove every cross-file call-site claim from the earlier packets.

## Fourth-Pass Research Log

I re-read the live source with focus on the validator (`156-191`), period/start-index logic (`193-205`, `995-1006`), output selection (`335-370`), CPU/CUDA helper dispatch (`373-487`), and desired-position assembly (`1019-1154`). I also used current Rust and Serde guidance to pressure-test whether the earlier packet was being appropriately conservative about public API shape, validation, and deserialization behavior.

The main external references consulted in this pass were:

1. Rust API Guidelines Checklist, especially `C-VALIDATE`, `C-HIDDEN`, `C-CALLER-CONTROL`, and future-proofing guidance.
2. Rust API Guidelines Documentation and Future proofing sections, for public-item docs and public-surface minimization.
3. Serde field and container attribute documentation, to re-check what the per-field defaults on `KqsrStrategyParams` actually commit to during deserialization.
4. The Rust Book and standard-library `Result` docs, to separate legitimate recoverable outcomes from silent contract masking.

## What Fourth Pass Sharpened

The current third-pass packet got the broad framing right. It was right to keep this in `Small surgical refactor`, right to treat KQSR as a parity-sensitive strategy surface rather than a cleanup playground, and right to avoid premature performance work in the repeated indicator-dispatch paths. The live file still does not justify a larger architectural rewrite.

What the packet missed or overstated is narrower and more important.

First, the `build_kqsr_desired_positions` fail-open criticism is directionally right but too blunt. The branch at `1028-1039` mixes two very different cases: malformed series shape, and a legitimate “no tradeable bar remains” case. The latter is not obviously a bug. `compute_kqsr_start_t` intentionally returns `first_valid.max(1)` and adds one more bar when `trade_on_next_bar` is true (`995-1005`), so `start_t == n` can happen on short datasets or when the first usable signal arrives on the last bar. In that situation, returning an all-flat vector is a sensible terminal outcome, not hidden corruption. The safer conclusion is: **keep** the flat-output behavior for `start_t >= n`, but stop treating signal-length mismatch as equivalent. If hardening is warranted, it should happen earlier in the existing fallible signal builders rather than by immediately changing the public builder signature to `Result<Vec<i32>, String>`.

Second, the sharper direct-file contract gap is not only “a validator may be bypassed elsewhere”; it is that the validator in this file is weaker than the file’s own state machine suggests. `validate_kqsr_params` only requires `oversold` and `overbought` to be finite, ordered, and within `[0, 100]` (`172-183`), yet the desired-position logic exits against a fixed neutral of `50.0` (`1043`, `1117-1146`). That means threshold values crossing the neutral line materially change holding behavior, and in extreme cases can collapse entries into near-immediate exits. The same issue exists for `prb_threshold`: the validator allows any value in `[0, 1]` (`187-188`), but the runtime uses symmetric tail checks `<= threshold` and `>= 1 - threshold` (`1073-1075`). Once `prb_threshold > 0.5`, those regions overlap and the PRB filter becomes intentionally or accidentally permissive. I would not tighten either range immediately, because that would change accepted persisted/IPC configs, but the current packet underweighted this direct-file semantic ambiguity. The safe-now step is documentation and targeted tests that pin current behavior; validation tightening is a coordinated product decision, not a local cleanup.

Third, the output-resolution concern is real, but the current packet’s implied fix is too strong. `resolve_output_id` tries exact preferred IDs, then a relaxed contains-based match, then finally falls back to the first declared output (`335-370`). That is risky for true multi-output indicators like `kdj`, `srsi`, `qqe`, and `prb`, because a catalog rename or reorder could silently pick the wrong series. But a blanket “fail closed whenever preferred IDs miss” could also regress single-output cases and any currently tolerated aliasing. The narrower safe change is to fail closed or at least emit a loud diagnostic **only when the descriptor has multiple outputs and none of the preferred IDs match**. For single-output descriptors, the current fallback is much less concerning. This is exactly the sort of change that needs current-catalog tests before landing.

Fourth, the packet treated visibility narrowing as if it were probably safe-now. From the bundle we can verify that the module itself is public (`lib.rs:34`) and only `KqsrStrategyParams` is re-exported at the crate root (`lib.rs:123`). That does **not** mean the other public items are safe to narrow immediately. Downstream users can still reach `ta_app_core::kqsr::...` directly. Changing those helpers from `pub` to `pub(crate)` is a public API / SemVer break unless this crate explicitly documents that the module surface is internal. The third-pass packet was right that the surface is broader than clearly necessary, but it underweighted the compatibility risk of acting on that observation. If the real concern is rustdoc noise rather than ABI removal, `#[doc(hidden)]` on internal helpers is the safer interim tool.

Fifth, the file has a genuine route-diagnostics blind spot that the current packet did not call out. In both `compute_indicator_f64` and `compute_indicator_f32`, KQSR-local diagnostics are recorded only when a CUDA helper attempt is actually made and fails (`392-419`, `448-475`). If a GPU-preferring backend is passed but `prepared_cuda_inputs` is `None`, or if the descriptor does not satisfy the local CUDA-support check, the helper quietly executes on CPU and records nothing. I am **not** recommending a hard error from this file alone; upstream route planning may intentionally allow this. But if KQSR-local diagnostics are meant to be truthful about helper fallback, this is a direct-file observability gap and a better target than broad refactor.

Sixth, the third-pass packet did not make the serialization compatibility angle explicit enough. `KqsrStrategyParams` is serde-backed and uses per-field defaults (`24-41`, `140-152`). Serde’s default behavior for self-describing formats is to ignore unknown fields unless `deny_unknown_fields` is added, and missing fields use the declared defaults. That makes the current deserialization posture an active compatibility contract for IPC and persistence, not just a style choice. Any move to stricter parsing belongs in deferred redesign, not in the safe-now bucket.

Finally, one earlier recommendation should be carried forward with caution because I could not independently re-open its cross-file evidence in this bundle: the claim that a live metrics path bypasses `validate_kqsr_params`. That may still be true, and if the cited call site still exists it remains a worthwhile surgical fix. But this fourth pass cannot improve confidence in that claim beyond what the earlier packet already established.

## Implementation Priority

### Safe-now hardening

The safest next changes are narrow and local.

First, keep `Small surgical refactor` as the bucket and preserve the current strategy-local organization. The fast/slow split is justified by current reuse and aligns with Rust API guidance that functions may expose intermediate results to avoid duplicate work.

Second, pin the legitimate no-trade case before changing behavior. Add a focused test that exercises `compute_kqsr_start_t(..., trade_on_next_bar = true)` producing `start_t == close.len()`, then verify that `build_kqsr_desired_positions` returns an all-flat vector. That prevents a future “error hardening” pass from accidentally turning a valid terminal state into a regression.

Third, if malformed-shape hardening is desired, do it in the existing `Result`-returning signal builders instead of in the public desired-position builder. Explicitly validate that every assembled indicator series has `candles.close.len()` elements in `compute_kqsr_fast_signals`, `compute_kqsr_slow_signals`, and the `f32` variants. That catches the real contract violation earlier, preserves the current public builder signature, and avoids conflating shape bugs with “no remaining tradeable bar” outcomes.

Fourth, document the validator’s current semantic posture rather than silently tightening it. `KqsrStrategyParams`, `validate_kqsr_params`, `compute_kqsr_start_t`, and `build_kqsr_desired_positions` should say that exits use a fixed neutral of `50.0`, that `prb_threshold` above `0.5` creates overlapping long/short PRB acceptance regions, and that the helper may legitimately return an all-flat vector when no tradeable bar remains.

Fifth, if output-resolution hardening is touched now, keep it narrow: multi-output no-match should become an error or at least a diagnostic; single-output fallback can remain. Anything broader than that should wait until the current dynamic-indicator descriptors are pinned in tests.

Sixth, if route-truthfulness matters for this strategy, add a diagnostic-only path for GPU-preferring calls that cannot even attempt the CUDA helper because prepared inputs are absent. That improves observability without changing execution semantics.

### Larger redesign still deferred

Several tempting changes should stay deferred.

Do **not** change `build_kqsr_desired_positions` wholesale to `Result<Vec<i32>, String>` as the first move. That is a broad API change, and the live source shows at least one legitimate flat-output case that should not be upgraded to an error.

Do **not** tighten `validate_kqsr_params` to require `oversold < 50 < overbought` or `prb_threshold <= 0.5` without a product-level decision. Those rules may be desirable, but they would change accepted configs and possibly invalidate persisted settings or artifacts.

Do **not** narrow the helper surface from `pub` to `pub(crate)` as a “safe cleanup” unless the crate’s public API policy explicitly permits that break. If public-doc noise is the problem, prefer documentation or `#[doc(hidden)]` first.

Do **not** add `#[serde(deny_unknown_fields)]` casually. The current field-default posture is compatibility-friendly for deserialization, and stricter parsing would have IPC/persistence blast radius.

Do **not** convert quiet CPU fallback on GPU-preferring paths into a hard error from this file alone. That could break mixed-route callers or tests that currently rely on graceful CPU execution.

## Verification If Changed

Keep the existing KQSR-focused verification from the earlier packet, especially the file-local tests and any representative CPU/GPU parity suites that already exist for this strategy.

If this file changes, the additional targeted checks I would require are:

- a unit test pinning the `start_t == len` / all-flat outcome for `trade_on_next_bar`
- a unit test for series-length mismatch hardening in the signal builders, if added
- a test for the chosen multi-output `resolve_output_id` behavior when preferred IDs miss
- a test that documents current `prb_threshold > 0.5` overlap behavior, unless validation is intentionally tightened instead
- if diagnostic-only GPU hardening is added, a test showing the diagnostic appears when a GPU-preferring backend cannot attempt the CUDA helper because prepared inputs are absent

If any output-resolution or CUDA-helper behavior changes, keep CPU/GPU parity and route-diagnostic tests in scope. Nothing in this file warrants a GPU ABI or kernel-shape change by itself, so verification should remain at the helper/parity layer rather than spilling into kernel-contract churn.

## Regression And Feature-Preservation Constraints

Preserve current KQSR entry and exit semantics unless a clearly broken contract is being corrected. In particular, do not “fix” the desired-position builder by converting every flat-output path into an error; `start_t >= n` is plausibly valid and should remain valid unless the start-index contract itself is redesigned.

Treat public-surface changes as compatibility changes, not as stylistic cleanup. Because `kqsr` is a public module, helper visibility tightening can break downstream direct-module users even though only `KqsrStrategyParams` is re-exported at the crate root.

Treat serde changes as persistence and IPC changes. Field defaults and unknown-field tolerance are part of the current behavior whether or not they were originally added for convenience.

Keep route-diagnostic changes diagnostic-only unless caller contracts are re-audited. Quiet CPU fallback may be undesirable for observability, but turning it into a hard failure from this file alone could regress mixed CPU/GPU flows.

Keep performance work subordinate to correctness. The repeated descriptor lookup and parameter `Vec` construction are real, but nothing in the live source shows a compelling reason to touch them before the narrower contract/documentation issues are settled.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: `kqsr.rs` remains a `Small surgical refactor`, not a redesign candidate.

The third-pass packet was right about the general posture: this file is strategy policy, parity-sensitive, and not the place for speculative cleanup. But it was too aggressive about two fixes in particular. Converting the desired-position builder to a fallible API is broader than the direct evidence supports, and visibility narrowing is not safe-now on a public module surface.

The better fourth-pass takeaway is narrower:

- preserve the current no-trade `start_t >= n` behavior
- catch true malformed-shape issues earlier in the fallible signal builders
- document the validator’s neutral/threshold semantics before tightening them
- keep output-resolution hardening limited to genuine multi-output ambiguity
- treat public-surface and serde-strictness changes as coordinated compatibility work, not local cleanup

That keeps the action bucket unchanged while making the next pass safer, sharper, and less likely to introduce regressions than the current packet’s broader fix list.
