# Artifact Retention Tags Compatibility Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to artifact retention and tag compatibility so saved runs, meta artifacts, Stage 5 artifacts, and policy-evaluation artifacts preserve user tags, retention exemptions, and cross-family tag semantics without breaking existing files.

## Scope Boundaries

In scope:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/metaService.ts`
- tag normalization, retention behavior, and cross-family artifact tag editing

Out of scope:

- artifact hydration/detail/replay payload shapes
- options-expression artifact tagging
- deleting artifacts as a primary workflow improvement

## Current User-Facing Behavior

Artifacts across several non-options families already carry `tags`. Users can edit tags for saved runs from History and for saved Meta/Stage 5/policy-evaluation artifacts from Meta Analysis. On the backend, tag updates are case-insensitive deduplicated and written back into the JSON artifact itself.

Retention is already partly tag-aware: pruning functions keep the newest untagged artifacts while preserving older tagged artifacts. That makes tags the de facto retention-preservation signal today, even though that policy is mostly implicit.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/metaService.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/analysisStore.ts`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` tests
- `web/src/services/historyService.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`

## Strengths Already Present

- tags are already supported across multiple artifact families
- update behavior already trims empties and deduplicates case-insensitively
- retention logic already treats non-empty tags as preservation signals
- summaries and detail views already expose tags to users
- the command boundary already supports tag edits without direct frontend file mutation

## Weaknesses / Failure Modes

- retention semantics are implicit rather than documented
- freeform tags can mix user labels, workflow markers, and de facto retention controls without category guidance
- cross-family compatibility is behavioral rather than explicitly contract-driven
- casing and ordering are normalized enough for storage but not yet documented as part of compatibility
- future reserved tags could accidentally collide with existing user tag vocabulary

## Regressions To Avoid

- deleting or rewriting existing user tags during normalization changes
- making tagged artifacts newly eligible for pruning
- introducing family-specific tag behavior without documenting it
- breaking case-insensitive deduplication
- forcing users onto a new structured tag format that invalidates old artifacts
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because tags are persisted directly in JSON artifacts and must preserve predictable absent-versus-present semantics.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because deterministic ordering and stable serialization principles are useful when artifact tags influence pruning and identity-sensitive workflows.
- [What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because cross-family tag compatibility should be treated as a schema-level contract, not only UI behavior.
- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because tag edits are a backend mutation surface exposed through typed commands.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because History and Meta tag-edit flows already rely on store/service-level behavioral tests.

## Proposed Improvement Approach

1. Freeze the current additive tag contract:
   - JSON array of strings
   - trimmed
   - empty strings removed
   - case-insensitive dedupe
   - original surviving text preserved
2. Document retention semantics explicitly: non-empty tags currently protect older artifacts from pruning.
3. Introduce documented reserved-tag guidance only if it remains backward-compatible with freeform user tags.
4. Keep tag editing behavior uniform across run, meta, Stage 5, and policy-eval artifact families.
5. Expand tests around pruning and tag normalization so compatibility stays intentional.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the canonical stored tag shape and normalization rules.
- [ ] Document which artifact families must support tag editing.
- [ ] Document current retention behavior for tagged versus untagged artifacts.
- [ ] Decide whether reserved tag prefixes are needed and, if so, keep them additive only.
- [ ] Add or tighten tests for case-insensitive dedupe and empty-tag removal.
- [ ] Add or tighten tests proving tagged artifacts survive pruning.
- [ ] Add or tighten tests proving cross-family tag editing keeps the same semantics.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `run_artifacts.rs` tests
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/historyService.test.ts"` from `web/`
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/metaService.test.ts"` from `web/`
- manually verify one tagged saved run, one tagged meta artifact, and one pruning pass on untagged artifacts
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve the current freeform tag array format
- if reserved tags are introduced, they must coexist with existing user-authored tags
- do not convert tags into a structured object unless an explicit migration and compatibility layer is added
- treat retention semantics as documented policy before changing any pruning thresholds
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/06_artifacts/12_art_hydration_detail_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/22_meta_stage3_report_flow_2026-04-09.md`

