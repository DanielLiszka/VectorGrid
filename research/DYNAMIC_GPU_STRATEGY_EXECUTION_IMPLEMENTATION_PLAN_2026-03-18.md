# Dynamic GPU Strategy Execution Implementation Plan

Last updated: 2026-03-18
Scope: `ta_desktop_demo` conventional optimization/backtest stack
Goal: add a real dynamic GPU strategy execution path by turning the existing compiled strategy plan and GPU predicate program into a runtime contract executed by one generic GPU kernel path.

## 1. Executive Summary

The repo already has most of the compiler-side pieces for dynamic strategy execution:

- `StrategyPlanBackend::{Cpu, GpuUnified}` in `crates/app_core/src/strategy_plan.rs`
- `IndicatorRouteKind::{Cpu, GpuKernel, GpuSingle}` in `crates/app_core/src/strategy_plan.rs`
- `CompiledStrategyPlan` and `compile_strategy_plan(...)` in `crates/app_core/src/strategy_plan.rs`
- `compile_dynamic_plan_for_double_ma_config(...)` in `crates/app_core/src/strategy_dynamic.rs`
- `CompiledGpuPredicateProgram` and `compile_gpu_predicate_program(...)` in `crates/app_core/src/strategy_gpu_program.rs`
- a mature resident-data CUDA execution substrate in `crates/app_core/src/gpu_backtest_kernel.rs`

The missing piece is not "a scripting engine" and it is not "NVRTC-first specialization."
The missing piece is a runtime bridge that turns:

- `CompiledStrategyPlan`
- `CompiledGpuPredicateProgram`

into a reusable GPU executable consumed by a generic program-driven backtest kernel.

That should be the v1 implementation target.

## 2. Repo-Specific Read

The current codebase already supports:

- CPU dynamic-plan execution as a correctness path in `crates/app_core/src/strategy_dynamic.rs`
- dynamic Double-MA plan compilation in `crates/app_core/src/double_ma.rs`
- GPU dynamic preflight compilation in `crates/app_core/src/double_ma.rs`
- backend capability/blocker logic in `crates/app_core/src/double_ma_backend.rs`
- semantics gating like `trade_on_next_bar=true` in `crates/app_core/src/double_ma_strategy.rs`
- CUDA PTX build and loading from `crates/app_core/build.rs`, `crates/app_core/src/kernels/double_crossover.cu`, and `crates/app_core/src/gpu_backtest_kernel.rs`

The current gap is:

- the GPU side compiles a predicate program as a preflight check
- but there is no generic GPU runtime executable object and no generic program-driven kernel that consumes that program

That means the compiler half is ahead of the runtime half.

## 3. What This Feature Is And Is Not

This feature is:

- dynamic strategy execution inside the conventional optimizer/backtester
- based on compiled strategy plans, not free-form scripts
- a backend/runtime feature first
- a path that should coexist with existing specialized kernels

This feature is not:

- an ML scoring system
- a scripting language project
- an NVRTC-first JIT compiler project
- a mandate to replace current specialized Double-MA-family GPU kernels

## 4. Core Decision

Do not start with:

- arbitrary scripting
- a separate ad hoc dynamic strategy format
- NVRTC specialization

Start with this runtime contract:

1. UI / request / builder or headless authoring
2. `StrategyIr`
3. `CompiledStrategyPlan` with `StrategyPlanBackend::GpuUnified`
4. `CompiledGpuPredicateProgram`
5. uploaded GPU executable
6. generic GPU backtest kernel

That is the shortest path that matches the current repo.

## 5. Valid V1 Scope

V1 should be deliberately narrow.

Supported v1 execution subset:

- long/short entry and exit logic from compiled condition groups
- `long_only`
- `allow_flip`
- `trade_on_next_bar`
- `stop_loss_pct`
- `take_profit_pct`
- optional regime gating only when the regime series already exists on device or is already part of the staged runtime path

Explicitly out of scope for v1 GPU dynamic execution:

- Kaufman stop exit
- execution overlays like cooldown and persistence
- Sortino and Omega objectives in GPU dynamic mode
- unsupported MAs and indicators already blocked by current kernel-path checks
- every strategy family in the repo on day one
- NVRTC specialization
- graphical strategy-builder UI as the first milestone

This scope matches the current blocker logic in `crates/app_core/src/double_ma_backend.rs` and the semantic gatekeeping in `crates/app_core/src/double_ma_strategy.rs`.

## 5.1 User-defined v1 product shape

The repo already has a generic-enough IR, but the user-facing v1 should still be narrower than the full IR surface.

Recommended user-facing v1:

- one new strategy family in the optimize workspace
- working name: `DynamicDoubleMa`
- backend contract still uses `StrategyIr`
- IR `profile` remains `DoubleMa` in v1
- authoring surface is a constrained builder or JSON import that emits valid `StrategyIr`

Why this is the right first product shape:

- it fits the current `DoubleMa` optimization engine and sweep model
- it reuses current semantics and route expectations
- it avoids pretending the repo can safely expose arbitrary cross-profile custom strategies on day one

## 5.2 Guard rails to keep

The repo already has hard IR validation in `crates/app_core/src/strategy_ir.rs`.
That should remain the backend hard-stop layer.

Existing backend hard caps:

- `IR_MAX_INDICATORS = 64`
- `IR_MAX_CONDITIONS = 256`
- `IR_MAX_GROUPS = 64`
- `IR_MAX_GROUP_CONDITIONS = 64`

Recommended stricter product caps for v1 user-defined strategies:

- max indicators exposed in UI/import assist: `12`
- max conditions exposed in UI/import assist: `24`
- max groups exposed in UI/import assist: `8`
- max conditions per group exposed in UI/import assist: `8`

Reason:

- backend hard caps protect safety
- lower product caps keep GPU dynamic execution predictable and the builder understandable

## 5.3 Allowed v1 authoring surface

Allow only:

- `StrategyIr.profile = DoubleMa`
- one fast MA spec
- one slow MA spec
- single-series indicator slots only
- condition ops already supported by `ConditionOp`:
- `gt`
- `ge`
- `lt`
- `le`
- `cross_up`
- `cross_down`
- condition groups using `all` or `any`
- signal bindings for `long_entry_group`, `long_exit_group`, `short_entry_group`, and `short_exit_group`
- execution policy fields already validated through `StrategyConfig`

Disallow in v1 user-defined GPU mode:

- matrix or multi-row indicator outputs
- indicators requiring ambiguous output selection
- any dynamic indicator node that cannot normalize to one scalar series per bar
- custom user-provided functions
- custom code snippets
- arbitrary textual scripts

## 5.4 Binding rules for v1

The user-facing builder or import validator should enforce these higher-level rules in addition to backend IR validation:

- long-only mode requires both `long_entry_group` and `long_exit_group`
- bidirectional mode requires all four bindings
- at least one entry group must be bound
- empty groups are invalid
- duplicate IDs are invalid
- MA types and dynamic indicator IDs must come from the catalog, not unconstrained free text

These are product guard rails, not replacements for backend IR validation.

## 6. Architecture To Build

### 6.1 Correctness model

Formalize the execution roles like this:

- legacy static evaluator = backward-compatibility reference
- CPU dynamic executor = canonical semantics for dynamic strategies
- GPU dynamic executor = accelerated execution of the same compiled plan

Every new GPU-dynamic shape should first be proven against the CPU dynamic executor before it is accepted as a GPU path.

### 6.2 Runtime objects

Promote the current preflight concept into explicit runtime objects.

Recommended shapes:

```rust
pub struct StrategyHandle {
    pub plan: CompiledStrategyPlan,
    pub capability: StrategyCapabilityReport,
    pub gpu_program: Option<CompiledGpuPredicateProgram>,
}

pub struct GpuStrategyExecutable {
    pub plan_fingerprint_hex: String,
    pub device_id: u32,
    pub abi_version: u32,
    pub d_indicator_slots: DeviceBuffer<GpuIndicatorSlotAbi>,
    pub d_conditions: DeviceBuffer<GpuConditionAbi>,
    pub d_groups: DeviceBuffer<GpuConditionGroupAbi>,
    pub d_group_condition_indexes: DeviceBuffer<u32>,
    pub d_signals: DeviceBuffer<GpuSignalBindingsAbi>,
}
```

These types do not need to be exact names, but the separation does matter:

- `StrategyHandle` is a compiled strategy object at the Rust level
- `GpuStrategyExecutable` is the uploaded device-ready executable

### 6.3 Executable cache key

Cache the device executable by:

- `plan_fingerprint_hex`
- device ID
- kernel ABI version
- any execution flag that changes layout or meaning

Do not cache only by ad hoc request signatures once the executable layer exists.

### 6.4 Pair-invariant vs pair-variant bindings

Every value touched by the compiled plan should be classified as one of:

Pair-invariant:

- resident close/open/high/low/volume-derived series
- `ma_source`
- regime filters
- fixed helper indicators
- any strategy-shape-dependent but grid-independent slot

Pair-variant:

- fast MA sweep matrix
- slow MA sweep matrix
- later swept indicator families

This preserves the architecture already used by `crates/app_core/src/gpu_backtest_kernel.rs`:

- resident market data stays resident
- invariant slots are prepared once
- only sweep-dependent matrices are recomputed per optimization run

## 7. Kernel Design

### 7.1 Keep specialized kernels

Keep the current hard-coded crossover kernels in `crates/app_core/src/kernels/double_crossover.cu`.

They remain:

- the fastest built-in path
- the preferred path for the current supported specialized strategies

### 7.2 Add one generic program-driven kernel path

Add one generic kernel family alongside the specialized path.

Conceptually:

```cpp
extern "C" __global__
void strategy_backtest_tm_program_f32(
    const GpuIndicatorSlotAbi* slots,
    const GpuConditionAbi* conditions,
    const GpuConditionGroupAbi* groups,
    const uint32_t* group_condition_indexes,
    const GpuSignalBindingsAbi* signals,
    const SlotBindingAbi* slot_bindings,
    /* existing market / metrics / flags inputs */
);
```

The kernel should:

- resolve slot values for the current pair and bar
- evaluate condition predicates
- reduce conditions into groups using existing `all` / `any` semantics
- resolve long-entry / long-exit / short-entry / short-exit
- feed those booleans into the existing trade-state and metric accumulation flow

The generic path should reuse the existing metrics engine body and execution semantics.
Do not build a second backtest engine just for dynamic strategies.

### 7.3 ABI requirement

The ABI uploaded to CUDA must be stable and versioned.

Recommended ABI structs:

- `GpuIndicatorSlotAbi`
- `GpuConditionAbi`
- `GpuConditionGroupAbi`
- `GpuSignalBindingsAbi`
- `SlotBindingAbi`

Add an explicit `abi_version` and reject mismatches at runtime.

## 8. Capability And Routing Model

The current `gpu_kernel_blockers_for_request(...)` in `crates/app_core/src/double_ma_backend.rs` should evolve into a structured routing report.

Recommended shape:

```rust
pub struct StrategyCapabilityReport {
    pub gpu_static_supported: bool,
    pub gpu_dynamic_supported: bool,
    pub cpu_dynamic_supported: bool,
    pub reasons: Vec<String>,
}
```

Recommended routing:

- built-in + supported -> existing specialized GPU kernel
- dynamic + supported -> generic program-driven GPU kernel
- dynamic + unsupported -> CPU dynamic executor
- unsupported + unsafe -> explicit rejection

This keeps backend selection legible and testable.

## 9. Existing Files And Their Roles

### 9.1 Compiler and plan files

- `crates/app_core/src/strategy_plan.rs`
  Purpose: canonical compiled-plan schema, route assignment, backend selection, plan fingerprint anchor.
- `crates/app_core/src/strategy_dynamic.rs`
  Purpose: dynamic-plan compilation from existing strategy configs, dynamic indicator slot staging, CPU dynamic evaluation.
- `crates/app_core/src/strategy_gpu_program.rs`
  Purpose: GPU predicate compilation from `CompiledStrategyPlan`.

### 9.2 Semantic gate and routing files

- `crates/app_core/src/double_ma_strategy.rs`
  Purpose: semantic validation such as `trade_on_next_bar=true` and realistic execution constraints.
- `crates/app_core/src/double_ma_backend.rs`
  Purpose: GPU-kernel blocker logic and route decision summary.
- `crates/app_core/src/double_ma.rs`
  Purpose: current optimization entrypoint, CPU dynamic shadow path, and current GPU dynamic preflight seam.

### 9.3 CUDA runtime files

- `crates/app_core/src/gpu_backtest_kernel.rs`
  Purpose: resident market-series cache, PTX loading, pinned uploads, scratch reuse, valid-pair compaction, and kernel dispatch substrate.
- `crates/app_core/src/kernels/double_crossover.cu`
  Purpose: existing specialized backtest kernels and the natural location for one generic program-driven kernel family.
- `crates/app_core/build.rs`
  Purpose: PTX build packaging for `double_crossover.cu`.

## 10. File-By-File Implementation Plan

## 10.1 `crates/app_core/src/double_ma.rs`

Change this file from:

- compile dynamic GPU plan
- compile GPU predicate program
- cache only a preflight signature

to:

- `get_or_build_gpu_strategy_executable(...)`
- explicit plan/program/executable lifecycle
- CPU dynamic executor retained as the correctness oracle
- parity tests that compare GPU dynamic execution with CPU dynamic execution

Concrete tasks:

- replace `maybe_compile_dynamic_gpu_plan_preflight(...)` with executable build/use logic
- move executable build/cache out of the hot optimization loop where practical
- keep CPU dynamic shadow checks and extend them to GPU dynamic parity checks

## 10.2 `crates/app_core/src/double_ma_backend.rs`

Convert blocker logic into a structured capability report.

Concrete tasks:

- add `StrategyCapabilityReport`
- keep current blocker reasons, but return structured capability instead of only a flat blocker list
- make route selection explicit:
- static specialized GPU
- dynamic generic GPU
- CPU dynamic fallback

## 10.3 `crates/app_core/src/double_ma_strategy.rs`

Keep this file as the semantic front gate.

Concrete tasks:

- keep `trade_on_next_bar=true` enforcement
- add a `validate_strategy_cfg_for_gpu_dynamic(...)` helper only if the v1 subset needs a cleaner explicit validator
- do not let the dynamic GPU path bypass this validation layer

## 10.4 `crates/app_core/src/strategy_plan.rs`

Keep this as the canonical runtime contract.

Concrete tasks:

- preserve stable compiled-plan schema semantics
- add any plan-level ABI or execution capability metadata only if it is truly part of the runtime contract
- avoid creating a parallel "GPU-only dynamic plan" schema

## 10.5 `crates/app_core/src/strategy_gpu_program.rs`

Turn this from a pure compiler output file into the source of truth for uploaded program layout.

Concrete tasks:

- define stable uploadable ABI mirror structs
- version the ABI explicitly
- expose the exact compiled-program-to-ABI conversion helpers used by runtime upload
- add validation for layout constraints and index bounds

## 10.6 New backend runtime file

Add one new file:

- `crates/app_core/src/strategy_gpu_runtime.rs`

Purpose:

- own `StrategyHandle`
- own `GpuStrategyExecutable`
- own executable cache
- upload compiled-program tables into device buffers
- provide `get_or_build_gpu_strategy_executable(...)`

Keep this separate from `gpu_backtest_kernel.rs` so plan/executable lifecycle does not get buried inside the generic CUDA runtime substrate.

## 10.7 `crates/app_core/src/gpu_backtest_kernel.rs`

Add a generic entrypoint rather than replacing the current specialized path.

Concrete tasks:

- add `run_strategy_program_gpu_kernel(...)`
- bind uploaded executable tables
- bind pair-invariant slots
- bind pair-variant MA matrices
- reuse resident series cache, scratch buffers, valid-pair compaction, and dispatch policy

This file should remain the host-side CUDA workhorse.

## 10.8 `crates/app_core/src/kernels/double_crossover.cu`

Add one generic program-driven backtest kernel path.

Concrete tasks:

- define device-side ABI mirrors matching Rust upload structs
- add generic predicate/group evaluation
- route signal booleans into the existing trade-state/metric loop
- keep current specialized crossover kernels intact

V1 should add exactly one generic kernel path, not multiple optimized variants.

## 10.9 `crates/app_core/build.rs`

Keep the PTX build path simple.

Concrete tasks:

- continue building `double_crossover.cu`
- ensure the generic program-driven kernel ships in the same PTX bundle
- do not add NVRTC as a required build/runtime dependency for v1

## 10.10 `crates/app_core/src/lib.rs`

Export the new runtime objects only where needed.

Concrete tasks:

- export `strategy_gpu_runtime` module types if they are part of crate-facing APIs
- keep internal-only runtime cache details private

## 10.11 New Tauri command file

Add one new command module:

- `src-tauri/src/commands_dynamic_strategy.rs`

Recommended commands:

- `validate_strategy_ir`
- `preview_strategy_plan`
- `run_dynamic_strategy_optimization`

Do not overload `run_double_ma_optimization` with raw `StrategyIr`.
Keep the user-defined strategy path explicit at the Tauri boundary even if it reuses internal Double-MA-family execution code.

## 10.12 Tauri integration files to modify

- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`

Tasks:

- export the new command functions
- register them in `tauri::generate_handler!`
- keep normal Double-MA requests untouched

## 10.13 Frontend files to modify for product exposure

- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/stores/taskStore.ts`

If a dedicated builder panel is added:

- `web/src/stores/dynamicStrategyStore.ts`
- `web/src/services/dynamicStrategyService.ts`
- `web/src/components/DynamicStrategyBuilderPanel.vue`

The builder should emit constrained `StrategyIr`, not a second parallel backend format.

## 11. Authoring Surface Plan

Do not start with a full graphical custom-strategy builder.

### 11.1 V1 authoring surface

V1 should be backend-first and research/headless friendly.

Recommended v1 authoring inputs:

- existing static strategy configs compiled through `compile_dynamic_plan_for_double_ma_config(...)`
- JSON-backed `StrategyIr` fixtures and tests
- optional debug/report output showing compiled plan and route assignments
- optional constrained UI builder for `DynamicDoubleMa` only after headless validation lands

This gives the repo a real dynamic GPU execution path without coupling success to a new UI builder.

### 11.2 V2 authoring surface

After the runtime path is correct, add a constrained authoring UI.

Recommended v2 surfaces:

- constrained condition-group builder
- indicator selection from dynamic indicator catalog
- signal-binding configuration
- capability preview showing CPU-only vs GPU-dynamic support

Likely frontend files:

- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

Do not begin with arbitrary scripting text boxes.

## 11.3 Recommended Tauri/API surface

`validate_strategy_ir` request:

- `strategy_ir`

`validate_strategy_ir` response:

- normalized `strategy_ir`
- validation errors or warnings
- current product-cap guard-rail violations

`preview_strategy_plan` request:

- `strategy_ir`
- `backend: Cpu | GpuUnified`

`preview_strategy_plan` response:

- `compiled_plan`
- `compiled_gpu_program` when GPU-unified preview is valid
- `capability_report`
- indicator route summary

`run_dynamic_strategy_optimization` request:

- `data_id`
- `strategy_ir`
- `backend`
- `objective`
- `top_k`
- `include_all`
- `heatmap_bins`
- fast/slow sweep bounds or explicit period lists

`run_dynamic_strategy_optimization` response:

- normal optimization result envelope
- compiled-plan summary
- capability report
- route used

## 11.4 Recommended frontend rollout

Phase 1 frontend:

- add a hidden or research-only `DynamicDoubleMa` option
- support JSON `StrategyIr` import/export
- show capability preview and backend route preview
- show guard-rail violations before run

Phase 2 frontend:

- add a constrained builder for indicators, conditions, groups, and bindings
- source all indicator IDs and outputs from the catalog
- keep advanced/raw JSON import as a power-user path

The frontend should never promise arbitrary "GPU custom strategies."
It should promise constrained custom strategies that compile to supported IR.

## 12. Phase Order

## 12.1 Phase 1: Formalize runtime contracts

Deliverables:

- `StrategyCapabilityReport`
- `StrategyHandle`
- `GpuStrategyExecutable`
- versioned GPU ABI structs

No generic kernel execution yet.

## 12.2 Phase 2: Uploadable executable layer

Deliverables:

- compiled GPU program to ABI translation
- device buffer upload path
- executable cache keyed by plan fingerprint + device + ABI version

Still no optimization routing change yet.

## 12.3 Phase 3: Generic GPU kernel runtime

Deliverables:

- generic program-driven kernel in `double_crossover.cu`
- host entrypoint in `gpu_backtest_kernel.rs`
- runtime execution against uploaded executable

## 12.4 Phase 4: Double-MA-family integration

Deliverables:

- `double_ma.rs` uses executable build/use instead of preflight-only compile
- dynamic GPU routing driven by capability report
- CPU dynamic fallback remains intact

## 12.5 Phase 5: Parity and performance hardening

Deliverables:

- CPU dynamic vs GPU dynamic parity tests
- targeted benchmarks
- acceptance thresholds for fallback, correctness, and runtime overhead

## 12.6 Phase 6: Broader strategy-family rollout

Only after Double-MA-family success:

- extend to other dynamic-plan-capable profiles
- expand supported slot types and regime paths
- decide whether specialized optimization or NVRTC is even needed

## 12.7 Phase 7: Frontend authoring

Only after backend/runtime success:

- expose constrained dynamic-strategy authoring in conventional optimize UX
- add capability preview and validation messages
- avoid shipping an authoring UI that targets a still-fluid runtime contract
- start with JSON import/export and preview before the full builder

## 13. Verification Plan

## 13.1 Unit tests

Add or extend tests for:

- `CompiledStrategyPlan` stability
- `CompiledGpuPredicateProgram` ABI translation
- executable cache identity and invalidation
- capability report routing decisions
- unsupported-feature rejection
- guard-rail violations for over-large or structurally invalid user-defined strategies

## 13.2 Parity tests

Add parity coverage for:

- CPU dynamic vs GPU dynamic on the same compiled plan
- current static specialized GPU vs generic GPU where a plan shape overlaps
- `trade_on_next_bar`, `long_only`, `allow_flip`, `stop_loss_pct`, `take_profit_pct`
- user-defined `DoubleMa`-profile plans with non-trivial condition groups

## 13.3 Integration tests

Add integration coverage in:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Tests should prove:

- executable build happens once per cache key
- optimization reuses executable cache
- unsupported shapes fall back to CPU dynamic cleanly
- supported shapes stay on GPU dynamic route without silent fallback
- raw imported `StrategyIr` is rejected when it violates product caps or unsupported output shapes

## 13.4 Benchmarks

Benchmark separately:

- compile plan only
- compile GPU predicate program only
- build/upload executable only
- first-use end-to-end path
- warm-cache optimization path
- specialized GPU kernel vs generic GPU kernel for overlapping Double-MA shapes

## 14. Risks

## 14.1 Architecture risk

Risk:

- growing a second backtest engine for dynamic strategies

Mitigation:

- reuse current trade-state and metrics flow inside the generic kernel

## 14.2 Semantic drift risk

Risk:

- GPU dynamic semantics diverge from CPU dynamic semantics

Mitigation:

- CPU dynamic executor remains the canonical semantics oracle
- require parity tests before enabling routes

## 14.3 Runtime complexity risk

Risk:

- plan compilation, executable upload, kernel launch, and fallback logic become entangled in `double_ma.rs`

Mitigation:

- isolate executable lifecycle in `strategy_gpu_runtime.rs`

## 14.4 Performance risk

Risk:

- generic program-driven kernel is correct but slower than hoped

Mitigation:

- ship it first
- measure it
- only then decide if shape specialization or NVRTC is justified

## 14.5 Product risk

Risk:

- trying to ship the UI builder before the runtime contract stabilizes

Mitigation:

- backend-first rollout
- constrained UI only after backend/runtime success

## 15. What Not To Do Yet

Do not start with:

- free-form user scripting
- a high-level-to-CUDA transcompiler project
- NVRTC specialization
- replacing current specialized kernels
- broad cross-strategy rollout on day one
- a custom UI builder before backend/runtime parity exists

## 16. Recommended Implementation Order

1. Add capability report and executable/runtime contract types.
2. Add versioned Rust/CUDA ABI structs for uploaded programs.
3. Add `strategy_gpu_runtime.rs` with executable cache and upload logic.
4. Add one generic program-driven kernel path in `double_crossover.cu`.
5. Add `run_strategy_program_gpu_kernel(...)` in `gpu_backtest_kernel.rs`.
6. Replace dynamic GPU preflight in `double_ma.rs` with executable build/use.
7. Add CPU dynamic vs GPU dynamic parity tests.
8. Benchmark cold-start, warm-cache, and overlap with specialized kernels.
9. Only then decide whether to expose constrained frontend authoring.

## 17. Implementation Checklist

## 17.1 Contracts

- [ ] Add `StrategyCapabilityReport`.
- [ ] Add `StrategyHandle`.
- [ ] Add `GpuStrategyExecutable`.
- [ ] Add explicit GPU ABI versioning.
- [ ] Add serialization or debug-print support where useful for diagnostics.
- [ ] Document v1 product caps separately from backend hard caps.

## 17.2 Compiler and program layer

- [ ] Keep `CompiledStrategyPlan` as the one true plan contract.
- [ ] Keep `CompiledGpuPredicateProgram` as the one true GPU predicate contract.
- [ ] Add ABI mirror structs derived from compiled GPU program data.
- [ ] Add bounds/layout validation for uploaded program tables.
- [ ] Reject matrix or non-scalar indicator outputs for v1 user-defined GPU mode.

## 17.3 Runtime executable layer

- [ ] Create `crates/app_core/src/strategy_gpu_runtime.rs`.
- [ ] Implement executable cache keyed by plan fingerprint + device + ABI version.
- [ ] Implement one-time device upload of indicator slots, conditions, groups, indexes, and signals.
- [ ] Add executable cache hit/miss diagnostics for tests and profiling.

## 17.4 Generic kernel path

- [ ] Add generic program-driven kernel ABI structs to `double_crossover.cu`.
- [ ] Add generic condition evaluation.
- [ ] Add generic group reduction.
- [ ] Add generic signal binding resolution.
- [ ] Reuse existing trade-state and metric accumulation flow.
- [ ] Keep specialized kernels intact.

## 17.5 Host CUDA integration

- [ ] Add `run_strategy_program_gpu_kernel(...)` to `gpu_backtest_kernel.rs`.
- [ ] Bind pair-invariant series once.
- [ ] Bind pair-variant MA sweep matrices per run.
- [ ] Reuse resident market-series cache and scratch buffers.
- [ ] Reuse valid-pair compaction and launch-shape tuning.

## 17.6 Double-MA-family integration

- [ ] Replace `maybe_compile_dynamic_gpu_plan_preflight(...)` with executable build/use.
- [ ] Keep CPU dynamic shadow path as the correctness oracle.
- [ ] Add GPU dynamic route selection driven by `StrategyCapabilityReport`.
- [ ] Preserve CPU dynamic fallback for unsupported dynamic shapes.
- [ ] Add a path that accepts user-provided `StrategyIr` with `profile=DoubleMa`.

## 17.7 Strategy semantics and blockers

- [ ] Preserve `trade_on_next_bar=true` enforcement.
- [ ] Preserve cost and stop/take-profit validation.
- [ ] Keep Kaufman stop exit blocked for v1 GPU dynamic mode.
- [ ] Keep cooldown/persistence overlays blocked for v1 GPU dynamic mode.
- [ ] Keep unsupported objectives and unsupported MAs blocked for v1 GPU dynamic mode.
- [ ] Enforce stricter product-level caps for user-defined strategies.
- [ ] Enforce binding rules for long-only vs bidirectional user-defined plans.

## 17.8 Testing

- [ ] Add unit tests for ABI translation.
- [ ] Add executable cache-key tests.
- [ ] Add CPU dynamic vs GPU dynamic parity tests.
- [ ] Add tests for supported-feature routing.
- [ ] Add tests for unsupported-feature CPU fallback.
- [ ] Add tests proving specialized kernels still route as before.

## 17.9 Benchmarking

- [ ] Benchmark compile-only overhead.
- [ ] Benchmark executable upload overhead.
- [ ] Benchmark first-use end-to-end overhead.
- [ ] Benchmark warm-cache optimization overhead.
- [ ] Benchmark generic GPU vs specialized GPU for overlapping shapes.

## 17.10 Frontend follow-up

- [ ] Do not add a graphical builder in v1.
- [ ] Add `src-tauri/src/commands_dynamic_strategy.rs`.
- [ ] Export/register dynamic-strategy Tauri commands.
- [ ] Add JSON-backed `StrategyIr` import/export first.
- [ ] If frontend authoring is added, update `optimizeStore.ts`, `optimizeRequestBuilder.ts`, `optimizeService.ts`, `strategyOptimizationCapabilities.ts`, and `taskStore.ts`.
- [ ] Add a constrained `DynamicDoubleMa` option rather than promising arbitrary custom profiles.
- [ ] Add capability-preview UI instead of silent fallback behavior.

## 18. Exit Criteria

This feature is ready to move beyond research/prototype status only when:

- a compiled plan can be turned into a reusable GPU executable
- the generic GPU kernel executes the compiled program correctly
- CPU dynamic vs GPU dynamic parity is proven on the supported v1 subset
- unsupported dynamic shapes fall back or reject cleanly with explicit reasons
- specialized kernels remain intact and continue to serve current built-in routes
- first-use and warm-cache overhead are understood well enough to justify product exposure

Until then, dynamic GPU strategy execution should remain a constrained backend/runtime feature, not a broad user-facing "custom strategy builder" promise.
