# Public Release Operations Plan

Last updated: 2026-04-09

## Purpose

Define the public-release operations plan so this repository can publish understandable releases, stable asset naming, supported-platform expectations, and post-release ownership without relying on private maintainer memory.

## Scope Boundaries

In scope:

- public release checklist expectations
- release note and changelog discipline
- release asset naming and packaging expectations
- supported-platform and supported-variant communication
- post-release follow-up and rollback expectations

Out of scope:

- low-level build implementation details already covered elsewhere
- options-specific release planning
- continuous deployment automation beyond what a public release process should expect

## Current User-Facing Behavior

The repo already has meaningful release/build planning:

- build variants are documented
- frontend build handoff is documented
- local-vs-CI authority is documented
- bundle and packaging concerns are documented

What is still missing is the public release-operations layer:

- no documented public release checklist
- no documented release-note structure
- no explicit supported-platform/support-window statement
- no post-release triage or rollback plan in the improvement set

That means the repo can build software, but the public-release story is still under-specified.

## Current Code Surface

- `RELEASE.md`
- `README.md`
- `.github/`
- `.github/workflows.disabled/`
- `src-tauri/tauri.conf.json`
- build and packaging scripts referenced by release docs
- current absence of repo release-note or changelog process docs

## Current Verification Surface

- manual review of release-facing docs
- local build verification across intended variants
- perf-gate outputs for candidate builds
- future GitHub releases page once public releases are created

## Strengths Already Present

- the repo already distinguishes build variants and benchmark posture carefully
- release/build docs already exist instead of being fully ad hoc
- artifact and report naming quality is already a recognized concern elsewhere in the plan set
- the improvement program already values evidence-backed verification

## Weaknesses / Failure Modes

- release publishing can become inconsistent without a checklist
- public users may not know which platform/build combinations are actually supported
- release notes can drift into benchmark claims without enough context
- release assets may be hard to compare or trust if naming is inconsistent
- there is no documented post-release monitoring or hotfix path

## Regressions To Avoid

- publishing releases with unclear build variant or GPU requirements
- claiming support for platforms or variants that are not actually exercised
- mixing internal experiment assets with public release assets
- letting release notes become marketing copy instead of operational guidance
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [About releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
  - relevant because releases are the public packaging surface users actually consume
- [Managing releases in a repository](https://docs.github.com/articles/creating-releases)
  - relevant because GitHub releases are the likely public distribution channel and support draft/publish workflows with attached assets
- [Automatically generated release notes](https://docs.github.com/enterprise-server%403.19/repositories/releasing-projects-on-github/automatically-generated-release-notes)
  - relevant because generated notes can help, but still need repo-specific curation
- [Release note content type](https://docs.github.com/en/contributing/style-guide-and-content-model/release-note-content-type)
  - relevant because user-facing release notes should explain impact and preparation, not just list commits
- [Releasing and maintaining actions](https://docs.github.com/en/actions/sharing-automations/creating-actions/releasing-and-maintaining-actions)
  - relevant because it models versioned release discipline, semver expectations, and ongoing maintenance posture in a public ecosystem
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Define one release checklist that must be satisfied before a public release is tagged or published.
2. Make the checklist cover:
   - build variant identity
   - supported platform statement
   - verification summary
   - release asset naming
   - release-note content
   - post-release follow-up owner
3. Keep release notes user-facing and operational:
   - what changed
   - who should care
   - known constraints
   - GPU/CPU variant notes
   - migration or compatibility notes
4. Treat benchmark claims as a separate evidence-backed surface:
   - benchmark numbers may be linked, but not substituted for release guidance
5. Define post-release follow-up:
   - first-run smoke on the released artifact
   - issue triage window
   - rollback or hotfix criteria
6. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the public release checklist for this repo.
- [ ] Define the supported platform and build-variant statement format.
- [ ] Define the public release-note structure and minimum required sections.
- [ ] Define release asset naming expectations.
- [ ] Define where benchmark evidence may be linked and where it must not dominate release notes.
- [ ] Define a post-release validation and triage window.
- [ ] Define rollback or hotfix criteria for broken public releases.
- [ ] Cross-link the release process from `README.md` and `RELEASE.md` if the repo begins shipping public releases.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- verify the checklist against one candidate release end to end
- verify that release notes accurately name the build variant and support constraints
- verify release assets match the documented naming convention
- verify the first public release can be understood without private maintainer context
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- start with a lightweight but truthful release checklist
- keep prereleases distinct from stable releases if the quality bar differs
- if the repo is not ready for a broad support promise, document a narrower supported-platform statement instead of vague language
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/03_frontend_build_handoff_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/15_public_repo_community_health_2026-04-09.md`
