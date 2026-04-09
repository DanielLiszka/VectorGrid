# Meta Evidence Flags Plan

Last updated: 2026-04-09

## Purpose

Define the current Stage 3 meta evidence-flag contract so warning thresholds, post-holdout refresh behavior, frontend/report surfacing, and the distinction between warnings and hard gates remain explicit and stable.

## Scope Boundaries

In scope:

- per-group `evidence_flags` generation
- run-level warning aggregation based on flagged groups
- post-holdout flag refresh behavior
- frontend display and preview behavior for evidence flags
- artifact/report implications of evidence quality warnings

Out of scope:

- hard gating rules that eliminate groups entirely
- Stage 5 ML ranking and confidence logic except where it can be confused with evidence flags
- broader Stage 3 grouping/clustering beyond how it affects evidence thresholds
- options workflows

## Current User-Facing Behavior

Stage 3 grouped results can carry evidence flags that warn about thin, weak, or noisy support. These flags are not themselves hard rejections. They are serialized into grouped results, counted into run-level warnings, and lightly surfaced by the frontend in preview and summary behavior. Current behavior includes:

- flagged groups still remain selectable unless other gating logic removes them
- a run-level warning is added when any grouped result carries evidence flags
- after holdout evaluation, evidence flags are refreshed again using the holdout-enriched group state
- frontend preview behavior uses evidence flags as warning context rather than as the primary ranking field
- workbook/export surfaces are more score-centric than evidence-centric today

## Current Code Surface

- `crates/app_core/src/meta_select.rs`
  - `compute_meta_group_evidence_flags()`
  - `refresh_meta_group_evidence_flags()`
  - run-level evidence warning generation
- `src-tauri/src/commands_meta.rs`
  - post-holdout evidence-flag refresh and holdout-time warning aggregation
- `web/src/services/metaService.ts`
  - frontend consumption of `group.evidence_flags` and preview-summary warnings
- `web/src/components/MetaWorkspacePanel.vue`
  - grouped result presentation surface that indirectly exposes evidence quality
- `src-tauri/src/run_artifacts.rs`
  - persistence of grouped results and warning-bearing artifacts
- `src-tauri/src/commands_reports_meta_xlsx.rs`
  - current workbook/report surfaces for grouped outputs

## Current Verification Surface

- tests in `crates/app_core/src/meta_select.rs`
  - especially `meta_select_populates_evidence_flags_for_thin_groups`
- Stage 3 / holdout tests in `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.test.ts`
- report/export tests in `src-tauri/src/commands_reports_meta_xlsx.rs`
- artifact replay/detail tests where grouped warnings matter

## Strengths Already Present

- evidence flags are already computed centrally in Rust instead of being inferred heuristically in the UI
- warning thresholds are explicit and small enough to reason about
- flags are additive warnings rather than overloaded gates
- holdout refresh behavior already exists, so evidence can evolve with richer evaluation data
- run-level warning strings already communicate when evidence is thin or noisy

## Weaknesses / Failure Modes

- frontend surfacing is relatively weak compared with the backend detail available in `evidence_flags`
- it is easy to confuse evidence flags with the separate hard-gating logic
- flags are string-coded, which is easy to persist but easy to under-document
- report exports currently emphasize scores more strongly than evidence quality
- post-holdout refresh can change the warning picture, but the before/after distinction is not strongly exposed

## Regressions To Avoid

- turning evidence flags into silent gates without documenting the semantic change
- weakening current warning thresholds without updating tests and UX language together
- renaming flag strings in a way that breaks replay, report interpretation, or frontend assumptions
- dropping evidence warnings from artifacts while leaving them in transient UI state only
- claiming workbook exports fully cover evidence quality when the strongest evidence surface is still the JSON/artifact payload
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because evidence flags are a transparency surface; warning semantics are more valuable when they are explicit and interpretable rather than hidden inside rankings
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because part of the broader meta/ML stack depends on distinguishing confidence-like signals from noisy or weak evidence
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because evidence flags are persisted as structured JSON fields that should keep stable semantics
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because evidence warnings cross the command boundary and should remain structured rather than reduced to opaque text too early
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document evidence flags as a warning system that is separate from hard-gate rejection.
2. Record every current threshold that can produce an evidence flag.
3. Document the post-holdout refresh path so flag changes after holdout are expected rather than surprising.
4. Clarify how evidence flags are surfaced today:
   - per-group JSON payload
   - run-level warning strings
   - frontend preview/summary behavior
   - comparatively weaker workbook exposure
5. Preserve string stability or introduce a versioned compatibility plan before renaming existing flags.
6. Improve future docs and reports so evidence quality is not overshadowed by score-only presentation.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document every current evidence threshold and resulting flag family.
- [ ] Document that evidence flags are warnings, not direct gates.
- [ ] Document the post-holdout evidence refresh step.
- [ ] Document run-level warning aggregation when flagged groups exist.
- [ ] Document current frontend preview/summary use of `evidence_flags`.
- [ ] Document current report/export limitations around evidence visibility.
- [ ] Add or maintain backend tests for thin-group evidence behavior.
- [ ] Add or maintain frontend/report tests if evidence surfacing changes.
- [ ] Preserve current string-coded flag compatibility unless a migration exists.
- [ ] Keep artifact replay and saved-result interpretation aligned with the documented warning semantics.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run targeted tests in `crates/app_core/src/meta_select.rs`
- run targeted Stage 3 / holdout tests in `src-tauri/src/commands_meta.rs`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run targeted report/export tests in `src-tauri/src/commands_reports_meta_xlsx.rs`
- manually inspect one grouped result JSON payload and one meta report/output path for evidence visibility
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve the current warning-only meaning of evidence flags until there is an explicit product decision to tighten them
- if frontend or workbook surfaces become richer, make those changes additive to the current artifact payload
- keep older evidence-flag strings replayable even if future labels or explanations become more user-friendly
- prefer stronger documentation and surfacing before changing the threshold rules themselves
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/10_meta_ml_confidence_reports_2026-04-09.md`
- `REPO_Improvement/08_ml/23_meta_grouping_clustering_2026-04-09.md`
- `REPO_Improvement/08_ml/24_meta_scoring_weights_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
