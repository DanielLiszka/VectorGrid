# Optimization Runtime And Strategy Route Implementation Plan

## Surface

Primary files:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `src-tauri/src/commands_optimize.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/optimizer/src/*`

## Current Strengths

- Double MA has mature CPU, GPU-only, and GPU-kernel routes.
- Composite profiles route through established Double MA product paths where appropriate.
- Capability summaries exist in frontend utilities.
- Backend metadata captures route and fallback information.
- Optimizer support crates keep grid, stream, and multi-objective helpers separate from app orchestration.

## Improvement Plan

### OPT-01: Single Route Capability Authority Check

Risk: Low to medium.

Problem:
Route policy is described in several places: frontend capability utilities, request builders, AGENTS policy, app-core route code, and UI copy. Drift is easy.

Implementation:

- Add a generated or test-checked route capability matrix comparing frontend declared capability against Rust route policy for supported strategies.
- Start with a static JSON fixture emitted from Rust or duplicated in tests with clear ownership.
- Include strategy, frontend requested backend, `requested_backend_normalized`, `resolved_target_backend`, ADX setting, OHLCV requirements, CPU fallback, GPU-kernel eligibility, and validation support.
- Distinguish frontend request labels from Rust-resolved targets. For example, frontend `GpuOnly`/`GpuHybridKernel` capability text can normalize to `GpuKernel` while the resolved backend becomes `GpuKernel` or `GpuHybridKernel`.

Files:

- `crates/app_core/src/composite_profile_route.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/validationRequestBuilder.test.ts`

Verification:

- `cargo test -p ta_app_core composite_profile_route`
- `npm --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts src/services/optimizeRequestBuilder.test.ts src/services/validationRequestBuilder.test.ts`

### OPT-02: Optimization Request Preflight Schema

Risk: Low.

Problem:
The request builder handles many string inputs, presets, strategy-specific ranges, and backend hints. A typed preflight summary would make failures more user-friendly and testable.

Implementation:

- Add a `buildOptimizationPreflight` model with structured readiness, normalized ranges, estimated candidate count, dataset field requirements, route expectation, and blocking reasons.
- Use it in Simple Mode, Advanced Optimize, command palette primary action, and pre-run status.
- Keep backend request construction unchanged initially.

Files:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/workbenchActionModel.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/GuidedWorkspacePanel.vue`

Verification:

- `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/utils/workbenchActionModel.test.ts src/components/OptimizeWorkspacePanel.test.ts src/components/GuidedWorkspacePanel.test.ts`

### OPT-03: Optimization Route Evidence In Artifacts

Risk: Low.

Problem:
Run artifacts should consistently preserve the exact route policy, requested backend, selected backend, fallback reason, GPU device, and candidate population characteristics.

Implementation:

- Define a compact route evidence object under optimization result metadata.
- Backfill artifact summaries to show route evidence in History.
- Add report/export fields where optimization route matters to release claims.

Files:

- `crates/app_core/src/double_ma_result.rs`
- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/analysisService.ts`

Verification:

- `cargo test -p ta_app_core double_ma`
- `cargo test -p ta_desktop_demo_app --lib run_artifacts`
- `npm --prefix web run test -- src/services/historyService.test.ts src/services/analysisService.test.ts`

### OPT-04: CompositeMo Optimization Evidence And Defaults

Risk: Medium.

Problem:
CompositeMo remains CPU-bound and research-heavy. Users need clearer defaults and evidence boundaries so it is not confused with the GPU product routes.

Implementation:

- Add explicit `cpu_research_route` or equivalent metadata for CompositeMo outputs.
- Surface why GPU is unavailable and which follow-up validation/meta stages are recommended.
- Add guardrails for candidate count and search depth in Simple/Advanced UI.

Files:

- `crates/app_core/src/composite_mo.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/services/analysisService.ts`

Verification:

- `cargo test -p ta_app_core composite_mo`
- `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/services/analysisService.test.ts`

### OPT-05: Progress And Cancellation Contract Audit

Risk: Medium.

Problem:
Optimization, validation, campaigns, ML, and options each have progress/cancel paths. Users need consistent cancellation semantics: cancelled, failed, completed-with-warning, or no-op.

Implementation:

- Inventory every long-running command and its cancel token.
- Add a shared frontend status classifier for cancellation states.
- Add tests that cancellation does not persist misleading artifacts.

Files:

- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/services/runControlService.ts`
- `web/src/stores/taskStore.ts`

Verification:

- `cargo test -p ta_desktop_demo_app --lib commands_optimize`
- `npm --prefix web run test -- src/services/runControlService.test.ts src/stores/taskStore.lifecycle.test.ts`

## Acceptance Criteria For This Guide

- Frontend capability text and Rust route policy agree for every supported strategy/backend/ADX combination.
- No optimization default changes are introduced by route-metadata work.
- CPU fallback remains available for every CUDA-gated route.
- New route evidence is additive in response payloads and artifacts.
- Benchmark claims are tied to `scripts/run_cpu_perf_gate.ps1` or `scripts/run_gpu_perf_gate.ps1` evidence.

## Deferred Higher-Risk Work

- GPU CompositeMo route.
- Full dynamic strategy GPU execution beyond current route constraints.
- Strategy-local rewrite of all capability metadata.
- New optimizer algorithms without benchmark authority.
