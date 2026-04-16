# Fourth-Pass Review: `crates/app_core/src/meta_select.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/meta_select.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass packet is directionally right on one important point: this file does contain deliberate compatibility and methodology choices that should not be tightened casually. But it is too abstract to be the last word on a public, re-exported workflow surface.

The live source still mixes public request/result types, ranking methodology, grouping compatibility, and train-versus-holdout stage ownership in one module. That means the review has to do more than say “preserve tolerant behavior.” It also has to separate the genuinely safe hardening from the changes that would quietly alter ranking semantics, IPC payload meaning, or persisted meta artifacts.

My bucket call is **unchanged**: `Small surgical refactor` should stand. The reason is not that the file is already clean. The reason is that the safest justified work is still narrow: clarify the public contract, pin deterministic ordering, and leave the methodology changes deferred.

## Fourth-Pass Research Log

1. **Rust API Guidelines — Documentation and Dependability**
   - Why consulted:
     - `meta_select.rs` exports request/result types and helper functions through `crates/app_core/src/lib.rs:182-187`, so the public contract matters even if most callers currently come through higher-level orchestration.
   - Constraint informed:
     - stage ownership and no-op fields should be documented explicitly rather than left implicit.

2. **Serde field attribute guidance**
   - Why consulted:
     - `MetaSelectionRequest` relies heavily on `#[serde(default)]`, and the result types use `skip_serializing_if` on optional fields.
   - Constraint informed:
     - defaults and omitted fields are part of the wire contract; shared fields that are stage-2-only here are still user-visible contract surface.

3. **Rust standard-library docs for `HashMap` iteration and `slice::sort_by`**
   - Why consulted:
     - this file builds aggregates out of a `HashMap<String, GroupAcc>` (`crates/app_core/src/meta_select.rs:433`) and then sorts only by `meta_score` (`725-727`).
   - Constraint informed:
     - equal-score ordering is not deterministic today because `HashMap` iteration is arbitrary while `sort_by` is stable.

4. **RFC 8785 (JSON Canonicalization Scheme)**
   - Why consulted:
     - the module has a local `canonical_json` helper (`1435-1476`) and public key helpers (`1004-1115`).
   - Constraint informed:
     - deterministic key formation is important, but a “replace this with a standards-compliant canonicalizer” change would be compatibility-sensitive and should not be treated as casual cleanup.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep the bucket at `Small surgical refactor`, and it was right that tolerant parsing and compatibility behavior should not be converted into hard failures without coordinated review. That still applies to `artifact_backend_validation_approved` (`1320-1360`), the fallback-heavy shortlist logic (`1168-1313`), and the legacy-versus-generic grouping compatibility path (`1020-1115`, plus the tests at `2006-2113`).

It was also right that any change to grouping keys, canonicalization, or clustered grouping needs stronger justification than “cleaner code.” Those are semantics-bearing helpers, not just utility functions.

### What the third-pass packet missed or underweighted

#### 1. The stage split is broader than just “holdout knobs exist”

The earlier packet lineage already noticed that holdout execution is elsewhere, but the current third-pass packet underweights how much of the public type surface that affects.

This file exposes holdout-related request knobs in `MetaSelectionRequest` (`192-220`), holdout-specific weights in `MetaScoreWeights` (`18-41`), and output fields like `final_score` / `holdout` in `MetaCandidateAggregate` (`283-287`) plus `holdout` in `MetaSelectionResult` (`327-337`). But the actual train-stage computation inside `meta_select_from_artifacts` uses only the train-side weights (`684-691`) and always returns `final_score: None`, `holdout: None` at both group and result level (`713-715`, `784`).

That is not just a documentation nicety. It means a direct caller of the public re-exported API can reasonably infer behavior that this function does not implement. The third-pass packet should have said this explicitly.

#### 2. There is one concrete code hardening justified now: equal-score ordering is nondeterministic

This is the strongest missed finding.

`groups` is accumulated in a `HashMap<String, GroupAcc>` (`433`). Aggregates are then built from `for (key, mut acc) in groups` (`620`), and finally sorted only by descending `meta_score` (`725-727`). Because the iteration order of `HashMap` is arbitrary, equal-score groups inherit arbitrary pre-sort order. Because `sort_by` is stable, ties stay in that arbitrary order.

That matters in two places:

- serialized/report-visible group order can change across runs for equal scores;
- `top_k` truncation (`726-727`) can pick different members at the tie boundary.

This is not a “large redesign.” It is a small, justified hardening opportunity: keep `meta_score` as the primary sort key, but add a deterministic tie-breaker such as `key`.

#### 3. Two result counters have narrower semantics than their names imply

The third-pass packet talks about tolerant behavior in general but does not call out that two public result counters are easy to overread.

- `artifacts_used` increments before shortlist emptiness is known (`460-465`), so an artifact can be counted as “used” and then still contribute nothing because `selected.is_empty()` later rejects it (`490-493`).
- `candidates_considered` increments only after same-key dedupe inside an artifact (`509-523`, `572`), so it is not the raw number of shortlisted candidate rows.

Those may be intentional, but they are not self-explanatory. For a public result payload, that is contract ambiguity, not just internal bookkeeping.

#### 4. `pass_rate` currently excludes datasets whose artifacts matched but produced no surviving shortlisted candidate

This is a real methodology choice, and it deserves to be named as such.

A dataset is added to `datasets_considered` only after the artifact has a non-empty shortlisted set (`490-500`). Later, `dataset_universe` is derived from that filtered set (`616`), and `pass_rate` is computed as `datasets_covered / dataset_universe` (`664-668`).

So if a matched artifact is rejected by the shortlist gate, it does not count against any group’s `pass_rate`. That can materially improve `pass_rate` and downstream `meta_score` (`684-691`) compared with a denominator based on all matched datasets.

I am **not** recommending changing this in a small pass. But the third-pass packet should have called it out as a ranking-policy decision, because changing it later would absolutely move results.

#### 5. Clustered per-dataset collapse is “first surviving candidate wins,” and that depends on upstream candidate ordering

Inside each artifact, the selector computes a group key, then uses `seen_keys` to keep only the first candidate that lands in a given group (`509-523`). In clustered mode, that means multiple nearby parameterizations from the same dataset collapse to one vote, and the chosen representative for scoring is whichever shortlisted candidate appeared first.

The shortlist helpers preserve upstream candidate order; they filter but do not locally re-rank (`1178-1218`, `1225-1313`). So the current behavior depends on the ordering contract of `validation/res/candidates`, but this file does not document that dependency or test it explicitly.

Again, I would not change the behavior casually. But the third-pass packet should have treated this as a concrete approximation boundary rather than leaving the discussion at “heuristic surface” level.

#### 6. Dataset identity fallback is weaker than the repo’s own artifact guidance suggests

`artifact_dataset_key` uses `csv_path`, then `label`, then an artifact index fallback (`932-955`). Meanwhile, the repo guidance says saved run artifacts may include dataset identity/content-fingerprint metadata when available (`context__REPO_MAP.md:1429`).

That means the current grouping coverage math can still over-merge or over-split datasets if path/label fallbacks are weak or inconsistent. This is especially relevant because `dataset_key` feeds both `datasets_considered` and per-group `dataset_keys`, which in turn drive `datasets_covered` and `pass_rate`.

This is **not** a safe uncoordinated change because it touches persistence/report/ML assumptions, but it is a real missed risk.

### What earlier review guidance overstated or needs tighter framing

The third-pass packet is too weak where it stays generic, but some earlier review lineage also needs tightening.

First, the inherited idea that line `751` contains a broken encoding artifact does not hold up well against the live source. The string there contains a literal middle-dot separator (`·`), which may be a style choice someone dislikes, but it is not obviously corrupted text.

Second, a generic recommendation to “add skip-reason counters” is only safe if it stays internal or debug-only. Adding public counters to `MetaSelectionResult` would widen a re-exported IPC/persistence surface. That is not necessarily wrong, but it is not “free” for a small pass.

Third, replacing `canonical_json` with an RFC 8785 crate under a standards-compliance banner would be too aggressive for this bucket. The current helper is a local deterministic compatibility surface (`1435-1476` plus the public key helpers at `1004-1115`), not just disposable utility code.

## Implementation Priority

### Safe-now hardening

1. **Stabilize equal-score ordering in the final aggregate sort.**
   - Keep `meta_score` as the primary key.
   - Add a deterministic tie-breaker, most conservatively `key`.
   - Add a unit test that proves tie ordering is stable and that `top_k` truncation becomes reproducible at the boundary.

2. **Add explicit rustdoc on stage ownership and field semantics.**
   - `MetaSelectionRequest`: document that holdout execution fields and holdout weighting are not consumed by `meta_select_from_artifacts` itself.
   - `MetaCandidateAggregate` / `MetaSelectionResult`: document that `final_score` and `holdout` are stage-2 augmentation outputs, not train-stage outputs.
   - `meta_select_from_artifacts`: document that it is the train-stage selector and that direct callers should not expect populated holdout/final scores.

3. **Clarify the meaning of public counters without renaming them.**
   - Document that `artifacts_used` means “matched and admitted past artifact-level validation checks,” not necessarily “contributed to final groups.”
   - Document that `candidates_considered` is post-dedupe within an artifact.
   - This is a docs/comments change, not a request/result shape change.

4. **Add a note or test around current clustered same-dataset collapse semantics if the team intends to preserve them.**
   - Not because the current rule is definitely correct, but because the current rule is not obvious.

### Larger redesign still deferred

1. **Revisit the `pass_rate` denominator deliberately.**
   - Decide whether shortlist-rejected datasets should still count in the denominator.
   - Treat this as a product/ranking-policy change, because it will move meta rankings and gate outcomes.

2. **Revisit same-dataset clustered collapse deliberately.**
   - Decide whether “first candidate wins” is the intended rule, or whether the file should choose best-by-score / median / some other explicit per-dataset representative.
   - Audit upstream candidate ordering before changing anything.

3. **Audit dataset identity selection against persisted artifact identity/fingerprint metadata.**
   - Any change here touches pass-rate semantics, grouping coverage, and possibly ML/meta replay assumptions.

4. **Split or narrow the public train-stage vs holdout-stage contract only in a coordinated pass.**
   - The shared request/result types are a real clarity problem, but changing them would hit IPC, report generation, saved artifacts, and likely frontend request/result expectations.

## Verification If Changed

At minimum, if this file changes:

- `cargo test -p ta_app_core meta_select::tests:: -- --nocapture`
- `cargo bench -p ta_app_core --bench meta_select_workflow --no-run`

If the change affects serialized ordering, public fields, or train/holdout contract semantics:

- `cargo test --workspace --no-run`
- `npm --prefix web run parity:outputs`

If deterministic ordering is fixed, add a file-local test that creates two equal-score groups and asserts stable output order across repeated construction. If clustered same-dataset collapse is changed, add a file-local regression test that proves the chosen per-dataset representative rule, rather than relying on incidental candidate array order.

## Regression And Feature-Preservation Constraints

Do not change `meta_group_key*` or `canonical_json` semantics casually. Those helpers are part of grouping compatibility and saved/report-visible output behavior.

Do not silently change the meaning of `pass_rate`, `artifacts_used`, or `candidates_considered` under a “cleanup” banner. Those fields already have observable semantics, even if the semantics are currently underdocumented.

Do not rename, remove, or repurpose holdout-related request/result fields in a file-local pass. They are re-exported from `crates/app_core/src/lib.rs:182-187` and are part of a wider command/report contract.

Do not add public debug counters casually. If more diagnostics are needed, start with internal counters, logs, or a deliberately expanded debug contract rather than opportunistically widening `MetaSelectionResult`.

A deterministic tie-breaker is the exception: it is a real hardening improvement, but it still changes observable ordering for equal-score groups. Treat that as an intentional output stabilization and update any ordering-sensitive snapshots or report tests accordingly.

If dataset-key logic changes, audit all downstream consumers that compare or aggregate datasets across saved artifacts. The repo guidance already treats artifacts as canonical cross-workflow handoff objects, so a dataset-identity tweak is not a local-only concern.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: `Small surgical refactor` is still the right action bucket.

But the third-pass reasoning is too compressed. The live file is not just a generic “heuristic policy surface.” It is also a public, re-exported contract surface with at least one concrete reliability issue today: nondeterministic equal-score ordering caused by `HashMap` iteration plus a single-key final sort.

So the right next step is narrower and sharper than the current third-pass packet suggests:

- make the final ordering deterministic;
- document the train-stage versus holdout-stage contract explicitly, including holdout weights and `final_score` semantics;
- document the current meaning of the public counters;
- defer denominator, clustered-collapse, dataset-identity, and request/result reshaping decisions until they can be changed deliberately and verified across the wider meta/report/artifact workflow.

That keeps the bucket small, preserves current behavior where the behavior is clearly policy-laden, and still fixes the one low-risk defect that should not wait.
