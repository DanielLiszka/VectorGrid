# Verification Expectations Plan

Last updated: 2026-04-08

## Purpose

Define what a credible verification section must look like in `REPO_Improvement` docs so future implementation plans remain evidence-backed and executable.

## Scope Boundaries

In scope:

- verification standards for implementation-plan docs
- expectations for frontend, backend, Tauri, perf-gate, and manual verification references
- verification specificity and realism

Out of scope:

- writing or running the actual implementation code
- options-related verification planning
- replacing targeted verification with one giant test command

## Current User-Facing Behavior

Every existing implementation-plan doc already includes a verification section, and the best ones point to real repo test files or commands. The current standard works, but it is not yet isolated into one dedicated verification-expectations doc.

## Current Code Surface

- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_MAP.md`
- repo verification surfaces such as:
  - `cargo test`
  - `npm --prefix web test`
  - parity scripts
  - perf gates
  - desktop/manual verification paths

## Current Verification Surface

- section-complete audits
- manual spot-checks of doc verification sections
- repo tests and scripts referenced by the existing docs

## Strengths Already Present

- current docs already prefer targeted verification over generic claims
- the repo has strong existing verification surfaces across frontend, backend, Tauri, and perf
- performance-sensitive work already has dedicated gate scripts

## Weaknesses / Failure Modes

- verification sections can still be too generic if not policed
- some docs can list commands without explaining why they are the right verification surfaces
- there is not yet one place that states what “good verification planning” means

## Regressions To Avoid

- using vague verification like “run tests” without naming the real surfaces
- pointing to irrelevant or overly broad commands when a tighter regression exists
- omitting manual verification where desktop or UX behavior matters
- ignoring perf gates for performance-sensitive surfaces

## External Research Summary

- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because UI verification should target real user-visible flows
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because many frontend docs should verify store-driven behavior directly
- [Vitest mocking guide](https://vitest.dev/guide/mocking.html)
  - relevant because store/service/component seams often need isolated verification
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because many desktop flows are command-boundary regressions, not just component regressions

## Proposed Improvement Approach

1. Require verification sections to name real repo commands or test files.
2. Prefer narrow, high-signal verification over broad catch-all commands.
3. Require manual verification notes for desktop or visual behavior that automated tests do not fully cover.
4. Call out perf-gate expectations explicitly when a surface is performance-sensitive.

## Checklist

- [ ] Require named repo tests/scripts in every verification section.
- [ ] Prefer targeted verification over generic “run everything.”
- [ ] Include manual verification for desktop/visual behaviors when needed.
- [ ] Include perf gates where a surface is performance-sensitive.
- [ ] Keep verification tied to the actual regressions named in the doc.

## Verification Plan

- spot-check future docs for concrete commands and test files
- reject verification sections that are too generic to execute confidently
- keep this doc aligned with the repo’s real test and script inventory

## Rollout / Fallback / Compatibility Notes

- apply this verification standard to future waves immediately
- older docs can be strengthened opportunistically if their verification sections become clearly stale
- if a surface lacks credible automated verification, the doc should say so explicitly rather than faking confidence

## Related Docs

- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/00_meta/08_priority_rules_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
