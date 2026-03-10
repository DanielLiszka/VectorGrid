# Backend Refactor And Cleanup Tracker

## Goal

Reduce backend technical debt without changing product behavior.

Current product behavior target:
- exact `Grid` optimization only
- no user-facing approximate optimization modes
- no regressions in optimization, validation, meta, or artifact workflows

Primary debt areas:
- [`crates/app_core/src/double_ma.rs`](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs) is too large and mixes orchestration, backend resolution, execution dispatch, result shaping, and tests
- [`crates/app_core/src/composite_cpu.rs`](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/composite_cpu.rs) repeats grid-only mode handling across multiple entry paths
- strategy fixture/preset builders are duplicated across benches, tests, and research binaries
- legacy approximate-mode branching still exists in core even though product behavior now normalizes to grid

## Constraints

- Prefer small, reversible refactors over large rewrites
- Preserve CPU-only builds
- Avoid changing public request/response contracts unless necessary
- Keep research-only code isolated if retained
- Add or update tests at each extraction step
- No cleanup pass should depend on another unfinished cleanup pass

## Refactor Strategy

### Phase 1: Stabilize The Boundary

Objective:
- make grid-only product behavior explicit in one place
- avoid duplicate coercion logic

Target work:
- keep [`src-tauri/src/commands_optimize.rs`](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_optimize.rs) as the product boundary
- add a shared helper for product-mode normalization if another command path needs it
- document that core still contains legacy research paths intentionally

Acceptance:
- app-facing optimization commands always execute in `Grid`
- targeted command tests remain green

### Phase 2: Split `double_ma.rs` By Concern

Objective:
- reduce file size and local cognitive load without changing runtime behavior

Proposed extraction order:
1. mode/backend normalization helpers
2. grid execution helpers
3. result shaping / heatmap / aggregation helpers
4. research-only approximate mode helpers
5. test-only fixtures/builders

Proposed module shape:
- `crates/app_core/src/double_ma/mod.rs`
- `crates/app_core/src/double_ma/types.rs`
- `crates/app_core/src/double_ma/request.rs`
- `crates/app_core/src/double_ma/grid.rs`
- `crates/app_core/src/double_ma/result.rs`
- `crates/app_core/src/double_ma/research_modes.rs`
- `crates/app_core/src/double_ma/test_support.rs`

Notes:
- do not try to delete approximate-mode code during the same pass as file splitting
- first move code mechanically, then simplify

Acceptance:
- `double_ma` behavior is byte-for-byte equivalent at the request/result level
- moved functions keep the same tests or gain equivalent tests near the new module

### Phase 3: Consolidate Grid-Only Composite Handling

Objective:
- remove repeated mode checks in [`crates/app_core/src/composite_cpu.rs`](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/composite_cpu.rs)

Target work:
- centralize grid-only mode validation in one helper
- centralize repeated `let mode_used = OptimizationModeResolved::Grid;`
- reduce repeated request normalization boilerplate across ASR/VCC/VEMR/ROCB/composite entry paths

Acceptance:
- no behavior change in composite optimization routes
- composite tests continue to pass

### Phase 4: Centralize Strategy Fixtures

Objective:
- stop repeating small strategy preset builders in benches/tests/research tools

Likely candidates:
- `trend_quality_strategy_for_kernel`
- `regime_hybrid_strategy_for_kernel`
- `breakout_strategy_for_kernel`

Target location:
- a test/research support module, not production runtime code unless genuinely needed there

Acceptance:
- no duplicated fixture definitions in active benchmark/research paths
- fixture usage stays obvious and local enough for tests to remain readable

### Phase 5: Decide Fate Of Legacy Research Paths

Objective:
- make an explicit choice instead of leaving accidental half-dead code

Options:
- keep approximate modes as research-only and move them into `research_modes.rs`
- or delete them after enough evidence and after the split/refactor is stable

Decision criteria:
- are they still used by any internal benchmark/research bin?
- do they provide unique research value?
- does keeping them materially complicate core maintenance?

Acceptance:
- approximate modes are either clearly isolated or fully removed

## Safety Rules

- never combine extraction, rename, and logic change in one commit-sized step
- when moving code, keep function names/signatures stable first
- do not refactor tests and runtime logic in the same patch unless required
- prefer adapter shims during intermediate steps
- keep research binaries compiling until a conscious removal decision is made

## Verification Matrix

Run after each phase as applicable:

- `cargo check -p ta_desktop_demo_app`
- `cargo test -p ta_desktop_demo_app commands_optimize -- --nocapture`
- `cargo test -p ta_app_core double_ma -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `npm run typecheck`
- `npm run lint`

Performance-sensitive steps:
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

Real-data guardrail when touching optimization orchestration:
- rerun [`crates/app_core/src/bin/real_data_optimizer_eval.rs`](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/real_data_optimizer_eval.rs) on AAPL before and after

## Risks

- moving code out of `double_ma.rs` can silently break feature-gated CUDA paths
- shared helper extraction can accidentally couple product and research behavior
- over-DRY refactors can hurt benchmark/test readability
- deleting legacy branches too early can make later research harder

## Non-Goals

- no new optimization algorithms
- no strategy behavior changes
- no frontend redesign in this plan
- no broad serialization/API redesign

## Checklist

- [x] Normalize product-facing optimization commands to exact `Grid`
- [x] Add a test that product mode normalization forces `Grid`
- [x] Remove obvious dead helper/warning-only code where safe
- [ ] Add a small core helper for legacy optimization-mode normalization, if needed
- [x] Extract `double_ma` request/mode/backend resolution helpers into a dedicated module
- [x] Extract `double_ma` strategy validation helpers into a dedicated module
- [x] Extract shared MA kind lookup/validation helpers into a dedicated module
- [ ] Extract `double_ma` grid execution helpers into a dedicated module
- [x] Extract `double_ma` result shaping / aggregation helpers into a dedicated module
- [x] Move approximate/research-only mode helpers behind a clearly named module boundary
- [x] Move `double_ma` test fixtures/builders into a dedicated support module
- [x] Consolidate repeated grid-only mode validation in `composite_cpu.rs`
- [x] Centralize shared strategy preset builders used by benches/tests/research bins
- [ ] Re-run targeted optimization and validation regression checks after each extraction phase
- [ ] Re-run real-data evaluator after any orchestration refactor
- [ ] Decide whether approximate optimization code is retained as research-only or removed entirely
