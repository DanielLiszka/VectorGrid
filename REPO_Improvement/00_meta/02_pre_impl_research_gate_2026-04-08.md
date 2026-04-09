# Pre-Implementation Research Gate

Last updated: 2026-04-08

## Purpose

Define the mandatory research workflow that must happen before any non-options implementation-plan document is created under `REPO_Improvement/`.

## Scope Boundaries

In scope:

- every future detailed plan doc created from the meta catalog
- repo grounding
- external research and source selection
- sub-agent usage for context gathering

Out of scope:

- writing code
- replacing repo-specific analysis with generic external advice
- options-related work

## Current User-Facing Behavior

There is no user-facing behavior. This is an internal planning control.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_MAP.md`
- `AGENTS.md`
- `README.md`
- `RELEASE.md`
- topic-specific routing files and tests

## Current Verification Surface

- manual review of created plan docs
- tracker alignment in `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`

## Strengths Already Present

- the repo already has a strong routing anchor in `REPO_MAP.md`
- the meta plan already requires external research
- the repo has enough existing tests, harnesses, and artifacts to ground plan work in evidence

## Weaknesses / Failure Modes

- plan docs can drift into assumption-backed writing if repo grounding is skipped
- external research can become shallow or vendor-mismatched if sources are not curated
- different topics need different primary sources, and a generic source list is not enough
- without a repeatable gate, document quality will vary by session

## Regressions To Avoid

- creating plan docs from memory alone
- citing secondary summaries when primary documentation is available
- skipping tests, scripts, or artifact formats that already define behavior
- writing implementation steps before constraints and edge cases are enumerated

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) confirms that heavy command work should be async and that command boundaries need explicit argument / return / error contracts.
- [Tauri: State Management](https://v2.tauri.app/develop/state-management/) reinforces that managed state requires explicit mutability and concurrency handling.
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html) and [Vue: Composables](https://vuejs.org/guide/reusability/composables.html) support separating shared state and reusable logic before making UI plans.
- [Pinia: State](https://pinia.vuejs.org/core-concepts/state.html) and [Pinia: Testing Stores](https://pinia.vuejs.org/cookbook/testing.html) support treating store state and test setup as first-class plan inputs.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) and [Vitest Mocking Guide](https://vitest.dev/guide/mocking.html) support user-visible verification, isolated tests, and careful mocking.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is a useful reference for request / response / artifact shape discipline.

## Proposed Improvement Approach

Every detailed implementation-plan doc should follow this research gate:

1. Identify the exact repo surface and the specific planned doc slug.
2. Read `REPO_MAP.md`, then open the narrow routing files and tests for that topic.
3. Enumerate the current code surface, current verification surface, and any artifact formats involved.
4. Use sub-agents when the topic spans multiple layers or when parallel context gathering will reduce blind spots.
5. Browse official documentation and primary references relevant to the topic.
6. Capture the useful external constraints, not just links.
7. Only then draft the detailed plan document with repo facts, research summary, checklist, and verification plan.

## Checklist

- [ ] Identify the exact plan-doc slug and scope boundary.
- [ ] Read `REPO_MAP.md` and topic-specific routing files first.
- [ ] Inspect the current code paths, request/response shapes, and relevant tests.
- [ ] Inspect current scripts, harnesses, and artifact files if the topic touches them.
- [ ] Gather external primary sources and official docs relevant to the topic.
- [ ] Record at least one repo-specific risk and one external constraint before drafting.
- [ ] Draft the implementation plan only after repo and external research are complete.
- [ ] Include a concrete checklist and verification plan in the finished doc.

## Verification Plan

- Require every new plan doc to contain an `External Research Summary` section.
- Require every new plan doc to list current code surface and current verification surface.
- Require every new plan doc to include regressions to avoid and a step-by-step checklist.
- Reject plan docs that rely on general advice without repo grounding.

## Rollout / Fallback / Compatibility Notes

- Apply this gate immediately to Wave 1 docs.
- If primary documentation is weak or missing, the doc must explicitly note that gap instead of faking certainty.
- If a topic proves too broad after research, split it and update the tracker instead of compressing it.

## Related Docs

- `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
