# Non-Double-MA Metric Rollout Plan

## Goal

Extend the three recently added Double-MA metrics to the rest of the backtest optimization application:

- `downside_deviation`
- `ulcer_index`
- `martin_ratio`

This document is scoped to the non-Double-MA strategy families and is intended to guide implementation across optimization, GPU execution, validation, export, and tests.

## Current State

### What is already done

- The shared `Metrics` surface already carries the three fields via `ta_strategies::double_ma::Metrics`.
- The generic composite CPU metrics path in `crates/strategies/src/composite_cpu/mod.rs` already computes:
  - `downside_deviation`
  - `ulcer_index`
  - `martin_ratio`
- Shared validation structures already expose and summarize the new fields in `crates/app_core/src/composite_validate.rs` and `crates/app_core/src/validation_pipeline.rs`.
- Double-MA already has end-to-end support on its active CPU, GpuKernel, export, and validation routes.

### Main remaining gap

For the non-Double-MA GPU strategy adapters, the new metrics are still placeholder zeroes on the device-eval path. The current dedicated GPU modules populate `sortino` and `omega`, but hardcode these three fields to `0.0`:

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`

That means:

- CPU optimization/validation can already compute the metrics where they flow through the shared composite CPU path.
- GPU optimization/validation for these families does not yet produce real values for the three metrics.
- Validation summaries can carry the fields, but GPU-originated values remain incomplete until the adapters are updated.

## Strategy Matrix

| Strategy family | CPU metric support | Dedicated GPU path | Current GPU status for new metrics | Priority |
| --- | --- | --- | --- | --- |
| `ASR` | Yes | Yes | Zero-filled | High |
| `VEMR` | Yes | Yes | Zero-filled | High |
| `VCC` | Yes | Yes | Zero-filled | High |
| `VCET` | Yes | Yes | Zero-filled | High |
| `ROCB` | Yes | Yes | Zero-filled | High |
| `MKCS` | Yes | Yes | Zero-filled | High |
| `KQSR` | Yes | Yes | Zero-filled | High |
| `CARM` | Yes | Yes | Zero-filled | High |
| `KVDF` | Yes | Yes | Zero-filled | High |
| `CDMC` | Yes | Yes | Zero-filled | High |

Notes:

- “CPU metric support” here refers to the shared composite CPU/eval path already being able to compute the richer `Metrics` payload.
- The priority is uniformly high because the missing work is concentrated in the GPU adapter layer, and these strategies already participate in GPU optimization and GPU validation.

## Architectural Recommendation

Do not treat this as ten separate metric inventions. Treat it as one repeated adapter rollout with a single standard:

1. Keep the `Metrics` contract fixed across all strategies.
2. Make each dedicated GPU path produce the same metric semantics as the composite CPU path.
3. Reuse shared validation/reporting once per-strategy GPU metric parity is in place.

The preferred rollout shape is:

1. Implement device/native metric calculation in each GPU strategy adapter.
2. Decode those values into `Metrics`.
3. Verify CPU/GPU parity for in-sample eval first.
4. Then verify walk-forward, CSCV/PBO, SPA, reopt-walk-forward, trade-bootstrap, and export surfaces.

## Implementation Workstreams

### 1. GPU adapter rollout

For each of the following files:

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`

Replace the placeholder metric construction with real values for:

- `downside_deviation`
- `ulcer_index`
- `martin_ratio`

Preferred implementation order inside each adapter:

1. Reuse already available path statistics if the adapter already computes per-step return aggregates and drawdown state.
2. If the current CUDA row contract is too narrow, widen the row format in the corresponding kernel launch/decode path.
3. Only fall back to host recomputation if there is no safe device-native path and the strategy is not hot-path sensitive. This should be the exception, not the default.

### 2. Kernel and row-contract updates

Some adapters may only need host-side decode changes. Others may require widening the CUDA output contract.

For each strategy, explicitly answer:

- Does the kernel already expose enough primitives to derive the metrics?
- Can `downside_deviation` be derived from existing negative-return aggregates?
- Can `ulcer_index` and `martin_ratio` be derived from existing drawdown path state?
- If not, which additional accumulators must be emitted from the kernel row?

Rule:

- Prefer adding a small number of stable row fields over strategy-specific host-side recompute detours.

### 3. Validation parity

Shared validation structures already support the new metrics. Once the GPU adapters are fixed, verify that the following composite validation flows propagate real values instead of zeros:

- in-sample candidate metrics
- walk-forward summaries
- reopt-walk-forward summaries
- friction-stress summaries
- regime-slice summaries
- advanced GPU validation route smoke tests

Primary file:

- `crates/app_core/src/composite_validate.rs`

Secondary shared file:

- `crates/app_core/src/validation_pipeline.rs`

### 4. Export and reporting parity

Confirm that GPU-originated optimization results for the affected strategies surface the new metrics anywhere a `Metrics` payload is serialized or shown.

Check:

- backend result payload construction
- CSV export paths
- validation summary payloads
- frontend summary formatting where strategy-specific results are shown

Expected touchpoints will likely stay mostly in existing result plumbing, because the struct fields already exist.

## Verification Standard

Each strategy family should clear the same verification ladder.

### A. CPU/GPU metric parity

Add or update tests proving that GPU-produced metrics match CPU-produced metrics for:

- `downside_deviation`
- `ulcer_index`
- `martin_ratio`

Use the existing parity tests in `crates/app_core/src/composite_validate.rs` as the primary harness.

### B. Validation surface coverage

For each strategy with a GPU validation path, ensure there is at least one passing test covering:

- in-sample GPU parity
- walk-forward GPU parity or near-parity
- advanced route smoke

If the strategy already has CSCV, SPA, reopt-walk-forward, or trade-bootstrap GPU tests, extend those expectations to ensure the new fields are populated where relevant.

### C. Optimization/export coverage

For each strategy, verify:

- optimizer/top-k output contains real metric values
- CSV/export output contains non-zero/non-placeholder values where expected

### D. Performance gates

Any strategy whose GPU path requires kernel-row widening or added accumulators must be checked for regression risk.

At minimum:

- targeted `cargo test -p ta_app_core --lib --features cuda,cuda-backtest-kernel ...`
- relevant targeted benchmarks where they exist
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after the broader rollout batch

## Suggested Rollout Order

Phase the work by expected implementation similarity and risk.

### Phase 1: straightforward dedicated GPU adapters

- `ROCB`
- `MKCS`
- `KQSR`
- `CARM`
- `KVDF`
- `CDMC`

Reason:

- These adapters appear to share a simpler “decode into `Metrics`” shape and currently zero-fill the new fields in a uniform way.

### Phase 2: medium-complexity adapters

- `VEMR`
- `VCET`

Reason:

- These paths are still likely straightforward, but include more strategy-specific signal shaping and route nuances.

### Phase 3: highest attention adapters

- `ASR`
- `VCC`

Reason:

- These modules are more bespoke and already carry more custom evaluation details, so they should be treated as higher parity/perf-risk targets.

## Risks

### Semantics drift

If each strategy computes the three metrics with slightly different guards or denominator rules, validation and ranking become inconsistent.

Mitigation:

- Match the composite CPU semantics exactly.
- Keep epsilon guards and clamping rules aligned with existing metric behavior.

### Hidden zeroes in validation

Because validation structs already carry these fields, zero-filled GPU metrics can look “valid” while actually being placeholders.

Mitigation:

- Add explicit parity assertions for the three fields.
- Prefer tests that fail on exact zero defaults for trending fixtures.

### GPU perf regressions

Adding accumulators can increase register pressure, row width, or decode overhead.

Mitigation:

- Batch rollout by phase.
- Benchmark after each phase, not just at the end.
- Avoid unnecessary host-side replay in the optimization hot path.

## Deliverables Checklist

- [ ] Replace zero-filled `downside_deviation` in all ten non-Double-MA GPU strategy adapters.
- [ ] Replace zero-filled `ulcer_index` in all ten non-Double-MA GPU strategy adapters.
- [ ] Replace zero-filled `martin_ratio` in all ten non-Double-MA GPU strategy adapters.
- [ ] Verify CPU/GPU in-sample metric parity for all ten families.
- [ ] Verify walk-forward or advanced validation GPU coverage for all ten families.
- [ ] Verify export/result payloads surface real values for the three metrics.
- [ ] Run targeted GPU tests for each phase.
- [ ] Run `scripts/run_gpu_perf_gate.ps1 -Mode candidate` after the final rollout batch.

## Implementation Notes

- This rollout does not require changing the optimizer objective enum unless we later decide to expose any of the three metrics as first-class optimization objectives.
- The immediate goal is metric availability and parity, not objective-surface expansion.
- Shared validation/reporting work should stay minimal; most of the remaining effort is in the per-strategy GPU execution adapters.
