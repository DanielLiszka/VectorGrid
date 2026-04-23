# Archive Policy Plan

Last updated: 2026-04-09

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
- `REPO_Improvement/README.md`
  - current entrypoint doc that distinguishes canonical current surfaces from historical snapshots
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
  - will govern future supersede/archive updates
- `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
  - should reflect archive/supersede changes once they occur
- `REPO_MAP.md`
  - current cross-tree routing surface for when specialist or historical docs remain useful
- `Frontend Review/`
- `research/`
- `CUDA kernel review/`
- `SASS research/`
- `scripts/baselines/`

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
5. Treat planning-layer quality as part of the implementation scope: keep the program auditable, explicit, refreshable, and easy for future contributors to execute without guesswork.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Archive And Supersede Rules

- keep a document active when it is still the current planning authority and can be safely updated in place
- mark a document superseded when a newer document replaces it as the planning authority for the same surface
- mark a document archived when it remains useful as historical context but should no longer receive routine edits
- delete a document only when it is an exact duplicate or an accidental placeholder with no historical value

## Required Pointer Rules

- every superseded document must include a clear forward pointer to the canonical replacement
- tracker, index, and execution-ledger references must be updated in the same change as the supersede or archive action
- historical documents must retain enough context that a contributor can understand why they are no longer active
- archive or supersede actions must preserve short-path discoverability rather than hiding state in commit history alone
- when specialist or historical trees remain in place outside `REPO_Improvement/`, active entry docs must point to the current canonical file instead of silently relying on folder names alone

## Cross-Tree Archive Posture

- `REPO_Improvement/` is the active planning and execution tree for the non-options improvement program.
- `Frontend Review/`, `research/`, `CUDA kernel review/`, and `SASS research/` are retained specialist or historical trees.
  - keep them linkable for provenance and historical context
  - do not silently delete or relocate them just to make the top level look cleaner
  - when one of those trees contains a still-useful authority doc, route to the exact file from `REPO_MAP.md` or another current entrypoint instead of assuming the folder itself communicates state
- `scripts/baselines/` is the evidence tree for retained performance, parity, and review artifacts.
  - keep active baseline/sample packs discoverable through manifests, readmes, or current routing docs
  - do not treat baseline retention as a reason to move current authority docs out of their natural home

## Checklist

- [x] Define archive versus supersede semantics.
- [x] Require forward pointers for superseded docs.
- [x] Update tracker/index when archive state changes.
- [x] Preserve historical context rather than deleting useful planning history.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- spot-check future archive changes for tracker/index updates
- verify superseded docs point to their replacements
- ensure archived docs are not still treated as active queue items
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify the definition of done, refresh triggers, and any superseded/historical pointers are explicit enough that another contributor could continue the program without guesswork
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- apply this policy before the tree grows much larger
- prefer supersede notes over deletion
- if a doc is merged into another, preserve a reference trail in the tracker or related-doc sections
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/00_meta/05_status_legend_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
- `REPO_Improvement/00_meta/12_program_change_log_2026-04-08.md`
