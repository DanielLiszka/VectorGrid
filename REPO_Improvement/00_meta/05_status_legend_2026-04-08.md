# Status Legend Plan

Last updated: 2026-04-09

## Purpose

Define the status vocabulary used by the `REPO_Improvement` program so tracker entries, wave summaries, and future execution states mean the same thing everywhere.

## Scope Boundaries

In scope:

- planned-status meanings for the document program
- distinctions between created, section-complete, active, completed, archived, and superseded states
- status usage in tracker/index/meta docs

Out of scope:

- application runtime states
- Git workflow status terms
- options-related planning state

## Current User-Facing Behavior

The meta plan already lists the intended status vocabulary:

- `planned`
- `drafted`
- `active`
- `partially_executed`
- `completed`
- `archived`

In practice, the tracker also uses operational terms such as `created` and `section-complete`. Those meanings are currently understandable from context, but they are not yet frozen into one explicit legend.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - defines the status model
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - uses created counts, active wave, and section-complete audit language
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
  - clarifies that file existence alone is not completion
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
  - will govern future status transitions

## Current Verification Surface

- tracker/index spot checks
- manual review of wave summaries

## Strengths Already Present

- the high-level status vocabulary already exists
- the tracker already distinguishes file creation from quality completion in practice
- the program already avoids pretending that every file is “done”

## Weaknesses / Failure Modes

- `created`, `section-complete`, and `completed` are easy to conflate
- there is no one document that defines when a doc moves from one status to another
- wave summaries can imply stronger completion than is actually intended

## Regressions To Avoid

- marking a doc complete just because the file exists
- using `active` to mean both “current queue” and “best available doc”
- archiving or superseding docs without clear status updates
- drifting tracker language across waves
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because status definitions need precise must/should language
- [GitHub tasklists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-tasklists)
  - relevant because checklist completion should not be confused with whole-document lifecycle completion
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because verification-backed completion should remain distinct from mere draft existence

## Proposed Improvement Approach

1. Define each status term precisely.
2. Add auxiliary definitions for:
   - created
   - section-complete
   - superseded
3. Use the same vocabulary in the tracker, program index, and future execution notes.
4. Preserve the distinction between document readiness and code-execution progress.
5. Treat planning-layer quality as part of the implementation scope: keep the program auditable, explicit, refreshable, and easy for future contributors to execute without guesswork.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define `planned`, `drafted`, `active`, `partially_executed`, `completed`, and `archived`.
- [ ] Define `created`, `section-complete`, and `superseded`.
- [ ] Clarify how statuses appear in tracker and wave summaries.
- [ ] Prevent “file exists” from being treated as “completed.”
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- verify tracker language matches the legend after each wave
- verify future governance docs use the same status terms consistently
- spot-check that archived or superseded docs are labeled explicitly
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify the definition of done, refresh triggers, and any superseded/historical pointers are explicit enough that another contributor could continue the program without guesswork
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- apply the legend without rewriting historical wave summaries unless they are misleading
- if new status terms are added later, document them here first
- preserve existing tracker wording where it already matches the intended meaning
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/00_meta/11_archive_policy_2026-04-08.md`
