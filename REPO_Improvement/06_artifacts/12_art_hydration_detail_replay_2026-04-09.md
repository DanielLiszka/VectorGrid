# Artifact Hydration Detail Replay Split Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the split between artifact hydration, artifact detail, and family-specific replay so each path preserves its own contract and consumers do not silently depend on the wrong payload type.

## Scope Boundaries

In scope:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- typed replay/detail/hydration payload boundaries

Out of scope:

- family-specific replay semantics already covered in narrower replay docs
- raw artifact save/index logic
- options-related artifact flows

## Current User-Facing Behavior

The repo already uses three distinct artifact consumption paths:

- hydration for restoring current workflow state
- detail for history/detail inspection and rerun/open-report actions
- family-specific replay for Meta/Stage5/policy-eval result restoration

Those payloads are intentionally different sizes and shapes. Hydration is the minimal workflow restore contract, detail is a richer inspection contract, and family replay responses are typed result-restoration contracts for aggregate artifact families.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` tests
- `web/src/services/historyService.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`

## Strengths Already Present

- the repo already does not reuse one payload for every artifact consumer
- hydration, detail, and replay each have clear entry points
- workflow sync already knows how to skip redundant hydration
- history detail already knows how to consume richer detail-only fields
- family replay already uses typed responses rather than generic raw artifact JSON

## Weaknesses / Failure Modes

- the three payload families are close enough to be conflated by future changes
- field additions in one payload can be copied into another without justification
- frontend consumers may accidentally rely on detail-only fields in hydration or replay-only fields in history detail
- current distinctions are more obvious in code than in written documentation

## Regressions To Avoid

- making hydration depend on detail-only fields
- treating family replay as if it were workflow hydration
- widening detail or replay payloads without updating the consuming services and tests
- silently merging payload semantics because the raw artifact JSON happens to contain enough data
- weakening identity-based skip logic by changing the minimal hydration shape carelessly

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because hydration, detail, and replay are distinct payload schemas with different consumers.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because payload shape compatibility depends on clear absent-versus-present field semantics.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because all three paths are typed command boundaries between backend and frontend.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because each payload family restores or informs different layers of shared frontend state.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the distinction between these payload families is already enforced through stateful service/store tests.

## Proposed Improvement Approach

1. Freeze hydration, detail, and replay as three separate artifact-consumption contracts.
2. Document which consumers are allowed to use each payload type.
3. Keep minimal hydration small and purpose-built rather than expanding it toward detail.
4. Keep family replay typed and result-oriented rather than turning it into raw artifact access.
5. Expand tests that prove consumers stay on the right payload family.

## Checklist

- [ ] Document the purpose and minimum field set of hydration payloads.
- [ ] Document the purpose and richer field set of detail payloads.
- [ ] Document the purpose of family-specific replay payloads.
- [ ] Document which frontend services consume each payload type.
- [ ] Add or tighten tests proving history/detail does not rely on hydration-only behavior.
- [ ] Add or tighten tests proving workflow sync does not rely on detail-only fields.
- [ ] Add or tighten tests proving family replay remains separate from hydration.

## Verification Plan

- run targeted `run_artifacts.rs` tests
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run `npm --prefix web test -- src/services/workflowArtifactsSync.test.ts`
- run `npm --prefix web test -- src/stores/workflowArtifactsStore.test.ts`
- manually verify one history detail load, one workflow sync hydration, and one Meta-family replay flow

## Rollout / Fallback / Compatibility Notes

- preserve the split payload model rather than collapsing it
- prefer adding a new typed payload over overloading an existing one
- if a field moves between payload families, update every consumer and test in the same change
- keep options-specific payloads out of this non-options contract doc

## Related Docs

- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/05_meta_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/06_stage5_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/07_policy_eval_artifact_replay_2026-04-09.md`
