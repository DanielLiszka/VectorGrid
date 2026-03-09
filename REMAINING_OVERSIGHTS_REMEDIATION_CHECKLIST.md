# Remaining Oversights Remediation Checklist

Last updated: March 8, 2026

## Scope

This document tracks the remaining product and technical oversights identified after the recent frontend/backend cleanup work.

It is intended to be the working checklist for follow-up remediation.

## Decision Notes

- Short borrow / financing / locate cost modeling is currently treated as an accepted non-goal unless product requirements change.
- Full support for `Sortino` and `Omega` remains a desired outcome.
- Validation must remain strict: it should only run against an optimization artifact for the same strategy, same asset/dataset, and same timeframe.
- `GpuKernel` is treated as a unified GPU-preferred route, not a strict GPU-or-fail switch. It may route to non-kernel GPU or CPU fallback paths, but those routes must be decided cleanly and surfaced explicitly.

## Findings

### 1. Backend validation context is enforced only in the frontend

Severity: High

Problem:
- The Vue frontend blocks validation when the currently loaded dataset/timeframe/strategy does not match the optimization artifact.
- The backend validation commands do not enforce the same rule.
- Direct Tauri/headless callers can still bypass the guard and validate mismatched runs.

Relevant files:
- [validationService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/validationService.ts#L555)
- [validationService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/validationService.ts#L1038)
- [commands_validation.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_validation.rs#L173)
- [commands_validation.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_validation.rs#L270)

Remediation checklist:
- [x] Add backend-side artifact-context validation for strategy, dataset identity, and timeframe.
- [x] Ensure Tauri validation commands reject mismatched artifact/data combinations.
- [x] Keep the frontend preflight, but treat it as UX help rather than the only enforcement layer.
- [x] Add backend tests that prove mismatched validation requests fail.

### 2. Optimization UI still exposes controls that are ignored or silently overridden for some strategies

Severity: High

Problem:
- The global Optimize panel always shows `Mode`, `Backend`, and downstream analysis settings even when the selected strategy path does not honor them.
- Composite CPU-like requests are forced to CPU / Grid / no heatmap in the request builder.
- Composite MO ignores the global backend mode entirely and sends `mode: null`.
- This makes the UI look more capable than the underlying request path really is.

Relevant files:
- [OptimizeWorkspacePanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue#L789)
- [OptimizeWorkspacePanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue#L813)
- [AdvancedWorkspacePanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/AdvancedWorkspacePanel.vue#L504)
- [optimizeRequestBuilder.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optimizeRequestBuilder.ts#L1300)
- [optimizeRequestBuilder.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optimizeRequestBuilder.ts#L1485)
- [optimizeRequestBuilder.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optimizeRequestBuilder.ts#L1488)
- [optimizeRequestBuilder.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optimizeRequestBuilder.ts#L1618)
- [optimizeRequestBuilder.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optimizeRequestBuilder.ts#L1619)

Remediation checklist:
- [x] Make Optimize-panel controls strategy-aware.
- [x] Hide or disable backend/mode controls when the selected strategy cannot use them.
- [x] Hide or relabel heatmap/top-k controls when the selected strategy path does not produce those outputs.
- [x] Add UI tests for strategy-specific control visibility and request mapping.

### 3. GPU-kernel preflight is incomplete for `Sortino`, `Omega`, and some kernel-only limits

Severity: Medium

Problem:
- The UI exposes `Sortino` and `Omega`.
- Double-MA backend routing can still choose the kernel path before all kernel-only restrictions are checked.
- The kernel then rejects unsupported objectives or excessive `top_k` later.
- This causes late failures instead of clean preflight rejection or fallback.

Relevant files:
- [OptimizeWorkspacePanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue#L774)
- [OptimizeWorkspacePanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue#L781)
- [OptimizeWorkspacePanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue#L783)
- [double_ma.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs#L3059)
- [double_ma.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs#L3122)
- [gpu_backtest_kernel.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs#L3651)
- [gpu_backtest_kernel.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs#L3658)

Remediation checklist:
- [x] Expand backend preflight so kernel-only objective and `top_k` restrictions are checked before kernel route selection.
- [x] Decide routing policy for unsupported kernel objectives:
- [x] Keep unified-route fallback behavior for `GpuKernel`, but decide it before entering the kernel path.
- [x] Add tests covering the new kernel preflight blockers and existing route behavior.
- [x] Preserve or improve full support for `Sortino` and `Omega`, even if the implementation route is CPU or non-kernel GPU.

### 4. Explicit GPU requests still downgrade to CPU in some backend paths

Severity: Medium

Problem:
- Some paths treat a user-requested GPU backend as “GPU if possible, else CPU”.
- That is observable through route metadata, but it is weaker than a strict explicit-backend contract.

Relevant files:
- [composite_cpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/composite_cpu.rs#L343)
- [composite_cpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/composite_cpu.rs#L365)
- [double_ma.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs#L3152)
- [double_ma.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs#L3164)

Remediation checklist:
- [x] Decide explicit GPU policy.
- [x] Keep `GpuKernel` as an explicitly named unified route with surfaced fallback metadata, rather than a hidden implicit downgrade.
- [x] Keep `Auto` as the broadest route-selection mode, while preserving explicit-route metadata for `GpuKernel`.
- [x] Add tests/assertions for the chosen contract and route blockers.

### 5. Automatic CUDA device selection is heuristic-only and does not fail closed

Severity: Medium

Problem:
- The app now picks one preferred CUDA device automatically.
- Device choice is currently “highest VRAM wins”.
- If preferred-device lookup fails, some backend paths still normalize to device `0`.
- The UI can show a device-selection error, but execution does not necessarily stop on that condition.

Relevant files:
- [cuda_device.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/cuda_device.rs#L18)
- [cuda_device.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/cuda_device.rs#L25)
- [double_ma.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs#L57)
- [double_ma.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs#L58)
- [composite_mo.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/composite_mo.rs#L846)
- [commands_system.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_system.rs#L28)

Remediation checklist:
- [x] Decide whether preferred-device resolution failure should abort GPU execution.
- [x] Remove implicit `device_id = 0` fallback when it would mask selection failures.
- [x] Add tests for preferred-device resolution error propagation.
- [x] Keep the current VRAM-first heuristic for now.

### 6. Short-enabled backtests do not appear to model borrow / financing / locate costs

Severity: Medium

Problem:
- Short strategies can look better than live reality if stock borrow fees, financing costs, or locate friction are not modeled.
- The code clearly supports generic trading cost fields, but I did not find active short-specific cost fields.

Relevant files:
- [mod.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/strategies/src/double_ma/mod.rs#L257)
- [mod.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/strategies/src/double_ma/mod.rs#L263)
- [mod.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/strategies/src/double_ma/mod.rs#L347)
- [executionRisk.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/utils/executionRisk.ts#L37)

Status:
- Accepted non-goal for now unless product requirements change.

Checklist:
- [x] Explicitly record that short borrow / financing / locate modeling is out of scope for the current product.
- [ ] Revisit only if live-trading realism becomes a priority.

### 7. Post-meta ML remains embedded inside Meta rather than surfaced as a distinct final stage

Severity: Low

Problem:
- The repo design calls for a distinct post-meta Stage 5 finalizer surface.
- The implementation exists, but the workflow is still folded into Meta.
- This hides the intended progression and makes the feature feel less first-class.

Relevant files:
- [STAGE5_POST_META_ML_RESEARCH_AND_IMPLEMENTATION.md](/C:/Rust%20Projects/ta_desktop_demo/research/STAGE5_POST_META_ML_RESEARCH_AND_IMPLEMENTATION.md#L187)
- [MetaWorkspacePanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue#L716)
- [metaService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/metaService.ts#L2283)

Remediation checklist:
- [x] Decide whether to keep ML/Stage 4 embedded in Meta or promote it to an explicit final workflow stage.
- [x] Promote it to an explicit workflow finalizer step while keeping the existing Meta panel implementation.
- [x] Keep the existing backend implementation; this is mainly a workflow/presentation decision.

### 8. CLI still exposes stale same-bar execution plumbing

Severity: Low

Problem:
- The CLI still accepts `trade_on_next_bar`.
- The backend now rejects `false`, so the flag is effectively stale surface area.

Relevant files:
- [ta_cli.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/ta_cli.rs#L125)
- [ta_cli.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/ta_cli.rs#L254)

Remediation checklist:
- [x] Remove stale `trade_on_next_bar` CLI surface or hard-lock it to `true`.
- [x] Update any CLI docs/help text accordingly.

## Execution Order

Recommended order of implementation:

1. [x] Backend validation-context enforcement
2. [x] Strategy-aware Optimize-panel control gating
3. [x] GPU-kernel preflight cleanup for `Sortino` / `Omega` / kernel-only limits
4. [x] Explicit-GPU contract cleanup
5. [x] CUDA preferred-device failure behavior
6. [x] CLI stale-surface cleanup
7. [x] Stage 5 workflow surfacing

## Completion Criteria

- [x] Validation cannot be run against mismatched artifact/data context from either frontend or direct backend command paths.
- [x] The Optimize UI never shows important controls that are silently ignored for the current strategy path.
- [x] `Sortino` and `Omega` are supported cleanly, with a clear route policy and no confusing late kernel failure.
- [x] Explicit GPU requests have a deliberate, tested contract.
- [x] Single-device CUDA selection is deterministic and does not silently mask device-resolution failures.
