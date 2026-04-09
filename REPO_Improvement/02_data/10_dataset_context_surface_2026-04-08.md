# Dataset Context Surface Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for the frontend dataset-context surface so the active dataset, timeframe, field availability, source path, and restore state are shown consistently across the shell and dependent workflows.

## Scope Boundaries

In scope:

- `web/src/stores/dataStore.ts`
- user-visible dataset context fields
- dataset readiness and shell-context display
- startup restore cues and base/current dataset surface semantics

Out of scope:

- low-level CSV parsing
- low-level resample execution internals
- options datasets

## Current User-Facing Behavior

The data store already exposes a rich dataset-context surface. Current user-facing fields include:

- `dataId`
- `datasetLabel`
- `datasetRange`
- `dataLen`
- `dataTimeframe`
- `dataTimeframeHint`
- `datasetContext`
- `dataFields`
- `dataFieldHint`
- `csvPath`
- `baseDataId`
- `baseDataMeta`
- `baseCsvPath`
- `baseDatasetLabel`
- `baseDatasetContext`

The shell and dependent panels already use those fields to determine whether data is loaded, what timeframe is active, and whether derived timeframe context or field limitations should be shown. The store also carries startup-restore posture and recent-path information.

## Current Code Surface

- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/src/utils/core/datasetMeta.ts`

## Current Verification Surface

- data-store action tests
- `web/src/services/validationService.test.ts`
- `web/src/services/optimizeService.test.ts`
- shell/component tests that consume dataset readiness or context

## Strengths Already Present

- the store already separates current dataset state from base dataset state
- timeframe hints and field hints already exist
- dataset context already includes source/load-path flavor text instead of only raw paths
- startup restore and recent-path behavior are already represented in state
- optimization and validation services already consume dataset context for artifact and compatibility logic

## Weaknesses / Failure Modes

- dataset-context fields are rich enough to drift in meaning if not documented explicitly
- the difference between base and current/derived dataset state is easy to blur
- startup restore cues can become misleading if not kept aligned with actual load state
- field-hint and context text can diverge from actual capability gating if they are treated as cosmetic copy

## Regressions To Avoid

- showing a derived/current timeframe as if it were the base dataset
- losing field-availability hints that currently explain disabled capabilities
- breaking startup-restore or recent-path context cues
- letting shell context and service-level artifact context disagree

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because dataset context is shared across shell, optimize, and validation surfaces
- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because dataset-context fields are an explicit shared contract, not incidental locals
- [PROV-Overview](https://www.w3.org/TR/prov-overview/)
  - relevant because dataset context is really a user-facing provenance summary
- [Window.localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because startup snapshot and related dataset preferences are persisted client-side

## Proposed Improvement Approach

1. Define dataset context as a UI contract with explicit field families:
   - identity
   - timeframe
   - range/length
   - source/load path
   - field availability
   - startup restore
2. Document the difference between base dataset fields and current active dataset fields.
3. Treat `datasetContext`, `datasetRange`, and `dataFieldHint` as authoritative explanatory surfaces, not disposable copy.
4. Align shell-visible dataset context with artifact/meta generation so users are never shown conflicting dataset identity.
5. Add tests around startup restore, base/current timeframe divergence, and field-hint updates.

## Checklist

- [ ] Document the full dataset-context field inventory and meanings.
- [ ] Document base versus current dataset surface semantics.
- [ ] Document startup-restore and recent-path behavior.
- [ ] Document field-availability and timeframe-hint behavior.
- [ ] Add tests for shell-visible dataset context after load, clear, and timeframe change.
- [ ] Add tests for startup restore and base/current context transitions.
- [ ] Add tests proving artifact/meta builders stay aligned with visible dataset context.

## Verification Plan

- run relevant data-store tests
- run `npm test -- src/services/optimizeService.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run shell/component tests that consume dataset readiness
- manually verify load, clear, startup restore, and timeframe change flows in the app shell

## Rollout / Fallback / Compatibility Notes

- preserve current dataset-context field names unless a deliberate migration is written
- prefer additive clarifications over changing user-visible context semantics abruptly
- keep the base/current distinction explicit if the data surface becomes richer later

## Related Docs

- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
- `REPO_Improvement/02_data/26_derived_dataset_identity_2026-04-08.md`
- `REPO_Improvement/02_data/03_csv_load_contract_2026-04-08.md`

