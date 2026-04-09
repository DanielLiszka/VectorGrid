# Migration Freeze Scope Plan

Last updated: 2026-04-09

## Purpose

Define the current migration-freeze scope guard so narrow approved-change windows stay enforceable when the repo is in a constrained migration or stabilization phase.

## Scope Boundaries

In scope:

- `scripts/check_migration_freeze_scope.mjs`
- allowed exact paths and allowed prefixes
- how scope-restricted windows should be documented and enforced
- relationship between local status checks and freeze policy

Out of scope:

- general Git workflow policy
- options-specific freeze windows
- broad release governance beyond scoped freeze enforcement

## Current User-Facing Behavior

The repo already contains a concrete scope-freeze script that inspects `git status --porcelain` and fails when local changes extend beyond a small approved path set. This is not hypothetical policy; it is executable scope enforcement.

## Current Code Surface

- `scripts/check_migration_freeze_scope.mjs`
  - current executable scope guard
- `README.md`
  - current product/release framing that can be affected by freeze windows
- `RELEASE.md`
  - release/build notes that may be part of approved freeze-scope edits
- `web/`
  - currently approved prefix in the shipped script
- `scripts/baselines/`
  - currently approved prefix in the shipped script

## Current Verification Surface

- run `node scripts/check_migration_freeze_scope.mjs`
- manual review of the approved path allowlist in the script
- `git status --porcelain` as the underlying changed-path source

## Strengths Already Present

- the freeze policy is already executable, not just prose
- allowed exact paths and prefixes are explicit
- rename handling is already accounted for in the script
- the guard already uses Git’s stable porcelain output rather than ad hoc parsing

## Weaknesses / Failure Modes

- the allowlist can drift away from the intended migration scope if not reviewed
- a freeze script is only as good as the narrative around when it should be used
- users can misunderstand the current allowlist as a permanent repo policy instead of a scoped constraint

## Regressions To Avoid

- broadening freeze scope silently
- leaving the allowlist stale relative to the actual migration phase
- using the freeze script without documenting what the active scope is protecting
- rewriting the guard in a way that weakens rename or normalized-path handling
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Git status documentation](https://git-scm.com/docs/git-status)
  - relevant because the freeze script relies on porcelain output as its authoritative changed-path source
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because freeze rules should be explicit about must/should behavior
- [GitHub Docs: task lists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-task-lists)
  - relevant because freeze windows should pair executable checks with human-readable task/state tracking
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Treat the current scope-freeze script as an executable contract for narrow migration windows.
2. Document the intent of each allowlist change whenever the script is updated.
3. Keep freeze enforcement local and explicit rather than implicit in reviewer memory.
4. Preserve exact-path and prefix-path distinctions in the allowlist.
5. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current purpose of the freeze script.
- [ ] Document allowed exact files and allowed prefixes as a deliberate scope.
- [ ] Review allowlist changes whenever the migration phase changes.
- [ ] Keep the script tied to a human-readable migration note or release note.
- [ ] Preserve normalized-path and rename handling.
- [ ] Add or tighten checks if the freeze mechanism expands.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- run `node scripts/check_migration_freeze_scope.mjs`
- verify the allowlist matches the current intended freeze window
- spot-check changed-path parsing with representative rename and edited-file cases
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the freeze script intentionally narrow
- if the migration window closes, either retire the script or update its scope explicitly
- avoid leaving historical allowlists active without context
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/09_non_options_scope_boundary_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
- `REPO_Improvement/00_meta/08_priority_rules_2026-04-08.md`
