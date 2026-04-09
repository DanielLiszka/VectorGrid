# Program Change Log Plan

Last updated: 2026-04-08

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

## Checklist

- [ ] Define what changes require a program change-log entry.
- [ ] Keep wave completions and governance changes visible.
- [ ] Avoid duplicating the tracker’s full per-doc detail.
- [ ] Preserve a concise historical record inside the repo.

## Verification Plan

- verify major wave transitions and governance changes are recorded consistently
- verify tracker and change-log summaries do not contradict each other
- verify count changes in the log match audited counts

## Rollout / Fallback / Compatibility Notes

- start using the change-log policy for future waves rather than rewriting all earlier history
- keep the log concise enough to maintain
- if a wave is reverted or re-baselined, note that explicitly instead of erasing the prior state

## Related Docs

- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
- `REPO_Improvement/00_meta/11_archive_policy_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
