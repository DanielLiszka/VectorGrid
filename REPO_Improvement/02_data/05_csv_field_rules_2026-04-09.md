# CSV Field Rules Plan

Last updated: 2026-04-09

## Purpose

Define the current CSV field-availability rules so ingestion, data-panel messaging, and downstream optimization/validation gating remain consistent when OHLCV coverage varies.

## Scope Boundaries

In scope:

- CSV field detection and canonical field flags
- data-panel `dataFields` and `dataFieldHint` behavior
- field-driven strategy gating and capability messaging
- base versus derived dataset field preservation

Out of scope:

- new file formats
- options datasets
- full strategy-capability redesign outside field-availability concerns

## Current User-Facing Behavior

The current data pipeline already treats field availability as first-class behavior. `crates/app_core/src/csv_compat.rs` parses CSVs through the compatibility loader and marks whether open, high, low, close, and volume are actually present. `src-tauri/src/state.rs` exposes those flags as part of the loaded-data meta. `web/src/stores/dataStore.ts` turns that into `dataFields` and `dataFieldHint`, and shared helpers in `web/src/utils/core/candleFields.ts` plus request builders use those same flags to disable or warn on strategies that require missing fields.

Current user-facing behavior therefore includes:

- missing volume explicitly disables VWMA/VPWMA-style usage in hints and builders
- missing high/low disables FRAMA and other high-low-dependent paths
- missing open is surfaced rather than silently assumed
- close is treated as the essential field for meaningful backtest/validation usage

## Current Code Surface

- `crates/app_core/src/csv_compat.rs`
- `src-tauri/src/state.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/utils/core/candleFields.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `crates/app_core/src/csv_compat.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`

## Strengths Already Present

- field detection already happens at ingestion rather than being guessed later in the workflow
- field-availability state already flows through both backend meta and frontend summaries
- field-driven capability gating already exists in request builders and shared utilities
- the data panel already exposes a user-facing field hint instead of hiding limitations

## Weaknesses / Failure Modes

- field rules are split across loader code, store hints, shared utilities, and request builders
- there is no dedicated plan that says which field gaps are fatal, warning-only, or acceptable
- derived timeframes rely on correct field-preservation semantics that are not yet frozen in one document
- contributors can accidentally add strategy gating in one layer without updating the shared field-rule story

## Regressions To Avoid

- treating missing open/high/low/volume as silently available
- letting derived datasets report different field availability from their base dataset incorrectly
- changing field-driven strategy gating without updating hints and tests together
- weakening close-field requirements in a way that creates misleading downstream behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because field-rule clarity starts at the CSV compatibility target.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because loaded-data meta already behaves like a field-availability contract with optional booleans and downstream consumers.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because field hints and gating should derive from one shared meta source rather than per-panel heuristics.

## Proposed Improvement Approach

1. Document the meaning of each canonical field flag and where it originates.
2. Separate fatal requirements such as close from softer capability losses such as missing volume.
3. Document how field availability propagates through load, resample, store, request building, and validation.
4. Freeze the current data-panel hint rules and their relationship to downstream request gating.
5. Keep field-availability checks centralized through shared helpers where possible.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the canonical OHLCV field flags and how they are populated.
- [ ] Document which strategy or source families require each field group.
- [ ] Document current `dataFieldHint` messaging rules.
- [ ] Document how resampled datasets preserve or drop field flags.
- [ ] Add or tighten tests if field semantics change in loaders, stores, or request builders.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_app_core csv_compat` or targeted loader tests
- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- run targeted request-builder tests for field-gated strategies
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- prefer documenting current field rules before refactoring gating logic
- preserve the current close-first safety posture
- avoid adding panel-local field heuristics when shared helpers already exist
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/06_csv_timestamp_rules_2026-04-09.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
