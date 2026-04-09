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

## External Research Summary

- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because scoped model identity and compatibility start with a small explicit input contract
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the built scope object is a persisted/transported shape, not an incidental local variable
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the scope object is sent across the command boundary for ML actions
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because effective scope depends on store-backed workflow state, not just one input field

## Proposed Improvement Approach

1. Document the builder as a permissive constructor and the service as the effective-scope authority.
2. Record the fallback order for effective scope explicitly.
3. Keep missing-timeframe behavior documented as an action-layer guard, not a builder failure.
4. Preserve the distinction between current scope and current source meta path, since both are derived in adjacent service logic.
5. Tie scope-builder changes to ML action tests because the builder’s effects appear indirectly in product behavior.

## Checklist

- [ ] Document the exact fields emitted by `metaMlScopeBuilder`.
- [ ] Document current defaults and nullable-timeframe behavior.
- [ ] Document effective-scope fallback order in `metaService`.
- [ ] Document that later ML actions are stricter than the builder.
- [ ] Preserve tests for trimming/default behavior in `metaMlScopeBuilder.test.ts`.
- [ ] Preserve service tests that exercise effective-scope behavior through ML actions.
- [ ] Treat any scope expansion beyond strategy/timeframe as a compatibility-sensitive change.

## Verification Plan

- run `npm --prefix web test -- src/services/metaMlScopeBuilder.test.ts`
- run `npm --prefix web test -- src/services/metaService.test.ts`
- manually verify ML refresh/train/apply behavior before and after loading a meta result

## Rollout / Fallback / Compatibility Notes

- preserve current lightweight scope shape unless there is an explicit migration plan
- keep service fallback behavior explicit if a later UI introduces more direct scope controls
- prefer additive scope fields over reinterpreting current ones

## Related Docs

- `REPO_Improvement/08_ml/29_ml_conf_scope_identity_2026-04-09.md`
- `REPO_Improvement/08_ml/41_meta_request_builders_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
