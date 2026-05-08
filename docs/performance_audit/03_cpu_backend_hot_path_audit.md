# CPU Backend Hot Path Audit

Date: 2026-05-06
Status: audit plan

## Goal

Find CPU-side bottlenecks that reduce performance even when GPU kernels are fast. This includes pure CPU routes, CPU fallback, CPU tails under GPU routes, Tauri orchestration, serialization, allocation, cloning, and repeated recomputation.

## Scope

- `crates/app_core/`
- `crates/strategies/`
- `crates/optimizer/`
- `src-tauri/src/commands_*.rs`
- CPU fallback inside GPU-capable workflows
- command/state orchestration overhead

## Audit Questions

1. Which workflows spend material time outside the advertised GPU kernel?
2. Which CPU routes allocate or clone large buffers repeatedly?
3. Which validation modules recompute indicators or returns unnecessarily?
4. Which Tauri commands serialize large payloads that could be summarized or paged?
5. Which artifact/report paths block the user after compute is complete?
6. Which routes use CPU fallback silently or late?

## Hot Path Inventory

| Area | Files to inspect first | Bottleneck risks |
|---|---|---|
| Double-MA optimization | `crates/app_core/src/double_ma.rs` | indicator recompute, buffer allocation, top-k reduction, export-all materialization |
| GPU kernel host path | `crates/app_core/src/gpu_backtest_kernel.rs` | CPU prep/tail, transfer staging, result materialization |
| Double-MA validation | `crates/app_core/src/double_ma_validate.rs` | repeated candidate reruns, validation module recompute |
| Composite validation | `crates/app_core/src/composite_validate.rs` | route fallback, mixed finalist sets, CPU-tail modules |
| Validation pipeline | `crates/app_core/src/validation_pipeline.rs` | walk-forward loops, bootstrap, SPA, CSCV/PBO, allocations |
| Options expression | `crates/app_core/src/options_expression.rs` | tuple/event loops, quote lookup, stress lanes |
| Options data store | `crates/app_core/src/options_data_store.rs` | dataset load/indexing, quote-quality summaries |
| Campaign runner | `src-tauri/src/commands_campaign.rs` | dataset prep, preload, artifact matching, validation follow-up |
| Artifacts | `src-tauri/src/run_artifacts.rs` | JSON parse/write, catalog scan, hydration |
| Reports | `src-tauri/src/commands_reports*.rs` | XLSX generation, JSON model parsing |

## Measurement Techniques

- Criterion benches for inner-loop Rust paths.
- Existing workflow gates for product-adjacent paths.
- `cargo test -- --nocapture` only for correctness, not timing authority.
- Add scoped `Instant` timing only when it becomes structured output and not noisy logs.
- Use Windows Performance Recorder or ETW only as companion diagnostics if needed.
- Avoid ad hoc `Measure-Command` as final authority.

## CPU Optimization Checklist

### Allocation And Cloning

- [ ] Search for repeated `clone()` of large candles, results, artifacts, and option rows.
- [ ] Search for `vec![0; n]` or `vec![NaN; n]` in hot loops where full fill is not behavior.
- [ ] Inspect `Vec::with_capacity` opportunities only where every element is written before read.
- [ ] Check top-k/finalist materialization paths for over-collection.
- [ ] Check JSON/report builders for avoidable full-object clones.

### Recompute

- [ ] Identify repeated indicator generation for same dataset/config.
- [ ] Identify repeated candidate backtests in validation/follow-up.
- [ ] Identify repeated options quote/event path scans.
- [ ] Check campaign validation follow-up for duplicated optimization output parsing.

### Serialization

- [ ] Measure Tauri command response sizes.
- [ ] Identify responses that return full rows when summary plus follow-up command would suffice.
- [ ] Check artifact hydration paths for repeated JSON parsing.
- [ ] Avoid changing response shape without compatibility plan.

### State Locks And Orchestration

- [ ] Inspect Tauri state locks around long computations.
- [ ] Ensure locks are not held across heavy compute when avoidable.
- [ ] Check cancellation/polling overhead for long workflows.
- [ ] Check campaign progress throttling overhead.

## Candidate Immediate Wins

1. Add structured timing to validation module phases.
2. Add response-size diagnostics for top results, validation results, options artifacts, and history hydration.
3. Benchmark candidate drilldown/local-stability cold versus cached paths.
4. Profile artifact load/save and report generation separately from optimization.
5. Audit large `clone()` and full-fill allocations in validation and options paths.

## Validation

- CPU-only build:
  - `cargo check -p ta_app_core`
  - `cargo test -p ta_app_core`
- Performance:
  - `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- If artifacts/reports change:
  - `node scripts/run_output_parity_check.mjs`

## Regression Watchpoints

- Allocation optimizations using uninitialized memory must follow repo safety rules.
- Do not change scoring/order while optimizing.
- Do not alter artifact schemas for timing fields unless optional.
- Do not use runtime milliseconds in identity fingerprints.
- Avoid log spam in hot loops.

## Definition Of Done

- CPU hot paths have phase timing or benchmark coverage.
- Top CPU bottlenecks are ranked by measured impact.
- Any optimization has before/after evidence under the correct gate.
- CPU-only build and correctness tests remain green.
