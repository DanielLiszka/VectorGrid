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

## Checklist

- [ ] Document the intended ownership split between `README.md` and `REPO_MAP.md`.
- [ ] Review top-level links for stale or duplicate routing content.
- [ ] Keep benchmark/product framing in README and code-routing detail in `REPO_MAP.md`.
- [ ] Keep `AGENTS.md` aligned with the repo-map-first navigation rule.
- [ ] Update both docs together when a major workflow entrypoint changes.
- [ ] Add or tighten doc review steps if top-level routing confusion returns.

## Verification Plan

- manually review `README.md`, `REPO_MAP.md`, `AGENTS.md`, and `RELEASE.md` together
- confirm top-level links point to current files
- confirm new contributors can route to the right surface without broad repo search

## Rollout / Fallback / Compatibility Notes

- keep current top-level docs additive and role-specific
- if sections are moved, prefer linking rather than copy-pasting the same material twice
- preserve stable file names when possible because other docs already point at them

## Related Docs

- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
- `REPO_Improvement/00_meta/07_dependency_map_2026-04-08.md`
- `REPO_Improvement/00_meta/06_folder_map_2026-04-08.md`
