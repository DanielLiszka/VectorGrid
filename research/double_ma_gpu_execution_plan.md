
# Double-MA optimization backend: comprehensive GPU-first implementation plan

## Scope

This plan is scoped to the files in the review bundle:

- `gpt54_feature_review_map.md` (`## 1. Double MA optimization backend`)
- `double_ma.rs`
- `double_ma_backend.rs`
- `double_ma_mode.rs`
- `double_ma_result.rs`
- `double_ma_strategy.rs`
- `commands_optimize.rs`

The goal of this plan is to move the Double-MA optimization path toward a **fully GPU-executed**, **VRAM-resident where sensible**, **non-silent-fallback** architecture, while also covering the practical code edits needed in the attached files.

I split the work into:

1. **Direct edits in the attached files** — these are the main body of this document.
2. **Adjacent runtime/kernel dependencies** — these are clearly called out where the attached files need a hook into code that is not in this bundle.

---

## Executive summary

There are six high-value changes that should happen first:

1. **Stop forcing `Grid` in the Tauri command layer**  
   `commands_optimize.rs:58-60` currently rewrites every request mode to `Grid`, which makes `Auto`, `CoarseToFine`, and `GpuVdps` unreachable from the product path.

2. **Introduce an explicit GPU execution policy**  
   The request type should distinguish:
   - `BestEffort`
   - `PreferGpuResident`
   - `StrictGpuResident`

   That prevents “GPU” requests from silently resolving to `GpuOnly` or CPU paths when the actual execution is not end-to-end device-resident.

3. **Refactor `double_ma.rs` around a run-scoped context and caches**  
   Hoist invariant work out of each MA-param run:
   - log returns
   - base `first_valid`
   - `KaufmanStopSeries`
   - host regime filter
   - specialized device regime filters

4. **Make hybrid routing residency-aware**  
   In `optimize_double_ma_gpu_hybrid_kernel_phase1`, treat “CPU-preferred by `gpu_speedups.csv`” differently from “kernel unsupported”.  
   If an indicator is kernel-capable, `PreferGpuResident` / `StrictGpuResident` should skip CPU staging and stay on the kernel/device path.

5. **Fix PIT as the biggest remaining device-builder outlier**  
   `build_pressure_impulse_trend_regime_device` still stages multiple indicators to host and re-uploads them.  
   EFI/BOP/VOSC/CFO should be moved to `DeviceF32` directly, with ACOSC remaining gated until parity is acceptable.

6. **Add follow-up reuse hooks**  
   Drilldown and local-stability currently recompute everything from scratch on CPU.  
   Add a follow-up cache token and backend-aware matrix generation so `optimize -> drilldown -> local stability` can reuse already-prepared state.

If I were sequencing this into PRs, I would do:

- **PR1**: product mode unblocking + request/result telemetry
- **PR2**: run-scoped caches + static eval context hoisting
- **PR3**: hybrid residency policy + strict GPU enforcement
- **PR4**: PIT cleanup + specialized device-regime cache
- **PR5**: follow-up cache + backend-aware drilldown/local stability
- **PR6**: `GpuVdps` controller abstraction (host implementation first, device implementation second)

---

## Implementation tracker

Use this checklist to track only the parts of this guide that are still valid against the current repo state.

### Validity notes

- As of `2026-03-19`, the guide is still directionally correct for product-path mode handling, GPU-residency policy, hybrid residency behavior, PIT device-native staging, follow-up reuse, and `GpuVdps` cleanup.
- Do **not** treat this guide as claiming that GPU validation does not exist. Fixed-candidate GPU validation already exists in the current repo; remaining work is about fallback transparency, route policy, and throughput.
- The original first item was incomplete if read literally. Product-path mode unblocking required both:
  - Tauri command-layer normalization
  - frontend request shaping / mode preservation
- Legacy removed aliases such as `GpuProgressive` / `GpuProgressiveSearch` should remain deprecated compatibility inputs, not become first-class modes again.

### Checklist

- [x] PR1.A: Stop forcing `Grid` for Double-MA requests at the Tauri command boundary when exact-grid outputs are not requested.
- [x] PR1.B: Keep Composite CPU grid-only at the product boundary.
- [x] PR1.C: Preserve `Auto`, `CoarseToFine`, and `GpuVdps` through frontend Double-MA request shaping when exact-grid outputs are not requested.
- [x] PR1.D: Preserve official non-grid mode values in Vue store mode state / options instead of silently normalizing them back to `Grid`.
- [x] PR1.E: Prevent recommended frontend output defaults from silently re-enabling grid heatmap defaults for non-grid modes.
- [x] PR1.F: Update product-path tests for Tauri normalization and frontend request shaping.
- [x] PR1.G: Update stale frontend capability text that claimed the Double-MA family was exact-grid-only.

- [x] PR2.A: Replace the current loose `double_ma_mode.rs` mode-selection inputs with a route-aware context struct.
- [x] PR2.B: Make `Auto` route-aware instead of only choosing `Grid` vs `CoarseToFine` from output shape and combo count.
- [x] PR2.C: Add explicit GPU execution policy types to the request/result path (`BestEffort`, `PreferGpuResident`, `StrictGpuResident`).
- [x] PR2.D: Add request/result telemetry fields that make residency / downgrade outcomes explicit.

- [x] PR3.A: Refactor `double_ma.rs` around a run-scoped static evaluation context.
- [x] PR3.B: Hoist invariant work out of each MA-param run (`log_returns`, `first_valid`, `KaufmanStopSeries`, shared regime inputs).
- [x] PR3.C: Add run-scoped regime caches for host and device-prepared regime data.
- [x] PR3.D: Enforce GPU execution policy during backend resolution instead of allowing silent downgrade success paths.
- [x] PR3.E: Treat `GpuOnly` as a downgrade / fallback outcome rather than a success case for GPU-resident execution goals.

- [x] PR4.A: Make hybrid phase-1 routing residency-aware.
- [x] PR4.B: Distinguish "CPU-preferred by heuristic" from "kernel unsupported" in hybrid routing.
- [x] PR4.C: Keep kernel-capable indicators on device when `PreferGpuResident` / `StrictGpuResident` is requested.
- [x] PR4.D: Surface hybrid fallback reasons clearly in route metadata.

- [x] PR5.A: Move PIT device builders toward device-native construction where parity allows.
- [x] PR5.B: Keep EFI/BOP/VOSC/CFO on `DeviceF32` instead of staging to host and re-uploading.
- [x] PR5.C: Gate ACOSC separately until parity / stability is acceptable.
- [x] PR5.D: Add or update PIT parity coverage for any device-native migration.

- [x] PR6.A: Add follow-up cache tokens / handles to optimization results.
- [x] PR6.B: Reuse prepared state in drilldown instead of recomputing everything from scratch on CPU.
- [x] PR6.C: Reuse prepared state in local-stability evaluation where valid.
- [x] PR6.D: Make follow-up execution backend-aware, including non-grid and GPU-backed runs.

- [x] PR7.A: Introduce a cleaner `GpuVdps` controller abstraction in `double_ma.rs`.
- [x] PR7.B: Separate frontier/controller logic from the underlying evaluation backend.
- [x] PR7.C: Land host-side controller cleanup first.
- [x] PR7.D: Land device-side controller / frontier management only after host abstraction and parity coverage are stable.

- [x] Verification.A: Expand unit/integration coverage for route policy, downgrade transparency, and result metadata.
- [x] Verification.B: Add or update benchmarks that measure warm product-path behavior rather than only isolated microbenchmarks.
- [x] Verification.C: Run CPU perf gate before handoff for performance-sensitive executor changes.
- [x] Verification.D: Run GPU perf gate before handoff for CUDA / backtest-kernel executor changes when GPU is available.

Performance gate note: `2026-03-19` local candidate runs completed. The final CPU and GPU perf gates both pass after refreshing stale CPU workflow baselines and fixing two CPU-bench compile issues uncovered during gate execution (`double_ma_opt.rs` non-CUDA import gating and `dynamic_strategy_cpu.rs` stale request literals).

- Warm follow-up benchmark coverage now exists in `crates/app_core/benches/double_ma_followup_workflow.rs` for cached vs cold `optimize -> drilldown -> local stability` product-path behavior on CPU and GPU-enabled builds.
- CPU workflow regression gate: passed at `runtime_ms=1721` vs refreshed workflow baseline `1744`
- CPU aggregate perf gate workflow budget: passed at `workflow_runtime_ms=1721` vs CPU perf baseline `1721`
- CPU criterion gate: passed at `criterion_mean_ms=124.35` vs baseline `125.31`
- CPU dynamic strategy ratio gate: passed at `1.074238` vs baseline `1.070352`
- GPU kernel perf gate: passed at `criterion_mean_ms=76.63` vs baseline `155.29`, with dynamic strategy ratio `1.03`

### Suggested status labels

When updating this checklist during implementation, prefer one of:

- `done`: implemented and verified locally
- `partial`: code landed, but follow-up verification or adjacent-file work remains
- `blocked`: waiting on adjacent runtime/kernel support, parity work, or perf evidence

---

## Current-state summary by file

| File | Current issue | Why it matters |
|---|---|---|
| `commands_optimize.rs` | Forces all product requests to `OptimizationMode::Grid` (`58-60`) | Prevents app-path usage of `Auto`, `CoarseToFine`, and `GpuVdps` |
| `double_ma_mode.rs` | `Auto` only chooses `Grid` vs `CoarseToFine`; no GPU-residency context (`16-23`) | Mode selection ignores your GPU-first objective |
| `double_ma_backend.rs` | Route/blocker logic is string-based and not residency-aware (`42-104`) | Hard to enforce “must stay on GPU” behavior cleanly |
| `double_ma_result.rs` | Matrix metadata is grid-only (`57-94`) and there is no residency/follow-up metadata | Product/UI compatibility logic is too tightly coupled to grid assumptions |
| `double_ma_strategy.rs` | Validation is good, but profile identity / route classification is not centralized | Repeated `uses_*` checks leak across optimization code |
| `double_ma.rs` | Major host escape hatches and repeated invariant recomputation | This is the core file to change for end-to-end GPU performance |

---

## Target architecture

### What “good” looks like

For a GPU-first Double-MA run, the intended flow should be:

```text
Tauri command
  -> request preserved (mode + GPU execution policy)
  -> core route assessment
  -> mode resolution aware of GPU policy
  -> run-scoped cache/context created
  -> GPU kernel / GPU hybrid path selected
  -> device-native regime filter or device-cached host fallback used
  -> MA generation/eval/search kept on GPU as far as practical
  -> result includes residency + route summary + follow-up cache token
  -> drilldown/local stability can reuse cached state
```

### Policy levels

I recommend making the execution goal explicit:

- **`BestEffort`**
  - Current behavior.
  - GPU is nice to have.
  - CPU / `GpuOnly` fallback is allowed.

- **`PreferGpuResident`**
  - Prefer kernel/device execution when possible.
  - Override “CPU-preferred” heuristics if the indicator is kernel-capable.
  - Fallback still allowed, but must be surfaced explicitly.

- **`StrictGpuResident`**
  - Do not silently downgrade.
  - Reject:
    - `CpuOnly`
    - `GpuOnly` (because it is not an end-to-end GPU evaluator)
    - hybrid routes that still require CPU staging for non-kernel-capable indicators
    - objective/feature blockers that force host execution

This policy belongs in the core request type, not just the product layer.

---

## File-by-file implementation plan

# 1) `commands_optimize.rs`

## Current state

Relevant code:

- `normalize_product_optimization_mode` forces `Grid`: `commands_optimize.rs:58-60`
- the product tests explicitly assert that all non-grid modes are rewritten to `Grid`: `commands_optimize.rs:119-137`

That is the first thing blocking real app-path measurement of mode improvements.

## Required edits

### A. Change normalization to preserve non-grid modes when outputs allow it

**Recommendation:** do not fully delete normalization in the first patch. Replace it with a compatibility function that only coerces to `Grid` when the requested output shape *requires* exact-grid semantics.

Suggested replacement:

```rust
fn normalize_product_optimization_mode(req: &DoubleMaRequest) -> OptimizationMode {
    let include_all = req.include_all.unwrap_or(false);
    let heatmap_bins = req.heatmap_bins.unwrap_or(0) as usize;
    let grid_outputs_requested =
        include_all || heatmap_bins > 0 || req.export_all_csv_path.is_some();

    match req.mode {
        OptimizationMode::Grid => OptimizationMode::Grid,
        // Preserve non-grid modes when the response shape does not require exact grid metadata.
        mode if !grid_outputs_requested => mode,
        // Transitional behavior: exact-grid outputs still pin to Grid.
        _ => OptimizationMode::Grid,
    }
}
```

Then update the command path:

```rust
#[tauri::command]
pub async fn run_double_ma_optimization(
    mut req: DoubleMaRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    req.mode = normalize_product_optimization_mode(&req);
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let sink = TauriProgressSink { app: app2 };
        ta_app_core::run_double_ma_optimization_blocking_with_candles(
            req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
```

### B. Add tests for transitional behavior

Replace the current “always force grid” test with two tests:

```rust
#[cfg(test)]
mod tests {
    use super::normalize_product_optimization_mode;
    use ta_app_core::{Backend, DoubleMaRequest};
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::StrategyConfig;

    fn base_req() -> DoubleMaRequest {
        DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "x".to_string(),
            fast_range: (5, 25, 1),
            slow_range: (20, 100, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Auto,
            top_k: Some(20),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            gpu_execution: Default::default(),
            persist_followup_cache: false,
        }
    }

    #[test]
    fn product_mode_normalization_preserves_non_grid_when_outputs_allow_it() {
        let mut req = base_req();
        req.mode = OptimizationMode::GpuVdps;
        assert_eq!(
            normalize_product_optimization_mode(&req),
            OptimizationMode::GpuVdps
        );

        req.mode = OptimizationMode::CoarseToFine;
        assert_eq!(
            normalize_product_optimization_mode(&req),
            OptimizationMode::CoarseToFine
        );
    }

    #[test]
    fn product_mode_normalization_pins_exact_grid_outputs_to_grid() {
        let mut req = base_req();
        req.mode = OptimizationMode::GpuVdps;
        req.heatmap_bins = Some(64);
        assert_eq!(
            normalize_product_optimization_mode(&req),
            OptimizationMode::Grid
        );
    }
}
```

## Notes

This is a **transitional product change**. The final state should be to remove normalization entirely once the UI path fully tolerates:

- `matrix_index = None`
- `best_col = None`
- `top_cols = None`

for non-grid modes.

---

# 2) `double_ma_mode.rs`

## Current state

Relevant code:

- `resolve_optimization_mode`: `double_ma_mode.rs:5-25`
- `validate_optimization_mode`: `double_ma_mode.rs:27-56`

Right now `Auto` only sees:

- `include_all`
- `heatmap_bins`
- `export_all_csv_path`
- `total_combos`

It does **not** see:

- requested backend class
- actual route viability
- GPU execution policy
- whether kernel/hybrid is available for the request

## Required edits

### A. Replace the loose parameter list with a context struct

This makes the function extensible and lets it reason about GPU policy.

```rust
use ta_optimizer::OptimizationMode;

use crate::double_ma::{GpuExecutionPolicy, OptimizationModeResolved};

pub(crate) struct ModeSelectionContext<'a> {
    pub requested_mode: OptimizationMode,
    pub include_all: bool,
    pub heatmap_bins: usize,
    pub export_all_csv_path: Option<&'a str>,
    pub total_combos: usize,
    pub gpu_execution: GpuExecutionPolicy,
    pub gpu_kernel_viable: bool,
}

pub(crate) fn resolve_optimization_mode(ctx: ModeSelectionContext<'_>) -> OptimizationModeResolved {
    match ctx.requested_mode {
        OptimizationMode::Grid => OptimizationModeResolved::Grid,
        OptimizationMode::CoarseToFine => OptimizationModeResolved::CoarseToFine,
        OptimizationMode::GpuVdps => OptimizationModeResolved::GpuVdps,
        OptimizationMode::Auto => {
            let grid_outputs_requested =
                ctx.include_all || ctx.export_all_csv_path.is_some() || ctx.heatmap_bins > 0;

            if grid_outputs_requested {
                return OptimizationModeResolved::Grid;
            }

            if matches!(
                ctx.gpu_execution,
                GpuExecutionPolicy::PreferGpuResident | GpuExecutionPolicy::StrictGpuResident
            ) && ctx.gpu_kernel_viable && ctx.total_combos >= 250_000
            {
                return OptimizationModeResolved::GpuVdps;
            }

            if ctx.total_combos > 500_000 {
                OptimizationModeResolved::CoarseToFine
            } else {
                OptimizationModeResolved::Grid
            }
        }
    }
}
```

### B. Keep validation strict

The current validation rules are correct and should stay strict.

I would keep `validate_optimization_mode` mostly as-is, but add one small comment in code: mode validation is about **output compatibility**, not backend viability. Backend viability belongs in `double_ma_backend.rs` / `double_ma.rs`.

### C. Update tests

```rust
#[cfg(test)]
mod tests {
    use super::{resolve_optimization_mode, ModeSelectionContext};
    use crate::double_ma::{GpuExecutionPolicy, OptimizationModeResolved};
    use ta_optimizer::OptimizationMode;

    #[test]
    fn auto_prefers_gpu_vdps_for_large_gpu_resident_searches() {
        let mode = resolve_optimization_mode(ModeSelectionContext {
            requested_mode: OptimizationMode::Auto,
            include_all: false,
            heatmap_bins: 0,
            export_all_csv_path: None,
            total_combos: 1_000_000,
            gpu_execution: GpuExecutionPolicy::StrictGpuResident,
            gpu_kernel_viable: true,
        });
        assert!(matches!(mode, OptimizationModeResolved::GpuVdps));
    }

    #[test]
    fn auto_stays_grid_when_exact_grid_outputs_are_requested() {
        let mode = resolve_optimization_mode(ModeSelectionContext {
            requested_mode: OptimizationMode::Auto,
            include_all: true,
            heatmap_bins: 0,
            export_all_csv_path: None,
            total_combos: 1_000_000,
            gpu_execution: GpuExecutionPolicy::StrictGpuResident,
            gpu_kernel_viable: true,
        });
        assert!(matches!(mode, OptimizationModeResolved::Grid));
    }
}
```

---

# 3) `double_ma_backend.rs`

## Current state

Relevant code:

- `gpu_kernel_blockers_for_request`: `double_ma_backend.rs:42-104`
- `build_indicator_route_plan_summary`: `double_ma_backend.rs:123-141`

This file already holds the right *kind* of logic, but it is too stringly-typed and not aware of the GPU execution goal.

Two specific problems:

1. `GpuOnly` is still “GPU” from the point of view of routing, even though it is **not** GPU-resident end-to-end.
2. hybrid planning does not distinguish:
   - kernel unsupported
   - CPU-preferred by heuristic

That distinction is critical for your goal.

## Required edits

### A. Add a helper that decides whether CPU staging is actually necessary

This edit is directly actionable inside the attached files because the route plan already exposes:

- capability tier
- route tier
- `gpu_preferred_by_speedup`

```rust
use crate::double_ma::{
    kernel_regime_filters_enabled, Backend, GpuExecutionPolicy, IndicatorRouteDecisionSummary,
    IndicatorRoutePlanSummary,
};

pub(crate) fn should_cpu_stage_indicator(
    decision: &crate::ma_policy::HybridRouteDecision,
    gpu_execution: GpuExecutionPolicy,
) -> bool {
    use crate::ma_policy::{HybridCapabilityTier, HybridRouteTier};

    if decision.route != HybridRouteTier::CpuStaged {
        return false;
    }

    // Important distinction:
    // - CpuStaged because kernel is unsupported -> still true
    // - CpuStaged only because speedup heuristics prefer CPU -> false when caller wants residency
    if matches!(
        gpu_execution,
        GpuExecutionPolicy::PreferGpuResident | GpuExecutionPolicy::StrictGpuResident
    ) && decision.capability == HybridCapabilityTier::GpuKernel
    {
        return false;
    }

    true
}
```

### B. Add a strict-GPU blocker helper

```rust
pub(crate) fn strict_gpu_route_blockers(
    fast_ma: &str,
    slow_ma: &str,
    strategy: &StrategyConfig,
    regime_filter_present: bool,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
) -> Vec<String> {
    let mut blockers = gpu_kernel_blockers_for_request(
        fast_ma,
        slow_ma,
        strategy,
        regime_filter_present,
        objective,
        top_k,
        include_all,
    );

    let plan = crate::ma_policy::plan_double_ma_hybrid_route(fast_ma, slow_ma);
    if should_cpu_stage_indicator(&plan.fast, GpuExecutionPolicy::StrictGpuResident) {
        blockers.push(format!(
            "fast MA '{}' cannot stay GPU-resident under current hybrid/kernel capabilities",
            fast_ma
        ));
    }
    if should_cpu_stage_indicator(&plan.slow, GpuExecutionPolicy::StrictGpuResident) {
        blockers.push(format!(
            "slow MA '{}' cannot stay GPU-resident under current hybrid/kernel capabilities",
            slow_ma
        ));
    }

    blockers
}
```

### C. Enrich indicator route summaries

Today the summary does not say **why** a route is CPU staged. Add a `staging_reason` and `residency_tier`.

**Edit `double_ma.rs` struct definition too** (covered later), then wire it here:

```rust
fn staging_reason_for_decision(
    decision: &crate::ma_policy::HybridRouteDecision,
) -> Option<String> {
    use crate::ma_policy::{HybridCapabilityTier, HybridRouteTier};

    if decision.route != HybridRouteTier::CpuStaged {
        return None;
    }

    Some(match decision.capability {
        HybridCapabilityTier::GpuKernel if !decision.gpu_preferred_by_speedup =>
            "cpu_staged_but_kernel_capable".to_string(),
        HybridCapabilityTier::GpuKernel =>
            "cpu_preferred_by_speedup".to_string(),
        HybridCapabilityTier::CudaOnly =>
            "kernel_unsupported_cuda_only".to_string(),
        HybridCapabilityTier::CpuOnly =>
            "cpu_only".to_string(),
        HybridCapabilityTier::Unsupported =>
            "unsupported".to_string(),
    })
}
```

Then populate:

```rust
let fast = IndicatorRouteDecisionSummary {
    indicator_id: plan.fast.indicator_id,
    capability: map_hybrid_capability_label(plan.fast.capability).to_string(),
    route: map_hybrid_route_label(plan.fast.route).to_string(),
    gpu_preferred_by_speedup: plan.fast.gpu_preferred_by_speedup,
    staging_reason: staging_reason_for_decision(&plan.fast),
    residency_tier: if plan.fast.route == crate::ma_policy::HybridRouteTier::GpuKernel {
        "device_native".to_string()
    } else {
        "cpu_staged".to_string()
    },
};
```

## Notes

This file should remain a **pure route/policy helper** file. It should not accumulate execution-side caches. Execution-side caching belongs in `double_ma.rs`.

---

# 4) `double_ma_result.rs`

## Current state

Relevant code:

- `build_grid_matrix_index`: `double_ma_result.rs:57-94`
- `build_backend_optimization_result`: `double_ma_result.rs:96-168`

This file is already clean. The main issue is not correctness; it is that the result shape is too sparse for a GPU-first system.

Specifically, the result currently does not tell the caller:

- whether the execution stayed resident
- whether host staging happened
- whether a follow-up cache token is available
- whether the mode was exact-grid compatible vs search-based

## Required edits

### A. Extend the result context

Add the new fields to `BackendResultContext`:

```rust
pub(crate) struct BackendResultContext<'a> {
    pub mode_used: OptimizationModeResolved,
    pub backend_used: BackendUsed,
    pub fast_periods: &'a [u16],
    pub slow_periods: &'a [u16],
    pub fast_ma_label: &'a str,
    pub slow_ma_label: &'a str,
    pub fast_param_variants: &'a [Option<HashMap<String, f64>>],
    pub slow_param_variants: &'a [Option<HashMap<String, f64>>],
    pub num_combos: usize,
    pub num_candles: usize,
    pub runtime_ms: u64,
    pub vdps: Option<VdpsTelemetry>,
    pub hybrid_timing_ms: Option<HybridTimingMs>,
    pub backend_route: Option<String>,
    pub backend_fallback_reason: Option<String>,
    pub indicator_route_plan: Option<IndicatorRoutePlanSummary>,
    pub residency: Option<GpuResidencySummary>,
    pub followup_cache_key: Option<String>,
}
```

### B. Add result fields

In `double_ma.rs` add fields to `BackendOptimizationResult`, then set them here:

```rust
BackendOptimizationResult {
    best_params,
    best_metrics: result.best_metrics,
    mode_used,
    backend_used: ctx.backend_used,
    matrix_index,
    best_col,
    top_cols,
    num_combos: ctx.num_combos,
    num_candles: ctx.num_candles,
    runtime_ms: ctx.runtime_ms,
    top,
    all,
    heatmap: result.heatmap,
    vdps: ctx.vdps,
    hybrid_timing_ms: ctx.hybrid_timing_ms,
    backend_route: ctx.backend_route,
    backend_fallback_reason: ctx.backend_fallback_reason,
    indicator_route_plan: ctx.indicator_route_plan,
    residency: ctx.residency,
    followup_cache_key: ctx.followup_cache_key,
}
```

### C. Keep grid metadata optional and exact

`build_grid_matrix_index` should stay exact and grid-only. That is the correct behavior.  
The fix is not to “fake” grid metadata for non-grid modes. The fix is to make the product layer tolerate `None`.

## Notes

This file is the right place to keep the result builder thin. Avoid putting route/business logic here.

---

# 5) `double_ma_strategy.rs`

## Current state

Relevant code:

- `validate_strategy_cfg`: `double_ma_strategy.rs:3-377`

Validation is already strong. The improvement opportunity here is not “fix bugs”; it is to make the optimization code stop manually checking a dozen `uses_*` flags everywhere.

## Required edits

### A. Add a centralized regime/profile classifier

```rust
use ta_strategies::double_ma::StrategyConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DoubleMaRegimeProfile {
    None,
    Halftrend,
    TrendQuality,
    BreakoutMomentum,
    PressureImpulseTrend,
    SlopeEfficiencyTrend,
    MomentumTripleStack,
    OscillatorReversionConfluence,
    VolumeFlowConfirm,
    RegimeHybrid,
}

pub(crate) fn active_regime_profile(
    cfg: &StrategyConfig,
    halftrend_filter_present: bool,
) -> Result<DoubleMaRegimeProfile, String> {
    let mut active: Vec<DoubleMaRegimeProfile> = Vec::new();

    if halftrend_filter_present {
        active.push(DoubleMaRegimeProfile::Halftrend);
    }
    if cfg.uses_trend_quality_filter() {
        active.push(DoubleMaRegimeProfile::TrendQuality);
    }
    if cfg.uses_breakout_momentum_filter() {
        active.push(DoubleMaRegimeProfile::BreakoutMomentum);
    }
    if cfg.uses_pressure_impulse_trend_filter() {
        active.push(DoubleMaRegimeProfile::PressureImpulseTrend);
    }
    if cfg.uses_slope_efficiency_trend_filter() {
        active.push(DoubleMaRegimeProfile::SlopeEfficiencyTrend);
    }
    if cfg.uses_momentum_triple_stack_filter() {
        active.push(DoubleMaRegimeProfile::MomentumTripleStack);
    }
    if cfg.uses_oscillator_reversion_confluence_filter() {
        active.push(DoubleMaRegimeProfile::OscillatorReversionConfluence);
    }
    if cfg.uses_volume_flow_confirm_filter() {
        active.push(DoubleMaRegimeProfile::VolumeFlowConfirm);
    }
    if cfg.uses_regime_hybrid_filter() {
        active.push(DoubleMaRegimeProfile::RegimeHybrid);
    }

    match active.as_slice() {
        [] => Ok(DoubleMaRegimeProfile::None),
        [one] => Ok(*one),
        _ => Err("multiple Double-MA regime profiles are enabled simultaneously".to_string()),
    }
}
```

### B. Use the classifier inside validation

After existing field-level validation, do:

```rust
pub(crate) fn validate_strategy_cfg_for_optimization(
    cfg: &StrategyConfig,
    halftrend_filter_present: bool,
) -> Result<(), String> {
    validate_strategy_cfg(cfg)?;
    let _ = active_regime_profile(cfg, halftrend_filter_present)?;
    Ok(())
}
```

Then update call sites in `double_ma.rs`:

```rust
validate_strategy_cfg_for_optimization(&strategy, halftrend_filter.is_some())?;
```

### C. Add a route-shape helper

This avoids sprinkling `uses_execution_overlays` / `uses_kaufman_stop_exit` across route logic.

```rust
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct StrategyRouteShape {
    pub has_kaufman_stop_exit: bool,
    pub has_execution_overlays: bool,
}

pub(crate) fn strategy_route_shape(cfg: &StrategyConfig) -> StrategyRouteShape {
    StrategyRouteShape {
        has_kaufman_stop_exit: cfg.uses_kaufman_stop_exit(),
        has_execution_overlays: cfg.uses_execution_overlays(),
    }
}
```

## Notes

I would **not** do a giant stylistic rewrite of this file.  
The practical goal is to centralize profile identity and route-shape facts so `double_ma.rs` stops duplicating them.

---

# 6) `double_ma.rs`

This is the main file and the biggest part of the work.

I would break the edits into nine focused chunks.

---

## 6.1 Add request/result policy types

### Why

The core request type needs to express the execution goal explicitly.

### Edit location

Near the existing public request/result types around `double_ma.rs:65-345`.

### Code

```rust
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum GpuExecutionPolicy {
    #[default]
    BestEffort,
    PreferGpuResident,
    StrictGpuResident,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GpuResidencySummary {
    pub requested_policy: GpuExecutionPolicy,
    pub actual_backend: String,
    pub prices_resident: bool,
    pub fast_ma_resident: bool,
    pub slow_ma_resident: bool,
    pub regime_resident: bool,
    pub search_controller_resident: bool,
    pub cpu_fallback_used: bool,
    pub host_round_trips: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DoubleMaRequest {
    pub backend: Backend,
    pub data_id: String,
    pub fast_range: (u32, u32, u32),
    pub slow_range: (u32, u32, u32),
    pub fast_ma_types: Vec<String>,
    pub slow_ma_types: Vec<String>,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub export_all_csv_path: Option<String>,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub fast_ma_param_sweeps: Option<HashMap<String, (f64, f64, f64)>>,
    pub slow_ma_param_sweeps: Option<HashMap<String, (f64, f64, f64)>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    pub objective: ObjectiveKind,
    pub mode: OptimizationMode,
    pub top_k: Option<usize>,
    pub include_all: Option<bool>,
    pub heatmap_bins: Option<u16>,
    #[serde(default)]
    pub constraints: Option<DoubleMaConstraints>,
    #[serde(default)]
    pub vdps: Option<VdpsConfig>,
    #[serde(default)]
    pub gpu_execution: GpuExecutionPolicy,
    #[serde(default)]
    pub persist_followup_cache: bool,
}
```

Add follow-up hooks to the follow-up requests:

```rust
#[derive(Clone, Debug, Deserialize)]
pub struct DoubleMaDrilldownRequest {
    pub data_id: String,
    pub params: DoubleMaParamsResolved,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    pub bins: Option<usize>,
    #[serde(default)]
    pub backend: Option<Backend>,
    #[serde(default)]
    pub followup_cache_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DoubleMaLocalStabilityRequest {
    pub data_id: String,
    pub params: DoubleMaParamsResolved,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    pub objective: ObjectiveKind,
    pub radius: Option<u16>,
    pub step: Option<u16>,
    #[serde(default)]
    pub backend: Option<Backend>,
    #[serde(default)]
    pub followup_cache_key: Option<String>,
}
```

Add result fields:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct BackendOptimizationResult {
    pub best_params: DoubleMaParamsResolved,
    pub best_metrics: ta_strategies::double_ma::Metrics,
    pub mode_used: OptimizationModeResolved,
    pub backend_used: BackendUsed,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_index: Option<DoubleMaFastLtSlowIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_col: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_cols: Option<Vec<u32>>,
    pub num_combos: usize,
    pub num_candles: usize,
    pub runtime_ms: u64,
    pub top: Vec<(DoubleMaParamsResolved, ta_strategies::double_ma::Metrics)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<(DoubleMaParamsResolved, ta_strategies::double_ma::Metrics)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heatmap: Option<OptimizationHeatmap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vdps: Option<VdpsTelemetry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_timing_ms: Option<HybridTimingMs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indicator_route_plan: Option<IndicatorRoutePlanSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residency: Option<GpuResidencySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_cache_key: Option<String>,
}
```

### Why this matters

Without this change, every later improvement is forced to be heuristic-only and silently permissive.

---

## 6.2 Hoist static/invariant evaluation work out of each MA-param run

### Why

Inside the outer MA-param sweep loop (`double_ma.rs:4302-4406`), each per-variant run can recompute:

- close source lookup
- MA source lookup
- base `first_valid`
- log returns
- `KaufmanStopSeries`
- sometimes dynamic strategy context

That is avoidable. These inputs depend on candles / source / strategy / regime filter, not on the MA parameter variant.

### Edit locations

- `run_double_ma_optimization_blocking_with_candles`: `2618-4521`
- `optimize_double_ma_cpu_tiled`: `7379-7815`
- `optimize_double_ma_gpu_sweep_tiled`: `8111-8455`

### Code

Add a static context:

```rust
struct StaticEvalContext<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    close: &'a [f64],
    ma_prices: &'a [f64],
    prices_f32: Vec<f32>,
    lr: Vec<f64>,
    base_first_valid: usize,
    trade_offset: usize,
    kaufman_stops: Option<KaufmanStopSeries>,
}

fn build_static_eval_context<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
    regime_filter: Option<&[f64]>,
) -> Result<StaticEvalContext<'a>, String> {
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }

    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let mut base_first_valid = fv_close.max(fv_ma);

    if let Some(regime) = regime_filter {
        let fv_regime =
            first_finite_idx(regime).ok_or_else(|| "regime trend series all non-finite".to_string())?;
        base_first_valid = base_first_valid.max(fv_regime);
    }

    let kaufman_stops = compute_kaufman_stop_series(candles, strategy)?;
    if let Some(ks) = kaufman_stops.as_ref() {
        let fv_k_long = first_finite_idx(&ks.long)
            .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
        let fv_k_short = first_finite_idx(&ks.short)
            .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
        base_first_valid = base_first_valid.max(fv_k_long.max(fv_k_short));
    }

    Ok(StaticEvalContext {
        candles,
        close,
        ma_prices,
        prices_f32: ma_prices.iter().map(|&x| x as f32).collect(),
        lr: compute_log_returns_f64(close),
        base_first_valid,
        trade_offset: usize::from(strategy.trade_on_next_bar),
        kaufman_stops,
    })
}
```

Then change the CPU/GPU-sweep helpers to accept `&StaticEvalContext` rather than recomputing those pieces internally.

**Example signature change**:

```rust
fn optimize_double_ma_cpu_tiled(
    static_ctx: &StaticEvalContext<'_>,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    halftrend_trend: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    constraints: Option<&DoubleMaConstraints>,
    total_pairs: usize,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    let candles = static_ctx.candles;
    let close = static_ctx.close;
    let lr = &static_ctx.lr;
    let n = close.len();
    let mut first_valid = static_ctx.base_first_valid;
    if let Some(trend) = halftrend_trend {
        let fv_ht = first_finite_idx(trend)
            .ok_or_else(|| "regime trend series all non-finite".to_string())?;
        first_valid = first_valid.max(fv_ht);
    }
    let trade_offset = static_ctx.trade_offset;
    let kaufman_stops = static_ctx.kaufman_stops.as_ref();
    // ... existing body ...
}
```

### Why this matters

This change is especially important when:

- MA parameter variants are enabled
- `CoarseToFine` performs multiple passes
- `GpuVdps` performs multiple rungs
- follow-up flows reuse the same strategy/candle inputs

This is a direct win without changing scoring semantics.

---

## 6.3 Add run-scoped regime caches and unify regime preparation

### Why

The file already has a host cache for the generic combined regime filter (`2727`, `2999`, `3241`, `3331`, `6504-6522`), but it does **not** cache the specialized device regime builders across param variants and coarse/fine passes.

That means PIT/SET/MTS/etc. can be rebuilt repeatedly inside one logical optimization run.

### Edit locations

- outer run setup around `2727`
- `run_grid` around `2963-3380`
- `optimize_double_ma_gpu_hybrid_kernel_phase1` around `8503-9020`

### Code

Add run-scoped caches:

```rust
#[derive(Default)]
struct RunHostCaches {
    combined_regime: Option<Option<Vec<f64>>>,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[derive(Default)]
struct RunDeviceRegimeCaches {
    pit: Option<crate::gpu_backtest_kernel::DeviceRegimeFilter>,
    set: Option<crate::gpu_backtest_kernel::DeviceRegimeFilter>,
    mts: Option<crate::gpu_backtest_kernel::DeviceRegimeFilter>,
    orc: Option<crate::gpu_backtest_kernel::DeviceRegimeFilter>,
    vfct: Option<crate::gpu_backtest_kernel::DeviceRegimeFilter>,
    // Add a generic device regime filter slot later when that builder exists.
}
```

Add a unified regime input enum:

```rust
enum PreparedRegimeInput<'a> {
    Host(Option<&'a [f64]>),
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    Device(&'a crate::gpu_backtest_kernel::DeviceRegimeFilter),
}
```

Add a helper:

```rust
fn prepare_regime_input<'a>(
    host_cache: &'a mut RunHostCaches,
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    device_cache: &'a mut RunDeviceRegimeCaches,
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
    device_id: Option<u32>,
    expected_len: usize,
) -> Result<PreparedRegimeInput<'a>, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        if let Some(device_id) = device_id {
            if strategy.uses_pressure_impulse_trend_filter() {
                if device_cache.pit.is_none() {
                    device_cache.pit = Some(build_pressure_impulse_trend_regime_device(
                        candles, ma_source, device_id, strategy,
                    )?);
                }
                return Ok(PreparedRegimeInput::Device(device_cache.pit.as_ref().unwrap()));
            }
            if strategy.uses_slope_efficiency_trend_filter() {
                if device_cache.set.is_none() {
                    device_cache.set = Some(build_slope_efficiency_trend_regime_device(
                        candles, ma_source, device_id, strategy,
                    )?);
                }
                return Ok(PreparedRegimeInput::Device(device_cache.set.as_ref().unwrap()));
            }
            if strategy.uses_momentum_triple_stack_filter() {
                if device_cache.mts.is_none() {
                    device_cache.mts = Some(build_momentum_triple_stack_regime_device(
                        candles, ma_source, device_id, strategy,
                    )?);
                }
                return Ok(PreparedRegimeInput::Device(device_cache.mts.as_ref().unwrap()));
            }
            if strategy.uses_oscillator_reversion_confluence_filter() {
                if device_cache.orc.is_none() {
                    device_cache.orc = Some(build_oscillator_reversion_confluence_regime_device(
                        candles, ma_source, device_id, strategy,
                    )?);
                }
                return Ok(PreparedRegimeInput::Device(device_cache.orc.as_ref().unwrap()));
            }
            if strategy.uses_volume_flow_confirm_filter() {
                if device_cache.vfct.is_none() {
                    device_cache.vfct = Some(build_volume_flow_confirmation_regime_device(
                        candles, device_id, strategy,
                    )?);
                }
                return Ok(PreparedRegimeInput::Device(device_cache.vfct.as_ref().unwrap()));
            }
        }
    }

    let regime = ensure_combined_regime_filter_cached(
        &mut host_cache.combined_regime,
        candles,
        ma_source,
        strategy,
        halftrend_filter,
        expected_len,
    )?;
    Ok(PreparedRegimeInput::Host(regime))
}
```

Then use this helper from both `run_grid` and hybrid phase-1 instead of duplicating specialized-builder logic.

### Why this matters

This refactor gets you:

- a single place to choose host vs device regime preparation
- one-time specialized device filter construction per optimization run
- cleaner path to add a generic device regime filter later

---

## 6.4 Enforce GPU execution policy during backend resolution

### Why

`Backend::GpuOnly` is currently considered “GPU”.  
For a strict GPU-resident goal, that is misleading, because the implementation still builds host matrices and evaluates metrics on CPU.

### Edit locations

Main optimization entrypoint around:

- `2769-2822` (Auto backend resolution)
- `2824-2912` (fallback / mode adjustments)
- `2914-2944` (route summary)

### Code

After resolving the backend and building the route plan, enforce the policy:

```rust
fn enforce_gpu_execution_policy(
    gpu_execution: GpuExecutionPolicy,
    backend: &Backend,
    strict_gpu_blockers: &[String],
) -> Result<(), String> {
    if !matches!(gpu_execution, GpuExecutionPolicy::StrictGpuResident) {
        return Ok(());
    }

    match backend {
        Backend::GpuKernel { .. } | Backend::GpuHybridKernel { .. } => {}
        Backend::GpuOnly { .. } => {
            return Err(
                "StrictGpuResident rejects Backend::GpuOnly because metrics evaluation is still host-side"
                    .to_string(),
            );
        }
        Backend::CpuOnly => {
            return Err("StrictGpuResident rejects Backend::CpuOnly".to_string());
        }
        Backend::Auto { .. } => unreachable!("backend should already be resolved"),
    }

    if !strict_gpu_blockers.is_empty() {
        return Err(format!(
            "StrictGpuResident route is unavailable. Reasons: {}",
            strict_gpu_blockers.join("; ")
        ));
    }

    Ok(())
}
```

Then call it:

```rust
let strict_gpu_blockers = crate::double_ma_backend::strict_gpu_route_blockers(
    fast_ma.as_str(),
    slow_ma.as_str(),
    &strategy,
    regime_filter_expected,
    objective,
    top_k,
    include_all,
);

enforce_gpu_execution_policy(req.gpu_execution, &backend, &strict_gpu_blockers)?;
```

### Route summary

Keep `backend_route`, but also build a proper residency summary:

```rust
let residency = Some(GpuResidencySummary {
    requested_policy: req.gpu_execution,
    actual_backend: resolved_backend_label.to_string(),
    prices_resident: !matches!(backend, Backend::CpuOnly),
    fast_ma_resident: matches!(backend, Backend::GpuKernel { .. } | Backend::GpuHybridKernel { .. }),
    slow_ma_resident: matches!(backend, Backend::GpuKernel { .. } | Backend::GpuHybridKernel { .. }),
    regime_resident: false, // overwritten later if device regime is actually used
    search_controller_resident: matches!(mode_used, OptimizationModeResolved::Grid),
    cpu_fallback_used: matches!(backend, Backend::CpuOnly | Backend::GpuOnly { .. }),
    host_round_trips: Vec::new(),
});
```

Then mutate that summary later as actual execution decisions become known.

### Why this matters

This is the core policy barrier that keeps “GPU requested” from degenerating into “GPU-ish”.

---

## 6.5 Make hybrid phase-1 residency-aware

### Why

`optimize_double_ma_gpu_hybrid_kernel_phase1` is the most important practical bridge toward your goal because it already has:

- planner timing
- CPU prep timing
- GPU prep timing
- H2D timing
- kernel eval timing

It is the natural place to enforce residency policy and override CPU-preferred staging for kernel-capable indicators.

### Edit location

`double_ma.rs:8503-9020`

### Required changes

#### A. Accept the policy as an argument

```rust
pub(crate) fn optimize_double_ma_gpu_hybrid_kernel_phase1(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    allow_cpu_fallback: bool,
    eval_range: Option<(usize, usize)>,
    gpu_execution: GpuExecutionPolicy,
) -> Result<(
    OptimizationResult,
    HybridTimingMs,
    BackendUsed,
    Option<String>,
    Option<String>,
), String> {
    // ...
}
```

#### B. Use `should_cpu_stage_indicator`

Replace:

```rust
let fast_requires_cpu_precompute =
    route_plan.fast.route == crate::ma_policy::HybridRouteTier::CpuStaged;
let slow_requires_cpu_precompute =
    route_plan.slow.route == crate::ma_policy::HybridRouteTier::CpuStaged;
```

with:

```rust
let fast_requires_cpu_precompute =
    crate::double_ma_backend::should_cpu_stage_indicator(&route_plan.fast, gpu_execution);
let slow_requires_cpu_precompute =
    crate::double_ma_backend::should_cpu_stage_indicator(&route_plan.slow, gpu_execution);
```

#### C. Emit an override note when the CPU-preferred heuristic is bypassed

```rust
if !fast_requires_cpu_precompute
    && route_plan.fast.route == crate::ma_policy::HybridRouteTier::CpuStaged
{
    merge_optional_note(
        &mut route_note,
        Some(format!(
            "fast MA '{}' bypassed CPU-preferred staging to keep device execution",
            fast_ma_type
        )),
    );
}
if !slow_requires_cpu_precompute
    && route_plan.slow.route == crate::ma_policy::HybridRouteTier::CpuStaged
{
    merge_optional_note(
        &mut route_note,
        Some(format!(
            "slow MA '{}' bypassed CPU-preferred staging to keep device execution",
            slow_ma_type
        )),
    );
}
```

#### D. Fail earlier under strict policy

Before fallback logic:

```rust
if matches!(gpu_execution, GpuExecutionPolicy::StrictGpuResident) && !kernel_blockers.is_empty() {
    return Err(format!(
        "StrictGpuResident requires a kernel-executable hybrid route. Reasons: {blockers_text}"
    ));
}
```

#### E. Expand timing fields

I would extend `HybridTimingMs` to include:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct HybridTimingMs {
    pub planner_ms: u64,
    pub cpu_indicator_prep_ms: u64,
    pub gpu_indicator_prep_ms: u64,
    pub regime_prep_ms: u64,
    pub h2d_upload_ms: u64,
    pub kernel_eval_ms: u64,
    pub cpu_staged_indicator_count: u32,
    pub device_cache_hit_count: u32,
    pub host_upload_bytes: u64,
}
```

These fields do not have to be perfect on day one. Even partial measurements are useful.

### Why this matters

This single change converts hybrid from “sometimes silently CPU-shaped” into a path that can actually obey a GPU-residency objective.

---

## 6.6 Make PIT device-native where possible

### Why

`build_pressure_impulse_trend_regime_device` (`5894-6088`) is the clearest remaining device-builder outlier.

Current behavior:

- EFI -> `HostF32`, then upload
- BOP -> `HostF32`, then upload
- VOSC -> `HostF32`, then upload
- ACOSC -> CPU path, then upload
- CFO -> `HostF32`, then upload

By contrast, SET/MTS/ORC/VFCT use `DeviceF32` flows.

### Edit location

`double_ma.rs:5894-6088`

### Code

EFI should look like this:

```rust
let efi_output = compute_dynamic_indicator_cuda_with_inputs(
    &prepared_inputs,
    "efi",
    Some("value"),
    &[IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(strategy.pit_efi_period() as i64),
    }],
    Kernel::Auto,
    CudaOutputTarget::DeviceF32,
)
.map_err(|e| format!("pit efi failed: {e}"))?;
let (efi_matrix, efi_warmup_raw) = regime_device_matrix_from_output("pit efi", efi_output)?;
let efi_warmup = efi_warmup_raw.max(
    prepared_inputs
        .close_volume_first_valid()
        .saturating_add(strategy.pit_efi_period().saturating_sub(1)),
);
let efi_row = device_slice_ref_from_matrix_row(&efi_matrix, 0)
    .map_err(|e| format!("pit efi device row build failed: {e}"))?;
```

BOP:

```rust
let bop_output = compute_dynamic_indicator_cuda_with_inputs(
    &prepared_inputs,
    "bop",
    Some("value"),
    &[],
    Kernel::Auto,
    CudaOutputTarget::DeviceF32,
)
.map_err(|e| format!("pit bop failed: {e}"))?;
let (bop_matrix, bop_warmup) = regime_device_matrix_from_output("pit bop", bop_output)?;
let bop_row = device_slice_ref_from_matrix_row(&bop_matrix, 0)
    .map_err(|e| format!("pit bop device row build failed: {e}"))?;
```

VOSC:

```rust
let vosc_output = compute_cuda_device(IndicatorCudaDeviceRequest {
    indicator_id: "vosc",
    output_id: Some("value"),
    data: IndicatorCudaDeviceDataRef::Slice {
        values: prepared_inputs.volume_device().as_view(),
    },
    params: &[
        vector_ta::indicators::dispatch::ParamKV {
            key: "short_period",
            value: vector_ta::indicators::dispatch::ParamValue::Int(
                strategy.pit_vosc_short_period() as i64,
            ),
        },
        vector_ta::indicators::dispatch::ParamKV {
            key: "long_period",
            value: vector_ta::indicators::dispatch::ParamValue::Int(
                strategy.pit_vosc_long_period() as i64,
            ),
        },
    ],
    kernel: Kernel::Auto,
    target: CudaOutputTarget::DeviceF32,
})
.map_err(|e| format!("pit vosc failed: {e}"))?;

let vosc_output = crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput {
    output_id: vosc_output.output_id,
    series: match vosc_output.series {
        IndicatorCudaSeries::DeviceF32(matrix) => DynamicIndicatorCudaSeries::DeviceF32(matrix),
        IndicatorCudaSeries::HostF32(values) => DynamicIndicatorCudaSeries::HostF32(values),
    },
    warmup: vosc_output.warmup,
    rows: vosc_output.rows,
    cols: vosc_output.cols,
    pattern_ids: vosc_output.pattern_ids,
};

let (vosc_matrix, vosc_warmup) = regime_device_matrix_from_output("pit vosc", vosc_output)?;
let vosc_row = device_slice_ref_from_matrix_row(&vosc_matrix, 0)
    .map_err(|e| format!("pit vosc device row build failed: {e}"))?;
```

CFO:

```rust
let cfo_output = compute_dynamic_indicator_cuda_with_inputs(
    &prepared_inputs,
    "cfo",
    Some("value"),
    &[
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.pit_cfo_period() as i64),
        },
        IndicatorParamKV {
            key: "scalar".to_string(),
            value: IndicatorParamValue::Float(strategy.pit_cfo_scalar()),
        },
    ],
    Kernel::Auto,
    CudaOutputTarget::DeviceF32,
)
.map_err(|e| format!("pit cfo failed: {e}"))?;
let (cfo_matrix, cfo_warmup_raw) = regime_device_matrix_from_output("pit cfo", cfo_output)?;
let cfo_warmup = cfo_warmup_raw.max(
    prepared_inputs
        .source_first_valid()
        .saturating_add(strategy.pit_cfo_period().saturating_sub(1)),
);
let cfo_row = device_slice_ref_from_matrix_row(&cfo_matrix, 0)
    .map_err(|e| format!("pit cfo device row build failed: {e}"))?;
```

ACOSC should stay guarded until parity is proven. Add a helper gate:

```rust
fn pit_allow_cuda_acosc() -> bool {
    match std::env::var("VECTORBT_PIT_USE_CUDA_ACOSC") {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => false,
    }
}
```

Then keep the current CPU path as the default fallback until parity is good.

### Why this matters

This is the most obvious way to eliminate avoidable host/device round-trips in the current file.

---

## 6.7 Treat `GpuOnly` as a downgrade, not a success path

### Why

`GpuOnly` is still useful as a best-effort fallback, but it is not compatible with a “fully GPU executed backtest optimization process” objective.

Relevant code:

- auto resolution uses `GpuOnly` when kernel/hybrid is not available: `2770-2814`
- explicit kernel fallback can also land on `GpuOnly`: `2862-2892`
- `GpuVdps` may promote `GpuOnly` to hybrid: `2896-2911`
- `GpuOnly` grid dispatch uses `optimize_double_ma_gpu_sweep_tiled`: `2995-3033`
- that function uses host MA matrices and CPU metric evaluation: `8111-8455`

### Required edits

### A. Make the downgrade visible in route/residency metadata

When `backend == Backend::GpuOnly { .. }`, append:

```rust
merge_optional_note(
    &mut backend_fallback_reason,
    Some(
        "GpuOnly uses CUDA MA generation but host-resident metric evaluation; it is not an end-to-end GPU path."
            .to_string(),
    ),
);
```

### B. Reject it under strict policy

Covered already by `enforce_gpu_execution_policy`.

### C. Rename only the execution label, not the public enum

Do **not** rename the enum immediately. That would be noisy.  
Instead, report the actual execution kind in `GpuResidencySummary.actual_backend` as:

- `GpuKernel`
- `GpuHybridKernel`
- `GpuSweepHostEval`
- `CpuOnly`

This gives you honest telemetry without breaking the request contract.

---

## 6.8 Add follow-up reuse and backend-aware follow-up execution

### Why

Current follow-up functions:

- `compute_double_ma_drilldown_blocking_with_candles`: `2278-2399`
- `compute_double_ma_local_stability_blocking_with_candles`: `2402-2616`

Both recompute:

- combined regime filter
- `KaufmanStopSeries`
- MA matrices

They also only use the CPU path.

### Phase 1: backend-aware matrix generation

You already have `ma_matrix_with_backend_f64` (`7097-7179`). Use it here.

#### Drilldown example

Replace:

```rust
let (fast_mat, cols) = ma_matrix_cpu_f64(
    candles,
    fast_ma,
    &req.ma_source,
    &fast_periods,
    req.fast_ma_params.as_ref(),
    fast_alma_offset,
    fast_alma_sigma,
)?;
```

with:

```rust
let prices_f32 = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source)
    .iter()
    .map(|&v| v as f32)
    .collect::<Vec<f32>>();

let (fast_mat, cols) = ma_matrix_with_backend_f64(
    candles,
    fast_ma,
    &req.ma_source,
    &fast_periods,
    req.fast_ma_params.as_ref(),
    fast_alma_offset,
    fast_alma_sigma,
    req.backend.as_ref(),
    Some(prices_f32.as_slice()),
)?;
```

Do the same for the slow side and for local stability.

This does **not** make follow-up fully GPU-resident yet, but it immediately allows:

- GPU-backed matrix generation
- consistent MA generation backend between optimization and follow-up
- one obvious place to swap in a stronger cache later

### Phase 2: add a follow-up cache token

Add a simple cache for reused state:

```rust
static DOUBLE_MA_FOLLOWUP_CACHE: OnceLock<Mutex<HashMap<String, FollowupCacheEntry>>> =
    OnceLock::new();

#[derive(Clone)]
struct FollowupCacheEntry {
    ma_source: String,
    strategy: StrategyConfig,
    halftrend_filter: Option<HalfTrendRegimeFilter>,
    regime_filter: Option<Vec<f64>>,
    kaufman_stops: Option<KaufmanStopSeries>,
}
```

Key builder:

```rust
fn build_followup_cache_key(
    data_id: &str,
    ma_source: &str,
    fast_ma_type: &str,
    slow_ma_type: &str,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
) -> Result<String, String> {
    serde_json::to_string(&(data_id, ma_source, fast_ma_type, slow_ma_type, strategy, halftrend_filter))
        .map_err(|e| format!("follow-up cache key serialization failed: {e}"))
}
```

Store after optimization when `persist_followup_cache` is true, and return the token in `BackendOptimizationResult.followup_cache_key`.

Then the follow-up entry points can check for the token first and reuse:

- regime filter
- Kaufman stops

That is still a meaningful latency improvement even before full matrix reuse is implemented.

### Why this matters

This closes the user workflow loop without requiring a full runtime redesign.

---

## 6.9 Add a clean `GpuVdps` controller abstraction

### Why

`GpuVdps` currently uses GPU evaluation but CPU-side control structures:

- `Vec<(u16,u16)>`
- `BTreeSet`
- CPU ranking / refill logic
- optional GPU refill only for part of the process

The current implementation works, but it is not a VRAM-resident search controller.

### Edit location

`double_ma.rs:3609-4224`

### Recommendation

Do not try to jump straight to a full GPU controller in one patch.  
Introduce an interface first, then keep the current logic as the host implementation.

```rust
trait VdpsFrontierEngine {
    fn initial_axes(
        &mut self,
        fast_periods: &[u16],
        slow_periods: &[u16],
        total_pairs: usize,
        target_pairs: usize,
    ) -> Result<(Vec<u16>, Vec<u16>), String>;

    fn local_axes(
        &mut self,
        seed_pairs: &[(u16, u16)],
        fast_periods: &[u16],
        slow_periods: &[u16],
        fast_window: u16,
        slow_window: u16,
        max_period_candidates: usize,
    ) -> Result<(Vec<u16>, Vec<u16>), String>;

    fn refill_seeds(
        &mut self,
        elite_seeds: &[(u16, u16)],
        rung_fast: &[u16],
        rung_slow: &[u16],
        cfg: &VdpsResolvedConfig,
        fast_step_base: u16,
        slow_step_base: u16,
        rng: &mut StdRng,
    ) -> Result<Vec<(u16, u16)>, String>;
}
```

Then:

- `HostVdpsFrontierEngine` = current implementation
- later, in adjacent runtime/kernel work, add `CudaVdpsFrontierEngine`

### Why this matters

This keeps the current behavior deterministic and testable while creating a clean seam for device-side frontier orchestration later.

---

## Secondary direct edits worth doing in `double_ma.rs`

These are not the first four changes, but they are still worthwhile.

### A. Memoize MA batch fallback capability

`ma_matrix_cuda_host_f32` (`7997-8107`) tries a batch sweep and falls back to per-period calls on error.  
If the same MA/param combination always fails batch sweep, repeated errors cost time.

A small memo can avoid that repeated failure path:

```rust
static CUDA_MA_BATCH_CAPABILITY: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
```

Key by `(ma_type, params signature, ma_source requires_candles)`.

### B. Hoist `prices_f32` once

`optimize_double_ma_gpu_sweep_tiled` creates `prices_f32` per run (`8193`).  
With `StaticEvalContext`, it becomes a one-time allocation per optimization run.

### C. Use the active regime profile helper instead of repeated `uses_*` chains

This helps in:

- `regime_filter_expected`
- device-regime selection
- residency summary labeling

---

## Suggested `double_ma.rs` integration sketch

Below is a more complete sketch of what the main function should look like after the refactor. This is intentionally illustrative rather than fully drop-in.

```rust
pub fn run_double_ma_optimization_blocking_with_candles(
    req: DoubleMaRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let requested_backend = req.backend.clone();
    let normalized_backend = requested_backend.clone().with_preferred_cuda_device()?;

    let fast_ma_types = req.fast_ma_types;
    let slow_ma_types = req.slow_ma_types;
    let ma_source = req.ma_source;
    let strategy = req.strategy;
    let halftrend_filter = req.halftrend_filter;

    validate_strategy_cfg_for_optimization(&strategy, halftrend_filter.is_some())?;

    let fast_ma = fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = slow_ma_types[0].trim().to_ascii_lowercase();

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);

    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);
    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let export_all_csv_path = normalize_optional_path(req.export_all_csv_path);

    let route_plan_summary = build_indicator_route_plan_summary(&fast_ma, &slow_ma);
    let kernel_viable = gpu_kernel_blockers_for_request(
        &fast_ma,
        &slow_ma,
        &strategy,
        halftrend_filter.is_some(),
        req.objective,
        top_k,
        include_all,
    )
    .is_empty();

    let mode_used = resolve_optimization_mode(crate::double_ma_mode::ModeSelectionContext {
        requested_mode: req.mode,
        include_all,
        heatmap_bins,
        export_all_csv_path: export_all_csv_path.as_deref(),
        total_combos: total_pairs,
        gpu_execution: req.gpu_execution,
        gpu_kernel_viable: kernel_viable,
    });

    let strict_gpu_blockers = crate::double_ma_backend::strict_gpu_route_blockers(
        &fast_ma,
        &slow_ma,
        &strategy,
        halftrend_filter.is_some(),
        req.objective,
        top_k,
        include_all,
    );

    let mut backend = resolve_backend(...)?;
    enforce_gpu_execution_policy(req.gpu_execution, &backend, &strict_gpu_blockers)?;

    let mut host_caches = RunHostCaches::default();
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut device_regime_caches = RunDeviceRegimeCaches::default();

    let static_ctx = build_static_eval_context(
        candles,
        &ma_source,
        &strategy,
        ensure_combined_regime_filter_cached(
            &mut host_caches.combined_regime,
            candles,
            &ma_source,
            &strategy,
            halftrend_filter.as_ref(),
            candles.close.len(),
        )?,
    )?;

    // ... run mode, merge result, build residency summary, optionally store follow-up cache ...
}
```

---

## Test plan by file

### `commands_optimize.rs`

Add/replace tests for:

- non-grid mode preserved when output shape allows it
- grid-only output requests still normalize to `Grid`

### `double_ma_mode.rs`

Add tests for:

- `Auto + StrictGpuResident + kernel viable + large combos -> GpuVdps`
- `Auto + exact-grid outputs -> Grid`
- `Auto + no GPU pressure -> existing Grid/CoarseToFine behavior`

### `double_ma_backend.rs`

Add tests for:

- `should_cpu_stage_indicator` returns `false` for kernel-capable CPU-preferred indicators under `PreferGpuResident`
- `strict_gpu_route_blockers` includes CPU-staged non-kernel-capable indicators
- enriched route summary populates `staging_reason`

### `double_ma_result.rs`

Add tests for:

- non-grid result returns no `matrix_index`, `best_col`, `top_cols`
- result builder carries `residency` and `followup_cache_key`

### `double_ma_strategy.rs`

Add tests for:

- exactly one active regime profile is returned for each supported profile
- multiple profiles return an error
- `Halftrend + RegimeHybrid` is rejected via the profile helper path too

### `double_ma.rs`

Add tests for:

1. **strict GPU enforcement**
   - strict policy rejects `GpuOnly`
   - strict policy rejects CPU fallback
   - strict policy accepts kernel/hybrid when route is executable

2. **hybrid residency override**
   - a kernel-capable but CPU-preferred MA does not CPU-stage under `PreferGpuResident`
   - `cpu_indicator_prep_ms == 0` for that case

3. **specialized regime caching**
   - PIT/SET/MTS/VFCT/ORC regime builder is called once across MA-param variants
   - the cache is reused across coarse/fine phases

4. **PIT device-native path**
   - EFI/BOP/VOSC/CFO return device matrices
   - ACOSC fallback path remains deterministic

5. **follow-up reuse**
   - drilldown accepts `followup_cache_key`
   - local stability accepts `followup_cache_key`
   - cached regime filter and Kaufman stops are reused

6. **product parity**
   - `GpuVdps` app-path result comes through without product-layer normalization destroying it

---

## Perf-gate and observability plan

The attached files already expose useful timing fields, especially `HybridTimingMs`.  
I would formalize three classes of perf gate.

### 1. Route correctness gates

Fail GPU-first runs when:

- `backend_route` resolves to CPU
- `backend_route` resolves to `GpuOnly` under strict policy
- `backend_fallback_reason` is populated under strict policy

### 2. Residency gates

Fail warm GPU-resident runs when:

- `cpu_indicator_prep_ms > 0` for kernel-capable CPU-preferred indicators
- `h2d_upload_ms` remains large for the same request on warm runs
- `residency.fast_ma_resident == false` or `residency.slow_ma_resident == false`

### 3. Recompute gates

Track and regress on:

- repeated specialized regime builder creation within one optimization run
- repeated `compute_kaufman_stop_series` and `compute_log_returns_f64` across MA-param variants
- follow-up recomputation after a just-finished optimization

---

## Adjacent-file dependencies that are not in this bundle

The plan above is intentionally centered on the attached files, but a few deeper items do require adjacent modules.

### 1. Generic device regime filter

Needed to eliminate the remaining host combined-regime path in kernel/hybrid execution.  
This will require adjacent work in `gpu_backtest_kernel.rs` and possibly the dynamic-indicator/device-dispatch layer.

### 2. Persistent device matrix cache

To keep matrices VRAM-resident across requests and follow-ups, you will need adjacent runtime/cache work (likely the modules mentioned in the review map under GPU runtime / residency / VRAM cache).

### 3. Device-side `GpuVdps` controller

The abstraction can be added in `double_ma.rs` now, but the actual device implementation will need adjacent kernel/runtime code.

### 4. Frontend / request-builder updates

Not in the bundle, but required if you want the product path to:
- expose `GpuExecutionPolicy`
- send non-grid modes intentionally
- tolerate `matrix_index = None` for non-grid results
- preserve `followup_cache_key` into drilldown/local-stability requests

---

## Risk register

### Low-risk / high-ROI

- stop forcing `Grid`
- add request/result policy metadata
- hoist static eval context
- cache specialized device regime builders
- use backend-aware matrix generation in follow-ups

### Medium-risk

- strict GPU enforcement (behavioral change)
- hybrid CPU-preferred override for kernel-capable indicators
- enriched residency telemetry

### Higher-risk

- PIT device-native conversion for ACOSC parity
- generic device regime filter
- device-side `GpuVdps` frontier controller
- persistent VRAM caches across requests

---

## Recommended implementation order

### Step 1
`commands_optimize.rs`, `double_ma_mode.rs`, `double_ma_result.rs`

Goal:
- unblock non-grid modes at the product boundary
- add policy/result plumbing without changing the executor much

### Step 2
`double_ma_backend.rs`, `double_ma_strategy.rs`

Goal:
- centralize route classification
- add strict GPU and CPU-stage override helpers
- stop scattering `uses_*` checks

### Step 3
`double_ma.rs` — static eval context + run caches

Goal:
- remove repeated invariant recomputation
- cache specialized device regimes

### Step 4
`double_ma.rs` — hybrid residency behavior

Goal:
- override CPU-preferred staging when kernel-capable
- enforce strict GPU behavior cleanly

### Step 5
`double_ma.rs` — PIT cleanup

Goal:
- make PIT closer to SET/MTS/ORC/VFCT in device behavior

### Step 6
`double_ma.rs` — follow-up reuse and `GpuVdps` abstraction

Goal:
- improve workflow continuity
- create clean seam for later device-side frontier control

---

## Concrete “done” criteria

I would consider this bundle done for the first phase when all of the following are true:

1. The Tauri command path no longer destroys non-grid modes when outputs allow them.
2. A request can explicitly say `StrictGpuResident`.
3. `StrictGpuResident` never silently resolves to `CpuOnly` or `GpuOnly`.
4. Hybrid no longer CPU-stages kernel-capable indicators just because a heuristic prefers CPU.
5. Specialized device regime filters are built once per optimization run, not once per MA-param run.
6. PIT no longer stages EFI/BOP/VOSC/CFO through host memory.
7. Results include residency metadata and an optional follow-up cache key.
8. Drilldown/local stability can reuse optimization-produced follow-up state and accept an optional backend hint.

---

## Closing recommendation

If the near-term objective is “make the current code materially more GPU-first without a redesign”, the best return is:

1. **policy + product unblocking**
2. **run-scoped caches**
3. **hybrid CPU-preferred override**
4. **PIT device-native cleanup**

If the objective is “true VRAM-resident end-to-end workflow”, the attached files should implement the policy surface, cache hooks, and result metadata now, while adjacent runtime/kernel work supplies:

- persistent device matrices
- generic device regime filters
- device-side `GpuVdps` frontier management

That sequence keeps the work incremental, testable, and aligned with the code that already exists instead of forcing a broad rewrite.
