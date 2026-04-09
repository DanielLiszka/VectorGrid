# Batch Summary Artifacts Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for batch-summary artifact handling so legacy batch summaries and the current campaign-summary flow are clearly separated, reloadable, and operationally understandable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- relevant `metaService` tests around batch/campaign summary state

Out of scope:

- redesigning meta analysis itself
- campaign execution internals beyond how summaries are saved and reloaded
- options-related meta artifacts

## Current User-Facing Behavior

Users can currently resume or reveal the latest loaded batch/campaign summary from the meta workspace, but the surface is transitional:

- the frontend stores only the last summary path
- summary reload goes through generic artifact loading rather than a dedicated summary library
- current research-campaign flows produce `vectorgrid_campaign_summary`
- backend still carries legacy `vectorgrid_batch`/`vectorgrid_batch_summary` support

The result is usable, but not cleanly explained.

## Current Code Surface

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`

Key current ownership:

- `run_artifacts.rs` still provides `save_batch_summary_blocking()` and a `vectorgrid_batch_<ts>.json` suggestion path
- `commands_artifacts.rs` exposes `save_batch_summary`
- `metaService.ts` persists and reloads only the latest batch/campaign summary path in local storage
- `commands_meta.rs` accepts both legacy batch-summary and newer campaign-summary kinds during reload

## Current Verification Surface

- `metaService.test.ts`
- `metaServiceCampaignResume.test.ts`
- `metaServiceCampaignStrategy.test.ts`
- any backend tests around `run_artifacts.rs` summary save/load behavior

## Strengths Already Present

- legacy summary compatibility has not been abandoned
- the frontend can resume from a persisted last-summary path
- current campaign-summary kinds are already accepted alongside older batch-summary kinds
- reveal/resume behaviors already exist instead of requiring manual filesystem spelunking

## Weaknesses / Failure Modes

- this surface looks partially stale and partially current at the same time
- there is no dedicated summary library or discovery surface, only “last summary” recovery
- generic artifact loading masks the distinction between batch summaries and campaign summaries
- old batch-summary path generation still exists without a clearly primary producer in the current flow

## Regressions To Avoid

- breaking compatibility for existing saved summary files while tightening the current flow
- removing the “last summary” recovery path before a stronger replacement exists
- treating legacy batch summaries and current campaign summaries as identical if their semantics differ
- confusing users about whether a summary is resumable, revealable, or merely inspectable
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because batch/campaign summaries are structured artifacts and should not remain vaguely typed forever.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because summary payloads are currently handled as flexible JSON values across the Rust/TS boundary.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because summary save/load/reveal behavior relies on stable command payloads and responses.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the current UI remembers and resumes only the last summary path through shared client state and storage.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the batch/campaign summary surface is stateful, transitional, and easy to regress without targeted tests.

## Proposed Improvement Approach

1. Explicitly separate legacy batch-summary support from the current campaign-summary path.
2. Document whether `save_batch_summary_blocking()` is still an intended active path or legacy compatibility plumbing.
3. Keep the current “last summary path” recovery, but make its limitations explicit.
4. Add clearer type and kind checks so batch and campaign summaries can be handled deliberately.
5. Decide whether a future summary library should replace “last summary only,” but do not force that change in this first wave.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the difference between legacy batch summaries and current campaign summaries.
- [ ] Audit whether `save_batch_summary_blocking()` is still actively used or purely compatibility-oriented.
- [ ] Add or tighten tests for last-summary persistence and reload behavior.
- [ ] Add or tighten tests for kind discrimination across `vectorbt_batch_summary`, `vectorgrid_batch_summary`, and `vectorgrid_campaign_summary`.
- [ ] Decide whether summary reveal/resume flows need clearer UI wording about what is actually being resumed.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- run any targeted backend tests around `save_batch_summary_blocking()` and summary-kind loading
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve legacy summary compatibility while clarifying the preferred current path
- keep the last-summary path mechanism until a stronger library/index exists
- if cleanup risks breaking older saved summaries, favor explicit compatibility branches over removal
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
- `REPO_Improvement/06_artifacts/22_meta_stage3_report_flow_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
