# Optimize Store Plan

Last updated: 2026-04-09

## Purpose

Define the current `optimizeStore` contract so optimize-form state, command-state flags, progress text, dynamic-strategy preview state, and service handoff boundaries stay coherent.

## Scope Boundaries

In scope:

- `web/src/stores/optimizeStore.ts`
- optimize-form state ownership
- run/cancel/validate state flags
- snapshot application and service handoff
- GPU-status and dynamic-preview state already owned by the store

Out of scope:

- backend optimization behavior
- options store surfaces
- redesigning the optimize UI itself

## Current User-Facing Behavior

The optimize workspace is driven primarily by `optimizeStore`. It owns strategy selection, backend choice, parameter ranges, advanced controls, ML-related optimize-side state, status/progress text, command-disabled flags, and GPU summary state. The store delegates heavy work to `optimizeService`, but user-visible enablement, many form fields, and workflow snapshots still live in the store.

## Current Code Surface

- `web/src/stores/optimizeStore.ts`
  - owns the optimize workspace state, field setters, snapshot application, GPU refresh action, run/cancel actions, and dynamic strategy preview state
- `web/src/stores/optimizeStore.actions.test.ts`
  - covers action delegation, guard behavior, and GPU-status refresh behavior
- `web/src/services/optimizeService.ts`
  - receives store-driven actions and mutates store state during long-running flows
- `web/src/services/historyService.ts`
  - rehydrates optimize-relevant state from saved runs
- `web/src/utils/strategyOptimizationCapabilities.ts`
  - influences what optimize state is legal or meaningful for the selected strategy
- `web/src/stores/workflowStore.ts`
  - interacts with optimize results-tab and workflow shell state

## Current Verification Surface

- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/stores/workflowStore.test.ts`

## Strengths Already Present

- the store already centralizes a very broad optimize surface instead of distributing it across components
- actions mostly delegate to services rather than embedding long async logic directly in the store
- snapshot application already supports workflow restoration and saved-run replay
- command-disabled flags are explicit and user-visible

## Weaknesses / Failure Modes

- the store is large enough that ownership boundaries are easy to blur
- field state, command state, preview state, and derived UX state all coexist in one module
- some logic is store-owned and some is service-owned, so changes can drift if the boundary is not written down
- replay and snapshot behavior can silently regress because the store accepts many optional snapshot fields

## Regressions To Avoid

- moving service logic into the store without a clear ownership reason
- weakening current run/cancel/validate disabled-state semantics
- silently changing snapshot-application behavior for saved runs
- breaking dynamic preview or GPU summary state while touching unrelated form fields
- allowing `MlSignal` or other excluded paths to leak back into standard optimize strategy options
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia: State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because this file is a large state-ownership boundary
- [Pinia: Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because store actions should stay purposeful and not become a second service layer
- [Pinia: Getters](https://pinia.vuejs.org/core-concepts/getters.html)
  - relevant because user-facing derived state should stay distinct from mutable form state
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because optimize state is shared across services, workflow restoration, and UI surfaces
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document the store by state family:
   - core optimize form state
   - backend and GPU-state surface
   - dynamic-strategy preview state
   - command/progress flags
   - replay/snapshot application
2. Document what the store owns directly versus what `optimizeService` owns.
3. Make current action semantics explicit, especially run/cancel/GPU refresh.
4. Treat snapshot application as a compatibility-sensitive contract, not a convenience helper.
5. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Enumerate the main state families owned by `optimizeStore`.
- [ ] Document current default options and strategy/backend normalization behavior.
- [ ] Document command-state fields such as `runDisabled`, `canCancel`, and `validateDisabled`.
- [ ] Document current progress/status fields and their ownership boundary with `optimizeService`.
- [ ] Document dynamic strategy preview state and GPU summary state.
- [ ] Document snapshot-application behavior and replay-sensitive fields.
- [ ] Document which actions delegate to services and which mutate store state directly.
- [ ] Add or tighten tests if ownership or snapshot behavior changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run `npm --prefix web run test -- src/stores/workflowStore.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve external store shape where practical because multiple services and tests depend on it
- if store refactoring is needed later, keep snapshot compatibility explicit
- do not collapse command-state and form-state semantics together just to reduce file size
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/03_opt/12_opt_service_progress_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
