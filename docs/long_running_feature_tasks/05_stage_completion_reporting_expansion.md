# Stage Completion Reporting Expansion

Status: implemented for frontend packet generation and normalization
Risk: low to moderate
Primary value: make saved artifacts explain workflow progress and completion consistently

## Summary

Stage completion packets already exist for optimization, validation, and ML Signal. This task extends that contract to Meta, Stage 5/policy evaluation, options expression validation, and research campaign checkpoints so saved artifacts can explain what completed, what failed, and what can be resumed.

Implemented slice:

- Added stage ids and builders for Meta, Stage 5 ML confidence, policy evaluation, options expression validation, and campaign checkpoints.
- Added normalization/serialization tests for the expanded packet surface.
- Wired Meta and options services to emit the new packets where frontend artifacts are assembled.
- Preserved optional/legacy behavior; backend run-artifact repair remains a separate future hardening step.

## Current Anchors

- Frontend stage completion utilities: `web/src/utils/core/stageCompletion.ts`
- Workflow artifact store: `web/src/stores/workflowArtifactsStore.ts`
- Run artifact backfill: `src-tauri/src/run_artifacts.rs`
- Meta/Stage 5 surfaces: `web/src/services/metaService.ts`, `web/src/components/MetaWorkspacePanel.vue`
- Options surfaces: `web/src/services/optionsExpressionService.ts`, `web/src/components/OptionsExpressionPanel.vue`
- Campaign runner: `src-tauri/src/commands_campaign.rs`

## Additive Boundary

- New completion packets must be optional.
- Backfill must tolerate legacy artifacts.
- Do not alter run outcomes while adding reporting.
- Keep user-facing Stage 5 naming while preserving backend compatibility aliases.

## Implementation Checklist

### Phase 1: Contract Extension

- [x] Add stage ids for `meta_selection`, `stage5`, `policy_evaluation`, `options_validation`, and `campaign`.
- [x] Define required and optional fields for each stage.
- [x] Add normalization tests for unknown future stages.
- [x] Add serialization tests for new packet fields.
- [x] Document version and compatibility behavior through tests and stage packet contracts.

### Phase 2: Meta And Stage 5

- [x] Build Meta stage completion packets from live results.
- [ ] Backfill saved Meta artifacts in `run_artifacts.rs`.
- [x] Build Stage 5 completion packets from saved Stage 5 artifacts.
- [x] Keep policy evaluation separate from Stage 5.
- [x] Add packet data to saved artifact review surfaces where the frontend assembles artifacts.

### Phase 3: Options Expression

- [x] Add options optimization completion summary where absent.
- [x] Add options validation completion packets for saved follow-up validation artifacts.
- [x] Include dataset provenance, fill/price semantics, validation mode, and verdict where available.
- [x] Preserve options artifact replay and reopen behavior.

### Phase 4: Research Campaigns

- [x] Add campaign checkpoint completion packet builder for campaign summaries.
- [x] Include execution mode, preload policy, counts, evidence state, resume state, and checkpoint path in the packet contract.
- [x] Surface resumable vs restart-only status in packet fields.
- [ ] Add backend artifact-repair tests for legacy degraded and blocked resume states.

## Validation Checklist

- [x] `npm --prefix web run test -- stageCompletion optionsExpressionService metaService`
- [ ] `cargo test -p ta_desktop_demo_app run_artifacts`
- [x] Add targeted tests for Meta, Stage 5, options validation, and campaign packet builders.
- [x] Run saved artifact hydration-adjacent frontend tests for packet normalization.
- [x] Run repo quality gate for broad frontend/backend artifact changes.

## Regression Watchpoints

- Existing artifacts without stage completions must still load.
- Existing stage completion packet shapes must remain compatible.
- Artifact repair must not remove unrelated fields.
- Stage names must not collapse Meta, Stage 5, and policy evaluation into one ambiguous bucket.

## Definition Of Done

- Saved artifacts can show coherent stage completion history across optimization, validation, ML Signal, Meta, Stage 5, options, and campaign flows.
- Legacy artifacts hydrate and backfill safely.
- Tests cover normalization and backend repair/backfill.
