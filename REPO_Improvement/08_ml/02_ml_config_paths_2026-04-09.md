# ML Config Paths Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for ML Signal configuration and artifact-path construction so dataset context, score-cache paths, validation settings, and persisted artifact metadata stay coherent across the frontend and Tauri command boundary.

## Scope Boundaries

In scope:

- `web/src/services/mlSignalService.ts`
- `web/src/utils/core/datasetMeta.ts`
- `web/src/utils/core/artifactIdentity.ts`
- related optimize/strategy store fields consumed by ML Signal
- `src-tauri/src/commands_ml_signal.rs`

Out of scope:

- training algorithm internals
- non-ML optimization request builders
- options-related configuration paths

## Current User-Facing Behavior

ML Signal already constructs a substantial amount of configuration and path-bearing state:

- dataset export paths
- score input/cache paths
- model bundle id
- feature schema and manifest paths
- validation context and policy-sweep parameters
- saved run artifact payloads carrying ML metadata

Users experience this as “the ML workspace remembers enough context to export, score, validate, and reopen a run.” The risk is that this context is built in several layers rather than from one canonical config contract.

## Current Code Surface

- `web/src/services/mlSignalService.ts`
- `web/src/utils/core/datasetMeta.ts`
- `web/src/utils/core/artifactIdentity.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/strategyStore.ts`
- `src-tauri/src/commands_ml_signal.rs`

Key current ownership:

- `mlSignalService.ts` builds strategy config, validation context, dataset context state, and ML artifact payloads
- `datasetMeta.ts` and `artifactIdentity.ts` help normalize persisted dataset/artifact identity
- `commands_ml_signal.rs` exposes the command surface for dataset export, score-cache generation, validation, policy sweep, and model status

## Current Verification Surface

- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/services/historyService.test.ts` where ML artifacts are reopened
- any ML-signal command tests or payload-shape tests in Rust and TS

## Strengths Already Present

- ML Signal config assembly is already centralized in one frontend service rather than scattered across components
- the service already produces structured artifact payloads instead of raw anonymous blobs
- dataset context and artifact identity helpers are already reused
- Tauri command names already map clearly onto ML Signal tasks

## Weaknesses / Failure Modes

- `mlSignalService.ts` currently owns many different config-construction responsibilities
- path-bearing fields and strategy/validation settings can drift if not validated as a coherent contract
- frontend state and saved artifact state can diverge in naming or fallback behavior
- configuration intent is embedded in service logic more than in a documented schema

## Regressions To Avoid

- changing ML artifact-path field semantics without matching saved-run and reopen behavior
- letting dataset context and config fingerprint logic diverge across export, validation, and reopen flows
- adding more path-bearing state to the service without making the config contract clearer
- breaking the existing Rust command surface while renaming frontend fields casually
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because ML Signal config and artifact payloads already behave like a structured schema-bearing contract.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because Rust/TS exchange several ML payloads as flexible JSON values that still need stable meaning.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because ML Signal config paths and validation/policy-sweep requests cross the command boundary constantly.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because ML configuration is assembled from shared runtime stores rather than one local component.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because exported ML datasets remain CSV-anchored artifacts and should keep source/destination path semantics understandable.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Treat ML Signal config assembly as a formal request-and-artifact contract.
2. Split the conceptual inputs into explicit groups:
   - dataset context
   - model/bundle context
   - validation context
   - artifact path payloads
3. Audit field naming and fallback behavior across:
   - frontend store state
   - artifact payload persistence
   - Rust command requests/responses
4. Add targeted request-shape tests so ML-specific path fields cannot drift silently.
5. Keep the first improvement wave focused on clarity and validation rather than adding more ML fields.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the ML Signal config contract assembled by `mlSignalService.ts`.
- [ ] Separate dataset, model, validation, and artifact-path concerns in the documentation and tests.
- [ ] Add or tighten tests for ML artifact payload construction and persistence.
- [ ] Audit naming/fallback consistency between frontend payloads and `commands_ml_signal.rs`.
- [ ] Decide whether some current `mlSignalService.ts` helpers should become shared schema normalizers instead of ad hoc builders.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `MlSignalWorkspacePanel.test.ts`
- run relevant ML reopen tests in `historyService.test.ts`
- run targeted command/request tests around `commands_ml_signal.rs` if config fields change
- manually verify a bounded export -> score-cache -> validation roundtrip if path semantics change materially
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- keep existing field names stable until tests clearly pin the contract
- prefer extracting shared normalizers over broad service rewrites
- if a config cleanup weakens saved-run reopen behavior, keep the current explicit field set and document it better first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
- `REPO_Improvement/06_artifacts/30_core_dataset_meta_paths_2026-04-09.md`
