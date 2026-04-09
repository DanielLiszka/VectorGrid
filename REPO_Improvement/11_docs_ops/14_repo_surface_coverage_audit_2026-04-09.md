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
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - created-doc counts and wave state
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
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

## Checklist

- [ ] Audit the planned catalog against current `REPO_MAP.md`.
- [ ] Audit created docs against planned slots.
- [ ] Record any intentionally merged or superseded coverage decisions.
- [ ] Update tracker, index, and meta-plan counts together when coverage changes.
- [ ] Re-run the audit when the repo adds or renames meaningful non-options surfaces.
- [ ] Keep scope wording honest: non-options coverage only.

## Verification Plan

- compare the meta-plan folder counts against actual `REPO_Improvement/` file counts
- compare created planned-doc slots against the live catalog
- review `REPO_MAP.md` for uncovered non-options surfaces
- update the tracker and meta plan when uncovered surfaces are found

## Rollout / Fallback / Compatibility Notes

- keep coverage audits iterative; do not wait for major rewrites
- preserve historical tracker data when new surfaces expand the catalog
- if surfaces are merged into broader docs, record that explicitly rather than hiding the count change

## Related Docs

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/00_meta/program_index_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/06_repo_map_readme_hygiene_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/09_non_options_scope_boundary_2026-04-09.md`
