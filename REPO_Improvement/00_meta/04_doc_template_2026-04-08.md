# Implementation Doc Template Plan

Last updated: 2026-04-09

## Purpose

Define the canonical template for `REPO_Improvement` implementation-plan documents so future waves stay consistent, detailed, and auditable.

## Scope Boundaries

In scope:

- the default implementation-doc structure
- mandatory versus optional sections
- checklist and verification formatting expectations

Out of scope:

- the higher-level meta catalog
- content decisions for any specific feature doc
- options-related planning

## Current User-Facing Behavior

The program already operates with a de facto 14-section template, and the tracker already audits for section completeness. The current template exists in practice, but it is not yet isolated into one dedicated template doc that future waves can follow directly.

## Current Code Surface

- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - defines the current 14-section expectation
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
  - defines the minimum quality bar
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - audits section completeness
- all existing `NN_*.md` implementation-plan docs under `REPO_Improvement/`
  - provide the current live examples

## Current Verification Surface

- section-complete heading audits
- tracker/index synchronization
- manual spot-checks for repo grounding and research quality

## Strengths Already Present

- the template is already stable enough to audit mechanically
- existing docs already prove the template works across technical and process topics
- the structure is detailed enough to force repo grounding, research, and verification planning

## Weaknesses / Failure Modes

- the template rules are currently spread across multiple docs
- contributors can satisfy headings mechanically while still writing shallow sections
- there is no canonical example of what “good enough” content per section looks like
- optional expansions are not documented clearly

## Regressions To Avoid

- weakening the 14-section structure
- letting implementation docs omit checklist or verification sections
- silently changing section names in ways that break the audit
- reducing the template to a generic outline that loses repo specificity
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [GitHub tasklists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-tasklists)
  - relevant because the checklist is a working-state mechanism, not decorative Markdown
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because the template should distinguish mandatory versus optional sections clearly
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because verification sections should be concrete and executable
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because many docs describe structured contracts and should preserve schema-like discipline

## Proposed Improvement Approach

1. Freeze the default 14-section template in one dedicated doc.
2. Define which sections are mandatory and which may be expanded with topic-specific extras.
3. Provide one short content standard per section:
   - what it must contain
   - what it should avoid
4. Keep the template compatible with the current audit so older docs remain valid if they already meet the bar.
5. Treat planning-layer quality as part of the implementation scope: keep the program auditable, explicit, refreshable, and easy for future contributors to execute without guesswork.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Freeze the 14 mandatory headings in one canonical order.
- [ ] Define the minimum content expectation for each heading.
- [ ] Define when extra sections are appropriate.
- [ ] Preserve checklist and verification sections as mandatory.
- [ ] Keep the tracker audit aligned with the template wording.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- rerun the section audit after any template change
- spot-check new docs against the per-section content expectations
- update the quality-bar doc if the template meaning changes materially
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify the definition of done, refresh triggers, and any superseded/historical pointers are explicit enough that another contributor could continue the program without guesswork
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the existing heading names unless the tracker audit is updated in the same change
- if a future wave needs a richer template, extend it additively rather than replacing the baseline silently
- older docs should only be backfilled if the live standard materially changes
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
