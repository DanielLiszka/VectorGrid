# Implementation Doc Quality Bar Plan

Last updated: 2026-04-09

## Purpose

Define the minimum quality bar for every `REPO_Improvement` implementation-plan document so the document program stays comprehensive, research-backed, additive-only, and safe to use as the next step before code work.

## Scope Boundaries

In scope:

- all non-options implementation-plan docs under `REPO_Improvement/`
- required section structure
- required repo grounding
- required external research before plan writing
- required checklist and verification content

Out of scope:

- the higher-level meta catalog itself
- code implementation details for any one feature
- options-related planning docs

## Current User-Facing Behavior

This quality bar is user-facing in the sense that the document program is already being used as the planning layer for future work. The current tracker already audits implementation-plan docs for a 14-section template, and the active program explicitly requires additive improvements, no regressions, no functionality removal, substantial repo grounding, and extensive pre-plan external research. That means the quality bar is already partly enforced in practice, but it still needs one dedicated doc that states the standard clearly.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - active charter and catalog for the non-options improvement program
- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
  - current research-before-planning gate
- `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
  - additive-only and regression-safety expectations
- `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
  - tracks completion state and section-complete audits
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
  - current research-source policy

## Current Verification Surface

- section-complete audits run against all implementation-plan docs
- tracker/index synchronization checks
- repo-grounding checks performed during doc creation
- external research/source review during doc drafting

## Strengths Already Present

- the program already has a stable 14-section structure
- additive-only and non-regression rules are already explicit in the meta layer
- the user has already required extensive web research before drafting each implementation plan
- sub-agent usage is already permitted and encouraged when helpful
- existing docs already include repo code surfaces, verification plans, and checklists

## Weaknesses / Failure Modes

- the quality bar is currently spread across multiple meta docs and process memory
- it is possible to satisfy the section template while still writing a shallow doc
- external research requirements can become inconsistent if not stated as a minimum standard
- “section-complete” is currently tracked, but not defined as a stable contract

## Regressions To Avoid

- allowing implementation docs that skip repo grounding or current code surfaces
- allowing plans without external research summary or without official/primary sources when available
- weakening the additive-only and no-functionality-removal rule
- accepting checklist-light or verification-light docs as complete
- creating implementation docs without enough detail to guide later work safely
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema: What Is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because several implementation docs describe structured persisted or cross-layer contracts
- [Pinia: Testing](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because verification sections should use real repo-appropriate test surfaces
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
  - relevant because some implementation docs need browser/desktop verification planning
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because many implementation docs cover Tauri-facing boundaries and should cite current primary docs
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Freeze the minimum document standard:
   - repo-grounded
   - research-backed
   - additive-only
   - checklist-bearing
   - verification-bearing
2. Require all implementation docs to include, at minimum:
   - current user-facing behavior
   - current code surface
   - current verification surface
   - weaknesses/failure modes
   - regressions to avoid
   - external research summary
   - proposed approach
   - checklist
   - verification plan
   - rollout/compatibility notes
3. Treat sub-agents as a quality multiplier when a surface spans multiple code areas or verification layers.
4. Require external web research before drafting, and prefer official/primary sources whenever possible.
5. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Keep the 14-section implementation-doc template mandatory.
- [x] Require repo-grounded code and verification surfaces in every doc.
- [x] Require external research summary before the doc is considered complete.
- [x] Prefer official or primary sources when available; call out when only weaker sources exist.
- [x] Require additive-only, no-regression, no-functionality-removal framing in every doc.
- [x] Require a concrete checklist and verification plan in every doc.
- [x] Use sub-agents when they materially improve context gathering, code-surface coverage, or verification confidence.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- re-run the section-complete audit across all implementation-plan docs
- verify tracker/index counts match actual file counts after each creation wave
- spot-check new docs against current repo code surfaces before marking them complete
- ensure each newly created implementation doc cites current external research, not stale assumptions
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- if the quality bar is tightened later, update earlier docs that no longer meet the active standard
- do not mark a doc complete just because the file exists; completeness depends on the quality bar
- keep the quality bar additive and explicit so later waves do not regress in detail
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on a fresh audit of the current implementation-plan corpus plus a small doc-sync correction from the historical 2026-04-08 tracker reference to the canonical 2026-04-09 tracker.
- The current quality-bar authority surfaces remain:
  - `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
  - `REPO_Improvement/00_meta/04_doc_template_2026-04-08.md`
  - `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
  - `REPO_Improvement/00_meta/10_verification_expectations_2026-04-08.md`
  - `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
  - `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
- Fresh acceptance evidence:
  - `rg -n "all current planned implementation docs follow the required 14-section template|public-repo code-quality language" REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
  - targeted review of the canonical template and verification-authority docs
  - fresh section/link audit used for `11_docs_ops/10_research_source_policy_2026-04-08.md`
- The current tracker now explicitly records the corpus-wide evidence this quality-bar doc wanted frozen:
  - all current planned implementation docs follow the required 14-section template
  - all current planned implementation docs include the stronger public-repo code-quality and acceptance-evidence language
- This closure is intentionally based on documented policy plus tracker-backed corpus audit rather than a new enforcement script.
  - There is still no standalone automated “section-complete audit” tool, but the template, tracker assertions, and corpus spot checks are now strong enough to count the quality bar itself as implemented.

## Related Docs

- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
- `REPO_Improvement/00_meta/01_doc_creation_tracker_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
