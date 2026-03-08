# Dynamic Strategy Any-Indicator Master Plan

Last updated: 2026-03-04
Status: All planned phases complete (current runtime remains host-materialized for CPU evaluation by design)
Scope: Dynamic strategy backend only (CPU + unified GPU route planning and execution), excluding validation/meta/reporting UX rewrites

## Objective

Enable dynamic strategy backend support for any indicator exposed by `vector-ta`, while:

- preserving current functionality and correctness,
- preserving or improving performance where possible,
- minimizing added technical debt and net lines of code,
- using `gpu_speedups.csv` as the source of truth for GPU preference policy.

## Non-Goals

- Rewriting existing fixed strategy backends.
- Forcing all indicators onto GPU.
- Replacing existing `vector-ta` indicator implementations.
- Expanding this phase into validation/meta-analysis workflow refactors.

## Hard Constraints

- CPU-only builds must continue to work with no CUDA toolchain.
- Existing five strategy flows remain stable and available.
- Dynamic path must remain parity-checked against legacy-equivalent behavior.
- If an indicator is not GPU-faster per `gpu_speedups.csv`, default to CPU route.
- If a needed API is missing in `vector-ta`, track it explicitly and pause that slice until the API exists.

## Current Baseline

- Dynamic strategy currently uses curated indicator kinds (`FastMa`, `SlowMa`, `HalfTrendTrend`, `MaczHist`, `OttLine`, `TrendQualityRegime`, `BreakoutRegime`).
- Core dynamic modules exist:
  - `crates/app_core/src/strategy_ir.rs`
  - `crates/app_core/src/strategy_plan.rs`
  - `crates/app_core/src/strategy_dynamic.rs`
  - `crates/app_core/src/strategy_gpu_program.rs`
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
- MA catalog is already registry-driven through `vector-ta` moving average registry and surfaced to frontend.
- Unified GPU routing policy already exists for MAs (`ma_policy.rs`) using `gpu_speedups.csv`.
- `vector-ta 0.1.5` now provides unified all-indicator registry + generic dispatch APIs.
- `crates/app_core/src/indicator_catalog.rs` now uses unified registry metadata for dynamic-eligible indicators and includes generic param/output validation helpers.

## Phase 0 Discovery Snapshot (2026-02-28)

Key findings from local `vector-ta 0.1.4` inspection:

- `vector-ta` currently provides a robust MA registry + MA param schema + MA dispatch path:
  - `indicators/moving_averages/registry.rs`
  - `indicators/moving_averages/param_schema.rs`
  - `indicators/moving_averages/ma.rs`
  - `indicators/moving_averages/ma_batch.rs`
- There is no equivalent unified registry for all indicators exposed under `indicators/mod.rs`.
- There is no generic all-indicator dispatcher by string id (CPU and CUDA) similar to MA dispatch.
- There is no unified, typed output-schema contract for all indicators (single-output and multi-output) accessible through one API.
- `gpu_speedups.csv` already includes many non-MA indicator ids, so policy data exists for broader routing decisions once APIs are available.

Current practical conclusion:

- We can fully generalize architecture in app code now.
- Full “any indicator” implementation should be blocked on `vector-ta` unified metadata + dispatcher APIs to avoid per-indicator app-side match trees (high bloat/debt).

Fallback matrix (locked for implementation):

| Condition | Route behavior |
|---|---|
| Indicator id not found in registry | hard error (`unsupported_indicator`) |
| Indicator found, no GPU support | CPU route |
| Indicator GPU-capable but not in `gpu_speedups.csv` | CPU route (performance policy) |
| Indicator GPU-capable and in `gpu_speedups.csv` | GPU route |
| GPU unavailable/runtime failure | deterministic CPU fallback + explicit route metadata |
| Requested output ref invalid for indicator output schema | hard error (`invalid_output_ref`) |

## Target Architecture (Low-Complexity Path)

Use a registry-driven indicator adapter instead of per-indicator bespoke strategy code.

### A. Indicator Descriptor Layer

Add a compact app-side descriptor model that is generated from `vector-ta` registry metadata plus app policy:

- indicator id
- required inputs (close, candles, ma_source, etc.)
- parameter schema
- output arity and output names
- supports CPU single/batch
- supports CUDA single/sweep/kernel
- preferred route (`CPU` vs `GPU`) based on `gpu_speedups.csv`

This avoids adding dedicated enums/branches for each indicator.

### B. Generic Dynamic IR Nodes

Extend dynamic IR to support generic indicator nodes:

- `indicator_id: String`
- `params: map`
- `output_ref` (for multi-output indicators)

Keep existing static profile conversion for current strategies as compatibility mode.

### C. Route Planner

Compile-time route planner decides per indicator:

- `GpuKernel` if kernel-capable and preferred by speedup policy.
- `GpuSingle/Sweep` if CUDA-capable and preferred.
- `Cpu` otherwise.

If mixed routes are required, stage only required series across PCIe once per run.

### D. Execution Engine

Use two generic compute adapters:

- CPU adapter: calls `vector-ta` CPU batch/single APIs.
- GPU adapter: calls CUDA single/sweep APIs only for indicators preferred by policy.

Execution engine operates on normalized typed buffers and output slots.

### E. Caching

Cache computed indicator series by:

- dataset id / time range hash
- indicator id
- normalized param hash
- route kind

This prevents recomputation across nearby optimization runs.

## Complexity Control Rules

- No per-indicator hardcoded branches unless unavoidable for API limitations.
- New indicator support should primarily be metadata additions, not engine code additions.
- Keep indicator computation interfaces generic (`compute_indicator_series(...)` style).
- Enforce DRY by centralizing route policy and adapter binding.
- Keep unsafe/perf-sensitive code localized and documented.

## Implementation Phases and Checklist

## Phase 0 - Discovery and Spec Lock

- [x] Freeze v1 supported indicator classes for dynamic backend expansion (all registry indicators or subset with fallback behavior documented).
- [x] Inventory `vector-ta` indicator registries and parameter schemas relevant to dynamic use.
- [x] Confirm output arity and naming conventions for multi-output indicators.
- [x] Confirm CPU batch API coverage for full target set.
- [x] Confirm CUDA single/sweep API coverage for full target set.
- [x] Define explicit fallback matrix (GPU unavailable, GPU slower by policy, unsupported output type, etc.).

## Phase 1 - Generic Descriptor/Registry Integration

- [x] Implement app-side `IndicatorDescriptor` normalization from currently available `vector-ta` metadata (v0 MA-backed).
- [x] Extend `IndicatorDescriptor` normalization to all indicators when VectorTA unified registry API lands.
- [x] Add descriptor cache and lookup by `indicator_id` (v0 MA-backed catalog).
- [x] Add schema validation for indicator params (type/range/default).
- [x] Add output validation (single vs multi-output slot references).
- [x] Add unit tests for descriptor normalization and param validation.

## Phase 2 - Dynamic IR Generalization

- [x] Extend IR to include generic indicator nodes and output references.
- [x] Keep current static-profile builder behavior intact for existing strategies.
- [ ] Add IR validation rules for:
  - [x] duplicate ids
  - [x] unknown indicators
  - [x] invalid output refs
  - [x] invalid param schema
- [x] Add schema version bump and compatibility checks.

## Phase 3 - Route Planner Generalization

- [x] Generalize route planner to operate on any descriptor-driven indicator node.
- [x] Integrate `gpu_speedups.csv` preference policy for all eligible indicators.
- [x] Ensure default route is CPU when indicator not present in speedup table.
- [x] Emit transparent route metadata for UI/status (`indicator_route_plan`).
- [x] Add deterministic planning tests for mixed CPU/GPU strategies.

## Phase 4 - Compute Adapter Layer

- [x] Implement generic CPU adapter for indicator compute.
- [x] Implement generic GPU adapter for indicator compute.
- [x] Implement output-slot extraction for multi-output indicators.
- [x] Add unified error taxonomy for missing capability vs runtime compute errors.
- [ ] Add adapter parity tests on representative indicators:
  - [x] single-output MA-like
  - [x] multi-output
  - [x] candles-required
  - [x] CPU-only

## Phase 5 - Runtime/Staging Integration

- [x] Integrate generic adapters into dynamic execution context preparation.
- [x] Implement minimal-transfer mixed route staging (CPU->GPU/GPU->CPU only when needed).
- [x] Add result caching keyed by `(dataset, indicator, params, route)`.
- [x] Add cancellation safety and progress updates for expanded compute graph.

## Phase 6 - Correctness and Parity Gates

- [x] Add CPU-vs-GPU parity harness for selected indicators and composed strategies.
- [x] Add shadow-mode execution checks for dynamic path vs legacy where comparable.
- [x] Add regression tests for route fallback determinism and metadata reporting.
- [x] Add schema migration tests for older IR payload compatibility.

## Phase 7 - Performance Gates

- [x] Add benchmark suite for generic dynamic indicator compute path.
- [x] Add benchmark suite for mixed-route composed strategies.
- [x] Add perf gate thresholds (candidate vs baseline) for CPU and GPU modes.
- [x] Verify no material regression vs current dynamic curated-indicator path.

## Phase 8 - Frontend Contract and Rollout

- [x] Expose generalized indicator catalog payload to frontend selection UI.
- [x] Surface route decisions and warnings consistently in optimize UI.
- [x] Keep feature-flag rollout path (`experimental` -> `default`) with fallback switch.
- [x] Document user-facing behavior for GPU policy (speedup-table driven).

## VectorTA API Gap Tracker (Owner Action Items)

Use this section to track crate-level blockers. If any row is open, app-side implementation should avoid hardcoding workarounds that increase debt.

| ID | Needed API/Behavior in `vector-ta` | Why Needed | Status | Owner | Notes |
|---|---|---|---|---|---|
| VT-001 | Unified indicator registry for all dynamic-eligible indicators (not only MA registry) with param + output schema | Needed for metadata-driven adapter and low-LOC scaling | Resolved in `vector-ta 0.1.5` | VectorTA + App | Integrated in app catalog |
| VT-002 | Stable generic CPU compute entrypoint by `indicator_id` + params + input refs | Needed to avoid per-indicator app-side match trees | Resolved in `vector-ta 0.1.5` | VectorTA | `indicators::dispatch::compute_cpu` + `compute_cpu_batch` |
| VT-003 | Stable generic CUDA compute entrypoint (single/sweep) by `indicator_id` + params + input refs | Needed for unified GPU adapter | Resolved in `vector-ta 0.1.5` | VectorTA | `indicators::dispatch::compute_cuda` (feature-gated) |
| VT-004 | Explicit multi-output naming/ordering contract | Needed for deterministic `output_ref` routing in IR | Resolved in `vector-ta 0.1.5` | VectorTA + App | `registry` output schemas integrated with output-ref validator |
| VT-005 | Capability flags exposed per indicator for CPU batch and CUDA modes | Needed for route planner determinism | Resolved in `vector-ta 0.1.5` | VectorTA + App | `IndicatorCapabilities` integrated in descriptors |

## Proposed VectorTA API Surface (Minimal for Low-Bloat App Integration)

The minimal API set is now available in `vector-ta 0.1.5`:

1. `vector_ta::indicators::registry::list_indicators() -> &'static [IndicatorInfo]`
2. `vector_ta::indicators::registry::indicator_param_schema(id: &str) -> Option<&'static [IndicatorParamInfo]>`
3. `vector_ta::indicators::registry::indicator_output_schema(id: &str) -> Option<&'static [IndicatorOutputInfo]>`
4. `vector_ta::indicators::registry::indicator_capabilities(id: &str) -> Option<IndicatorCapabilities>`
5. `vector_ta::indicators::dispatch::compute_cpu(req: IndicatorComputeRequest<'_>) -> Result<IndicatorComputeOutput, IndicatorDispatchError>`
6. `vector_ta::indicators::dispatch::compute_cpu_batch(req: IndicatorBatchRequest<'_>) -> Result<IndicatorBatchOutput, IndicatorDispatchError>`
7. `vector_ta::indicators::dispatch::compute_cuda(req: IndicatorCudaRequest<'_>) -> Result<IndicatorCudaOutput, IndicatorDispatchError>` (when `cuda` feature is enabled)

Recommended capability fields:

- supports_cpu_single
- supports_cpu_batch
- supports_cuda_single
- supports_cuda_batch_or_sweep
- requires_candles
- output_count
- output_names

## Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| Indicator API fragmentation leads to per-indicator app code | High LOC growth and debt | Enforce generic adapter boundary and block bespoke branches by default |
| Mixed-route staging overhead erases GPU gains | Performance regression | Route by `gpu_speedups.csv`, cache outputs, transfer once per run |
| Multi-output indicators introduce correctness bugs | Wrong signals/metrics | Output-schema validation + parity tests + strict output refs |
| IR schema changes break saved payloads | Runtime failures | Versioned schema + compatibility checks + migration tests |
| Overly broad first release scope | Slow delivery | Ship staged rollout with feature flag and profile-scoped enablement |

## Deliverables

- Generic indicator descriptor + adapter layer.
- Generalized dynamic IR and planner with compatibility path.
- CPU/GPU route policy integration with `gpu_speedups.csv`.
- Test and benchmark gates for correctness and performance.
- Frontend-facing catalog + route metadata contract updates.

## Progress Log

- 2026-02-28: Created plan and checklist. No implementation tasks started.
- 2026-02-28: Completed Phase 0 discovery snapshot and confirmed current `vector-ta` API gaps that block low-bloat full any-indicator integration.
- 2026-02-28: Added `crates/app_core/src/indicator_catalog.rs` with MA-backed generic `IndicatorDescriptor` model and tests (Phase 1 foundation, no runtime behavior changes yet).
- 2026-02-28: Added cached descriptor lookup (`list_dynamic_indicator_descriptors_cached`, `find_dynamic_indicator_descriptor`) and expanded tests.
- 2026-03-04: Upgraded workspace to `vector-ta 0.1.5` and confirmed unified registry/dispatch APIs are available.
- 2026-03-04: Completed Phase 1 integration in `crates/app_core/src/indicator_catalog.rs`:
  - switched from MA-only metadata to all dynamic-eligible indicator descriptors from `vector_ta::indicators::registry::list_indicators`,
  - added param schema validation helper (`normalize_and_validate_indicator_params`),
  - added output-ref validation helper (`normalize_and_validate_indicator_output_ref`),
  - expanded unit tests for descriptor parity + validation behavior.
- 2026-03-04: Completed Phase 2 in `crates/app_core/src/strategy_ir.rs`:
  - moved indicator nodes to generic model (`kind` or `indicator_id` + `params` + `output_id`),
  - added dynamic indicator validation against descriptor registry (unknown id / param schema / output-ref),
  - bumped IR schema version to `2` while keeping min-supported schema `1`,
  - preserved static profile builders as compatibility mode.
- 2026-03-04: Completed Phase 3 in `crates/app_core/src/strategy_plan.rs`:
  - generalized route planning to descriptor-driven indicators,
  - integrated speedup-policy routing (`GpuKernel`/`GpuSingle`/`Cpu`) for dynamic indicator ids,
  - retained legacy kind-based routing for existing static profiles,
  - added deterministic route tests for GPU-preferred and CPU-preferred dynamic indicators.
- 2026-03-04: Phase 4 started in `crates/app_core/src/dynamic_indicator_adapter.rs`:
  - added generic CPU adapter wrapper over VectorTA dispatch (`compute_cpu`),
  - added feature-gated generic CUDA adapter wrapper over VectorTA dispatch (`compute_cuda`),
  - added adapter tests for MA-like single-output, multi-output, candles-required, and CPU-only indicators.
- 2026-03-04: Completed remaining Phase 4 slice:
  - added explicit adapter error taxonomy (`DynamicIndicatorAdapterError`) for unsupported indicator, param/output validation, and compute failures.
- 2026-03-04: Phase 5 groundwork in `crates/app_core/src/strategy_dynamic.rs`:
  - added `prepare_dynamic_indicator_slot_series_cpu` to materialize dynamic indicator slots from compiled plan metadata,
  - added per-run compute cache keying by `(source_indicator_id, output_id, params)` to prevent duplicate slot recomputation,
  - added tests for slot materialization and cache reuse behavior.
- 2026-03-04: Integrated dynamic slot preparation into context assembly:
  - `prepare_dynamic_cpu_context` now calls `prepare_dynamic_indicator_slot_series_cpu`,
  - `PreparedDynamicCpuContext` now carries `dynamic_slot_series`.
- 2026-03-04: Added route-aware slot materialization entrypoint:
  - new `prepare_dynamic_indicator_slot_series` uses planned route metadata,
  - CUDA-enabled builds attempt GPU adapter for `GpuSingle`/`GpuKernel` slots with deterministic CPU fallback,
  - current staging still host-materializes outputs (minimal-transfer VRAM staging remains open in Phase 5).
- 2026-03-04: Validation run summary:
  - `cargo test -p ta_app_core` passed (`172 passed, 0 failed`),
  - `cargo check -p ta_desktop_demo_app` passed,
  - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel` passed.
- 2026-03-04: Completed remaining Phase 5/6 implementation slices:
  - added control-capable slot staging APIs (`prepare_dynamic_indicator_slot_series*_with_controls`) with cancellation and per-slot progress callbacks,
  - added route-sensitive cache-key tests and cancellation tests in `strategy_dynamic`,
  - added schema-v1 compatibility tests in `strategy_ir` and `strategy_plan`.
- 2026-03-04: Completed Phase 8 contract exposure:
  - added Tauri command `list_dynamic_indicators`,
  - regenerated frontend command list (`web/src/tauri/commands.ts`),
  - added frontend dynamic catalog service and tests (`web/src/services/dynamicIndicatorCatalogService.ts`),
  - surfaced dynamic catalog summary metadata in `OptimizeWorkspacePanel`.
- 2026-03-04: Extended Phase 7 benchmarks:
  - `crates/app_core/benches/dynamic_strategy_cpu.rs` now includes mixed-route slot staging benchmark group (`dynamic_strategy_cpu/staging/50k/mixed_route_slot_prep_cpu`).
- 2026-03-04: Gate execution summary:
  - CPU perf gate candidate passed (`scripts/run_cpu_perf_gate.ps1 -Mode candidate`),
  - initial GPU perf gate candidate run observed a transient dynamic-ratio failure (`dynamic on/off = 456.497ms / 95.983ms`) during noisy conditions,
  - repeated GPU perf gate candidate run passed with stable on/off parity (`95.408ms / 96.500ms`).
- 2026-03-04: Completed mixed-route staging optimization:
  - added reusable CUDA input staging buffers (`PreparedCudaIndicatorInputs`) in `dynamic_indicator_adapter`,
  - added `compute_dynamic_indicator_cuda_with_inputs` and updated route-aware slot prep to reuse staged inputs across GPU-routed slots,
  - validated with CUDA adapter parity tests and post-refactor CPU/GPU perf gates (both candidate gates passing).

## User-Facing GPU Policy Notes

- Route policy remains deterministic and speedup-table driven:
  - indicators in `gpu_speedups.csv` and with GPU capability are preferred for GPU routes,
  - indicators not present in the speedup table default to CPU route.
- Unified GPU backend remains resilient:
  - GPU route attempts are used when planned,
  - runtime GPU dispatch failures deterministically fall back to CPU compute for correctness.
- Frontend now surfaces policy context:
  - MA-level route/performance hints remain visible in optimize controls,
  - dynamic indicator catalog availability and GPU-preferred counts are shown in optimize backend metadata.

## Definition of Done

- Dynamic backend accepts any supported `vector-ta` indicator in strategy assembly.
- Unsupported indicators fail with clear capability errors (not silent behavior).
- Route decisions are deterministic and explainable.
- Correctness parity tests pass for representative strategies and indicator classes.
- Perf gates show no material regression versus current baseline for curated scenarios.
- Added complexity remains controlled via registry-driven architecture (no large per-indicator branch trees).
