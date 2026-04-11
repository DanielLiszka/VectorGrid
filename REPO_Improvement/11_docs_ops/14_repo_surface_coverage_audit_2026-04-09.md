# Repo Surface Coverage Audit Plan

Last updated: 2026-04-09

## Purpose

Define how the non-options improvement program should audit repo-surface coverage so the document set stays comprehensive and honest as the codebase evolves.

## Scope Boundaries

In scope:

- auditing planned-doc coverage against current repo surfaces
- routing through `REPO_MAP.md`
- folder/count reconciliation for the non-options document program
- handling newly discovered uncovered surfaces

Out of scope:

- options-surface coverage
- code implementation changes
- project-wide architecture decisions unrelated to the audit process

## Current User-Facing Behavior

The current improvement program already claims broad non-options coverage. That claim is only safe if it is periodically audited against the actual repo surfaces and not only against the existing document tree.

## Current Code Surface

- `REPO_MAP.md`
  - canonical routing anchor for current repo surfaces
- `AGENTS.md`
  - current repo-workflow and navigation rules
- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - planned-doc catalog
- `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
  - created-doc counts and wave state
- `REPO_Improvement/00_meta/00_program_index_2026-04-09.md`
  - current-wave routing summary

## Current Verification Surface

- folder-count audits under `REPO_Improvement/`
- planned-versus-created slot review against the meta plan
- repo-grounding passes against `REPO_MAP.md` and current code directories
- manual spot audits when new surfaces are discovered

## Strengths Already Present

- the program already has a detailed meta catalog
- `REPO_MAP.md` already provides a code-first surface inventory
- tracker and index files already support explicit coverage accounting
- the user has already required comprehensive non-options coverage rather than partial samples

## Weaknesses / Failure Modes

- a large document program can look complete even if one real surface is missing
- coverage claims can drift when the repo changes faster than the docs
- folder counts alone do not prove surface coverage
- “non-options coverage” is a living target and needs repeated audits

## Regressions To Avoid

- claiming full coverage without comparing back to the live repo surface
- treating old counts as proof of current completeness
- letting newly added or renamed surfaces go unrepresented in the plan
- confusing “more docs” with “better coverage”
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [GitHub Docs: About READMEs](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because entrypoint docs must remain aligned with real repo surfaces
- [GitHub Docs: Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/basic-writing-and-formatting-syntax)
  - relevant because large cross-linked doc programs depend on clear structure and links
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because coverage-audit rules should be explicit

## Proposed Improvement Approach

1. Use `REPO_MAP.md` as the routing anchor for every coverage audit.
2. Compare planned-doc catalog, actual created docs, and current code surfaces together.
3. Treat newly discovered uncovered surfaces as a meta-plan update, not as an informal note.
4. Keep “covered” meaning “represented by a real planned doc or an intentional merged owner,” not just “mentioned somewhere.”
5. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Audit the planned catalog against current `REPO_MAP.md`.
- [x] Audit created docs against planned slots.
- [x] Record any intentionally merged or superseded coverage decisions.
- [x] Update tracker, index, and meta-plan counts together when coverage changes.
- [x] Re-run the audit when the repo adds or renames meaningful non-options surfaces.
- [x] Keep scope wording honest: non-options coverage only.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- compare the meta-plan folder counts against actual `REPO_Improvement/` file counts
- compare created planned-doc slots against the live catalog
- review `REPO_MAP.md` for uncovered non-options surfaces
- update the tracker and meta plan when uncovered surfaces are found
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep coverage audits iterative; do not wait for major rewrites
- preserve historical tracker data when new surfaces expand the catalog
- if surfaces are merged into broader docs, record that explicitly rather than hiding the count change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on a fresh repo-surface reconciliation plus a small doc-sync correction from the historical 2026-04-08 tracker/index references to the canonical 2026-04-09 tracker/index files.
- The current audit authority surfaces remain:
  - `REPO_MAP.md`
  - `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - `REPO_Improvement/README.md`
  - `REPO_Improvement/00_meta/00_program_index_2026-04-09.md`
  - `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
  - `REPO_Improvement/00_meta/execution_ledger_2026-04-09.md`
- Fresh acceptance evidence:
  - repo count audit: `TOTAL_MD=363`, `DOCS_OPS_MD=16`
  - `rg -n "Current Status|planned-doc creation status|second-pass implementation-doc hardening status" REPO_Improvement/00_meta/00_program_index_2026-04-09.md REPO_Improvement/README.md`
  - `rg -n "Current Folder Status|Completion Check" REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
- The fresh reconciliation confirms the current non-options coverage posture this doc is meant to freeze:
  - the canonical program index still records planned-doc creation status as `complete`
  - the canonical program index still records second-pass implementation-doc hardening status as `complete`
  - the live `11_docs_ops` folder count remains `16`, matching the planned section total
  - the ledger still tracks `356` planned implementation-plan documents, which matches the current program accounting rather than an outdated snapshot
- This closure does not claim that future repo changes are automatically audited.
  - It closes because the current routing map, meta catalog, tracker, and ledger are reconciled today and provide a repeatable audit procedure for later reruns.

## Related Docs

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
- `REPO_Improvement/00_meta/00_program_index_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/06_repo_map_readme_hygiene_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/09_non_options_scope_boundary_2026-04-09.md`
