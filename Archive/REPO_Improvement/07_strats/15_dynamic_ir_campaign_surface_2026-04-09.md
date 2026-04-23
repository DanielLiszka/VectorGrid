# Dynamic IR Campaign Surface Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for the research-campaign surface that batches optimization/validation runs, optional auto-meta/stage5 follow-ups, and campaign progress/reporting across strategy families.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_campaign.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- campaign request, progress, summary, and resume/retry behavior

Out of scope:

- Stage 3/4/5 scoring semantics beyond what the campaign surface orchestrates
- single-run optimize/validate commands outside the campaign path
- options-related campaigns

## Current User-Facing Behavior

The repo already supports a research-campaign surface that can:

- batch datasets
- choose execution mode (`full`, `optimize_only`, `validate_only`)
- apply validation configuration
- optionally auto-run meta and stage5 follow-ups
- control preload policy
- persist a campaign summary artifact
- resume incomplete campaigns from the saved summary path
- report progress and per-item diagnostics

This is already a significant orchestration surface, not just a thin loop around optimize commands.

## Current Code Surface

- `src-tauri/src/commands_campaign.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/stores/taskStore.ts`

## Current Verification Surface

- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- targeted backend tests in `src-tauri/src/commands_campaign.rs`

## Strengths Already Present

- campaign request and summary shapes are already explicit
- per-item diagnostics and summary artifacts already exist
- resume/retry behavior is already modeled, not left to manual reruns
- progress payloads already carry stage and counts
- strategy passthrough behavior already has direct frontend test coverage

## Weaknesses / Failure Modes

- the campaign surface orchestrates many other systems, so failures can be hard to localize
- latest-state, persisted-summary, and resumed-state behavior can drift
- per-item diagnostics are rich enough to regress without obvious UI breakage
- strategy passthrough and preload policy semantics can change quietly
- auto-meta/stage5 chaining increases coupling across multiple artifact families

## Regressions To Avoid

- breaking resume-from-summary behavior
- changing strategy passthrough semantics without test updates
- weakening per-item diagnostics or progress payload clarity
- breaking summary artifact persistence or artifact-library visibility
- making auto-meta/stage5 chaining more implicit or less inspectable
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Calling Rust from the frontend | Tauri](https://v2.tauri.app/develop/calling-rust/) is relevant because the campaign surface is a large typed command boundary with progress and result payloads.
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because campaign orchestration depends on coordinated latest-run, saved-summary, and progress state.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because campaign strategy routing and resume behavior are best protected at the service/store layer.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the campaign request, progress payload, and summary artifact are structured contracts.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because campaign summaries and downstream chaining rely on structured JSON-like request/result payloads.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat the campaign surface as an orchestration product surface with its own contract.
2. Freeze the request, progress, and summary payload responsibilities separately.
3. Keep resume/retry behavior and latest-run behavior explicitly aligned.
4. Expand tests around chaining, progress, and summary diagnostics.
5. Keep strategy passthrough and preload policy behavior visible and deterministic.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the campaign request contract, including execution mode and preload policy.
- [x] Document the progress payload and summary artifact responsibilities.
- [x] Document resume/retry semantics from a saved summary artifact.
- [x] Add or tighten backend tests for summary persistence and per-item diagnostics.
- [x] Add or tighten frontend tests for strategy passthrough, preload policy, and resume behavior.
- [x] Add or tighten progress-store tests for campaign stage/count updates.
- [x] Keep auto-meta/stage5 chaining explicit in docs and verification.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `src-tauri/src/commands_campaign.rs`
- run `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignResume.test.ts`
- run `npm --prefix web run test -- src/stores/taskStoreCampaignProgress.test.ts`
- manually verify one small campaign, then resume from its saved summary
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve saved summary artifacts as the source of truth for resume behavior
- keep strategy passthrough and preload policy explicit rather than inferred
- if campaign summaries change shape, update resume logic and tests in the same slice
- keep campaign orchestration additive and observable, not opaque
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by publishing the research-campaign contract in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md): request ownership, progress-payload ownership, saved-summary authority, and the explicitly supported strategy surface are now documented as one orchestration truth anchor instead of being scattered across the batch pane and backend code.
- Existing frontend campaign coverage remains the acceptance backbone and was re-verified:
  - strategy passthrough, preload policy, execution mode, and auto-meta/stage5 chaining in `metaServiceCampaignStrategy.test.ts`
  - saved-summary resume/retry/reveal behavior in `metaServiceCampaignResume.test.ts`
  - task/progress-stage counts in `taskStoreCampaignProgress.test.ts`
- Existing backend campaign coverage was re-verified with the current command test matrix in `src-tauri/src/commands_campaign.rs`, including:
  - supported strategy acceptance
  - strategy-family routing
  - validation-backend normalization
  - CompositeMo frontier ordering and shortlist diversity rules
- Stable truth anchors:
  - `web/src/services/metaService.ts`
  - `src-tauri/src/commands_campaign.rs`
  - `web/src/services/metaServiceCampaignStrategy.test.ts`
  - `web/src/services/metaServiceCampaignResume.test.ts`
  - `web/src/stores/taskStoreCampaignProgress.test.ts`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts src/services/metaServiceCampaignResume.test.ts src/stores/taskStoreCampaignProgress.test.ts`
  - `cargo test -p ta_desktop_demo_app campaign_ --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/27_meta_batch_registry_modes_2026-04-08.md`
- `REPO_Improvement/08_ml/28_meta_batch_resume_retry_2026-04-08.md`
- `REPO_Improvement/06_artifacts/22_meta_stage3_report_flow_2026-04-09.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`

