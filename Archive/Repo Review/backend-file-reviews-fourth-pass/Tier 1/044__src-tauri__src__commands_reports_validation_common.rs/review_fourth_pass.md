# Fourth-Pass Review: `src-tauri/src/commands_reports_validation_common.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_reports_validation_common.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Targeted hardening now; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet correctly treated this file as export-contract-sensitive and compatibility-sensitive, but it was still too defer-heavy for the current live source.

After re-reading the live file first, then the third-pass packet, the main conclusion is that the broad redesign should still stay deferred, but the “do nothing now” bucket should not stand. The file still contains bounded correctness defects with local blast radius in export shaping itself, not just structural debt:

1. copied nested regime-bucket metadata drifts out of sync with the outer candidate rows; and
2. a structurally present but semantically empty `run_verdict` object can clear every approved row during model assembly.

Those are safe-now hardening targets. They do not require re-architecting the file, changing validation-engine behavior, or changing GPU/runtime code.

## Fourth-Pass Research Log

I re-read the live source with focus on the row-building and post-processing paths, especially `parse_run_verdict()` at `src-tauri/src/commands_reports_validation_common.rs:698-758`, `sort_rows()` at `src-tauri/src/commands_reports_validation_common.rs:2264-2309`, `build_rows()` at `src-tauri/src/commands_reports_validation_common.rs:2311-2518`, `build_model_from_validation_res_with_context()` at `src-tauri/src/commands_reports_validation_common.rs:2649-2727`, and plateau sequencing across `apply_plateau_analysis()` at `src-tauri/src/commands_reports_validation_common.rs:1288-1317` and `build_model_from_report()` at `src-tauri/src/commands_reports_validation_common.rs:2581-2593`.

I re-read the third-pass packet first, then used the first- and second-pass packets only to check whether the live source had already closed earlier concerns or whether the third pass had narrowed the problem too aggressively.

I also checked the producer-side contract in `crates/app_core/src/validation_pipeline.rs`. The modern candidate verdict contract always includes `passed` in `ValidationCandidateVerdict` at `crates/app_core/src/validation_pipeline.rs:39-46`, and the modern run verdict contract always includes both `passed` and `approved` in `ValidationRunVerdict` at `crates/app_core/src/validation_pipeline.rs:69-95`. That matters because the export layer currently treats any object-shaped `run_verdict` as authoritative even when those required booleans are absent.

No external browsing was needed for this pass because the question was bounded to repo-local contracts and live source behavior.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **change**.

`Larger refactor but defer` is still right for the broad cleanup story, but it is no longer the right “next action” bucket. The live file has at least two bounded hardening tasks that are concrete, local, and evidence-backed. The better fourth-pass bucket is:

**`Targeted hardening now; larger redesign still deferred`**

### What the third-pass packet got right

The third-pass packet was correct that this file is not a good candidate for casual decomposition. It still mixes raw JSON traversal, approval semantics, ranking logic, and report narration, so broad refactoring remains high-risk.

It was also correct that run-level rejection can desynchronize nested regime-bucket data from the outer candidate row, and it was correct to keep legacy/compatibility behavior conservative unless the artifact-version contract is intentionally revisited.

### What the third-pass packet missed or underweighted

#### 1. The nested regime-bucket bug is broader than “approval label goes stale after run-level rejection”

The third pass found a real bug, but it scoped it too narrowly.

`build_rows()` copies outer-row metadata into `regime_bucket_rows` before sorting, including `row.rank = idx + 1` and `row.approval_label = approval_label.clone()` at `src-tauri/src/commands_reports_validation_common.rs:2392-2399`. After that, `sort_rows()` rewrites only the outer `row.rank` at `src-tauri/src/commands_reports_validation_common.rs:2306-2308`. That means nested bucket rank is already stale after the **initial** sort, even when there is no run-level rejection at all.

Then the third-pass bug still applies on top of that: run-level rejection mutates outer `row.final_approved`, `row.approval_label`, and `row.approval_reason` at `src-tauri/src/commands_reports_validation_common.rs:2659-2666`, but the nested bucket labels are never resynchronized.

So the real safe-now issue is not “patch one rejection path.” It is: **all copied nested regime-bucket presentation metadata needs one resync point after any row reorder or approval-state rewrite.**

That is a bounded local fix.

#### 2. A placeholder `run_verdict` object can falsely reject every approved row

This is the strongest missed finding in the current packet.

`parse_run_verdict()` treats any object-shaped `run_verdict` as present and defaults missing booleans to `false` at `src-tauri/src/commands_reports_validation_common.rs:698-758`. Then `build_model_from_validation_res_with_context()` applies run-level rejection to any parsed verdict where `approved == false` at `src-tauri/src/commands_reports_validation_common.rs:2658-2666`.

So a malformed payload like:

```json
{ "run_verdict": {} }
```

is enough to become “authoritative rejection” in export shaping. The outer rows can be downgraded to `not_approved` with the generic reason `"run verdict rejected final approval"` even though the verdict object carried no real approval signal.

That is not just broad compatibility ambiguity. It is a concrete false-negative path in the exporter.

It is also a good safe-now hardening candidate, because the producer-side modern contract in `crates/app_core/src/validation_pipeline.rs:69-95` always includes `passed` and `approved`. Requiring those fields, or at least requiring one clearly semantic field before treating `run_verdict` as authoritative, would not be narrowing the real modern contract.

#### 3. The compatibility problem is not only “malformed modern artifacts downgrade into compatibility mode”

The third-pass framing was too one-directional.

Global `compatibility_mode` is computed from `has_candidate_verdicts` and `has_run_verdict` at `src-tauri/src/commands_reports_validation_common.rs:2318-2329`. The candidate-side check is `any`, not `all`. Meanwhile, rows missing `validation_verdict.passed` still fall back to legacy shortlist approval at `src-tauri/src/commands_reports_validation_common.rs:2372-2380`, and `approval_label_and_reason()` only emits `compat_*` labels when the global compatibility flag stays true at `src-tauri/src/commands_reports_validation_common.rs:650-696`.

So mixed-schema artifacts can fail in both directions:

- some malformed shapes do downgrade into compatibility behavior; but
- other partially modern shapes can suppress compatibility mode globally and still let verdict-less rows inherit legacy shortlist approval or ordinary review labels.

That means the third-pass packet was right to defer broad compatibility redesign, but it understated how asymmetric the current failure modes are.

I would **not** change that full behavior in a small patch, because it affects how historical artifacts are classified. But it should be explicitly called out as deferred design debt, not treated as a one-sided fallback issue.

#### 4. `ExportCrossSummary.shortlist_count` is not derived from the same shortlist logic as the rest of the file

`parse_cross_summary()` computes `shortlist_count` as `robust_pareto && cross_ticker_pareto` only at `src-tauri/src/commands_reports_validation_common.rs:895-906`.

That is not the same as the file’s own shortlist semantics:

- `legacy_shortlist_flags()` adds CSCV filtering and fallback behavior at `src-tauri/src/commands_reports_validation_common.rs:554-633`; and
- `shortlist_rows()` later prioritizes `final_approved`, then hard-gate survivors, then legacy shortlist, then the remainder at `src-tauri/src/commands_reports_validation_common.rs:2548-2578`.

So the cross summary can report a shortlist count that does not match either legacy shortlist behavior or the actual displayed shortlist grouping.

I would not flip that behavior blindly without checking workbook copy/intent, but the third-pass packet should have flagged this as an internal consistency gap.

#### 5. `plateau_score` looks like a ranking tie-breaker, but cannot currently affect report ordering

`sort_rows()` includes `plateau_score` in the comparator at `src-tauri/src/commands_reports_validation_common.rs:2284-2288`.

But in the report path, plateau enrichment is only applied **after** `build_model_from_validation_res_with_context()` has already built and sorted the rows, because `build_model_from_report()` calls `apply_plateau_analysis()` afterward at `src-tauri/src/commands_reports_validation_common.rs:2590-2592`. `apply_plateau_analysis()` then rebuilds `approved_rows` and `shortlist_rows`, but it does **not** resort `all_rows` at `src-tauri/src/commands_reports_validation_common.rs:1288-1317`.

So the current code expresses a ranking policy that it does not actually deliver on the report path.

This is not the same kind of safe-now bug as the `run_verdict` issue, because changing row order is user-visible. But it is strong evidence that the file still contains policy/annotation coupling that should remain in the deferred redesign bucket.

#### 6. The older regime-support canonicalization issue remains live in the source

The live source still derives fallback support labels like `trend_ok`, `mixed`, `range_only`, and `weak` in `derive_regime_support_label()` at `src-tauri/src/commands_reports_validation_common.rs:289-301`, and `normalize_regime_support_label()` still only trims whitespace at `src-tauri/src/commands_reports_validation_common.rs:368-374`.

Earlier packets already raised downstream consumer-alignment risk here. The third-pass packet did not revisit it. I would not elevate it above the two local correctness bugs without re-opening the workbook/frontend consumers in the same change set, but it remains unresolved in the live source.

### Claims in the third-pass packet that are too weak or too strong

The third-pass packet was **too narrow** when it described the concrete bug as a run-level rejection problem. The nested metadata drift starts earlier than that because nested ranks are stale right after the initial sort.

It was **too defer-heavy** when it kept the overall action bucket unchanged. The placeholder-`run_verdict` rejection path is a real correctness issue with a bounded fix surface.

It was also **too one-directional** in how it described malformed artifact behavior. The live file does not only downgrade malformed data into compatibility shaping; it can also suppress compatibility signaling and treat partial modern data as authoritative enough to approve or reject rows.

## Implementation Priority

### Safe-now hardening

1. **Resynchronize nested regime-bucket metadata after any row reorder or approval rewrite.**  
   Add one helper that copies outer `row.rank` and `row.approval_label` back into each `regime_bucket_row` after sorting or after run-level approval-state mutation. Keeping `params_summary` in that helper as well is reasonable, since it is also copied metadata. This is the clean bounded fix for both the third-pass bug and the missed stale-rank bug.

2. **Make `run_verdict` authoritative only when it is structurally modern enough.**  
   A conservative rule is to treat `run_verdict` as present only when `approved` or `passed` is explicitly boolean, or when another clearly semantic field is present. Then use that stronger predicate both for run-level rejection and for the model’s compatibility decision. Given `ValidationRunVerdict` in `crates/app_core/src/validation_pipeline.rs:69-95`, this is a safe-now hardening step, not a schema redesign.

3. **Add direct unit coverage for these paths in this file.**  
   The current direct tests at `src-tauri/src/commands_reports_validation_common.rs:2730-2923` are route-summary tests only. They would not catch any of the issues above. Add targeted tests for:
   - initial sort keeping nested bucket rank synchronized;
   - run-level rejection keeping nested bucket approval labels synchronized;
   - empty `{}` `run_verdict` not clearing approvals;
   - partial `run_verdict` missing `approved`/`passed` not being treated as authoritative modern verdict.

### Larger redesign still deferred

1. **Decide mixed-schema artifact policy explicitly.**  
   The current `compatibility_mode` flag is global, but row completeness is not. That is not a good target for ad hoc local cleanup.

2. **Decide whether plateau is explanatory-only or allowed to influence ranking.**  
   Right now the comparator and the call order disagree.

3. **Unify shortlist semantics across the file before changing exported counts or labels.**  
   `legacy_shortlist_flags()`, `parse_cross_summary()`, and `shortlist_rows()` currently encode different shortlist ideas.

4. **Revisit regime-support canonicalization only with downstream consumers open.**  
   This still looks coordination-sensitive.

5. **Keep the larger module split / typed-DTO migration deferred.**  
   The file is still too semantics-sensitive for a cleanup-only refactor.

## Verification If Changed

Run the existing targeted backend tests:

- `cargo test -p ta_desktop_demo_app commands_reports_validation_common -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture`

Add direct tests in `commands_reports_validation_common.rs` for:

- stale nested rank after initial `sort_rows()`;
- stale nested approval label after run-level rejection;
- empty or placeholder `run_verdict` objects;
- partially modern candidate sets where some rows lack `validation_verdict.passed`.

If `run_verdict` hardening is implemented, verify at least one modern fully-populated verdict payload and one legacy/compatibility payload so the patch proves it preserves real modern artifacts while only rejecting placeholder verdict objects.

## Regression And Feature-Preservation Constraints

Do **not** change legacy shortlist precedence or compatibility behavior for clearly legacy artifacts in the same patch that hardens placeholder `run_verdict` handling. Those are different risk classes.

Do **not** change route-note wording, benchmark-sensitive narrative text, or composite-route copy in the same patch. The current safe-now work is about export correctness, not policy narration.

Do **not** silently change plateau-driven ordering as part of the nested-metadata sync fix. Plateau ordering is still a product/export-contract decision because row order is user-visible.

There is no GPU ABI or IPC risk here. This file is export-model shaping only. The real regression surface is persistence and report meaning: saved artifacts, workbook summaries, CSV exports, and any frontend flow that consumes those exports.

If `ExportCrossSummary.shortlist_count` is changed later, verify workbook/CSV copy carefully because the field’s current semantics are already ambiguous.

## Fourth-Pass Conclusion

The third-pass packet was directionally right about **deferring the large redesign**, but it was too conservative about the next actionable step.

The third-pass bucket should **not** stand as-is. The live source supports at least two bounded hardening steps now:

- synchronize copied nested regime-bucket metadata after sorting and run-level row mutation; and
- stop treating any object-shaped `run_verdict` as authoritative rejection input.

Those are local, evidence-backed fixes. They do not require changing validation-engine semantics or taking on the larger compatibility redesign.

The broader redesign still belongs in the deferred bucket: mixed-schema artifact policy, plateau-order semantics, shortlist unification, route-note ownership, and any decomposition of this file remain contract-sensitive and should not be folded into the safe-now patch.

So the right fourth-pass posture is:

**fix the local export-shaping bugs now, and keep the larger refactor deferred.**
