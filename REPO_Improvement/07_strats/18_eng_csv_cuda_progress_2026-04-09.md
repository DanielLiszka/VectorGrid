# CSV And CUDA Progress Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for long-running dataset-preparation, CSV-backed campaign preparation, and CUDA-staging progress so the repo surfaces truthful progress without breaking the current command flow.

## Scope Boundaries

In scope:

- campaign dataset preparation progress
- optimization progress events reused by campaigns
- CSV-backed preload and VRAM-prefetch feedback
- frontend task-center and runtime-status presentation for these phases

Out of scope:

- inventing a brand-new task system
- options-related progress surfaces
- changing the core optimization algorithms themselves

## Current User-Facing Behavior

The repo already has meaningful progress infrastructure, but it is uneven across phases. Current behavior includes:

- `double_ma_progress` events emitted from optimize commands
- `research_campaign_progress` events emitted from campaign orchestration
- task-center aggregation in `web/src/stores/taskStore.ts`
- dataset preparation and VRAM-prefetch phases inside `src-tauri/src/commands_campaign.rs`

At the same time, plain CSV loading through `load_price_data` remains a direct command with no incremental progress stream, and some CUDA-prefetch outcomes are surfaced as notes or stage labels rather than a unified progress model.

## Current Code Surface

- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_data.rs`
- `crates/app_core/src/progress.rs`
- `web/src/stores/taskStore.ts`
- `web/src/services/metaService.ts`
- `web/src/services/optimizeService.ts`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `web/src/stores/taskStoreCampaignProgress.test.ts`
- `web/src/stores/taskStoreEventThrottle.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/optimizeService.test.ts`
- targeted backend tests in `src-tauri/src/commands_campaign.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`

## Strengths Already Present

- the repo already has real progress events rather than spinner-only behavior
- campaign orchestration already distinguishes dataset stages and optimization stages
- the task center already tracks phases, progress percentages, event timelines, and toast/status behavior
- VRAM prefetch feedback already exists and is structured enough to reuse
- optimize and campaign surfaces already share part of the same progress vocabulary

## Weaknesses / Failure Modes

- CSV loading and campaign dataset preparation do not currently feel like one coherent progress story
- some important CUDA-prefetch outcomes are represented as notes instead of consistently typed progress states
- progress semantics are split across optimize commands, campaign commands, and frontend task aggregation
- throttling, phase naming, and percentages can drift across surfaces
- current behavior can over-report activity in some places and under-report it in others

## Regressions To Avoid

- breaking current `double_ma_progress` or `research_campaign_progress` consumers
- inventing fake precision for progress where only stage-level status is really known
- making task-center progress more visually active but less truthful
- changing command payload shapes without synchronized store/service test updates
- slowing down hot paths significantly just to emit richer progress metadata
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because heavy work and its typed results already cross the command boundary here.
- [Tauri event API](https://v2.tauri.app/es/reference/javascript/api/namespaceevent/) is relevant because the existing progress model depends on event emission and listener stability.
- [WAI-ARIA `progressbar` guidance](https://www.w3.org/TR/2010/WD-wai-aria-20100916/complete) is relevant because long-running progress surfaces should communicate determinate versus indeterminate state accurately.
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because progress should remain centralized in shared stores rather than fragmented across components.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because progress payloads already act like shared contracts between backend and frontend layers.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat CSV preparation, dataset preload, optimization, and CUDA staging as one staged progress story with distinct phase labels.
2. Keep percentages conservative and truthful; use stage transitions where exact percentages are not really known.
3. Document and freeze the progress payload semantics before broadening them.
4. Make CUDA-prefetch outcomes visible as typed progress states or typed diagnostics rather than ad hoc notes only.
5. Keep the existing task-center aggregation and event-throttling behavior as the central presentation layer.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current progress event types and their responsibilities.
- [ ] Document which phases are determinate, semi-determinate, or purely stage-based.
- [ ] Document how CSV preparation and CUDA-prefetch feedback should be surfaced together.
- [ ] Add or tighten tests for progress throttling, stage transitions, and campaign progress aggregation.
- [ ] Review whether `load_price_data` needs additive progress hooks or only clearer status messaging.
- [ ] Keep accessibility expectations explicit for any progress UI copy or widgets.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `npm --prefix web run test -- src/stores/taskStoreCampaignProgress.test.ts`
- run `npm --prefix web run test -- src/stores/taskStoreEventThrottle.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- manually inspect one campaign with dataset preload enabled and one without preload
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve existing event names unless a compatibility migration is documented
- prefer additive payload fields over changing existing ones
- use clearer stage/status copy before adding high-frequency progress churn
- if a phase is fundamentally indeterminate, keep it stage-based instead of fabricating precise percentages
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/07_strats/15_dynamic_ir_campaign_surface_2026-04-09.md`
- `REPO_Improvement/02_data/01_repo_csv_inventory_2026-04-09.md`
- `REPO_Improvement/01_shell/26_runtime_task_store_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
