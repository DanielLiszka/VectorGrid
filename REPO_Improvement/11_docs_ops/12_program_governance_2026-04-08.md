# Program Governance Plan

Last updated: 2026-04-09

## Purpose

Define the governance rules for the `REPO_Improvement` program so counts, wave state, quality rules, and document status stay synchronized as the implementation-plan set grows.

## Scope Boundaries

In scope:

- governance for the non-options `REPO_Improvement` document program
- update rules for `README`, `program_index`, `doc_creation_tracker`, and the meta plan
- status transitions, wave closeout, merge/split/supersede rules
- quality-bar and research-gate enforcement

Out of scope:

- code implementation governance outside the document program
- options-related planning programs
- Git workflow policies unrelated to this document tree

## Current User-Facing Behavior

The governance model already exists, but it is distributed across multiple docs:

- `REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `README.md`
- `00_meta/program_index_2026-04-08.md`
- `00_meta/doc_creation_tracker_2026-04-08.md`
- `11_docs_ops/10_research_source_policy_2026-04-08.md`
- `11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`

As of this document batch, the program already tracks:

- non-options-only scope
- additive-only / no-regression posture
- planned counts
- created counts
- active wave
- quality-bar expectations
- research-before-plan requirements

So governance is already real. The gap is that it has not yet been consolidated into one explicit operating contract.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - master charter, scope, catalog, and program rules
- `REPO_Improvement/README.md`
  - local entrypoint for the document tree
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
  - current-wave routing summary
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - counts, created-doc registry, and active queue
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
  - source-quality expectations
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
  - minimum implementation-doc standard

## Current Verification Surface

- file-count audits against the `REPO_Improvement/` tree
- section-complete audits against the 14-section template
- tracker/index synchronization checks
- spot checks against repo-grounded code surfaces and cited research

## Strengths Already Present

- the program already has a strong charter and scope boundary
- additive-only and research-before-plan rules are already explicit
- the tracker already distinguishes planned and created state
- wave-based progression already exists
- quality-bar enforcement already exists in practice

## Weaknesses / Failure Modes

- governance is spread across several docs instead of one authoritative operating contract
- tracker counts, created-doc lists, and wave labels can drift if updates are partial
- “complete” can be confused with “file exists” unless status rules are explicit
- merge/split/supersede behavior is not yet formalized
- README/index/tracker/meta updates are easy to do inconsistently during high-volume creation waves

## Regressions To Avoid

- tracker/count drift
- contradictory meta docs
- marking docs complete just because a file exists
- weakening the non-options or additive-only boundary
- skipping research-gate or quality-bar enforcement during later waves
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [GitHub task lists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-tasklists)
  - relevant because the program already depends on Markdown checklists as a working-state mechanism
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because governance language benefits from clear must/should/may distinctions instead of vague process wording
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because verification expectations in these docs should stay concrete and executable, not aspirational
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because governance for implementation docs should continue routing to real verification surfaces instead of generic prose
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Define one canonical governance contract for the document program.
2. Freeze ownership of the core meta docs:
   - meta plan owns scope/catalog
   - tracker owns counts/created docs/queue
   - program index owns current-wave routing
   - README owns local entry guidance
3. Define explicit status transitions:
   - planned
   - drafted
   - active
   - partially executed
   - completed
   - archived
4. Define merge/split/supersede rules so planned counts stay honest.
5. Add a wave-close checklist that requires:
   - file count sync
   - folder count sync
   - section audit
   - tracker/index update
   - next queue declaration
6. Preserve the current research gate, quality bar, sub-agent usage rule, and additive-only boundary.
7. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define ownership boundaries for README, meta plan, program index, and tracker.
- [ ] Define what “created”, “section-complete”, and “completed” each mean.
- [ ] Define merge, split, supersede, and archive update rules.
- [ ] Define the required wave-close sync checklist.
- [ ] Preserve non-options-only and additive-only rules explicitly.
- [ ] Preserve mandatory research-before-plan and quality-bar enforcement.
- [ ] Require tracker and index updates whenever a wave advances.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- verify tracker counts against actual file counts after each wave
- rerun the section-complete audit across implementation-plan docs after each wave
- verify the program index and tracker name the same active wave
- spot-check new docs for repo grounding and external research before marking the wave complete
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the governance doc additive; it should clarify process, not erase history
- if the program is re-baselined later, preserve archived wave history rather than rewriting it invisibly
- if planned docs are merged or split, update the meta plan and tracker together
- if a doc falls below the quality bar later, mark it for revision instead of pretending it is still complete
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
