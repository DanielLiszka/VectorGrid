# Research Source Policy

Last updated: 2026-04-09

## Purpose

Define the source hierarchy and citation policy for future implementation-plan documents in `REPO_Improvement/`.

## Scope Boundaries

In scope:

- source ranking
- required use of official / primary sources
- how to use web research alongside repo grounding
- how to document insufficient evidence

Out of scope:

- code implementation
- options-related planning

## Current User-Facing Behavior

There is no direct user-facing behavior. This is a planning and documentation policy.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_MAP.md`
- repo tests, scripts, and artifacts relevant to each topic

## Current Verification Surface

- manual review of future implementation-plan docs

## Strengths Already Present

- the repo already has strong internal routing docs
- the meta plan already requires external research
- the environment supports browsing and source attribution

## Weaknesses / Failure Modes

- mixed-quality sources can produce shallow or incorrect plans
- secondary summaries can hide important constraints
- repo-grounded plans can still be weak if external constraints are not checked

## Regressions To Avoid

- relying on memory when official docs exist
- using low-signal blogs as primary technical authorities
- copying external recommendations that do not fit the repo’s actual architecture
- failing to state when evidence is insufficient
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) and [Tauri: State Management](https://v2.tauri.app/develop/state-management/) are examples of preferred official product documentation for Tauri-specific topics.
- [Vue](https://vuejs.org/guide/scaling-up/state-management.html), [Pinia](https://pinia.vuejs.org/core-concepts/state.html), [Vitest](https://vitest.dev/guide/mocking.html), and [Playwright](https://playwright.dev/docs/best-practices) are examples of preferred official frontend/testing sources.
- [JSON Schema](https://json-schema.org/learn/getting-started-step-by-step), [rust_xlsxwriter](https://rustxlsxwriter.github.io/index.html), [scikit-learn TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html), and [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca) are examples of primary/official technical sources for schema, report, ML-evaluation, and execution topics.
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

Use this source hierarchy for every future plan doc:

1. current repo code, tests, scripts, and artifacts
2. official product/framework/library documentation
3. primary standards/specifications
4. authoritative vendor or tool documentation
5. only then high-signal secondary sources when primary material is insufficient

When evidence is weak:

- say so explicitly
- narrow the plan
- or mark the topic as needing more research before implementation
6. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Read the relevant repo files and tests before browsing.
- [ ] Prefer official docs over third-party summaries.
- [ ] Prefer standards/specs for file formats and schemas when available.
- [ ] Capture why each external source is relevant to the topic.
- [ ] Reject sources that are stale, low-signal, or off-topic.
- [ ] State when evidence is incomplete or conflicting.
- [ ] Include source links in every finished plan doc.
- [ ] Re-run research when the topic is time-sensitive or vendor behavior may have changed.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- require every new plan doc to include an external-research section
- require topic-appropriate primary or official sources where they exist
- reject uncited plan docs for technical topics
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- this policy applies immediately to all new Wave 1+ implementation-plan docs
- older seed docs can remain as-is unless they are rewritten
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
