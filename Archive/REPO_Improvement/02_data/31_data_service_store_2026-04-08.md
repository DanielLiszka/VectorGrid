# Data Service And Store Plan

Last updated: 2026-04-09

## Purpose

Define the current frontend data-layer split so `dataService` remains the Tauri/normalization boundary and `dataStore` remains the workflow state owner for loaded data, derived data, Polygon cache state, startup restore, and clear/reset flows.

## Scope Boundaries

In scope:

- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- data startup restore and deferred hydration
- Polygon cache/list/download/top-off/validate/repair flows

Out of scope:

- options datasets
- Rust-side data cache internals
- brand-new data providers

## Current User-Facing Behavior

The data layer already behaves like a two-part contract. `dataService` normalizes Tauri responses, enforces required-path checks, and keeps browse/sample/open/close helpers thin and stable. `dataStore` owns the real workflow: load/browse/sample actions, base-vs-derived dataset state, startup snapshot restore, deferred hydration, Polygon cache visibility, exact-range top-off behavior, moved-root rewriting for Polygon datasets, and workflow-artifact clearing on reset. The UI already depends on that split even though it is not written down explicitly.

## Current Code Surface

- `web/src/services/dataService.ts`
  - normalization of load/cache/Polygon payloads and filesystem/command wrappers
- `web/src/services/dataService.test.ts`
  - service-level normalization and command-wrapper coverage
- `web/src/stores/dataStore.ts`
  - frontend state owner for loaded data, derived data, startup restore, and Polygon flows
- `web/src/stores/dataStore.actions.test.ts`
  - store-level coverage for startup restore, Polygon flows, root rewriting, and clear-data behavior
- `web/src/components/DataWorkspacePanel.vue`
  - consumes current data-store state and action methods
- `web/src/utils/core/datasetMeta.ts`
  - artifact-oriented dataset metadata helper fed by data-store state

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/components/DataWorkspacePanel.test.ts`

## Strengths Already Present

- service normalization already happens at the boundary instead of leaking `unknown` payloads into components
- store state already preserves base-vs-derived identity and not just a single active dataset
- startup restore is already bounded and deferred rather than eager and destructive
- Polygon top-off behavior already preserves the active derived timeframe when it should
- clear-data flows already clear workflow-artifact state instead of leaving stale analysis attached

## Weaknesses / Failure Modes

- `dataStore.ts` is large enough that ownership boundaries are easy to lose
- several state updates reuse similar result-application logic across load/download/top-off flows
- `setControlsBusy()` style capability toggling is explicit but verbose and easy to drift
- `openPolygon()` and `closePolygon()` are intentionally thin, which makes their contract easy to underestimate
- market-data-root rewriting logic is correct but specialized enough to be fragile if changed casually

## Regressions To Avoid

- making browse/sample flows throw instead of remaining user-safe
- collapsing base and derived dataset state into one opaque active row
- eagerly scanning or restoring Polygon state during startup before the store is ready
- breaking exact-range top-off behavior or derived-timeframe preservation
- rewriting non-Polygon paths when market-data root changes
- leaving workflow artifacts intact after data reset
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because `dataStore` is already a domain state boundary rather than component-local state
- [Pinia: Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because most of the data workflow is already action-driven
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because `dataService` is a command-wrapper boundary
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV remains a real external dataset contract in the non-options workflow
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because dataset identity/provenance uses filesystem metadata downstream

## Proposed Improvement Approach

1. Make the service/store split explicit:
   - `dataService` owns normalization and command wrappers
   - `dataStore` owns workflow state and orchestration
2. Break the store into documented subdomains before any code refactor:
   - startup restore
   - base/derived dataset session state
   - Polygon cache and dataset selection
   - market-data-root management
3. Consolidate repeated result-application logic only when current behavior is preserved exactly.
4. Treat workflow-artifact clearing and dataset identity as non-negotiable parts of the data contract.
5. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current responsibility split between `dataService` and `dataStore`.
- [x] Document startup restore and deferred hydration behavior.
- [x] Document base vs derived dataset fields and status semantics.
- [x] Document Polygon cache/list/download/top-off/repair/validate flows.
- [x] Document market-data-root rewrite behavior for cached Polygon datasets.
- [x] Document clear-data side effects on workflow artifacts.
- [x] Add or tighten tests if store/service boundaries or result-application logic change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts`
- run `npm --prefix web run test -- src/components/DataWorkspacePanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current state semantics before attempting any structural split of `dataStore.ts`
- keep service normalization backward-compatible with current Tauri payload shapes
- if no-op service helpers are removed or formalized, update every store caller in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- No production-path code change was required in this slice. The service/store boundary the plan calls for is already present: `dataService.ts` remains the Tauri command and normalization edge, while `dataStore.ts` owns dataset session state, startup hydration, Polygon workflow orchestration, root rewriting, and reset behavior.
- Reused the previously hardened artifact-clear fallback instead of widening the store surface. `clearData()` now clears workflow artifacts through the current Pinia instance or the runtime Pinia fallback, and the store test proves stale run/validation state is dropped when data is reset.
- Reconfirmed that the intentionally thin service hooks stay honest rather than implicit. `dataService.test.ts` already freezes `openPolygon()` and `closePolygon()` as non-throwing no-op Vue workflow hooks with no Tauri traffic, which is the right public-facing contract for that boundary.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/components/DataWorkspacePanel.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/02_data/26_derived_dataset_identity_2026-04-08.md`
