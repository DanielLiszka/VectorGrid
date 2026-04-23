# Fourth-Pass Review: `src-tauri/src/commands_meta.rs`

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_meta.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file was worth a fourth pass because the third-pass packet was directionally sound on task-contract honesty and model-store hardening, but it was still aimed mostly at orchestration hygiene. The live source has a narrower, more local correctness issue that is easier to miss and more important than another round of generic cancellation commentary: the offline ML-policy-evaluation path is currently willing to rank candidates with holdout-derived `final_score` even though the evaluation label is also computed from the same holdout payload.

After re-reading the live file, the earlier packets, `src-tauri/src/state.rs`, and `crates/app_core/src/ml_confidence.rs`, I do **not** think this file has crossed into redesign territory. The current bucket should stand. The right adjustment is priority and scope: fix the local policy-evaluation integrity issues first, keep the read-only/status and atomic-write hardening, and narrow the cancellation recommendation so it does not pretend a command-layer patch can add real cooperative cancellation to CPU-bound ML training that currently has no cancel plumbing.

## Fourth-Pass Research Log

I re-read the live `commands_meta.rs` rather than treating the third-pass packet as ground truth, then cross-checked only the nearby files that materially constrain recommendations: `src-tauri/src/state.rs` for the shared run guard and cancel token, `src-tauri/src/lib.rs` / `src-tauri/src/commands.rs` for the public command surface, and `crates/app_core/src/ml_confidence.rs` for what Stage 5 / policy-evaluation training actually does.

I also re-checked the relevant official docs before deciding how hard to push on cancellation and filesystem recommendations. Tauri’s current command and state-management docs still frame commands and managed state as public runtime contracts, not incidental helpers. Tokio’s current task documentation still makes the critical point that `spawn_blocking` work cannot be force-aborted once it has started. Rust’s current `std::fs` docs still support temp-write-plus-rename as the clean bounded replacement primitive, and still warn that `remove_file` is not guaranteed to make deletion observable immediately on every platform. Those constraints matter here because this file uses `#[tauri::command]`, shared app state, `spawn_blocking`, direct JSON model writes, and direct model deletes.

The practical effect of that research is straightforward. The file still deserves conservative hardening, but the safe-now fixes should stay local to result integrity and bounded persistence behavior. A true backend cancellation retrofit would extend beyond this file into `ta_app_core::ml_confidence` training loops and is therefore not the same kind of change as the other safe-now items.

## What Fourth Pass Sharpened

### The current third-pass bucket should stand

The third-pass packet was right to keep this in `Small surgical refactor`. Nothing in the live file justifies a bucket jump to coordinated redesign. The biggest remaining issues are still fixable with targeted local edits plus a small amount of cross-file contract cleanup.

### What the third-pass packet got right

It correctly identified three real weaknesses that still stand in the live source.

First, `ml_confidence_model_status` is not truly read-only because status resolution still flows through `ml_confidence_model_path`, which calls `ml_confidence_models_dir`, which eagerly creates `app_local_data_dir/models/confidence` (`commands_meta.rs:278-292`, `895-900`). That is a real contract smell for a status-style command.

Second, the packet was right that command-family cancellation semantics are inconsistent. `run_meta_selection` takes the shared cancel token and checks it during scan and holdout work (`1933-1945`, `2018`, `2122`, `2768`), while `run_meta_stage4_experimental`, `run_meta_stage5_ml_confidence`, and `evaluate_ml_confidence_policy` only take the shared run guard and then hand blocking work to `spawn_blocking` without any analogous cooperative polling (`1876-1910`). `AppState` itself confirms that the run guard and cancel token are separate concepts (`state.rs:559-574`).

Third, the packet was right that model mutation currently writes directly to the final scope-local file path in both manual training and Stage 5 auto-train (`952-966`, `1524-1550`) and deletes that same path directly (`1033-1045`). That is still the right place to ask for atomic replacement hardening.

### What the third-pass packet missed: offline policy evaluation currently leaks holdout signal into the baseline it is trying to measure

This is the most important new finding from the fourth pass.

`evaluate_ml_confidence_policy_from_scoped_metas` computes the binary evaluation label from each group’s saved holdout aggregate via `group_holdout_label` (`608-629`, used at `1377-1379`). But the same evaluator then derives the candidate’s “meta” ranking score via `group_rank_score` (`1180-1184`, used at `1389-1394`). The problem is that `group_rank_score` prefers `final_score`, and this file’s own holdout application path recomputes `final_score` from `meta_score` **plus** the holdout fields `pass_rate`, `score_p10`, and `max_dd_p90` (`2601-2610`).

That means that whenever policy evaluation is operating on the saved meta artifacts it can actually label, the `meta_only` arm and the meta component inside `stage5_blended` are allowed to rank with a score that already includes the same holdout data being used as the target label. This is not a theoretical “maybe stale” concern. It is a direct evaluation-contamination path in the current file.

The safe fix is narrow and should stay narrow: change the **policy-evaluation** candidate build so it uses raw saved `meta_score`, not `group_rank_score` / `final_score`. I would **not** recommend changing `group_rank_score` globally in this pass, because the live Stage 4 / Stage 5 rerank path appears intentionally tied to the saved Meta artifact’s current displayed ranking (`1636-1647`). The integrity problem is specific to offline policy evaluation.

### What the third-pass packet also missed: the `meta_only` arm is not fully meta-only even on ties

`select_top_labels_by_metric` sorts by the requested metric first, but then breaks ties with `ml_confidence` before falling back to `meta_score` (`1218-1233`). `evaluate_policy_candidates` uses that same helper for both `meta_only` and `ml_only` (`1241-1248`).

For `ml_only`, using `meta_score` as a secondary tie-break is defensible. For `meta_only`, using `ml_confidence` as a tie-break is not. It lets the ML model influence the arm that is supposed to represent the non-ML baseline.

This is smaller than the `final_score` leakage because it only affects exact or near-exact tie regions, but it is still a real arm-purity bug. The bounded fix is to split or parameterize the helper so the meta arm breaks ties with a deterministic non-ML value such as original group order or explicit group key, not `ml_confidence`.

### What the third-pass packet underweighted: real cancellation for Stage 5 and policy evaluation is not a command-only patch

The packet was right to call the current semantics misleading. It was too optimistic about the shape of the fix.

From the live backend alone, the narrow fact pattern is this: the affected commands all use `spawn_blocking` (`946`, `957`, `997`, `1038`, `1882`, `1894`, `1906`), only `run_meta_selection` passes the shared cancel token into its blocking implementation (`1938-1945`), and the `ta_app_core::ml_confidence` training entry points do not currently accept a cancel token or progress sink (`ml_confidence.rs:1421-1705`). Tokio’s current task docs still say that `spawn_blocking` tasks cannot be aborted once they have started.

So the third-pass packet is directionally right that the current contract is misleading, but the safe-now fix is **not** “just wire cancel checks through Stage 5 and policy evaluation” as though this were equivalent to the existing `run_meta_selection` path. The safer immediate correction is task-contract cleanup on the frontend / task-store side for any command currently advertised as cancelable without backend cooperation. A real backend cancellation retrofit is larger and should stay deferred until `ta_app_core::ml_confidence` grows explicit cancel plumbing.

### What the third-pass packet overstated: reusing the shared run mutex for model CRUD would be a broader behavior change than the packet acknowledges

The packet was right to worry about overlapping train/apply/delete traffic against the same file. It was too casual about the proposed locking shape.

`AppState::try_begin_run` is app-global, not scope-local (`state.rs:559-567`). Reusing it for model CRUD would serialize unrelated long-running workflows and model mutations behind the same single run slot. That is materially broader than “make model writes safer.”

The bounded safe-now hardening is still temp-write-plus-rename for model replacement. That reduces partial-write exposure without redefining the command family’s concurrency semantics. If the product later decides that model mutation really needs exclusion, that should be a separate per-scope coordination design, not an incidental consequence of reaching for the existing global run mutex.

### One smaller local edge case the packet missed

`suggest_meta_report_path` tries the base filename and then suffixes `_2` through `_999`, but if every one of those paths already exists it returns the last `path` value anyway (`1955-1975`). In that exhaustion case, the caller can overwrite an existing report instead of failing closed.

This is a low-probability edge case and not a reason to expand the bucket, but it is a real local bug. If this helper is touched, the right outcome after suffix-space exhaustion is an error, not reuse of the last existing candidate path.

### One contract smell that is real but should stay deferred

`MetaStage4Result` exposes `model_healthy` as a plain `bool` (`218-226`), but the live Stage 4 path distinguishes three states internally: healthy, unhealthy, and legacy-with-no-health-metadata (`1586-1605`). The current code collapses the legacy case to `false` via `unwrap_or(false)` (`1586-1587`, serialized at `1832-1834`).

That is lossy and can mislead downstream consumers into reading “unknown legacy health” as “explicitly unhealthy.” I still would **not** change it in this pass, because the field shape is part of the saved-artifact/UI contract. It belongs in the deferred bucket, not the safe-now bucket.

## Implementation Priority

### Safe-now hardening

The first safe-now change should be **policy-evaluation baseline purity**, not more cancellation discussion. In `evaluate_ml_confidence_policy_from_scoped_metas`, build `PolicyEvalCandidate.meta_score` from raw saved `meta_score`, not `group_rank_score`. That keeps offline policy evaluation from using holdout-derived `final_score` inside the same artifact it is labeling from holdout.

The second safe-now change should be **arm purity on ties**. Split or parameterize `select_top_labels_by_metric` so the `meta_only` arm does not consult `ml_confidence` as a tie-breaker. This is small, local, and independent of any frontend contract migration.

After those two local correctness fixes, keep the earlier packet’s bounded hardening items: surface unreadable-artifact skips during meta scan (`327-330`, `2021-2029`), add an explicit warning when holdout evaluation processed zero usable datasets instead of silently returning `Ok(())` (`2767-3092`), make `ml_confidence_model_status` truly read-only by separating path construction from directory creation (`278-292`, `895-900`), and replace direct model writes with temp-write-plus-rename (`964-966`, `1549-1550`).

For cancellation, the safe-now move is **contract cleanup**, not a pretend backend retrofit. If any frontend/task-store surface currently marks Stage 4 / Stage 5 / ML-policy-evaluation / ML-train as cancelable in the same sense as `run_meta_selection`, that should be corrected first. That preserves honesty without forcing a bigger `ta_app_core` training redesign into a surgical pass.

Finally, the `_999` report-path exhaustion case is worth fixing if the helper is touched, but it is the lowest-priority item in this pass.

### Larger redesign still deferred

A real cooperative-cancellation path for Stage 4 auto-train, manual ML training, and ML-policy evaluation is deferred. It requires explicit cancel plumbing through `ta_app_core::ml_confidence` training and evaluation loops, not just different command wrappers.

Scope-local model-mutation locking is also deferred. Atomic replacement is enough for the current pass; introducing per-scope mutexes, lock files, or shared-run-slot participation is a broader concurrency-policy decision.

Finally, any attempt to fix the lossy `model_healthy` field or split the Stage 4 / Stage 5 alias into distinct saved-artifact families remains a coordinated contract migration, not a local cleanup.

## Verification If Changed

At minimum, rerun the existing targeted Rust coverage for this module and add new tests that do **not** merely mirror the production logic. The current in-file policy-evaluation reference helper repeats the same `group_rank_score` assumption as the implementation (`3310-3341`), so it will not catch the leakage fix by itself.

The highest-value new regression tests are:

1. A policy-evaluation fixture where saved groups have both `meta_score` and holdout-adjusted `final_score`, and those two rankings differ. The expected `meta_only` and `stage5_blended` offline-evaluation paths should follow raw `meta_score`, not `final_score`.
2. A tie fixture where two candidates have equal `meta_score` but different `ml_confidence`. The `meta_only` arm should be stable and deterministic without consulting `ml_confidence`.
3. A status-path test proving that `ml_confidence_model_status` does not create the confidence-model directory when it did not already exist.
4. A model-write test proving that the on-disk JSON is always parseable across replacement, and that no direct-final-path partial write is observable during normal replacement.
5. A helper test for `suggest_meta_report_path` that fails when all suffix slots are exhausted instead of silently returning an existing path.

The same repo-level commands from the earlier packets still make sense if this file changes:

- `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`

If task-store/frontend cancel semantics are changed instead of backend cancellation being retrofitted, rerun the relevant frontend tests that cover Meta task tracking and notifications in lockstep with the backend contract change.

## Regression And Feature-Preservation Constraints

Do **not** globally replace `group_rank_score` with raw `meta_score`. That helper is also used by the live Stage 4 / Stage 5 rerank path (`1645-1650`), and a global change would alter user-visible reranking on saved Meta artifacts. The purity fix belongs in offline policy evaluation only.

Do **not** advertise “real cancellation” for the `spawn_blocking` ML workflows unless the backend actually gains cooperative cancel checks. Contract cleanup is safer than pseudo-cancellation.

Do **not** route `ml_confidence_model_status` through the shared run slot just to stop directory creation. The read-only fix should stay read-only.

Do **not** solve model-write safety by blindly reusing the app-global run mutex. That would change concurrency semantics for unrelated workflows and is broader than the bug warrants.

Do **not** change `MetaStage4Result.model_healthy` from `bool` to `Option<bool>` in isolation. That is a saved-artifact / UI contract change and should only happen with coordinated consumer updates.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: this file is still a `Small surgical refactor`, not a redesign candidate.

The third-pass packet got the broad direction right on status-path mutation, direct-final-path model writes, and cancellation-contract inconsistency. What it missed is the sharper local correctness problem: offline ML-policy evaluation is currently willing to rank with holdout-derived `final_score`, and even the `meta_only` arm can let ML influence tie-breaks. Those two issues are more specific, more local, and more worth fixing now than another round of abstract cancellation advice.

So the fourth-pass conclusion is narrower and stricter. Keep the bucket. Keep the status and atomic-write hardening. Keep the warning work for unreadable artifacts and zero-dataset holdout runs. But reorder the priority so policy-evaluation integrity comes first, and narrow the cancellation recommendation so the safe-now fix is contract honesty rather than a backend cancellation retrofit that the current `ta_app_core::ml_confidence` APIs do not yet support.
