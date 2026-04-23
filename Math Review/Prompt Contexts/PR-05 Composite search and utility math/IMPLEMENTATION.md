# PR-05 Implementation Guide

Source response: `pr05_composite_search_math_all_passes.md`

Decision: `Valid`, but only for exactness-boundary clarification and heuristic-search labeling.

## Valid Scope
- [ ] Define composite exactness relative to a declared repaired domain, declared base ranker, and declared rerank operator.
- [ ] Treat `exact_grid_used=true` as an execution label, not proof that the full original request space was searched exactly.
- [ ] Keep prescan and non-exact samplers heuristic unless backed by a certificate or an explicit ex-post audit contract.
- [ ] Document heatmaps as sampled-bin summaries only, not proofs about unsampled geometry.

## Non-Goals
- [ ] Do not claim `exact_grid_used=true` proves full request-space exactness.
- [ ] Do not claim shortlist confidence rerank is full-domain exact.
- [ ] Do not let heatmap language imply unsampled frontier quality or smoothness.

## Additive Guardrails
- [ ] Current search modes remain available.
- [ ] Existing exact paths keep their current semantics.
- [ ] Any new labels should clarify meaning without removing existing outputs.

## Primary Touchpoints
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`
- `src-tauri/src/run_artifacts.rs`

## Implementation Checklist
- [ ] Define the repaired domain explicitly as the candidate universe actually searched after request/domain rewrite.
- [ ] Define the base ranker explicitly as the pre-rerank score used during search.
- [ ] Define the rerank operator explicitly as any later confidence, exactify, or robustness operator layered on top.
- [ ] Audit artifacts and UI/meta text so `exact_grid_used` is interpreted only relative to the repaired domain.
- [ ] Require either certificates or audited ex-post contracts before presenting a non-exact composite sampler as trustworthy.
- [ ] Limit heatmap semantics to sampled-bin max/mean/count summaries and keep unsampled claims out of labels.

## Verification Checklist
- [ ] Add artifact tests for repaired-domain, base-ranker, and rerank-operator labeling.
- [ ] Add regression tests proving `exact_grid_used` does not overclaim original-domain exactness.
- [ ] Add UI/report wording tests where heatmap or exactness labels are rendered.
- [ ] Add audit-harness coverage for any heuristic sampler that remains exposed.

## Rollout Checklist
- [ ] Update product-facing text in artifacts/reports before changing any defaults.
- [ ] Keep heuristic search opt-in or clearly marked until audited.
- [ ] Prefer additive status/meta lines over removing current result surfaces.

## Kill Criteria
- [ ] Reject if exactness language remains ambiguous between execution mode and request-space coverage.
- [ ] Reject if heatmaps or exact-grid labels still imply properties of unsampled candidates.
