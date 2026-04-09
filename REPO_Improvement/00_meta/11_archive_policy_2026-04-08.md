# Archive Policy Plan

Last updated: 2026-04-08

## Purpose

Define when and how `REPO_Improvement` docs should be archived, superseded, or left active so the document tree can evolve without losing historical context.

## Scope Boundaries

In scope:

- archive and supersede rules for `REPO_Improvement` docs
- when to keep historical docs versus replacing them
- tracker/index expectations for archived docs

Out of scope:

- Git archive branches
- application runtime artifacts
- options-related document programs

## Current User-Facing Behavior

The program already assumes that docs may eventually become archived, but the actual archive rules are still implicit. Right now the tree is small enough that this has not created confusion, but it will matter once more waves land and some docs are replaced or split.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - includes `archived` in the status model
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
  - will govern future supersede/archive updates
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - should reflect archive/supersede changes once they occur

## Current Verification Surface

- manual review of tracker and created-doc lists

## Strengths Already Present

- the status model already anticipates archival
- the tracker provides a natural place to record archived or superseded state
- current wave docs still have stable filenames that can be referenced historically

## Weaknesses / Failure Modes

- there is no explicit rule for when a doc should be archived instead of updated
- supersede relationships are not yet formalized
- archive state could become invisible if only filenames change

## Regressions To Avoid

- deleting old docs with no historical pointer
- archiving docs silently without updating the tracker
- keeping contradictory active docs side by side with no supersede note

## External Research Summary

- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because archive and supersede rules should be explicit
- [GitHub tasklists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-tasklists)
  - relevant because archived docs should not keep pretending to be active worklists

## Proposed Improvement Approach

1. Define the difference between:
   - active update
   - superseded doc
   - archived doc
2. Require a forward pointer when a doc is superseded.
3. Require tracker/index updates whenever archive state changes.
4. Preserve old docs for historical context unless they are exact duplicates.

## Checklist

- [ ] Define archive versus supersede semantics.
- [ ] Require forward pointers for superseded docs.
- [ ] Update tracker/index when archive state changes.
- [ ] Preserve historical context rather than deleting useful planning history.

## Verification Plan

- spot-check future archive changes for tracker/index updates
- verify superseded docs point to their replacements
- ensure archived docs are not still treated as active queue items

## Rollout / Fallback / Compatibility Notes

- apply this policy before the tree grows much larger
- prefer supersede notes over deletion
- if a doc is merged into another, preserve a reference trail in the tracker or related-doc sections

## Related Docs

- `REPO_Improvement/00_meta/05_status_legend_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/00_meta/12_program_change_log_2026-04-08.md`
