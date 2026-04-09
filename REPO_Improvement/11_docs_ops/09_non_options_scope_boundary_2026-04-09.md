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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because scope rules need clear must/should wording
- [GitHub Docs: Task lists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-task-lists)
  - relevant because the improvement program uses checklist-driven scope execution
- [GitHub Docs: Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/basic-writing-and-formatting-syntax)
  - relevant because scope markers and exclusions must stay readable across many docs
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Keep the program boundary explicit in the meta docs and in every ambiguous leaf doc.
2. Exclude options-only surfaces by default.
3. Only mention shared infrastructure when the non-options side is the subject of the doc.
4. Update counts and wording so “repo coverage” always means “non-options coverage” inside this program.
5. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Keep non-options-only scope explicit in the meta docs.
- [ ] Exclude options-only files and folders from new implementation docs.
- [ ] Mark shared-surface docs carefully when shared infrastructure is unavoidable.
- [ ] Keep tracker and summary wording honest about partial-repo scope.
- [ ] Review future doc proposals against the boundary before creation.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- review proposed doc targets against known options surfaces before creation
- review tracker/meta wording for scope honesty
- spot-check new docs for accidental options bleed-through
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve this boundary until the user explicitly broadens scope
- if the boundary changes later, update the meta plan, tracker, and folder descriptions together
- do not silently migrate options docs into this tree
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
