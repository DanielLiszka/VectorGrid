# Strategy Data Discovery Catalogs Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for the strategy-facing discovery catalog layer spanning dataset discovery, moving-average catalogs, dynamic-indicator catalogs, and GPU speedup telemetry.

## Scope Boundaries

In scope:

- repo and sample CSV discovery used by strategy workflows
- moving-average registry export
- dynamic-indicator registry export
- GPU speedup telemetry export
- shared frontend normalization and caching of these discovery surfaces

Out of scope:

- runtime execution after a request is built
- options-related data or catalog surfaces
- product features that do not consume discovery catalogs

## Current User-Facing Behavior

The repo already exposes a meaningful discovery layer before optimization begins. Users and higher-level workflows can already discover:

- sample CSVs
- repo CSV inventory
- Polygon cache datasets
- moving-average capability metadata
- dynamic-indicator descriptors
- GPU speedup telemetry

These are exposed through separate commands and services, but they all feed the same early-stage workflow decision space: what data exists, what indicators exist, what parameter shapes exist, and what GPU hints are worth showing.

## Current Code Surface

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/ma_registry.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `web/src/services/dataService.ts`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `web/src/services/gpuSpeedupsService.ts`
- `web/src/services/metaService.ts`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/services/movingAverageCatalogService.test.ts`
- `web/src/services/dynamicIndicatorCatalogService.test.ts`
- `web/src/services/gpuSpeedupsService.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- module-local tests in `crates/app_core/src/indicator_catalog.rs`

## Strengths Already Present

- discovery surfaces are already backend-owned rather than inferred in components
- the repo already distinguishes data discovery from indicator discovery and from performance-hint telemetry
- the frontend already normalizes and caches several of these catalogs in dedicated services
- meta/research flows already reuse repo CSV and Polygon dataset discovery instead of maintaining a second hidden inventory
- the catalogs already carry enough metadata to support request building, UX hinting, and higher-level orchestration

## Weaknesses / Failure Modes

- the discovery surfaces are related in practice but documented separately, which makes cross-surface drift easy
- caching behavior is not unified across all discovery services
- some consumers may treat advisory telemetry like hard capability truth
- data discovery and indicator discovery can become inconsistent in naming or freshness expectations
- repo CSV inventory and strategy catalog services live in different folders and can look unrelated to new contributors

## Regressions To Avoid

- breaking existing field names or normalization behavior across the catalog services
- collapsing advisory telemetry into hard legality or capability rules
- letting meta/research consumers diverge from the main strategy workflow's discovery surfaces
- silently changing cache behavior in one discovery service while others stay different
- broadening discovery scope without explicit performance and truncation rules
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because every discovery surface here crosses the command boundary as typed data.
- [Tauri Dialog plugin](https://v2.tauri.app/plugin/dialog/) is relevant because repo-managed discovery should stay distinct from user-selected file and folder browsing.
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because discovery catalogs should remain shared service-level truth rather than being re-normalized per component.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because the dataset-discovery side of this layer ultimately exists to surface CSV-backed strategy data sources.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because these catalogs all behave like structured compatibility contracts with optional enrichment.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat the discovery surfaces as one coherent pre-run catalog layer with multiple specialized feeds.
2. Document the boundary between hard capability, soft telemetry, and data inventory explicitly.
3. Keep normalization and cache semantics consistent enough that the services feel like one family.
4. Preserve reuse across optimize, meta, and campaign flows rather than letting each lane rediscover data differently.
5. Keep the discovery layer additive and low-risk: better contracts and better docs before broader behavior changes.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the shared role of dataset, MA, indicator, and speedup catalogs.
- [x] Document the difference between hard capability, advisory telemetry, and plain inventory.
- [x] Document cache behavior and forced-refresh expectations across the discovery services.
- [x] Add or tighten tests when a discovery payload shape changes.
- [x] Review cross-consumer reuse so optimize, meta, and campaign flows continue to share the same discovery truth.
- [x] Keep repo CSV discovery and user-picked dialog flows distinct in docs and tests.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/services/movingAverageCatalogService.test.ts`
- run `npm --prefix web run test -- src/services/dynamicIndicatorCatalogService.test.ts`
- run `npm --prefix web run test -- src/services/gpuSpeedupsService.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current service and command boundaries while clarifying their shared role
- prefer additive payload fields over repurposing existing ones
- keep telemetry advisory and keep inventories bounded
- if a discovery surface is promoted for wider reuse later, update all known consumers in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by expanding the discovery-catalog routing section in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md):
  - separated plain inventory, hard capability, and advisory telemetry responsibilities
  - named the backend/frontend owners for repo-data inventory, moving-average export, dynamic-indicator export, and GPU speedup telemetry
  - documented the cross-consumer test surfaces that now act as the discovery truth anchors
- Tightened backend discovery coverage in [commands_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_data.rs):
  - added a parser test for quoted/escaped CSV cells used by the GPU-speedup loader
  - added a workspace-table smoke test for `load_gpu_speedups_csv()` so root lookup plus finite parsing stay pinned
- Tightened Vue/meta discovery coverage in [metaService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/metaService.test.ts):
  - `batchRefresh()` now proves both Polygon and repo-data inventories render together
  - the test also pins the current visible distinction between `stocks (adj)` and `local` batch entries
- Kept scope intentionally additive:
  - no catalog payloads were widened
  - no cache invalidation rules were changed
  - repo-data discovery and user-picked file dialogs remain distinct
- Acceptance evidence:
  - `cargo test -p ta_desktop_demo_app commands_data --lib`
  - `npm.cmd --prefix web run test -- src/services/metaService.test.ts src/services/dataService.test.ts src/services/movingAverageCatalogService.test.ts src/services/dynamicIndicatorCatalogService.test.ts src/services/gpuSpeedupsService.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/02_data/01_repo_csv_inventory_2026-04-09.md`
- `REPO_Improvement/02_data/02_repo_csv_discovery_2026-04-09.md`
- `REPO_Improvement/03_opt/29_ma_catalog_registry_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
