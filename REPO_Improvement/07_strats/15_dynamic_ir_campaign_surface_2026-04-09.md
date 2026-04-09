# Dynamic IR Campaign Surface Plan

Last updated: 2026-04-09

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

## External Research Summary

- [Calling Rust from the frontend | Tauri](https://v2.tauri.app/develop/calling-rust/) is relevant because the campaign surface is a large typed command boundary with progress and result payloads.
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because campaign orchestration depends on coordinated latest-run, saved-summary, and progress state.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because campaign strategy routing and resume behavior are best protected at the service/store layer.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the campaign request, progress payload, and summary artifact are structured contracts.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because campaign summaries and downstream chaining rely on structured JSON-like request/result payloads.

## Proposed Improvement Approach

1. Treat the campaign surface as an orchestration product surface with its own contract.
2. Freeze the request, progress, and summary payload responsibilities separately.
3. Keep resume/retry behavior and latest-run behavior explicitly aligned.
4. Expand tests around chaining, progress, and summary diagnostics.
5. Keep strategy passthrough and preload policy behavior visible and deterministic.

## Checklist

- [ ] Document the campaign request contract, including execution mode and preload policy.
- [ ] Document the progress payload and summary artifact responsibilities.
- [ ] Document resume/retry semantics from a saved summary artifact.
- [ ] Add or tighten backend tests for summary persistence and per-item diagnostics.
- [ ] Add or tighten frontend tests for strategy passthrough, preload policy, and resume behavior.
- [ ] Add or tighten progress-store tests for campaign stage/count updates.
- [ ] Keep auto-meta/stage5 chaining explicit in docs and verification.

## Verification Plan

- run targeted tests in `src-tauri/src/commands_campaign.rs`
- run `npm --prefix web test -- src/services/metaServiceCampaignStrategy.test.ts`
- run `npm --prefix web test -- src/services/metaServiceCampaignResume.test.ts`
- run `npm --prefix web test -- src/stores/taskStoreCampaignProgress.test.ts`
- manually verify one small campaign, then resume from its saved summary

## Rollout / Fallback / Compatibility Notes

- preserve saved summary artifacts as the source of truth for resume behavior
- keep strategy passthrough and preload policy explicit rather than inferred
- if campaign summaries change shape, update resume logic and tests in the same slice
- keep campaign orchestration additive and observable, not opaque

## Related Docs

- `REPO_Improvement/08_ml/27_meta_batch_registry_modes_2026-04-08.md`
- `REPO_Improvement/08_ml/28_meta_batch_resume_retry_2026-04-08.md`
- `REPO_Improvement/06_artifacts/22_meta_stage3_report_flow_2026-04-09.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`

