# Meta Holdout Modes Plan

Last updated: 2026-04-09

## Purpose

Define the current Stage 3 holdout-mode contract so `light` versus `heavy` holdout behavior, holdout ranking controls, saved holdout rows, and route-specific warnings remain explicit, testable, and regression-resistant.

## Scope Boundaries

In scope:

- Stage 3 holdout enablement and supported-strategy gating
- `light` versus `heavy` holdout semantics
- holdout ranking controls:
  - `holdout_eval_top_n`
  - `holdout_eval_explore_n`
  - `holdout_eval_rank_by`
- holdout cluster representative count and shortlist interaction
- heavy-holdout validation payload construction
- holdout-row persistence and report impact

Out of scope:

- generic Stage 3 grouping/clustering behavior outside holdout use
- Stage 5 ML-confidence ranking and blend weights
- batch campaign resume/retry behavior
- options workflows

## Current User-Facing Behavior

The Meta workspace exposes holdout as a first-class Stage 3 control surface. Users can set a holdout tag, choose `light` or `heavy` mode, choose representatives per cluster, enable or disable holdout evaluation, optionally save per-dataset holdout rows, set top-N holdout evaluation, choose how holdout ranks are computed, and explore additional finalists. Current shipped behavior means:

- holdout is available only for a strategy allowlist rather than every Stage 3 strategy
- `light` holdout requests do not emit nested validation config, while `heavy` holdout does
- unsupported strategies silently lose holdout at request-build time instead of being allowed to drift into the backend
- `holdout_eval_rank_by` can ask for `auto`, `meta`, or `ml`, but the backend still owns final warning/fallback behavior
- saved holdout rows are optional and materially affect later workbook/report richness
- holdout re-ranking is not just cosmetic; it can alter Stage 3 `final_score`, ordering, warnings, and evidence status

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - user-facing holdout controls, mode selectors, reps selector, and related UI copy
- `web/src/stores/analysisStore.ts`
  - persisted/default holdout field values and Stage 3 action wiring
- `web/src/services/metaRequestBuilder.ts`
  - holdout-supported strategy gating, `light` versus `heavy` request shaping, heavy validation payload construction, and representative clamping
- `web/src/services/metaService.ts`
  - Stage 3 invocation, result/status formatting, and holdout-adjusted status messaging
- `src-tauri/src/commands_meta.rs`
  - command-boundary holdout handling, rank-mode fallback/warnings, holdout-adjusted scoring, and saved holdout-row persistence
- `crates/app_core/src/meta_select.rs`
  - `MetaHoldoutMode`, heavy-holdout config types, and Stage 3 aggregate structures
- `src-tauri/src/commands_reports_meta_xlsx.rs`
  - optional holdout-row sheet and workbook-level holdout summary output

## Current Verification Surface

- `web/src/services/metaRequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- Stage 3 tests embedded in `src-tauri/src/commands_meta.rs`
- Stage 3 request/type tests embedded in `crates/app_core/src/meta_select.rs`
- meta workbook tests in `src-tauri/src/commands_reports_meta_xlsx.rs`
- workflow/perf coverage in `crates/app_core/benches/meta_select_workflow.rs`

## Strengths Already Present

- holdout request shaping is already centralized instead of duplicated across components
- `light` and `heavy` modes are encoded explicitly rather than being inferred from partial fields
- heavy-holdout validation config already captures walk-forward, perturbation, bootstrap, and execution-engine detail
- representative count is clamped defensively before crossing the command boundary
- Stage 3 already surfaces weak-evidence and rank-mode warnings instead of flattening holdout issues into a single score
- saved holdout rows are already optional rather than always-on, which keeps default runs lighter

## Weaknesses / Failure Modes

- the actual contract is split across UI controls, builder normalization, backend interpretation, and report output, so it is easy to document only one layer and miss the others
- `holdout_mode` is easy to misread as merely a UI knob when it also changes emitted request shape
- `holdout_eval_rank_by` can appear more authoritative in the UI than it actually is, because unknown or weak-evidence cases still fall back in the backend
- heavy-holdout defaults are rich enough that silent drift in one nested field could meaningfully change evaluation cost or behavior
- saved holdout rows affect both policy/ML downstream usefulness and report richness, but that tradeoff is not yet described in one place
- representative count and explore-N controls interact with shortlist behavior in ways that are easy to under-document

## Regressions To Avoid

- accidentally enabling holdout for unsupported strategies without updating backend handling and tests together
- changing `light` versus `heavy` semantics without preserving request-shape compatibility
- weakening current representative clamps or default count behavior
- relaxing backend warning/fallback behavior for ambiguous holdout rank modes
- persisting holdout rows by default without a deliberate artifact/report/storage review
- changing heavy-holdout nested defaults without updating builder fixtures and Stage 3 expectations
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because holdout and walk-forward logic in this repo must remain chronological and should not drift toward shuffled-validation assumptions
- [scikit-learn Model Selection and Evaluation](https://scikit-learn.org/stable/model_selection.html)
  - relevant because Stage 3 holdout is an evaluation-design problem, not just a UI toggle, and rank/finalist behavior should be documented as part of evaluation design
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the Stage 3 request and saved meta artifacts behave like versioned JSON contracts with nested nullable/defaulted fields
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the shaped holdout payload crosses a structured command boundary and must stay serializable and shape-stable

## Proposed Improvement Approach

1. Document holdout as a staged contract:
   - strategy support
   - request shaping
   - backend evaluation behavior
   - reporting/artifact consequences
2. Make `light` and `heavy` mode differences explicit at the field level, not just in prose labels.
3. Separate “user asks for” semantics from “backend actually guarantees” semantics, especially for `holdout_eval_rank_by`.
4. Record every current default involved in heavy-holdout nested validation so later cost/behavior changes are treated as material edits.
5. Make representative count, explore-N, and top-N evaluation interactions explicit because they shape which groups are actually revisited.
6. Document when saved holdout rows should remain off by default and what downstream/reporting value they unlock when enabled.
7. Tie every sensitive rule to its current owner and test surface so future refactors know where behavior is actually enforced.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document every current holdout field in the Stage 3 request payload and whether it is ignored, nullable, defaulted, or always emitted.
- [ ] Document the current holdout-supported strategy set and unsupported-strategy fallback behavior.
- [ ] Document `light` versus `heavy` mode semantics and the exact nested heavy-holdout validation payload.
- [ ] Document current defaults for heavy-holdout walk-forward, perturbation, bootstrap, and iterative validation settings.
- [ ] Document `holdout_eval_top_n`, `holdout_eval_explore_n`, and `holdout_eval_rank_by` semantics separately.
- [ ] Document representative-count behavior, including current `1..=3` clamping.
- [ ] Document how holdout affects `meta_score`, `final_score`, ranking, warnings, and evidence flags.
- [ ] Document `save_holdout_rows` behavior and downstream workbook/report implications.
- [ ] Add or preserve tests for unsupported strategies, heavy-payload shaping, and rank-mode fallback/warning paths.
- [ ] Keep holdout UI copy, builder fixtures, backend tests, and report behavior aligned when any holdout semantic changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run targeted Stage 3 tests in `src-tauri/src/commands_meta.rs`
- run targeted Stage 3 type/default tests in `crates/app_core/src/meta_select.rs`
- run targeted workbook/report tests in `src-tauri/src/commands_reports_meta_xlsx.rs`
- manually inspect one `light` request payload and one `heavy` request payload
- manually confirm one Stage 3 run with saved holdout rows and one without saved holdout rows
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current top-level field names and nested heavy-holdout field names unless there is a deliberate compatibility plan
- prefer clearer warnings/status text over weakening backend fallback rules
- keep holdout-row persistence additive so older Stage 3 artifacts remain readable even if they do not contain saved per-dataset rows
- if a later UX simplification hides some holdout controls, keep the underlying request contract explicit in documentation and tests
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/23_meta_grouping_clustering_2026-04-09.md`
- `REPO_Improvement/08_ml/24_meta_scoring_weights_2026-04-09.md`
- `REPO_Improvement/08_ml/25_meta_evidence_flags_2026-04-09.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
- `REPO_Improvement/06_artifacts/22_meta_stage3_report_flow_2026-04-09.md`
