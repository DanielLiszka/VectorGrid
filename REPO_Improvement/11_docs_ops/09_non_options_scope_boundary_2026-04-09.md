# Non Options Scope Boundary Plan

Last updated: 2026-04-09

## Purpose

Define the current non-options scope boundary for `REPO_Improvement` so the program stays focused on the rest of the repo while excluding options-specific frontend and backend surfaces.

## Scope Boundaries

In scope:

- all current repo surfaces outside options-specific functionality
- mixed/shared infrastructure only where it materially serves non-options behavior
- boundary language used by the improvement-program docs

Out of scope:

- options-expression frontend/backend workflow docs
- options-only benchmarks or prompt packs
- using non-options docs to backdoor options planning

## Current User-Facing Behavior

The current `REPO_Improvement` program is explicitly non-options-only. That scope is already reflected in the meta plan, tracker, and user instructions, but it still needs one dedicated operating doc because the broader repo contains both options and non-options work.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
- `options_research/`
- options-specific files in `web/src/components/OptionsExpressionPanel.vue`, related stores/services, and matching backend commands

## Current Verification Surface

- manual scope review during doc creation
- tracker/meta-plan review
- path-level review against `options_research/` and known options surfaces

## Strengths Already Present

- the user-specified scope boundary is already explicit
- the meta plan already calls out non-options-only scope
- options-related folders and files are already recognizable enough to exclude intentionally

## Weaknesses / Failure Modes

- the repo mixes options and non-options work in the same tree
- some shared infrastructure touches both domains, which can blur the boundary if not stated carefully
- large document programs tend to drift into nearby surfaces unless the exclusion rules stay explicit

## Regressions To Avoid

- adding options-specific implementation docs under `REPO_Improvement`
- describing shared infrastructure in a way that silently reintroduces options scope
- letting counts and tracker claims imply full-repo coverage when the scope is intentionally narrower
- treating “non-options” as “everything except an obvious folder” without checking specific files

## External Research Summary

- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because scope rules need clear must/should wording
- [GitHub Docs: Task lists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-task-lists)
  - relevant because the improvement program uses checklist-driven scope execution
- [GitHub Docs: Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/basic-writing-and-formatting-syntax)
  - relevant because scope markers and exclusions must stay readable across many docs

## Proposed Improvement Approach

1. Keep the program boundary explicit in the meta docs and in every ambiguous leaf doc.
2. Exclude options-only surfaces by default.
3. Only mention shared infrastructure when the non-options side is the subject of the doc.
4. Update counts and wording so “repo coverage” always means “non-options coverage” inside this program.

## Checklist

- [ ] Keep non-options-only scope explicit in the meta docs.
- [ ] Exclude options-only files and folders from new implementation docs.
- [ ] Mark shared-surface docs carefully when shared infrastructure is unavoidable.
- [ ] Keep tracker and summary wording honest about partial-repo scope.
- [ ] Review future doc proposals against the boundary before creation.

## Verification Plan

- review proposed doc targets against known options surfaces before creation
- review tracker/meta wording for scope honesty
- spot-check new docs for accidental options bleed-through

## Rollout / Fallback / Compatibility Notes

- preserve this boundary until the user explicitly broadens scope
- if the boundary changes later, update the meta plan, tracker, and folder descriptions together
- do not silently migrate options docs into this tree

## Related Docs

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
