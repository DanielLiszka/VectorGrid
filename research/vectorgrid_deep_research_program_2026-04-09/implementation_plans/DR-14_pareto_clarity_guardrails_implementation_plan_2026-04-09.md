# DR-14 Implementation Plan — Pareto Clarity and Guardrails

Last updated: 2026-04-09

Status: proposed bounded additive implementation plan derived from the accepted `DR-14` research response.

## Source Inputs

- research response: `responses/deep-research-report (12).md`
- synthesis judgment: `Adopt`
- synthesis anchor: `FIRST_FIVE_RESPONSE_SYNTHESIS_2026-04-09.md`

## Purpose

Improve the current `CompositeMo` / Pareto workflow so it is harder to misread, less likely to imply false certainty, and more useful as a trade-off review surface without turning it into a broader multi-objective product expansion.

## Repo-Fit Summary

The repo already has a real Pareto workflow:

- `CompositeMo` is a distinct CPU-side multi-objective path
- the Pareto panel already exists
- validation already exposes `robust_pareto` and `cross_ticker_pareto` summaries

What is still weak is the product framing:

- the Pareto panel is sparse
- trade-off meaning is implicit
- confidence and recommendation language can remain too easy to over-interpret

This plan keeps the work inside the existing Pareto/results/validation surfaces rather than expanding the underlying optimization workflow.

## Scope Boundaries

In scope:

- `CompositeMo` result framing in the existing Pareto/results surfaces
- clearer trade-off explanation
- bounded representative-candidate labels
- conservative confidence wording

Out of scope:

- new multi-objective algorithms
- automatic “best candidate” selection logic that pretends preferences are solved
- portfolio-style multi-objective expansion
- widening `CompositeMo` backend scope

## Current Code Surface

- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/services/validationService.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`

## Current Strengths To Preserve

- current Pareto view and chart interaction
- current validation shortlist logic using `robust_pareto` and `cross_ticker_pareto`
- current `CompositeMo` CPU-preferred route messaging
- existing Top/Overview/Pareto separation

## Problems This Plan Solves

- the Pareto surface currently tells the user where points are, but not clearly what that means
- multi-objective outputs can be mistaken for a single “best answer”
- current messaging does not strongly enough separate non-dominance from robustness

## Proposed Outcome

The first-wave outcome should be:

1. a clearer Pareto explanation surface
2. bounded representative labels for the frontier
3. conservative confidence wording carried into the Pareto and related validation summaries

The result should help the user answer:

- what trade-off am I looking at?
- why is this point on the frontier?
- what does this not prove?

## Implementation Slices

### Slice 1 — Clarify the Pareto panel copy

Strengthen the panel so it explicitly states:

- Pareto points are non-dominated under the chosen objectives
- the panel visualizes trade-offs, not proof of robustness
- validation remains necessary before trusting a candidate

This should live close to existing `paretoMeta` and empty-state/help text, not in a separate modal or doc-only location.

### Slice 2 — Add bounded representative labels

Add lightweight labels for representative frontier roles that do not pretend to solve the final choice problem.

Safe first-wave labels:

- maximize-objective extreme
- minimize-objective extreme
- current selected frontier point

Optional later label, only if the heuristic is simple and explicit:

- compromise / trade-off candidate

Guardrail:

- do not label anything as “best” unless the user supplied a preference rule

### Slice 3 — Align Pareto and validation confidence language

Reinforce in the current validation summaries that:

- `robust_pareto` and `cross_ticker_pareto` are filter/admission signals
- frontier membership is not the same thing as validation approval
- conservative wording should be shared across Pareto and validation result text

### Slice 4 — Keep `CompositeMo` route and scope honest

Preserve the current route posture already reflected in the repo:

- `CompositeMo` remains CPU-preferred
- no widening into GPU fixed-candidate expansion from this plan
- no new “magic recommend” automation

## UI / UX Notes

The Pareto panel should become more explicit, not dramatically more complex.

Preferred additions:

- a short truth sentence near the chart
- a compact representative summary band
- a small confidence note tied to validation

Avoid:

- a large new dashboard
- preference wizards
- verbose academic terminology dumps

## Checklist

- keep the work inside the existing `CompositeMo`, Pareto, results, and validation surfaces
- add explicit Pareto truth-in-labeling without changing optimization behavior
- clarify that frontier membership does not prove robustness or validation approval
- add only bounded representative frontier labels for safe first-wave roles
- do not introduce an automatic “best candidate” recommendation without explicit user preferences
- preserve current CPU-preferred `CompositeMo` route posture
- keep Top, Overview, and Pareto responsibilities distinct
- add targeted analysis-service, Pareto panel, and shared validation-wording tests as needed
- verify frontend lint, frontend typecheck, and targeted frontend tests

## Verification Plan

- targeted `analysisService` tests for representative-label and meta-text generation
- targeted Pareto panel component tests
- targeted validation-service tests if shared confidence wording changes
- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- targeted `npm --prefix web run test -- ...`

## Regression Guardrails

- do not change `CompositeMo` optimization behavior in this plan
- do not replace Top or Overview with Pareto-first navigation
- do not add a false “recommended winner” concept without explicit user preferences

## Exit Criteria

- the Pareto panel explicitly frames frontier meaning and non-certainty
- representative labels exist for a bounded set of frontier roles
- confidence wording is more conservative and consistent with validation summaries
- current `CompositeMo` route/scope stays unchanged

## Related Docs

- `research/vectorgrid_deep_research_program_2026-04-09/FIRST_FIVE_RESPONSE_SYNTHESIS_2026-04-09.md`
- `REPO_Improvement/04_results/04_results_pareto_2026-04-09.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/05_validation/` relevant shortlist/validation docs already in the repo-improvement tree

## Deferred Follow-Ups

- richer compromise heuristics only after the first representative-label pass proves useful
- larger multi-objective candidate-analysis surfaces
- any deeper `CompositeMo` productization beyond clarify/constrain UX
