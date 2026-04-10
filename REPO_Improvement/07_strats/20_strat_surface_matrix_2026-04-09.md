# Strategy Surface Matrix Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for the repo's overall non-options strategy surface matrix: which strategy kinds exist, what capabilities they advertise, where they are supported, and how that matrix is reflected across optimize, advanced controls, and meta/campaign surfaces.

## Scope Boundaries

In scope:

- strategy capability summaries and route-preview messaging
- optimize and advanced-panel gating driven by strategy kind
- strategy availability across optimize, validation, and campaign/meta surfaces
- the cross-layer matrix that maps a strategy kind to supported controls and workflows

Out of scope:

- options-related surfaces
- implementation details of any one strategy family beyond their matrix implications
- new strategy invention

## Current User-Facing Behavior

The repo already has a real strategy matrix even if it is not named that way in the code. Current behavior is spread across:

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/services/metaService.ts`

That matrix already controls:

- whether a strategy uses the global backend selector
- whether it uses the global mode selector
- whether heatmaps, top-k, include-all, export-all, and advanced shortlist controls apply
- how composite route previews are described
- whether a strategy can participate in campaign/meta flows

## Current Code Surface

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaService.ts`

## Current Verification Surface

- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/components/OptimizeWorkspacePanel.test.ts`
- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/metaRequestBuilder.test.ts`

## Strengths Already Present

- the repo already has explicit capability-summary code rather than leaving all gating implicit in components
- composite route-preview behavior is already centralized instead of duplicated in copy strings everywhere
- optimize and advanced panels already consume the same capability summary
- meta/campaign restrictions such as the current `DynamicDoubleMa` exclusion are already surfaced intentionally
- the matrix already spans more than just backend support; it includes control and workflow availability

## Weaknesses / Failure Modes

- the matrix is spread across several files and therefore easy to drift
- strategy kinds can be supported in one surface and rejected in another if the matrix is not explicitly maintained
- capability copy can overstate route guarantees if it drifts from backend truth
- lower-traffic strategy kinds can regress quietly because the matrix is mostly exercised through common strategies
- the matrix is conceptually central, but it does not yet have its own dedicated implementation doc

## Regressions To Avoid

- showing controls that the request builder or backend will later reject
- hiding supported controls because one layer's capability matrix lagged behind another
- changing strategy availability in campaigns or meta flows without synchronized messaging and tests
- drifting composite route previews away from actual backend route policy
- letting one-off component logic bypass the shared matrix
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because a strategy matrix should remain centralized shared logic rather than fragmenting across components.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because matrix-driven gating is best protected with service/store/component contract tests.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because strategy-surface promises eventually become typed backend requests and rejections.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the matrix effectively governs which request/result contract variants are valid for which strategies.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because matrix drift often shows up first as visible control and copy mismatches in real UI flows.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat the strategy surface matrix as a first-class contract across optimize, advanced, and meta/campaign flows.
2. Freeze the authority order: backend truth and request-builder legality first, then matrix copy and UI gating.
3. Keep every strategy kind mapped explicitly rather than relying on scattered special cases.
4. Add targeted tests for lower-traffic strategy kinds and cross-surface mismatches.
5. Keep matrix changes additive, synchronized, and evidence-backed.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every current non-options strategy kind and its major workflow availability.
- [x] Document which layer owns hard legality versus explanatory copy.
- [x] Document cross-surface differences such as campaign/meta exclusions.
- [x] Add or tighten tests for strategy-specific advanced-control visibility and request legality.
- [x] Add or tighten tests for matrix reuse between Optimize, Advanced, and Meta surfaces.
- [x] Review whether any component still bypasses the shared matrix with local special cases.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `npm --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current strategy ids and user-facing strategy labels
- update matrix-driven copy and gating in the same change as any legality change
- prefer central capability updates over component-local conditionals
- if a strategy remains unsupported in a surface, keep the restriction explicit and user-visible
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by expanding the strategy-surface matrix truth anchors in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md):
  - documented the authority order between backend/request-builder legality, capability summaries, and component copy
  - named the central owners: `strategyOptimizationCapabilities.ts`, `compositeProfileGpuPolicy.ts`, `OptimizeWorkspacePanel.vue`, `AdvancedWorkspacePanel.vue`, and the Meta campaign request/service layer
  - documented the current campaign/meta exclusions and route-preview responsibilities explicitly
- Tightened matrix coverage with a new `MlSignal` capability test in [strategyOptimizationCapabilities.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/utils/strategyOptimizationCapabilities.test.ts):
  - pins that `MlSignal` stays outside optimize-only controls
  - pins its dedicated workflow copy as dataset-export / score-cache / validation artifacts rather than optimizer heatmaps or export-all grids
- Reused the existing cross-surface proofs rather than widening scope:
  - optimize/advanced matrix rendering stays covered by `OptimizeWorkspacePanel.test.ts` and `AdvancedWorkspacePanel.test.ts`
  - campaign/meta strategy legality stays covered by `metaServiceCampaignStrategy.test.ts` and the `campaign_` Rust test set in `commands_campaign.rs`
  - no component-local bypasses were introduced
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts src/components/OptimizeWorkspacePanel.test.ts src/components/AdvancedWorkspacePanel.test.ts src/services/metaServiceCampaignStrategy.test.ts`
  - `cargo test -p ta_desktop_demo_app campaign_ --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/07_strats/02_strat_dynamic_double_ma_2026-04-09.md`
- `REPO_Improvement/07_strats/06_strat_composite_cpu_2026-04-09.md`
- `REPO_Improvement/07_strats/07_strat_composite_mo_2026-04-09.md`
- `REPO_Improvement/07_strats/08_strat_ml_signal_2026-04-08.md`
