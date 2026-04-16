# Fourth-Pass Review: `crates/app_core/src/double_ma_result.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/double_ma_result.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_result.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_result.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_result.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass packet reached the right bucket, but it flattened the live issues into a broad “compatibility seam / serde-default / invariant coverage” framing that is too abstract for this file’s actual risk profile.

`double_ma_result.rs` is still a small result-shaping module, but it sits on a real contract boundary: it turns raw optimizer output into `BackendOptimizationResult`, which is serialized, reused by follow-up flows, and carries matrix/highlight metadata that external consumers can interpret as more precise than it really is. That makes the right fourth-pass job narrower than “refactor or not?” The sharper question is which invariants should be made explicit now without changing wire shape, and which ideas should stay deferred because they would create IPC, artifact, or UI regressions.

My conclusion after reopening the live file and its local callers is that the third-pass bucket should **stand**, but the recommended change set should be tightened. The safe-now work is tests, comments, and development-time invariant checks. The earlier packets were too willing to imply that new runtime diagnostics or new hard-fail behavior would be a straightforward improvement.

## Fourth-Pass Research Log

I re-read the live source first, then validated the surrounding contracts before judging the third-pass packet.

The primary file review stayed centered on `double_ma_result.rs:15-255`. The core seams are `resolve_params` (`41-65`), `build_grid_matrix_index` (`67-104`), and `build_backend_optimization_result` (`106-255`). The local tests at `257-436` confirm that the current packet’s field-preservation framing is real, but they do not pin the edge behaviors that matter most here.

To check whether the result builder is merely cosmetic or feeds later logic, I reviewed the local optimization caller in `double_ma.rs`. That established four important facts. First, optimization currently enforces exactly one fast MA type and one slow MA type, but it still supports MA **parameter sweeps** via `fast_ma_param_sweeps` and `slow_ma_param_sweeps` (`double_ma.rs:5042-5095`). Second, variant runs are executed separately and then merged, with `fast_ma_id` / `slow_ma_id` rewritten per variant pair (`double_ma.rs:7308-7452`). Third, the caller passes `work_items = result.num_combos` and `n` back into `BackendResultContext` even though `OptimizationResult` already contains those counts (`double_ma.rs:7487-7504`, `7672-7684`). Fourth, `route_diagnostics` is not a generic “anything odd happened” note sink; it is currently GPU route provenance coming from the kernel path (`double_ma.rs:7582-7585`) and is covered by exact-string tests (`double_ma.rs:25622-25760`).

I also checked whether the resolved MA params are purely descriptive or later reused. They are not just cosmetic. Follow-up code and tests pass `res.best_params.fast_ma_params` and `res.best_params.slow_ma_params` forward into local-stability flows (`double_ma.rs:24980-24988`), and validation code treats candidate MA params as part of the GPU-fixed-candidate uniformity contract (`double_ma_validate.rs:844-855`, `889-900`). That means a silent param-resolution miss is still an internal invariant problem, but it is more than a logging inconvenience.

For external guidance, I consulted the current Rust API Guidelines dependability material, the standard-library documentation for `debug_assert!` / `debug_assert_eq!`, and Serde’s field-attribute documentation. Those sources support three review constraints that fit this file well: validate internal assumptions where practical, prefer development-time invariant checking when release behavior should not change, and preserve omission behavior for already-serialized optional fields unless all consumers move together.

## What Fourth Pass Sharpened

### The current third-pass bucket should stand

This file still belongs in `Small surgical refactor`. Nothing in the live source justifies a bucket increase. The module is small, structurally readable, and not the right place for broad representation churn.

The third-pass packet was correct about the most important high-level posture: preserve the DTO shape, avoid broad refactors, and prioritize invariant coverage over cleanup motion.

### What the current third-pass packet got right

The third-pass packet correctly treated this module as a compatibility-sensitive boundary rather than an algorithmic hotspot. That remains the right base assumption. `double_ma_result.rs` does not execute kernels, compute indicators, or own search logic; it finalizes a result contract that other layers consume. The packet was also right to avoid recommending broad structural cleanup. The live source does not show a “messy file” problem. It shows a small number of narrow invariant seams.

The packet was also directionally right that omission behavior matters. `fast_ma_params`, `slow_ma_params`, `matrix_index`, `best_col`, `top_cols`, and the other optional result fields are part of a serialized surface. Changing when they are present or absent can ripple into saved artifacts, UI interpretation, and replay flows.

### What it missed or underweighted

The sharpest missed issue is that this module now has **two sources of truth for counts**. `OptimizationResult` already carries `num_combos` and `num_candles`, but `build_backend_optimization_result` ignores those fields and emits `ctx.num_combos` and `ctx.num_candles` instead (`double_ma_result.rs:106-109`, `225-236`). The current caller keeps them aligned by explicitly setting `work_items = result.num_combos` and passing `n` from the same run (`double_ma.rs:7487-7504`, `7672-7684`), so this is not a demonstrated live bug. But it is latent drift debt in a contract-finalizer. The third-pass packet never called out that this function is already carrying redundant state that can quietly diverge later.

The second missed issue is more specific than the earlier “matrix metadata can be omitted” framing. `build_grid_matrix_index` only keys matrix metadata on `(fast_len, slow_len)` (`double_ma_result.rs:67-100`). That is fine for plain grid runs. But optimization also supports MA **parameter sweeps** (`double_ma.rs:5064-5095`), and those sweeps are merged after rewriting per-variant IDs (`double_ma.rs:7308-7452`). In other words, this file can currently emit `best_col` and `top_cols` that are only a **period-cell projection**, not a unique combo identity, whenever multiple MA parameter variants share the same lengths. That may be intentional because the merged heatmap is also period-projected (`double_ma.rs:7061-7118`), but it is not documented or tested here. The earlier packets missed that distinction and therefore missed the real compatibility question: whether downstream consumers understand these fields as highlights on a projected period grid or as identifiers for unique result rows.

The third missed issue is that silent param omission is not merely “diagnosability debt.” `resolve_params` currently does `.get(...).cloned().unwrap_or(None)` for both sides (`double_ma_result.rs:48-55`). Earlier packets framed this mostly as observability. That is too weak. A result row with missing `fast_ma_params` / `slow_ma_params` can affect follow-up fidelity because those fields are later reused by local-stability and validation code (`double_ma.rs:24980-24988`; `double_ma_validate.rs:844-855`, `889-900`). This is still not proof of a current production bug, because the live optimization path carefully constructs ids and variant arrays, but it means the right hardening lens is “internal semantic mismatch,” not just “missing note in logs.”

### What the earlier packet overstated or weakly supported

The earlier packet leaned too hard on generic serde/default language. That is not the highest-value next step for this file. The DTO types in `double_ma.rs` do carry `serde` attributes, and those are contract-sensitive, but the live issues in `double_ma_result.rs` are assembly-time invariant problems: count-source drift, period-projection ambiguity, and silent param lookup fallback. Generic serde round-trip tests are secondary here unless the actual serialized shape is being changed.

The earlier packet also suggested runtime diagnostics or even internal errors as plausible fixes for omission paths. That is riskier than it sounds. This module does not own a neutral diagnostic surface. The nearby `route_diagnostics` field is already GPU-path provenance, not a generic invariant log (`double_ma.rs:7582-7585`, `25622-25760`). Repurposing it here would blur semantics and likely confuse consumer expectations. Turning these late-stage omissions into hard `Err`s would be even riskier: it would convert a run that already produced a coherent optimization result into a tail-end failure after the expensive work is done.

Finally, the statement that there is “no meaningful performance issue” is a little too strong. There is still no evidence that this file is worth micro-optimizing as CPU work. But the file does eagerly clone MA labels and MA param `HashMap`s into every resolved `best`, `top`, `all`, and shortlist row (`double_ma_result.rs:48-64`, `118-149`, `157-223`). That is less a hot-loop compute issue than an IPC/artifact payload-size issue. The correct conclusion is not “optimize it now,” but “do not pretend there is zero cost if this result surface grows.”

## Implementation Priority

### Safe-now hardening

The best immediate work is small and conservative.

First, add development-time invariant checks rather than release-path behavior changes. `debug_assert_eq!` is a good fit for the duplicated-count seam: verify that `ctx.num_combos == result.num_combos` and `ctx.num_candles == result.num_candles` before building the final DTO. The same pattern fits the “this should not happen” branches in `resolve_params` and `build_grid_matrix_index`. If an out-of-range MA variant id or an impossible Grid-mode matrix lookup appears in development or tests, it should light up loudly there while preserving today’s release behavior.

Second, document the matrix semantics where they are easiest to misread. A short comment near `build_grid_matrix_index` should make it explicit that `matrix_index`, `best_col`, and `top_cols` are period-grid projection metadata. When MA param sweeps are merged, multiple resolved candidates can map to the same column by design. That is the kind of contract note that prevents consumer-side misuse without changing any field shape.

Third, add tests for the actual seam behavior instead of generic “more coverage.” The missing tests I would prioritize are: a count-alignment test, a Grid+param-sweep test that documents duplicate/shared column projection behavior, and a test that makes the current tolerant omission path explicit if the team chooses to keep it. Those tests are more valuable than generic serde round trips because they pin the behaviors this module truly owns.

### Larger redesign still deferred

A variant-aware matrix identity is the main redesign candidate, but it should stay deferred. If the product ever needs `best_col` or `top_cols` to identify unique `(fast_len, slow_len, fast_variant, slow_variant)` combinations, that is a versioned contract change, not a small cleanup. It would have to move together with any UI heatmap/highlight consumer, saved artifact reader, report exporter, and possibly validation-follow-up builder.

Payload deduplication is the other deferred redesign. If artifact size or IPC volume becomes a measured pain point, the likely fix is to stop cloning the same MA labels and param maps into every row and instead move to side tables or interned result metadata. That would be an IPC/persistence redesign, not a backend correctness fix, and it should not be folded into the current bucket.

## Verification If Changed

If the safe-now hardening above is implemented, the required verification stays targeted.

At minimum, run the existing `double_ma_result` unit tests and add new unit coverage for the new invariants. A focused command like `cargo test -p ta_app_core double_ma_result -- --nocapture` remains appropriate for the file-local changes.

Because the sharper missed edge is the period-projection behavior under MA parameter sweeps, also run the existing MA-sweep integration coverage in `double_ma.rs`, especially the current `cpu_ma_param_sweep_increases_combo_count_and_reports_params` path (`double_ma.rs:22621-22693`), and extend it or add a nearby test that checks the emitted matrix/highlight metadata semantics.

If the team touches param-resolution behavior rather than only assertions/comments/tests, add one follow-up or validation-path check that consumes those params, because `fast_ma_params` / `slow_ma_params` are part of later local-stability and validation uniformity logic. That is the fastest way to prove the refactor did not silently change downstream candidate interpretation.

No GPU perf gate or CUDA-specific verification is justified for the safe-now recommendations, because they do not change kernel code, residency logic, or route planning. If someone goes beyond the safe-now scope and changes serialized field behavior, then output-parity and cross-layer consumer checks become mandatory.

## Regression And Feature-Preservation Constraints

Preserve the current serialized field surface for `BackendOptimizationResult` and `DoubleMaParamsResolved`. The safe-now recommendations do not require adding fields, renaming fields, or changing `skip_serializing_if` behavior, and they should stay that way.

Do not overload `route_diagnostics` with result-finalize invariant notes. That field already has route-provenance meaning and existing exact-string expectations on the GPU path. Mixing semantic layers there would create confusing diagnostics and brittle downstream behavior.

Do not turn the current tolerant omission paths into hard user-visible errors without a coordinated caller review. A late-stage failure from `build_backend_optimization_result` would change successful optimization runs into tail-end failures across CPU and GPU paths, and it would be much more disruptive than the current packet implied.

Treat `best_col` and `top_cols` as projected heatmap metadata unless and until all consumers are audited. Changing them to mean “unique candidate identity” would be a real contract change, especially under MA param sweeps.

If anyone pursues payload deduplication later, treat it as IPC/artifact versioning work. The repeated `String` / `HashMap` cloning in this file is real, but changing that representation casually would create replay and compatibility risk that is out of proportion to the current bucket.

## Fourth-Pass Conclusion

`Bucket unchanged.` The current third-pass action bucket of `Small surgical refactor` should **stand**.

What changed in this fourth pass is not the bucket, but the precision of the action. The third-pass packet was right to avoid broad refactors and to treat this module as a compatibility seam. It missed, however, that the live file’s sharper risks are narrower and more concrete than generic serde/default concerns: duplicated count sources, undocumented period-grid projection semantics under MA parameter sweeps, and silent param lookup fallback that can matter to follow-up fidelity.

The best next step is therefore conservative hardening: add debug assertions, add targeted tests, and document the projection semantics. Do **not** introduce new runtime diagnostics, repurpose `route_diagnostics`, or turn these late-stage omissions into hard failures under the current bucket. Save any variant-aware matrix contract or payload-dedup redesign for a coordinated, versioned follow-up.
