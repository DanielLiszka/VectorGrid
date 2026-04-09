# Repo Map README Hygiene Plan

Last updated: 2026-04-09

## Purpose

Define the current hygiene contract between `README.md`, `REPO_MAP.md`, and related routing docs so contributors can find the right surface quickly without duplicating or contradicting repo guidance.

## Scope Boundaries

In scope:

- `README.md`
- `REPO_MAP.md`
- routing versus product-framing responsibilities
- link hygiene between top-level guidance docs
- current contributor-facing navigation clarity

Out of scope:

- code implementation details
- options-specific planning docs
- GitHub issue or PR templates

## Current User-Facing Behavior

The repo already uses a two-layer documentation split:

- `README.md` for product framing, benchmark posture, quick start, and high-level layout
- `REPO_MAP.md` for contributor routing into concrete code and workflow surfaces

That split is already useful, but it is only safe if the two docs remain intentionally differentiated rather than becoming two competing “main docs.”

## Current Code Surface

- `README.md`
- `REPO_MAP.md`
- `AGENTS.md`
- `RELEASE.md`
- `web/README.md`
- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`

## Current Verification Surface

- manual doc review for top-level routing consistency
- contributor task starts that rely on `README.md` and `REPO_MAP.md`
- link and path review during doc changes

## Strengths Already Present

- the repo already separates product framing from routing guidance
- `REPO_MAP.md` already names stable entry points and code-first routing files
- `README.md` already stays closer to product posture and benchmark framing
- `AGENTS.md` already points contributors to `REPO_MAP.md` before broad search

## Weaknesses / Failure Modes

- README and repo-map responsibilities can drift back together over time
- routing links can become stale because the docs are large and frequently edited
- it is easy to repeat the same content in both files with different wording
- contributor onboarding quality depends heavily on these docs staying distinct

## Regressions To Avoid

- turning `README.md` into a giant routing file
- turning `REPO_MAP.md` into product-marketing copy
- keeping conflicting quick-start or scope language across top-level docs
- leaving stale links or retired surfaces in the routing docs
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [GitHub Docs: About READMEs](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because README should stay a clear entrypoint rather than a dumping ground
- [GitHub Docs: Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/basic-writing-and-formatting-syntax)
  - relevant because link and structure hygiene matter for long repo docs
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because routing docs benefit from clear must/should distinctions when defining ownership
- [Autolinked references and URLs](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/autolinked-references-and-urls?formCode=MG0AV3)
  - relevant because large repo docs rely on readable link conventions

## Proposed Improvement Approach

1. Keep `README.md` responsible for product framing and quick-start guidance.
2. Keep `REPO_MAP.md` responsible for code and workflow routing.
3. Review shared links and scope wording together whenever either doc changes.
4. Prefer adding one clear pointer over duplicating a whole section in both files.
5. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the intended ownership split between `README.md` and `REPO_MAP.md`.
- [x] Review top-level links for stale or duplicate routing content.
- [x] Keep benchmark/product framing in README and code-routing detail in `REPO_MAP.md`.
- [x] Keep `AGENTS.md` aligned with the repo-map-first navigation rule.
- [x] Update both docs together when a major workflow entrypoint changes.
- [x] Add or tighten doc review steps if top-level routing confusion returns.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- manually review `README.md`, `REPO_MAP.md`, `AGENTS.md`, and `RELEASE.md` together
- confirm top-level links point to current files
- confirm new contributors can route to the right surface without broad repo search
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep current top-level docs additive and role-specific
- if sections are moved, prefer linking rather than copy-pasting the same material twice
- preserve stable file names when possible because other docs already point at them
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
- `REPO_Improvement/00_meta/07_dependency_map_2026-04-08.md`
- `REPO_Improvement/00_meta/06_folder_map_2026-04-08.md`
