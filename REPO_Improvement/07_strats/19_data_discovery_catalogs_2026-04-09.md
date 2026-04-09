# Strategy Data Discovery Catalogs Plan

Last updated: 2026-04-09

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

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because every discovery surface here crosses the command boundary as typed data.
- [Tauri Dialog plugin](https://v2.tauri.app/plugin/dialog/) is relevant because repo-managed discovery should stay distinct from user-selected file and folder browsing.
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because discovery catalogs should remain shared service-level truth rather than being re-normalized per component.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because the dataset-discovery side of this layer ultimately exists to surface CSV-backed strategy data sources.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because these catalogs all behave like structured compatibility contracts with optional enrichment.

## Proposed Improvement Approach

1. Treat the discovery surfaces as one coherent pre-run catalog layer with multiple specialized feeds.
2. Document the boundary between hard capability, soft telemetry, and data inventory explicitly.
3. Keep normalization and cache semantics consistent enough that the services feel like one family.
4. Preserve reuse across optimize, meta, and campaign flows rather than letting each lane rediscover data differently.
5. Keep the discovery layer additive and low-risk: better contracts and better docs before broader behavior changes.

## Checklist

- [ ] Document the shared role of dataset, MA, indicator, and speedup catalogs.
- [ ] Document the difference between hard capability, advisory telemetry, and plain inventory.
- [ ] Document cache behavior and forced-refresh expectations across the discovery services.
- [ ] Add or tighten tests when a discovery payload shape changes.
- [ ] Review cross-consumer reuse so optimize, meta, and campaign flows continue to share the same discovery truth.
- [ ] Keep repo CSV discovery and user-picked dialog flows distinct in docs and tests.

## Verification Plan

- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/services/movingAverageCatalogService.test.ts`
- run `npm --prefix web test -- src/services/dynamicIndicatorCatalogService.test.ts`
- run `npm --prefix web test -- src/services/gpuSpeedupsService.test.ts`
- run `npm --prefix web test -- src/services/metaService.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current service and command boundaries while clarifying their shared role
- prefer additive payload fields over repurposing existing ones
- keep telemetry advisory and keep inventories bounded
- if a discovery surface is promoted for wider reuse later, update all known consumers in the same change

## Related Docs

- `REPO_Improvement/02_data/01_repo_csv_inventory_2026-04-09.md`
- `REPO_Improvement/02_data/02_repo_csv_discovery_2026-04-09.md`
- `REPO_Improvement/03_opt/29_ma_catalog_registry_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
