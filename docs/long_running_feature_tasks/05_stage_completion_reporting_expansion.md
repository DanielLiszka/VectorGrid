# Stage Completion Reporting Expansion

Status: proposed
Risk: low to moderate
Primary value: make saved artifacts explain workflow progress and completion consistently

## Summary

Stage completion packets already exist for optimization, validation, and ML Signal. This task extends that contract to Meta, Stage 5/policy evaluation, options expression validation, and research campaign checkpoints so saved artifacts can explain what completed, what failed, and what can be resumed.

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

- [ ] Add stage ids for `meta_selection`, `stage5`, `policy_evaluation`, `options_validation`, and `campaign`.
- [ ] Define required and optional fields for each stage.
- [ ] Add normalization tests for unknown future stages.
- [ ] Add serialization tests for new packet fields.
- [ ] Document version and compatibility behavior.

### Phase 2: Meta And Stage 5

- [ ] Build Meta stage completion packets from live results.
- [ ] Backfill saved Meta artifacts in `run_artifacts.rs`.
- [ ] Build Stage 5 completion packets from saved Stage 5 artifacts.
- [ ] Keep policy evaluation separate from Stage 5.
- [ ] Add UI display in saved artifact review surfaces.

### Phase 3: Options Expression

- [ ] Add options optimization completion summary where absent.
- [ ] Add options validation completion packets for saved follow-up validation artifacts.
- [ ] Include dataset provenance, fill/price semantics, validation mode, and verdict where available.
- [ ] Preserve options artifact replay and reopen behavior.

### Phase 4: Research Campaigns

- [ ] Add campaign checkpoint completion packets from campaign summaries.
- [ ] Include execution mode, preload policy, counts, evidence state, resume state, and checkpoint path.
- [ ] Surface resumable vs restart-only status.
- [ ] Add tests for legacy degraded and blocked resume states.

## Validation Checklist

- [ ] `npm --prefix web run test -- src/utils/core/stageCompletion.test.ts src/stores/workflowArtifactsStore.test.ts`
- [ ] `cargo test -p ta_desktop_demo_app run_artifacts`
- [ ] Add targeted tests for Meta, Stage 5, options validation, and campaign backfill.
- [ ] Run saved artifact hydration tests.
- [ ] Run repo quality gate for broad frontend/backend artifact changes.

## Regression Watchpoints

- Existing artifacts without stage completions must still load.
- Existing stage completion packet shapes must remain compatible.
- Artifact repair must not remove unrelated fields.
- Stage names must not collapse Meta, Stage 5, and policy evaluation into one ambiguous bucket.

## Definition Of Done

- Saved artifacts can show coherent stage completion history across optimization, validation, ML Signal, Meta, Stage 5, options, and campaign flows.
- Legacy artifacts hydrate and backfill safely.
- Tests cover normalization and backend repair/backfill.

