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
- `REPO_MAP.md`
  - maintenance-helper routing note for when this narrow guard should be used
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

- [x] Document the current purpose of the freeze script.
- [x] Document allowed exact files and allowed prefixes as a deliberate scope.
- [x] Review allowlist changes whenever the migration phase changes.
- [x] Keep the script tied to a human-readable migration note or release note.
- [x] Preserve normalized-path and rename handling.
- [x] Add or tighten checks if the freeze mechanism expands.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

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

## Execution Notes

- This item closed by tightening the current freeze helper and documenting its intended use as an opt-in migration/stabilization guard rather than a permanent repo policy.
- Fresh scope surfaces now aligned for this contract:
  - `scripts/check_migration_freeze_scope.mjs`
  - `RELEASE.md`
  - `REPO_MAP.md`
- The script was tightened by removing the stale `VUE_VITE_MIGRATION_MASTER_CHECKLIST.md` allowlist entry, which no longer exists in the repo.
- The human-readable posture is now explicit:
  - `RELEASE.md` states that the helper is only for explicit migration or stabilization windows
  - `REPO_MAP.md` classifies it as a narrow maintenance helper rather than a general quality gate
- Fresh acceptance evidence for the static contract:
  - reviewed the current exact-file allowlist and prefix allowlist in `scripts/check_migration_freeze_scope.mjs`
  - confirmed normalized-path and rename handling remain intact in the script
  - confirmed the repo docs now describe when to use the helper and what kind of scope it is expected to guard
- Fresh runtime evidence in a disposable worktree using the updated script:
  - `node scripts/check_migration_freeze_scope.mjs` passed with only the script itself changed
  - adding an in-scope `web/README.md` change still passed
  - adding an out-of-scope `REPO_Improvement/README.md` change failed as expected
- This closure is intentionally about the guard definition and its documentation.
  - The script is expected to fail on ordinary broad-scope repo work, because that is the point of a narrow freeze guard.
  - It now has enough context and cleanup to count as a maintained, trustworthy helper instead of a stale unexplained script.

## Related Docs

- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/09_non_options_scope_boundary_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
- `REPO_Improvement/00_meta/08_priority_rules_2026-04-08.md`
