# Stage5 Artifact Replay Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to saved Stage 5 artifact replay so typed replay loading, alias debt, selection retention, and action-surface constraints remain explicit and stable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- saved Stage 5 replay/load/reveal/delete flows

Out of scope:

- Saved Meta report-open behavior
- policy-eval replay
- options-related artifact replay

## Current User-Facing Behavior

Saved Stage 5 supports refresh, load, reveal, and delete, but intentionally does not expose `Open report`. Loading a saved Stage 5 artifact re-renders the Stage 5 result surface inside the Meta workspace and refreshes the saved family list while preserving selection intent.

There is also a naming/alias debt: the frontend still uses some `stage4Saved*` state names even though the visible family label is `Saved Stage 5`. That debt is survivable today, but it needs to be documented rather than left implicit.

## Current Code Surface

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.test.ts`

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `src-tauri/src/run_artifacts.rs` replay/list tests

## Strengths Already Present

- Stage 5 replay already uses a typed backend response
- the current action surface is explicit and intentionally narrower than Saved Meta
- saved-list refresh and selection logic already exist
- family-specific labels and empty states are already explicit in the Meta workspace
- tests already cover typed replay loading and the absence of report-open behavior

## Weaknesses / Failure Modes

- Stage 4/Stage 5 alias debt is easy to worsen if not documented
- the replay response currently mirrors the same file path into both Stage 4 and Stage 5 fields, which is subtle
- future UI work could accidentally add report expectations to a family that does not currently have a real report artifact
- refresh and selection retention can drift if identity rules are not frozen

## Regressions To Avoid

- breaking the current stage4/stage5 alias compatibility without a coordinated cleanup
- assuming separate Stage 4 and Stage 5 artifact files when the current replay contract does not promise that
- accidentally adding or implying report-open support where no real Stage 5 report artifact exists
- clearing the selected Stage 5 artifact on refresh
- making Stage 5 replay depend on raw artifact structure instead of typed replay output

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because the replay response is acting as a typed contract between backend and Meta UI.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because Stage 5 replay state depends on stable JSON result semantics.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because saved Stage 5 replay is a command-mediated state restoration path.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because Stage 5 replay restores shared Meta state and family selection state.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because current guarantees around replay and selection retention are stateful and test-driven.

## Proposed Improvement Approach

1. Freeze Saved Stage 5 replay as a typed restoration contract with a deliberately narrower action surface than Saved Meta.
2. Document the Stage 4/Stage 5 alias debt and how it is tolerated today.
3. Keep `no report-open action` as an explicit product rule until a real Stage 5 report artifact exists.
4. Preserve typed replay ownership and selected-item retention through refresh.
5. Expand tests around alias compatibility, replay loading, and action-surface constraints.

## Checklist

- [ ] Document the typed Stage 5 replay response and how the frontend consumes it.
- [ ] Document the current Stage 4/Stage 5 naming alias debt.
- [ ] Document why Saved Stage 5 does not currently expose `Open report`.
- [ ] Document selection-retention behavior on refresh.
- [ ] Add or tighten tests for typed replay loading and alias compatibility.
- [ ] Add or tighten tests proving report-open remains intentionally unavailable for this family.
- [ ] Keep replay semantics separate from artifact-library indexing concerns.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run targeted `run_artifacts.rs` Stage 5 replay/list tests
- manually verify refresh, load, reveal, and delete flows inside Saved Stage 5

## Rollout / Fallback / Compatibility Notes

- preserve current alias compatibility until there is a deliberate cleanup plan
- do not invent report-open semantics for Stage 5 without a real backend artifact
- prefer additive documentation and tests over renaming state casually
- if alias cleanup happens later, migrate tests and UI labels in one coordinated change

## Related Docs

- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/08_ml/31_stage5_result_surface_2026-04-08.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/07_policy_eval_artifact_replay_2026-04-09.md`
