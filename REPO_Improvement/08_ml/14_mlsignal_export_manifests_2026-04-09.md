## Purpose

Define the improvement plan for ML Signal export-manifest artifacts so dataset exports, split manifests, and related sidecars remain explicit, discoverable, and contract-stable.

## Scope Boundaries

In scope:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- export manifest and split manifest generation, persistence, and presentation

Out of scope:

- model-bundle manifests under the ML confidence model directory
- generic run artifact manifests unrelated to ML Signal dataset export
- options-related manifest surfaces

## Current User-Facing Behavior

The ML Signal export path already produces more than a CSV:

- export manifest
- optional split manifest
- feature schema sidecar
- summary display of the generated artifact paths

That is a strong provenance story, but only if the manifest family is treated as a first-class contract instead of a set of loosely related side files.

## Current Code Surface

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- ML Signal unit tests in `crates/app_core/src/ml_signal.rs`
- `MlSignalWorkspacePanel.test.ts`
- `mlSignalService.test.ts`
- saved-run detail coverage in `run_artifacts.rs`

## Strengths Already Present

- manifests are already named artifacts, not hidden implementation details
- the frontend already carries manifest paths in the ML artifact payload
- the workspace summary already exposes manifest-related information
- saved-run detail can preserve related ML attachment paths

## Weaknesses / Failure Modes

- the repo has several ML-adjacent manifest concepts, which can cause naming confusion
- manifest creation, attachment, and display are split across backend, service, and artifact layers
- it is easy to show a manifest path without stating what the manifest guarantees
- optional split-manifest behavior can be hard to distinguish from always-generated export manifest behavior

## Regressions To Avoid

- changing manifest naming or meaning without synchronized UI/service/artifact updates
- letting split-manifest semantics blur with export-manifest semantics
- losing manifest paths during saved-run reopen or artifact persistence
- reducing manifest visibility while adding more ML sidecar complexity

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the manifests should be treated as versioned contract artifacts rather than incidental logs.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because manifest payloads move through JSON-based storage and artifact attachment layers.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because manifest paths and metadata cross the frontend/backend boundary in export responses.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because the manifests describe a CSV-backed ML dataset export.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because manifest-path state is presented across the workspace and saved-run reopen flows.

## Proposed Improvement Approach

1. Treat export-manifest artifacts as a distinct ML sidecar family with explicit roles.
2. Document which sidecars are always generated and which are conditional.
3. Keep manifest naming, attachment, and display consistent across:
   - fresh export
   - score-cache generation follow-ons
   - saved-run reopen
4. Add tests for manifest-path attachment and display.
5. Prefer clearer manifest provenance over more condensed summary text.

## Checklist

- [ ] Document the export manifest and split manifest roles clearly.
- [ ] Add or tighten tests for manifest generation and path attachment.
- [ ] Audit frontend summary text so users know which manifest they are looking at.
- [ ] Confirm saved-run reopen preserves relevant manifest paths.
- [ ] Keep manifest naming/version semantics stable unless a deliberate migration exists.

## Verification Plan

- run `crates/app_core/src/ml_signal.rs` tests covering export-manifest and split-manifest output
- run `MlSignalWorkspacePanel.test.ts`
- run `mlSignalService.test.ts`
- run relevant saved-artifact tests if manifest attachment behavior changes

## Rollout / Fallback / Compatibility Notes

- preserve current manifest file roles and naming unless there is a strong compatibility plan
- keep manifest visibility explicit in the workspace and saved-run detail
- if a cleanup obscures manifest provenance, restore the clearer path-by-path presentation

## Related Docs

- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/08_ml/12_mlsignal_feature_schema_2026-04-09.md`
- `REPO_Improvement/08_ml/13_mlsignal_split_plan_2026-04-09.md`
- `REPO_Improvement/06_artifacts/23_mlsignal_report_path_semantics_2026-04-09.md`
