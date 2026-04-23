# Validation Shortlist Admission Plan

Date: 2026-04-02

## Purpose

This plan defines a low-debt path to improve which optimization candidates are admitted into validation before the existing validation robustness machinery runs.

The goal is not to replace validation scoring. The goal is to stop feeding validation a shortlist that is usually just a single pre-ordered optimization top-k slice.

This plan is intentionally separate from the campaign runner plan. The campaign runner is orchestration work. Shortlist admission is candidate-selection policy.

## Progress Tracking

- Phase 0: completed
- Phase 1: completed
- Phase 2: completed
- Phase 3: completed
- Phase 4: completed

Current implementation status:

- The validation workspace now exposes an explicit admission-policy selector with `legacy_topk`, `scalar_union`, `pareto_frontier`, and `auto`.
- `legacy_topk` remains the default compatibility path.
- Scalar-union shortlist admission is implemented for standard validation request shaping.
- `CompositeMo` validation admission now reads saved `pareto` rows directly and `auto` resolves to `pareto_frontier` for that artifact shape.
- Campaign validation now carries `validation_config.admission_policy` and applies the same policy family on the backend before building validation requests.
- Validation request artifacts now persist `admission_policy` plus `admission_provenance`.
- Artifact config fingerprints now ignore derived `admission_provenance` so skip/reuse logic does not drift on derived shortlist metadata.
- Applying a saved run artifact now restores the saved validation admission policy back into the validation form.
- The validation workspace now previews the current shortlist policy against `legacy_topk` on the same source artifact before execution.
- Saved-run history detail now surfaces admission summary and admission provenance detail for validation artifacts.
- Product defaults remain `legacy_topk` for compatibility; recommended opt-in defaults are `scalar_union` for scalar routes and `auto` for `CompositeMo`.
- Dense `CompositeMo` Pareto frontiers now use deterministic diversity trimming when the frontier exceeds the validation candidate cap.

## Executive Summary

Today validation admission is still mostly:

- take optimization `top` rows in their existing order
- fall back to `best_params` if needed
- dedupe
- truncate to the validation `Candidates` cap

That is simple and predictable, but it biases validation toward the optimizer's existing scalar ordering.

The recommended improvement is:

- keep the current validation post-run robustness logic unchanged
- improve the pre-validation shortlist builder
- use a bounded multi-head union for scalar routes
- use a bounded Pareto-derived shortlist for `CompositeMo`
- preserve a legacy fallback mode for compatibility and debugging

## Why This Needs Its Own Plan

The shortlist problem sits between optimization and validation:

- optimization decides which candidates are retained and how they are ordered
- validation decides which retained candidates get rerun
- validation post-processing already performs more robust multi-objective reasoning after reruns finish

That means the right seam to change is admission into validation, not campaign orchestration and not the existing validation robustness pipeline.

## Current State

### Current admission behavior

- Validation extracts optimization `top` rows from the saved run result.
- If there are no `top` rows, it falls back to a single `best_params` candidate.
- The validation request builder preserves that incoming order, dedupes candidates, and truncates to `max_candidates`.

### Current strengths

- The current flow is deterministic and easy to reason about.
- It works across the existing optimization artifact shapes.
- Validation itself already computes `robust_pareto` and robust-score ordering after reruns.

### Current limitations

- A scalar optimization ranking can filter out diverse but robust candidates before validation ever sees them.
- `CompositeMo` currently has a Pareto frontier available for analysis, but validation admission still follows `top` / `best_params` instead of the raw frontier.
- Some advanced shortlist features already exist on the exact Double-MA kernel path, but they are route-gated and not the general validation-admission solution.

## Goals

- Improve candidate admission into validation without replacing the current validation engine.
- Preserve deterministic and explainable shortlist construction.
- Support scalar routes and `CompositeMo` cleanly without forcing one universal scalar score.
- Keep compatibility with current saved artifacts and current default workflows.
- Allow campaign runner templates to opt into improved admission later without owning the logic.

## Non-Goals

- Replacing post-validation `robust_pareto` or robust-score logic.
- Inventing one universal scalar metric that all strategies must optimize against.
- Requiring universal `Trade22` support before admission can improve.
- Turning validation admission into a backend-wide parallel search subsystem.

## Recommended Architecture

### Introduce an explicit admission policy

Add a validation admission policy concept, for example:

- `legacy_topk`
- `scalar_union`
- `pareto_frontier`
- `auto`

`auto` should choose the best supported policy for the originating optimization artifact:

- scalar routes -> bounded union shortlist
- `CompositeMo` -> bounded Pareto-derived shortlist
- fallback -> legacy ordering

### Preserve current validation post-processing

Do not change:

- hard gates
- `robust_pareto`
- folded CSCV scoring
- existing validation verdict logic

Those mechanisms already operate on rerun validation results. The missing step is who gets rerun.

### Capture provenance

The validation artifact should record:

- admission policy used
- source artifact id
- candidate heads used
- pre-cap candidate count
- post-dedupe count

That makes campaign diagnostics and compare views much easier.

## Likely Primary Touchpoints

### Current admission path

- `web/src/services/validationService.ts`
  - current extraction of optimization `top` rows and `best_params` fallback
- `web/src/services/validationRequestBuilder.ts`
  - current dedupe, truncation, and validation request assembly

### Optimization artifact readers

- `web/src/services/analysisService.ts`
  - current `pareto` collection and row ordering for `CompositeMo`
- saved artifact readers and compare flows that surface optimization top rows

### Validation backend and artifact layer

- `src-tauri/src/commands_validation.rs`
  - validation command entrypoints and potential provenance plumbing
- `crates/app_core/src/validation_pipeline.rs`
  - existing post-validation robust Pareto and robust-score logic that should remain unchanged
- `src-tauri/src/run_artifacts.rs`
  - validation artifact persistence if provenance is added there

### Existing exact-kernel shortlist features to reuse carefully

- `crates/app_core/src/double_ma.rs`
- `web/src/services/optimizeRequestBuilder.ts`

Those exact-kernel shortlist controls are useful precedent, but they should not become the only admission mechanism for the general validation flow.

## Candidate Admission Policy

### Scalar routes

For ordinary scalar optimization routes, prefer a bounded multi-head union:

- head 1: primary optimization ranking
- head 2: low drawdown bias
- head 3: strong risk-adjusted return bias such as Calmar or Omega
- optional head 4: strategy-specific head where the route already exposes a stable score mode

Then:

- union the heads
- dedupe by canonical params key
- preserve per-head provenance
- cap to validation `max_candidates`

This is better than inventing one new blended scalar because it preserves diversity without hiding arbitrary weights.

### `CompositeMo`

For `CompositeMo`, use the saved Pareto frontier directly when present:

- keep feasible frontier points first
- apply a simple bounded tie-break for display order
- optionally apply a diversity trim if the frontier is too dense
- fall back to `top` / `best_params` only when the frontier is unavailable

### Compatibility fallback

Always keep a `legacy_topk` mode so debugging, regression testing, and apples-to-apples comparisons remain easy.

## Phase Plan

### Phase 0: policy definition

- [x] Freeze the initial admission-policy names and semantics.
- [x] Freeze the default `auto` behavior.
- [x] Define the first scalar union heads and tie-break rules.
- [x] Define validation artifact provenance fields for admission.

Exit criteria:

- one written policy table exists
- `auto` behavior is deterministic and documented

### Phase 1: request and artifact plumbing

- [x] Add admission-policy support to validation request shaping.
- [x] Keep `legacy_topk` as the default compatibility path unless explicitly enabled elsewhere.
- [x] Add artifact provenance fields for admission policy and shortlist construction.
- [x] Ensure campaign templates can carry the policy without embedding the logic themselves.

Exit criteria:

- validation requests can declare admission policy
- validation artifacts can record how the shortlist was built

### Proposed First Implementation Slice

The safest first slice is a no-UI backend-compatible legacy-preserving path:

- [x] Add admission-policy plumbing with `legacy_topk` default.
- [x] Add scalar union support behind an explicit opt-in policy.
- [x] Record shortlist provenance in validation artifacts.
- [x] Add a compare/debug surface that can show legacy vs scalar-union selection on the same source artifact.

Exit criteria:

- improved shortlist logic exists without changing default user behavior
- the resulting shortlist is explainable from saved artifact provenance

### Phase 2: scalar union shortlist

- [x] Build reusable shortlist-head extraction helpers from optimization rows and metrics.
- [x] Implement bounded union, dedupe, and cap logic.
- [x] Handle missing metrics gracefully so weaker routes fall back rather than erroring.
- [x] Add tests for determinism, dedupe, and cap behavior.

Exit criteria:

- scalar routes can build a stable diversified shortlist from existing optimization artifacts

### Phase 3: `CompositeMo` Pareto admission

- [x] Teach validation admission to consume `pareto` rows when present.
- [x] Define a bounded ordering rule for feasible and infeasible frontier points.
- [x] Add optional diversity trimming for dense frontiers.
- [x] Keep best-params fallback for legacy or incomplete artifacts.

Exit criteria:

- `CompositeMo` no longer depends on scalar `top` ordering for validation admission when a frontier is available

### Phase 4: UI and campaign adoption

- [x] Surface admission policy in the validation UI and persist provenance into saved validation requests.
- [x] Add compare tooling to contrast legacy and improved admission on the same artifact.
- [x] Decide when campaign-runner defaults should switch from legacy admission to `auto`.
- [x] Document recommended per-strategy defaults if they differ.

Exit criteria:

- users can see how the shortlist was built
- campaign workflows can adopt improved admission without special-case code

## Testing Checklist

- [x] Add unit tests for scalar union head extraction and dedupe.
- [x] Add unit tests for cap behavior when the union exceeds `max_candidates`.
- [x] Add `CompositeMo` tests for Pareto admission ordering and fallback.
- [x] Add artifact round-trip tests for admission provenance fields.
- [x] Add regression tests ensuring `legacy_topk` behavior remains unchanged.
- [x] Add parity tests proving `legacy_topk` and opt-in improved admission can coexist on the same source artifact.

## Dependencies and Interactions

- This work can begin before universal `Trade22` support exists.
- A future universal `Core17` surface will improve head quality on routes that currently have weaker metric coverage.
- The campaign runner should consume the admission policy as configuration, not reimplement shortlist logic.

## Risks

- A too-clever shortlist policy can become harder to explain than the current top-k behavior.
- If head definitions are unstable, reproducibility and compare workflows suffer.
- `CompositeMo` frontier trimming can accidentally destroy useful diversity if the cap policy is too aggressive.
- Product defaults changed too early can make old and new validation artifacts harder to compare.

## Open Questions

- Which additional scalar heads become worthwhile once broader `Core17` metric availability lands?
- Do we want a campaign-level option to force `legacy_topk` for reproducibility studies even after improved policies are more widely adopted?

## Current Default Decision

- Keep `legacy_topk` as the shipped default for validation and campaign workflows in the current release cycle.
- Recommend `scalar_union` as the opt-in research default for scalar routes when users want broader candidate coverage before validation.
- Recommend `auto` as the opt-in research default for `CompositeMo`, where it resolves to `pareto_frontier` when frontier data is present.
- Revisit default-policy changes only after metric-contract unification improves cross-strategy metric coverage and artifact comparability.

## Recommendation

Proceed in this order:

1. define the admission-policy contract and provenance fields
2. implement scalar union shortlist support
3. add `CompositeMo` Pareto-derived admission
4. keep the current validation robustness pipeline unchanged

This gives validation a better candidate set without inventing a fake universal scalar score and without expanding campaign-runner scope.
