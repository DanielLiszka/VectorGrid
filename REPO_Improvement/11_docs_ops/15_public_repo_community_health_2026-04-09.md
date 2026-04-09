# Public Repo Community Health Plan

Last updated: 2026-04-09

## Purpose

Define the public-repo community-health and maintainer-operations plan so contributors and users can understand how to use, contribute to, get support for, and report problems in this repository without depending on private context.

## Scope Boundaries

In scope:

- public-repo contributor guidance
- community-health files and maintainer ownership expectations
- license visibility
- support and security reporting guidance
- CODEOWNERS and maintainer-routing expectations
- additive public-facing repo quality improvements

Out of scope:

- internal-only maintenance notes that should not live in a public repo
- feature implementation details
- options-specific planning
- legal advice beyond documenting repository-health expectations

## Current User-Facing Behavior

The repo already has strong technical documentation in `README.md`, `REPO_MAP.md`, `AGENTS.md`, and `RELEASE.md`. That helps code-aware contributors find their way around. But the public-repo health surface is still thin:

- there is no `LICENSE` file at the root
- there is no `CONTRIBUTING.md`
- there is no `SECURITY.md`
- there is no `CODEOWNERS`
- there is no visible support-resource doc
- `.github/workflows` is present, but active workflow automation is currently disabled

So the repo can be understandable to an insider while still feeling under-specified to an outside contributor or evaluator.

## Current Code Surface

- `README.md`
- `RELEASE.md`
- `REPO_MAP.md`
- `AGENTS.md`
- `.github/`
- `.github/workflows.disabled/README.md`
- repository root community-health file set or current absence of it

## Current Verification Surface

- manual inspection of root docs and `.github` contents
- GitHub repository community-profile review once the repo is public
- contributor onboarding attempts using only public docs

## Current Strengths Already Present

- the technical routing docs are already much better than a typical repo of this size
- the improvement program already cares about truthfulness and documentation hygiene
- benchmark and release posture are already documented more carefully than usual
- non-options scope boundaries are already explicit

## Current Weaknesses and Failure Modes

- public contributors do not have a defined contribution path
- there is no clear security-reporting instruction surface
- there is no explicit support path for usage questions versus bug reports
- ownership and review routing are undocumented
- lack of license visibility makes public-use expectations ambiguous
- the repo can appear less mature than it is because the community-health layer is absent

## Regressions To Avoid

- adding generic open-source boilerplate that does not match the actual maintainer posture
- promising community responsiveness or support levels that cannot be sustained
- mixing contribution policy with deep technical routing detail
- creating files that contradict existing README or release posture
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Setting up your project for healthy contributions](https://docs.github.com/github/building-a-strong-community/setting-up-your-project-for-healthy-contributions)
  - relevant because GitHub treats community-health files as a discoverable public quality surface
- [Setting guidelines for repository contributors](https://docs.github.com/en/articles/setting-guidelines-for-repository-contributors)
  - relevant because `CONTRIBUTING.md` should guide issue and PR quality rather than exist as boilerplate
- [Adding a code of conduct to your project](https://docs.github.com/articles/adding-a-code-of-conduct-to-your-project)
  - relevant because community standards and enforcement expectations matter once a repo is public
- [Adding a license to a repository](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/adding-a-license-to-a-repository)
  - relevant because public-source code without an explicit license leaves reuse expectations unclear
- [Licensing a repository](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)
  - relevant because GitHub explicitly notes that a public repo is not truly open source without a license
- [About code owners](https://docs.github.com/github/creating-cloning-and-archiving-repositories/about-code-owners)
  - relevant because review routing and ownership should be visible rather than insider-only
- [Adding a security policy to your repository](https://docs.github.com/en/code-security/getting-started/adding-a-security-policy-to-your-repository)
  - relevant because public users need a clear vulnerability-reporting path
- [Adding support resources to your project](https://docs.github.com/enterprise-cloud%40latest/communities/setting-up-your-project-for-healthy-contributions/adding-support-resources-to-your-project)
  - relevant because support questions should not be conflated with bug or security reporting
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Define the minimal public-repo community-health file set for this project:
   - `LICENSE`
   - `CONTRIBUTING.md`
   - `SECURITY.md`
   - `SUPPORT.md`
   - `CODEOWNERS`
   - optionally `CODE_OF_CONDUCT.md` if the maintainer is prepared to enforce it
2. Keep each file specific to this repo’s real operating model.
3. Route technical setup and architecture to `README.md` and `REPO_MAP.md`, not into contribution boilerplate.
4. Make contributor expectations concrete:
   - how to report bugs
   - how to propose scoped improvements
   - what verification is expected
   - what is out of scope
5. Make support and security reporting clearly separate from general issues.
6. Add ownership routing that reflects the actual maintainer/review model instead of pretending there is a large team if there is not.
7. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Step-by-Step Checklist

- [ ] Inventory which public community-health files are currently absent.
- [ ] Decide the minimum viable community-health file set for this repo.
- [ ] Draft repo-specific contribution guidance for scoped improvements, verification, and doc updates.
- [ ] Draft a support path that separates user help from bugs and vulnerabilities.
- [ ] Draft a security-reporting path that does not route vulnerabilities through normal issues.
- [ ] Choose and add an explicit license if the repo is intended to be open source.
- [ ] Decide whether `CODEOWNERS` is useful at the current maintainer size and add it only if it reflects reality.
- [ ] Decide whether a code of conduct will be actively enforced before adding one.
- [ ] Cross-link the new community-health files from `README.md` where appropriate.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- manually review the root and `.github` file set for the intended public-repo health files
- verify GitHub surfaces the expected links once the files exist
- test contributor discoverability by navigating only from `README.md` and the repo sidebar
- verify support, bug, and security paths are not contradictory
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- start with the minimum honest community-health set instead of a maximal template dump
- if support or code-of-conduct enforcement cannot be sustained, document a narrower but truthful posture
- keep technical routing in `README.md` and `REPO_MAP.md`; community-health files should link out rather than duplicate them
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/06_repo_map_readme_hygiene_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/16_public_release_operations_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
