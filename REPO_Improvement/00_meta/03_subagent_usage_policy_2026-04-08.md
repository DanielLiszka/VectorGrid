# Sub-Agent Usage Policy

Last updated: 2026-04-09

## Purpose

Define when and how sub-agents should be used during the `REPO_Improvement` program so parallel context gathering improves quality without creating tracker drift or shallow duplication.

## Scope Boundaries

In scope:

- sub-agent usage during implementation-doc creation
- bounded delegation for repo mapping, verification review, and research assistance
- how sub-agent findings are incorporated into the parent doc

Out of scope:

- generic code-generation delegation rules outside `REPO_Improvement`
- options-related planning work
- replacing primary repo inspection with agent summaries alone

## Current User-Facing Behavior

There is no direct product behavior here. The current behavior is procedural: the repo’s `AGENTS.md` and the meta plan already encourage or require sub-agent usage when it improves context coverage, and recent waves have already used sub-agents to scan separate shell, data, validation, and history surfaces in parallel before drafting docs.

## Current Code Surface

- `AGENTS.md`
  - directs beneficial use of sub-agents and parallel context gathering
- `REPO_Improvement/REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - defines sub-agent usage as part of the program
- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
  - requires bounded sub-agent exploration when helpful
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
  - treats sub-agents as a quality multiplier, not decoration
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
  - records wave progression that increasingly depends on parallel surface mapping

## Current Verification Surface

- parent-agent review of sub-agent summaries
- tracker/index synchronization after wave completion
- section-complete audits on the final parent-created docs

## Strengths Already Present

- the repo already has an explicit policy favoring sub-agents when beneficial
- recent waves proved sub-agents are useful for parallel shell/data/validation review
- the final document still goes through one parent-agent synthesis path
- sub-agent findings already help reduce blind spots before drafting

## Weaknesses / Failure Modes

- sub-agent usage is encouraged, but not yet documented as an operating policy
- agents can duplicate effort if delegation scope is vague
- sub-agent summaries can drift from final tracker state if not folded back immediately
- there is no written rule for when local parent inspection is still mandatory

## Regressions To Avoid

- treating sub-agent summaries as a substitute for reading key repo files
- spawning agents without bounded scopes
- allowing sub-agent outputs to update counts or tracker state without parent verification
- using sub-agents decoratively when the surface is small and local inspection is simpler
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because multi-surface planning benefits from clear ownership boundaries rather than duplicated authorities
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because bounded, test-aware context gathering is better than generic summaries
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because verification work benefits from targeted scenario coverage rather than broad unstructured exploration
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because the sub-agent policy should use clear must/should language instead of vague suggestions
- [GitHub tasklists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-tasklists)
  - relevant because the planning layer needs executable checklists, explicit acceptance criteria, and visible progress semantics rather than vague prose

## Proposed Improvement Approach

1. Define when sub-agents are required:
   - multi-layer frontend/backend/runtime topics
   - verification-surface scans
   - large wave queues with distinct topical clusters
2. Define when sub-agents are optional:
   - small, single-file, single-surface docs
3. Define a bounded delegation template:
   - exact doc slugs
   - files to inspect
   - facts to return
   - regressions to avoid
4. Keep parent-agent responsibilities explicit:
   - read the key files
   - validate the agent summary
   - create the final doc
   - update the tracker/index
5. Treat planning-layer quality as part of the implementation scope: keep the program auditable, explicit, refreshable, and easy for future contributors to execute without guesswork.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define when sub-agents are required versus optional.
- [ ] Define a bounded delegation template for future waves.
- [ ] Require parent verification of all sub-agent findings before doc creation.
- [ ] Require parent ownership of tracker/index updates.
- [ ] Record sub-agent use in wave summaries when it materially affected coverage.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- verify future wave docs still include parent-repo grounding, not only delegated summaries
- verify tracker/index changes are made only after parent count audits
- spot-check that delegated scopes stay bounded and non-overlapping
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify the definition of done, refresh triggers, and any superseded/historical pointers are explicit enough that another contributor could continue the program without guesswork
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- apply this policy to all future waves immediately
- if a sub-agent result conflicts with direct repo inspection, the repo wins and the conflict should be noted
- if a topic is too small for delegation, skip sub-agents and document locally
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
- `REPO_Improvement/00_meta/doc_creation_tracker_2026-04-08.md`
