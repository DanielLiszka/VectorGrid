# Advanced Strategy Drilldown And Local Stability Parity

Status: proposed
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

- [ ] VCC
- [ ] ROCB
- [ ] VCET
- [ ] MKCS
- [ ] KQSR
- [ ] CARM
- [ ] KVDF
- [ ] CDMC

Local stability missing:

- [ ] VCET
- [ ] MKCS
- [ ] KQSR
- [ ] CARM
- [ ] KVDF
- [ ] CDMC

## Implementation Checklist

### Phase 1: Family Inventory

- [ ] Map each missing family to its request struct, optimization result fields, validation adapter, and test helper.
- [ ] Identify the minimum candidate identity needed to re-run or inspect a candidate.
- [ ] Identify family-specific parameters that must be included in detail output.
- [ ] Document any family where local stability is mathematically inappropriate or needs a custom perturbation model.

### Phase 2: Candidate Detail For One Family

- [ ] Add a strategy-local detail builder for the selected family.
- [ ] Preserve existing error behavior for other families.
- [ ] Include candidate id, core params, metrics, route/provenance, and enough debug metadata to compare with optimization output.
- [ ] Add backend unit tests for success and malformed candidate input.
- [ ] Add frontend normalization tests for the new response shape.
- [ ] Add UI assertions that the detail surface no longer shows unsupported messaging for the implemented family.

### Phase 3: Local Stability For One Family

- [ ] Define family-local perturbation axes.
- [ ] Keep perturbation ranges bounded and deterministic.
- [ ] Reuse existing metric/stat helpers where appropriate.
- [ ] Add warnings when candidate shape or data length is insufficient for stability analysis.
- [ ] Add backend tests for stable and unstable synthetic cases.
- [ ] Add frontend tests for local stability summary rendering.

### Phase 4: Parity Rollout

- [ ] Repeat phases 2 and 3 family by family.
- [ ] Update capability summaries after each family lands.
- [ ] Update any "unsupported" tests to expect the narrower remaining unsupported set.
- [ ] Add a final matrix test that asserts implemented families no longer return "not implemented yet."

## Validation Checklist

- [ ] `cargo test -p ta_app_core composite_cpu`
- [ ] `npm --prefix web run test -- src/services/analysisService.test.ts`
- [ ] Add targeted component tests if UI layout changes.
- [ ] Run a small representative optimization and detail request for each implemented family.
- [ ] For performance-sensitive detail/stability changes, run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`.

## Regression Watchpoints

- Candidate detail must match the exact optimized candidate, not reconstruct a similar candidate.
- Stability perturbations must not mutate shared request state.
- Advanced strategy families that remain unsupported must still fail clearly.
- Frontend copy must not imply support before backend support exists.

## Definition Of Done

- At least one previously unsupported family has working candidate detail and/or local stability.
- Tests prove unsupported errors shrink only for implemented families.
- Existing supported families retain their output shape and tests.

