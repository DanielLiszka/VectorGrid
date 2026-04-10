# Meta Selection Request Contract Plan

Last updated: 2026-04-09

## Purpose

Define the Stage 3 meta-selection request contract so frontend defaults, holdout controls, grouping behavior, and backend acceptance rules stay explicit and testable.

## Scope Boundaries

In scope:

- `buildMetaSelectionRequest` input normalization
- holdout enablement, heavy-holdout sub-config, and supported-strategy gating
- grouping and clustering controls
- shortlist and fallback defaults
- command-boundary payload compatibility

Out of scope:

- Stage 4 and Stage 5 ranking logic
- campaign batching beyond the request fields it forwards
- options workflows

## Current User-Facing Behavior

Users configure Stage 3 meta selection from the Meta workspace with strategy kind, timeframe, tags, holdout mode, grouping, and optional heavy-holdout validation settings. The frontend already normalizes booleans, counts, buckets, and holdout sub-config before invoking Rust. Current behavior includes:

- unsupported strategies automatically lose holdout even if the UI draft asked for it
- heavy holdout emits a nested validation payload while light holdout does not
- `cluster_representatives` is clamped into the shipped `1..=3` range
- `strict_shortlist` is currently always emitted as `true` from the builder
- holdout ranking supports `auto`, `meta`, and `ml`, with backend fallback messaging when unknown values appear

## Current Code Surface

- `web/src/services/metaRequestBuilder.ts`
  - owns Stage 3 request shaping, holdout support checks, heavy-holdout validation payload construction, cluster bucket normalization, and current shortlist defaults
- `web/src/services/metaRequestBuilder.test.ts`
  - covers baseline build output, heavy-holdout payload construction, iid-only bootstrap mode, unsupported-strategy holdout disablement, and known holdout-supported strategy kinds
- `web/src/services/metaService.ts`
  - invokes `run_meta_selection`, persists the returned artifact/report path state, and threads Stage 3 results into later meta workflow steps
- `src-tauri/src/commands_meta.rs`
  - owns command-boundary request acceptance, holdout support checks, metric-surface warnings, selective holdout evaluation, and the Stage 3 artifact/report response
- `crates/app_core/src/meta_select.rs`
  - defines `MetaSelectRequest`, default weights, holdout-mode and validation config types, clustering config, shortlist behavior, and holdout-adjusted ordering logic

## Current Verification Surface

- `web/src/services/metaRequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- Stage 3 tests in `src-tauri/src/commands_meta.rs`
- app-core meta-selection tests in `crates/app_core/src/meta_select.rs`

## Strengths Already Present

- request shaping is already centralized instead of scattered across components
- current tests already pin a meaningful subset of defaults and holdout gating behavior
- backend request types already carry explicit defaults for weights, clustering, and shortlist semantics
- Stage 3 already emits warnings for metric-surface drift and thin holdout evidence instead of silently flattening those differences

## Weaknesses / Failure Modes

- builder defaults and backend defaults can drift because both layers normalize parts of the same request
- heavy-holdout validation payloads are rich enough that silent field drift would be hard to notice from the UI alone
- supported-holdout strategy lists are encoded behavior, not yet owned by a single written contract
- cluster bucket semantics, representative count, and strict-shortlist posture are product decisions that could be changed accidentally during later refactors
- saved request payloads and saved Stage 3 artifacts depend on the current field names even when the UI labels evolve

## Regressions To Avoid

- changing holdout defaults or holdout-supported strategy behavior without updating tests and docs together
- allowing unsupported combinations to reach Rust when the builder currently normalizes them away
- weakening current `strict_shortlist` behavior unintentionally
- changing cluster bucket or representative semantics without verifying result stability
- renaming request fields in a way that breaks saved workflows or request-builder fixtures
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) reinforces that ordered evaluation windows must stay chronological; that maps directly to this doc’s holdout and validation-subconfig sections.
- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because this request behaves like a versioned contract with required fields, nullable fields, and defaulted fields crossing a boundary.
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because the builder output becomes the invoke payload and must stay serializable and shape-stable.
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259) is relevant because the request and saved meta artifacts are JSON payloads whose null-versus-omitted choices matter.

## Proposed Improvement Approach

1. Document the request in four explicit sections: identity/filtering, holdout controls, grouping/clustering, and shortlist/fallback controls.
2. Distinguish frontend product defaults from backend-required normalization for every non-trivial field.
3. Enumerate which strategies currently support holdout and which validation path each family routes through.
4. Make heavy-holdout nested validation config explicit, including bootstrap mode, engine, and count normalization.
5. Tie every sensitive field or normalization rule to the file and test surface that currently owns it.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every top-level Stage 3 request field and whether it is required, optional, nullable, or defaulted.
- [x] Document holdout-supported strategy kinds and current unsupported-strategy fallback behavior.
- [x] Document light versus heavy holdout behavior and the nested heavy-holdout validation payload.
- [x] Document `holdout_eval_top_n`, `holdout_eval_explore_n`, and `holdout_eval_rank_by` semantics.
- [x] Document `cluster_config` and `cluster_representatives`, including current clamps.
- [x] Document `strict_shortlist` and any other builder-emitted hard defaults.
- [x] Document which defaults originate in the frontend builder versus `MetaSelectRequest` defaults in app-core.
- [x] Document current warning/fallback behavior for unknown or mismatched request combinations.
- [x] Add or tighten request-builder fixtures if any normalization rule changes.
- [x] Verify backend acceptance still matches the documented payload shape after contract edits.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run targeted Stage 3 tests in `src-tauri/src/commands_meta.rs`
- run targeted app-core tests in `crates/app_core/src/meta_select.rs`
- manually inspect one emitted light-holdout request and one heavy-holdout request
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current field names and current nullable-field behavior unless there is an explicit compatibility plan
- if defaults change, update builder tests, backend tests, and UI copy in the same change
- prefer documenting current shipped semantics before attempting a Stage 3 request simplification
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by surfacing the Stage 3 request contract directly in the Meta panel instead of leaving it implicit in `metaRequestBuilder.ts`.
- `MetaWorkspacePanel.vue` now shows Stage 3 summary cards for:
  - request scope
  - holdout mode/tag/save-rows posture
  - grouping/clustering posture
  - shortlist defaults
- The panel copy now states the Stage 3 scoring/evidence ownership separately from later Stage 5 ML weighting.
- `metaRequestBuilder.test.ts` now adds a contract test for representative clamping plus the hard-enabled shortlist defaults, while leaving backend acceptance unchanged.
- `REPO_MAP.md` was synced so contributors can find the Stage 3 request-contract summary quickly.
- Stable fixture / replay payload:
  - `strategy_kind = CompositeMo`
  - `timeframe = 1m`
  - `required_tag = holdout`
  - `eval_holdout = true`
  - `holdout_mode = heavy`
  - `strict_shortlist = true`
  - `allow_shortlist_fallback = false`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaRequestBuilder.test.ts src/services/metaService.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/26_meta_holdout_modes_2026-04-09.md`
- `REPO_Improvement/08_ml/23_meta_grouping_clustering_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
