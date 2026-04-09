# Meta Scope Builders Plan

Last updated: 2026-04-09

## Purpose

Define the current meta scope-builder contract so lightweight scope construction, effective-scope fallback behavior, and later ML-confidence action strictness remain explicit.

## Scope Boundaries

In scope:

- `metaMlScopeBuilder.ts`
- service-level effective-scope resolution
- relationship between builder output and later ML actions
- source-path resolution tied to the current meta result

Out of scope:

- deep ML-confidence training logic
- general batch request shaping
- options workflows

## Current User-Facing Behavior

Users do not interact with a standalone “scope builder” UI, but ML refresh/train/apply behavior depends on one. Current behavior means:

- the lightweight builder can emit a scope with a nullable timeframe
- the effective scope used by ML actions may come from the latest loaded meta result instead of only current input drafts
- later ML actions are stricter than the builder and can block when timeframe is still missing

## Current Code Surface

- `web/src/services/metaMlScopeBuilder.ts`
  - lightweight `{ strategy_kind, timeframe }` builder
- `web/src/services/metaService.ts`
  - effective-scope resolution and later action strictness
- `web/src/services/metaMlScopeBuilder.test.ts`
  - direct builder tests
- `web/src/services/metaService.test.ts`
  - indirect tests through ML actions

## Current Verification Surface

- `web/src/services/metaMlScopeBuilder.test.ts`
- `web/src/services/metaService.test.ts`

## Strengths Already Present

- scope-building is intentionally small and easy to reason about
- the service layer already distinguishes permissive construction from stricter action requirements
- latest-meta-result fallback makes the ML surface more resilient after a real meta run

## Weaknesses / Failure Modes

- it is easy to assume the builder output is the only scope source when the service can override it
- the builder’s permissiveness and the action layer’s strictness can look inconsistent without documentation
- because the builder is small, it can be overlooked even though it controls important ML action correctness

## Regressions To Avoid

- documenting timeframe as required in the builder when it is currently nullable
- removing latest-meta-result fallback without updating ML action behavior
- broadening scope semantics implicitly without documenting the change
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because scoped model identity and compatibility start with a small explicit input contract
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the built scope object is a persisted/transported shape, not an incidental local variable
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the scope object is sent across the command boundary for ML actions
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because effective scope depends on store-backed workflow state, not just one input field
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the builder as a permissive constructor and the service as the effective-scope authority.
2. Record the fallback order for effective scope explicitly.
3. Keep missing-timeframe behavior documented as an action-layer guard, not a builder failure.
4. Preserve the distinction between current scope and current source meta path, since both are derived in adjacent service logic.
5. Tie scope-builder changes to ML action tests because the builder’s effects appear indirectly in product behavior.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the exact fields emitted by `metaMlScopeBuilder`.
- [ ] Document current defaults and nullable-timeframe behavior.
- [ ] Document effective-scope fallback order in `metaService`.
- [ ] Document that later ML actions are stricter than the builder.
- [ ] Preserve tests for trimming/default behavior in `metaMlScopeBuilder.test.ts`.
- [ ] Preserve service tests that exercise effective-scope behavior through ML actions.
- [ ] Treat any scope expansion beyond strategy/timeframe as a compatibility-sensitive change.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaMlScopeBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- manually verify ML refresh/train/apply behavior before and after loading a meta result
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current lightweight scope shape unless there is an explicit migration plan
- keep service fallback behavior explicit if a later UI introduces more direct scope controls
- prefer additive scope fields over reinterpreting current ones
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/29_ml_conf_scope_identity_2026-04-09.md`
- `REPO_Improvement/08_ml/41_meta_request_builders_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
