# Program Change Log Plan

Last updated: 2026-04-09

## Purpose

Define how the `REPO_Improvement` program should record wave-level and governance-level changes so the document tree has a visible operational history.

## Scope Boundaries

In scope:

- wave-level creation milestones
- count changes, queue changes, and governance changes
- change-log granularity for the document program

Out of scope:

- application release notes
- Git commit history
- options-related planning changes

## Current User-Facing Behavior

The program already has change history, but it is spread across:

- tracker updates
- program-index updates
- wave completion sections
- ad hoc summaries in chat

That history exists, but it is not yet centralized as one explicit program-change log policy.

## Current Code Surface

- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - already records wave completions and counts
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
  - records the current wave and wave transitions
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
  - governs future change-log behavior

## Current Verification Surface

- tracker/index cross-checks
- file-count audits after waves

## Strengths Already Present

- the tracker already contains meaningful wave history
- count audits already provide objective checkpoints
- wave transitions are already being recorded consistently

## Weaknesses / Failure Modes

- history is duplicated across multiple docs
- governance changes can disappear into tracker edits without a clear audit trail
- there is no explicit rule for what deserves a change-log entry

## Regressions To Avoid

- changing counts, queues, or governance rules without leaving a history trace
- relying on chat memory instead of repo history for the document program
- duplicating the same change narrative in too many places
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [GitHub tasklists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-tasklists)
  - relevant because wave progress is operational state that should remain visible
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because the change-log policy should say what must be recorded versus what may be omitted

## Proposed Improvement Approach

1. Define a light-weight change-log rule:
   - wave completion
   - planned-count changes
   - folder-structure changes
   - governance-rule changes
2. Keep the tracker as the detailed ledger and the change log as the higher-level summary.
3. Avoid duplicating every created-doc bullet in the change log.
4. Treat planning-layer quality as part of the implementation scope: keep the program auditable, explicit, refreshable, and easy for future contributors to execute without guesswork.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Required Change-Log Entries

Record a program-level change-log entry when any of the following happen:

- a full wave is completed or re-baselined
- the planned-doc count changes
- a top-level folder is added, removed, or materially re-scoped
- governance rules change in a way that affects future document creation or execution
- the execution model changes in a way that affects how contributors use the corpus

Do not create a separate program-level change-log entry for every individual planned doc. Per-doc detail belongs in the tracker or the document itself.

## Program Change Log Entries

- `2026-04-08`
  - locked the active non-options improvement catalog as `v7`
  - fixed the full planned-doc scope at `356` planned slots
- `2026-04-09`
  - completed Waves `1` through `26` for planned-doc creation
  - completed the second-pass hardening sweep across the implementation-doc corpus
  - adopted the execution-ledger model for whole-document implementation tracking
  - promoted `00_program_index_2026-04-09.md` and `01_doc_creation_tracker_2026-04-09.md` as the canonical current-status meta docs

## Checklist

- [x] Define what changes require a program change-log entry.
- [x] Keep wave completions and governance changes visible.
- [x] Avoid duplicating the tracker’s full per-doc detail.
- [x] Preserve a concise historical record inside the repo.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- verify major wave transitions and governance changes are recorded consistently
- verify tracker and change-log summaries do not contradict each other
- verify count changes in the log match audited counts
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify the definition of done, refresh triggers, and any superseded/historical pointers are explicit enough that another contributor could continue the program without guesswork
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- start using the change-log policy for future waves rather than rewriting all earlier history
- keep the log concise enough to maintain
- if a wave is reverted or re-baselined, note that explicitly instead of erasing the prior state
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
- `REPO_Improvement/00_meta/11_archive_policy_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
