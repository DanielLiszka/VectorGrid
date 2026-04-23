# Fourth-Pass Review: `crates/app_core/src/double_ma_vdps.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/double_ma_vdps.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_vdps.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_vdps.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_vdps.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file needed a fourth pass because the live source and the current third-pass packet are no longer aligned on the nature of the risk.

The third-pass packet kept the right bucket, but it drifted into generic language about “tolerant behavior,” “compatibility helpers,” and “parsing.” The live file is not mainly a parsing or compatibility boundary. It is a small internal optimizer helper that still contains concrete algorithmic contract bugs in two places that materially affect survivor-set size and ranking fidelity: the refill helpers and partial-result score-mode preservation.

So the fourth pass was justified less to reopen the bucket than to correct the rationale. This should still be treated as a small surgical refactor, but for narrower and more concrete reasons than the third-pass packet states.

## Fourth-Pass Research Log

1. Re-read the live `double_ma_vdps.rs` implementation end to end, then traced its only meaningful orchestration call site inside the `GpuVdps` rung loop in `crates/app_core/src/double_ma.rs`. That was necessary to distinguish dead defensive code from live behavior, and to see whether helper-local defects are masked by caller-side clamps. They are not.

2. Re-read the current third-pass packet first, then the earlier review history. The first and second passes remained much closer to the live file than the third pass. The third-pass reframing is the main thing that needed adversarial correction.

3. Checked the current Rust standard-library documentation for `slice::partition_point`. The docs still state that the slice must already be partitioned according to the predicate. That supports lightweight documentation of the sorted/partitioned-period invariant for `vdps_period_window_slice` and `vdps_nearest_period`, but it does not support escalating that invariant into a new runtime-failure policy here.

4. Checked the current Rust standard-library docs for `BTreeSet` iteration order and the Rust API Guidelines documentation guidance. Those confirm that the current callers can satisfy the sorted-period invariant cheaply, and that concise precondition docs are the right hardening tool here.

5. Re-used repo guidance from `AGENTS.md` and `REPO_MAP.md` only where it changed the recommendation surface: this is a benchmark-sensitive optimizer helper, and CUDA-facing perf verification should go through the repo’s existing GPU perf gate rather than ad hoc timing.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**. This is still a `Small surgical refactor`.

That said, the **third-pass rationale should not stand as written**. The bucket is right because the fixes are local and bounded, not because the file is chiefly a tolerant policy or compatibility surface.

### What the current third-pass packet got right

The current third-pass packet was right about three things:

1. **Do not broaden scope into a redesign.** This helper file is not the place to retune the VDPS methodology, mutation coefficients, or frontier-shaping policy without broader evidence.
2. **Do not add hard runtime failures casually.** The sorted-period assumption behind `partition_point` is real, but the current callers already normalize or sort the relevant period vectors, so docs/debug assertions are a better fit than new production panics.
3. **Keep the action bucket small.** The live issues are real, but they are still narrow enough to fix without architecture churn.

### What the current packet missed, underweighted, or overstated

#### 1. The refill-helper bug is stronger than the packet described

The first-pass packet correctly identified the three refill helpers as overshooting their intended budgets, but the live bug is stronger than “overshoot.” In `vdps_inject_random_seed_pairs`, `vdps_inject_elite_mutation_pairs`, and `vdps_inject_differential_seed_pairs`, the loop condition compares `seen.len()` against `current.len() + target_count` while both `seen` and `current` grow together on every successful insert (`double_ma_vdps.rs:210-223`, `260-282`, `296-325`). With a duplicate-free starting `current`, that guard never converges when `target_count > 0`; the helper can keep appending until `max_attempts` is exhausted or the candidate pool is depleted.

So this is not a mild count drift. In the favorable case it can add far more than `target_count`, with the minimum-attempt floors (`128`, `192`, `256`) making small requested refills especially misleading.

The caller context makes this worse. After the GPU refill path, the CPU refill phase recomputes `refill_remaining`, calls the three helpers, and then directly commits `seed_pairs = next_seed_pairs` without any final truncate or invariant clamp (`double_ma.rs:6778-6839`). That means the helper-local bug is not contained. It can leak into the next rung’s search frontier and push the survivor set beyond `cfg.max_survivor_seeds` in practice.

This is the most important fourth-pass sharpening. It is a correctness issue, a workload-shaping issue, and a benchmark issue.

#### 2. The `score_mode` defect has a second live failure mode that earlier packets did not call out

Earlier packets already caught the main bug: `vdps_merge_partial_results` discards the requested `score_mode` and reranks merged partial results under `None` (`double_ma_vdps.rs:159-185`). The caller later reranks under the requested `score_mode` (`double_ma.rs:6708-6715`), but by then the merge helper may already have truncated away candidates that would have survived under the real ranking contract.

The fourth-pass sharpening is that this is **not only** an intermediate-pruning defect. It also has a final-result defect when `top_k == 0`.

The final-rung repair path only reranks `best_params` and `top` when `top_k > 0` (`double_ma.rs:6874-6889`). The explicit `else` branch clears `top` and leaves `best_params` untouched (`double_ma.rs:6889-6890`). So for a non-default `score_mode`, if the request shape lands on `top_k == 0`, the result can return a winner that was chosen under the plain objective instead of the requested score contract.

That is a more concrete and more user-visible defect than “partial merge drops score_mode semantics” by itself.

#### 3. The third-pass “tolerant parsing / compatibility” framing is weakly supported by the live call graph

This is the biggest overstatement in the current third-pass packet.

There is no parsing surface in `double_ma_vdps.rs`. The most “tolerant” helper here is `vdps_nearest_period`, which snaps a mutated integer back onto the allowed period grid (`double_ma_vdps.rs:226-245`). That is not a compatibility shim; it is part of the search heuristic itself.

Likewise, the request-level rung normalization already happens outside this file. `VdpsResolvedConfig::from_request` strips non-finite or non-positive rung values, clamps them into range, sorts them, and guarantees a terminal `1.0` rung (`double_ma.rs:1559-1584`). So the defensive non-finite fallback in `vdps_target_pairs` is not the important live contract. It is backup behavior beneath a sanitized request path.

The same is true for the sorted-period precondition. Main period arrays are produced in ascending order by `expand_u16_range` (`double_ma.rs:10132-10169`), the decimation helper re-sorts its output (`double_ma_vdps.rs:27-44`), and the alternate frontier-building path uses `BTreeSet` before collecting candidates. The precondition is real, but the live risk is not “hidden tolerant behavior at a public boundary.” The live risk is that internal helper contracts are wrong.

#### 4. Telemetry merge hardening is still reasonable, but it is not the best fourth-pass headline

`merge_vdps_telemetry` is structurally brittle if rung plans ever diverge, because it only checks `cur.rungs.len() == inc.rungs.len()` before zipping rung stats (`double_ma_vdps.rs:353-372`). That concern is valid.

But the current call graph weakens the urgency. `vdps_cfg` is built once per request (`double_ma.rs:5192-5196`), and telemetry is merged only when per-variant runs are folded back into the final result (`double_ma.rs:7394-7405`, `7672-7687`). In today’s live path, rung-plan mismatch is mainly a future-proofing concern, not the primary defect driving a fourth pass.

So this item belongs in safe-now hardening or a follow-on cleanup, but it should not outrank the refill and `score_mode` bugs.

#### 5. The third-pass verification advice is weakly grounded in the current test surface

The third-pass packet suggested:

- `cargo test -p ta_app_core double_ma_vdps -- --nocapture`

That filter is not obviously aligned with the visible test names in the current code. The tests visible in `double_ma.rs` are named `vdps_local_anchor_axes_include_center_and_trim_candidates`, `vdps_merge_partial_results_keeps_global_best`, `vdps_refill_mix_normalizes_when_sum_exceeds_one`, plus several `gpu_vdps_*` tests (`double_ma.rs:19173-19415`, `20772-20990`).

This is not a correctness bug in the source file, but it is a review-quality problem in the current packet. The fourth-pass version should recommend exact existing test names plus the missing focused additions.

## Implementation Priority

### Safe-now hardening

1. **Fix the three refill-helper loop guards.**
   Freeze a uniqueness-based goal length before the loop and compare against that stable target, rather than recomputing against a growing `current.len()`. This is a live correctness fix, not a heuristic redesign.

2. **Preserve `score_mode` inside `vdps_merge_partial_results`.**
   Thread `score_mode` into the helper, use it for merged ranking, and recompute `best_score` / `top_scores` with the same contract. This is the smallest change that prevents intermediate partial truncation from silently switching ranking semantics.

3. **Cover the final-rung `top_k == 0` case explicitly.**
   The easiest safe fix is to ensure the merge helper already returns `best_params` under the requested `score_mode`, so the later `top_k == 0` branch cannot preserve an objective-only winner by accident.

4. **Add the missing focused tests.**
   The current test surface does not pin the three highest-risk edges:
   - each refill helper must stop at the requested unique-add budget;
   - merged partial results must preserve non-`None` `score_mode` semantics;
   - non-`None` `score_mode` plus `top_k == 0` must still produce the correct best result.

5. **Keep sorted-period hardening lightweight.**
   Add concise doc comments and, at most, debug-only assertions to `vdps_period_window_slice` and `vdps_nearest_period`. Do not add new runtime panics to this hot helper layer.

6. **Treat unused-import cleanup as opportunistic only.**
   Removing the unused `objective_score` import is fine, but it is not what this fourth pass should prioritize.

### Larger redesign still deferred

1. **Do not retune the search heuristic itself in this pass.**
   Leave mutation radii, differential coefficients, decimation stride policy, and local-anchor frontier shaping alone. The live bugs are contract bugs, not evidence that the VDPS heuristic family is wrong.

2. **Do not redesign telemetry aggregation without a real multi-config requirement.**
   Hardening mismatched-rung-plan handling is reasonable, but this does not justify new aggregate result shapes or serialized payload changes.

3. **Do not chase micro-optimizations before the logic is corrected.**
   Container-choice changes (`BTreeSet` vs `HashSet`), allocation tuning, and sort/dedup micro-work are all lower value than repairing the current seed-budget and ranking-contract defects.

4. **Do not move this into a broader policy surface.**
   The file remains strategy-local VDPS helper code. The right outcome is sharper invariants and sharper tests, not a new generic abstraction.

## Verification If Changed

Use exact existing tests where possible, then add the missing focused coverage.

Recommended minimum verification:

- `cargo test -p ta_app_core vdps_merge_partial_results_keeps_global_best -- --nocapture`
- `cargo test -p ta_app_core vdps_local_anchor_axes_include_center_and_trim_candidates -- --nocapture`
- `cargo test -p ta_app_core gpu_vdps_quality_check_emits_overlap_and_regret -- --nocapture`
- one representative deterministic VDPS test such as:
  - `cargo test -p ta_app_core gpu_vdps_deterministic_same_seed_reproducible -- --nocapture`

Add and run focused tests for:

- `vdps_inject_random_seed_pairs` respecting requested unique-add count;
- `vdps_inject_elite_mutation_pairs` respecting requested unique-add count;
- `vdps_inject_differential_seed_pairs` respecting requested unique-add count;
- `vdps_merge_partial_results` preserving non-`None` `score_mode` ordering;
- the final `GpuVdps` path with non-`None` `score_mode` and `top_k == 0` returning the correct best candidate.

Because the refill fix changes next-rung workload shape, run the repo’s GPU perf authority before handoff for CUDA-enabled trees:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

If a local GPU gate is unavailable, at least run a representative VDPS benchmark slice from the existing `double_ma_opt` benchmark surface and compare frontier size / runtime before and after.

## Regression And Feature-Preservation Constraints

- **Performance risk:** fixing the refill helpers will change survivor-set size and later-rung workload. That is desirable if it restores the intended budget, but it still needs perf evidence because this is a benchmark-sensitive optimizer surface.

- **Compatibility / IPC / persistence risk:** `VdpsTelemetry` is surfaced through the optimization result (`double_ma.rs:7672-7687`). Keep the serialized field shape stable. If telemetry-merge hardening is touched, prefer semantic tightening over payload reshaping.

- **GPU ABI risk:** this file is not a CUDA ABI or kernel-layout surface. The risk is indirect: wrong helper behavior changes which candidate pairs are sent into later GPU work, not the kernel contract itself.

- **Validation risk:** I did not find this helper module participating directly in the validation entry path. Any validation impact is indirect, via different optimizer finalists, not via validator semantics. Do not broaden the change into `double_ma_validate.rs` unless optimizer-output baselines are being intentionally re-audited.

- **Precondition hardening risk:** do not convert the sorted-period assumptions behind `partition_point` into production panics. Current callers already build or preserve sorted period vectors; documentation and debug assertions are enough.

- **Heuristic-preservation risk:** do not retune nearest-period snapping, mutation deltas, or refill mix ratios while fixing the contract bugs. That would mix a correctness patch with a methodology change and make benchmark interpretation much harder.

- **Caller-side defense-in-depth:** if an adjacent caller guard is added in `double_ma.rs` after CPU refill, keep it strictly defensive (for example a final truncate or debug assertion) and ensure telemetry semantics such as `promoted_pairs` and `dropped_pairs` remain interpretable.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `Small surgical refactor` remains the right action bucket.

But the current third-pass packet is too weakly grounded in the live file. This is not primarily a tolerant or compatibility helper surface. The live code still contains two concrete defects:

1. the refill helpers can blow past their requested unique-add budgets and, because the caller does not re-clamp afterward, can enlarge the next-rung frontier beyond the intended survivor cap; and
2. partial-result merging still drops `score_mode`, with a further live defect when `top_k == 0` because the final rung never repairs `best_params` under the requested score contract.

The earlier packets were directionally closer than the third pass on those two issues. The safest fourth-pass recommendation is therefore narrow and evidence-backed:

- fix the refill loop guards;
- preserve `score_mode` through partial-result merging and the final `top_k == 0` path;
- add direct regression tests for those exact contracts;
- leave broader heuristic retuning and telemetry redesign deferred.

That preserves current behavior where it is deliberate, corrects behavior where it is plainly wrong, and keeps the file in the same small-surgical bucket without repeating the third-pass misframing.
