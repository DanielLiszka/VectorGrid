# Dependency Map Plan

Last updated: 2026-04-08

## Purpose

Define the dependency relationships inside the `REPO_Improvement` program so future docs can be created in an order that preserves context and reduces contradictory planning.

## Scope Boundaries

In scope:

- dependencies between meta docs
- dependencies between meta docs and implementation-plan docs
- wave sequencing implications for dependent docs

Out of scope:

- source-code dependency graphs
- runtime dependency injection
- options-related planning dependencies

## Current User-Facing Behavior

The dependency structure already exists implicitly:

- the meta plan sets scope and catalog
- the tracker and index set wave state
- the research gate and quality bar constrain all detailed docs
- implementation-plan docs depend on both repo facts and the meta rules

This works in practice, but the dependencies are scattered across process memory rather than written down.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
- all existing implementation-plan docs that cite each other under `Related Docs`

## Current Verification Surface

- tracker-based wave sequencing
- manual review of related-doc links
- section audits and count audits after waves

## Strengths Already Present

- the program already has a usable dependency backbone
- related-doc sections already encode some cross-doc links
- waves have generally respected prerequisite docs before narrower follow-ons

## Weaknesses / Failure Modes

- dependencies are not yet formalized in one place
- future wave selection could skip prerequisite docs unintentionally
- overlapping docs can be created out of order if dependency rules stay implicit

## Regressions To Avoid

- creating narrow docs before their governing meta docs exist
- changing a foundational doc without identifying downstream docs that depend on it
- treating related-doc links as optional nice-to-have instead of routing hints

## External Research Summary

- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because dependency clarity depends on clear authority boundaries
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because the doc program also benefits from one source of truth per concern
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because dependency rules should distinguish required prerequisites from helpful references

## Proposed Improvement Approach

1. Define the dependency backbone:
   - meta plan
   - tracker
   - index
   - research gate
   - quality bar
2. Mark which docs are prerequisite docs versus follow-on docs.
3. Use dependency information when choosing future queues.
4. Keep `Related Docs` sections aligned with the dependency map.

## Checklist

- [ ] Define the foundational meta-doc dependency chain.
- [ ] Define when an implementation doc depends on a meta doc.
- [ ] Use dependencies to shape future wave order.
- [ ] Keep related-doc references aligned with the dependency map.

## Verification Plan

- review future wave queues against the dependency map before creation
- spot-check new docs for missing prerequisite references
- update the dependency map when foundational docs are added or split

## Rollout / Fallback / Compatibility Notes

- apply this map to future waves without rewriting earlier waves unnecessarily
- if a dependency is discovered late, add the missing prerequisite doc before continuing the dependent queue
- keep the dependency map simple enough to use operationally

## Related Docs

- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/00_meta/04_doc_template_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
