# Docs Archive Policy Plan

Last updated: 2026-04-09

## Purpose

Define how non-options docs, evidence packs, and superseded planning records should be archived so the repo stays usable without losing history.

## Scope Boundaries

In scope:

- archival rules for docs and planning artifacts
- active versus historical doc separation
- superseded versus retained records
- lightweight evidence/sample retention inside the repo

Out of scope:

- deleting code or build artifacts from `target/`
- options-related archive programs
- generic Git history retention policy

## Current User-Facing Behavior

The repo already contains active docs, historical research, prompt packs, baselines, and improvement-program records. That history is valuable, but without explicit archive rules it is easy for active and superseded docs to blur together.

## Current Code Surface

- `REPO_Improvement/`
- `research/`
- `Frontend Review/`
- `SASS research/`
- `scripts/baselines/`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`

## Current Verification Surface

- manual doc-tree review
- improvement-program tracker review
- baseline/sample manifest review

## Strengths Already Present

- the repo already keeps history instead of rewriting it away
- active and historical material are often separated by folder
- the improvement program already has governance docs that can support archival rules

## Weaknesses / Failure Modes

- active and superseded docs can still be hard to distinguish
- there is no single archive rule for when a doc is retained in place, moved, or merely marked historical
- evidence packs can accumulate without a clear “active reference” marker

## Regressions To Avoid

- deleting useful historical context without replacement
- keeping superseded docs active-looking when they are no longer authoritative
- moving docs without updating the links that point to them
- treating archive cleanup as a reason to erase provenance

## External Research Summary

- [GitHub Docs: About READMEs](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because active entrypoint docs should remain easy to find
- [GitHub Docs: Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/basic-writing-and-formatting-syntax)
  - relevant because archived docs still need navigable links and labels
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because archive rules should be explicit about must/should behavior

## Proposed Improvement Approach

1. Keep active docs near active entrypoints.
2. Keep historical docs, but mark them clearly as archived or superseded.
3. Prefer archival markers and index updates over silent moves or deletions.
4. Preserve baseline/evidence packs that still support verification or claims.

## Checklist

- [ ] Define when a doc is active, historical, archived, or superseded.
- [ ] Define when a doc should be moved versus relabeled in place.
- [ ] Update indexes and cross-links when archive actions happen.
- [ ] Keep evidence/sample docs discoverable if active claims still depend on them.
- [ ] Avoid deleting provenance without replacement.

## Verification Plan

- review active entrypoint docs and confirm they do not point primarily at archived material
- review archive markers and folder placement after any doc move
- verify cross-links after archival cleanup

## Rollout / Fallback / Compatibility Notes

- prefer additive archive labels first
- use physical moves only when the active tree becomes meaningfully harder to navigate
- keep historical docs linkable after any archival action

## Related Docs

- `REPO_Improvement/11_docs_ops/05_baseline_samples_manifest_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
- `REPO_Improvement/00_meta/11_archive_policy_2026-04-08.md`
