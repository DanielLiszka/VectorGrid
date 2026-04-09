# Priority Rules Plan

Last updated: 2026-04-08

## Purpose

Define the rules for choosing future `REPO_Improvement` waves so the program stays additive, useful, and evidence-first instead of drifting toward arbitrary queue selection.

## Scope Boundaries

In scope:

- wave prioritization rules
- tie-breakers between candidate docs
- factors that make a doc high leverage

Out of scope:

- source-code task prioritization outside the document program
- feature expansion
- options-related backlog work

## Current User-Facing Behavior

The program already has an implicit prioritization style:

- finish foundational/meta docs before deeper follow-ons when needed
- prefer high-risk or high-leverage surfaces first
- favor areas where better docs unblock many later implementation slices

That logic has worked, but it is not yet written as one priority-rules document.

## Current Code Surface

- `REPO_Improvement/IMPROVEMENT_MASTER_PLAN_2026-04-08.md`
  - provides the broad improvement-order philosophy
- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - defines the full catalog
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
  - defines the active wave focus
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - records the actual wave queue

## Current Verification Surface

- wave-by-wave manual review of queue coherence
- tracker/index synchronization

## Strengths Already Present

- the program already tends to choose useful docs first
- foundational docs have been created before many narrow follow-ons
- wave grouping has generally respected topical coherence

## Weaknesses / Failure Modes

- priority criteria are still process memory rather than written policy
- there is no formal tie-breaker when multiple docs are equally plausible
- high-volume doc creation can drift into local convenience ordering

## Regressions To Avoid

- prioritizing low-leverage docs just because they are easy
- skipping prerequisite/meta docs when later docs depend on them
- letting wave queues become overly mixed and context-fragmented
- choosing docs that imply net-new feature work rather than additive improvement work

## External Research Summary

- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because verification-heavy or high-risk UX surfaces should rise in priority
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because store/service-heavy surfaces often have high leverage for later implementation work
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because the priority rules should distinguish must-have sequencing from optional nice-to-have ordering

## Proposed Improvement Approach

1. Define a ranked priority heuristic:
   - foundational governance dependency
   - cross-cutting unblocker value
   - high-risk regression surface
   - user-facing leverage
   - topical batching efficiency
2. Use a tie-breaker order:
   - prerequisite first
   - cross-cutting first
   - high-risk first
   - simpler/local first only if leverage is equal
3. Keep wave queues coherent by topic cluster.

## Checklist

- [ ] Define the priority heuristic for future waves.
- [ ] Define a tie-breaker order between candidate docs.
- [ ] Prefer cross-cutting unblockers before leaf docs.
- [ ] Keep waves topically coherent and additive-only.

## Verification Plan

- review each future wave queue against the priority heuristic before creation
- verify prerequisite docs are not skipped
- update the program index if the wave focus changes materially

## Rollout / Fallback / Compatibility Notes

- apply these rules starting with the next wave after this doc
- if urgency requires a deviation, note the reason in the tracker or change log
- keep the rules lightweight enough to use during active waves

## Related Docs

- `REPO_Improvement/IMPROVEMENT_MASTER_PLAN_2026-04-08.md`
- `REPO_Improvement/00_meta/07_dependency_map_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
