# Advanced Strategy Drilldown And Local Stability Parity

Status: implemented
Risk: moderate
Primary value: make advanced strategy optimization results inspectable and comparable after runs

## Summary

Several advanced composite strategy families can be optimized, but candidate-detail and local-stability drilldowns still return explicit "not implemented yet" errors for a subset of families. This task adds strategy-local detail and stability support in small family-by-family slices.

## Current Anchors

- Candidate detail and local stability errors: `crates/app_core/src/composite_cpu.rs`
- Analysis frontend service: `web/src/services/analysisService.ts`
- Analysis store/UI: `web/src/stores/analysisStore.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
- Strategy capability summaries: `web/src/utils/strategyOptimizationCapabilities.ts`
- Representative tests: `web/src/services/analysisService.test.ts`

## Additive Boundary

- Implement one family or one related family group per PR.
- Preserve existing behavior for unsupported families until that family is explicitly implemented.
- Keep strategy-local code organization; avoid a large generic execution layer.
- Do not alter optimization scoring while adding drilldown/stability.

## Candidate Families

Candidate detail missing:

- [x] VCC
- [x] ROCB
- [x] VCET
- [x] MKCS
- [x] KQSR
- [x] CARM
- [x] KVDF
- [x] CDMC

Local stability missing:

- [x] VCET
- [x] MKCS
- [x] KQSR
- [x] CARM
- [x] KVDF
- [x] CDMC

## Implementation Checklist

### Phase 1: Family Inventory

- [x] Map each missing family to its request struct, optimization result fields, validation adapter, and test helper.
- [x] Identify the minimum candidate identity needed to re-run or inspect a candidate.
- [x] Identify family-specific parameters that must be included in detail output.
- [x] Document any family where local stability is mathematically inappropriate or needs a custom perturbation model.

### Phase 2: Candidate Detail For One Family

- [x] Add a strategy-local detail builder for the selected family.
- [x] Preserve existing error behavior for other families.
- [x] Include candidate id, core params, metrics, route/provenance, and enough debug metadata to compare with optimization output.
- [x] Add backend unit tests for success and malformed candidate input.
- [x] Add frontend normalization tests for the new response shape.
- [x] Add UI assertions that the detail surface no longer shows unsupported messaging for the implemented family.

### Phase 3: Local Stability For One Family

- [x] Define family-local perturbation axes.
- [x] Keep perturbation ranges bounded and deterministic.
- [x] Reuse existing metric/stat helpers where appropriate.
- [x] Add warnings when candidate shape or data length is insufficient for stability analysis.
- [x] Add backend tests for stable and unstable synthetic cases.
- [x] Add frontend tests for local stability summary rendering.

### Phase 4: Parity Rollout

- [x] Repeat phases 2 and 3 family by family.
- [x] Update capability summaries after each family lands.
- [x] Update any "unsupported" tests to expect the narrower remaining unsupported set.
- [x] Add a final matrix test that asserts implemented families no longer return "not implemented yet."

## Validation Checklist

- [x] `cargo test -p ta_app_core --lib drilldown_smoke_runs`
- [x] `cargo test -p ta_app_core --lib local_stability_smoke_runs`
- [x] `npm --prefix web run test -- analysisService`
- [x] Add targeted component tests if UI layout changes.
- [x] Run a small representative optimization and detail request for each implemented family.
- [x] For performance-sensitive detail/stability changes, run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`.

## Regression Watchpoints

- Candidate detail must match the exact optimized candidate, not reconstruct a similar candidate.
- Stability perturbations must not mutate shared request state.
- Advanced strategy families that remain unsupported must still fail clearly.
- Frontend copy must not imply support before backend support exists.

## Definition Of Done

- All previously unsupported advanced composite families have working candidate detail and/or local stability.
- Tests prove unsupported errors shrink only for implemented families.
- Existing supported families retain their output shape and tests.
