# Stage 6B: GPU Kernel + VDPS Extension for Double-Like Strategies

Last updated: 2026-02-18  
Scope: Extend GPU kernel optimization support beyond plain Double MA to:
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

Out of scope:
- `CompositeMo` (MACZ + OTT + HalfTrend) remains CPU-oriented by design.

## Objective

Enable these three strategies to use:
- GPU kernel backend (`GpuKernel`)
- GPU VDPS mode (`GpuVdps`)

while preserving:
- Existing features
- Deterministic behavior in VDPS deterministic mode
- CPU-only fallback behavior

## Current State (baseline)

- UI now allows selecting `GpuVdps` for double-like kinds.
- Backend kernel gating now allows regime-filtered double-like variants (except unsupported overlays/exit combos).
- Regime computation remains host-side (`compute_combined_regime_filter`) and is now wired into kernel dispatch.
- Kernel path now consumes per-bar regime state for signal gating/switching.

## Acceptance Criteria

1. `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` can run with `backend=GpuKernel`.
2. `mode=GpuVdps` works for these three strategies without CPU-backend fallback.
3. Top-k ordering and metrics are deterministic for same seed/hardware (VDPS deterministic mode).
4. Existing `DoubleMa` behavior is unchanged (no regression).
5. `CompositeMo` path unchanged.
6. CPU-only builds still compile and run.

## Implementation Plan

### Phase 0: Safety + Interfaces
- [x] Add a focused feature gate / compatibility flag for regime-enabled kernel path.
- [x] Introduce kernel input contract for per-bar regime state/mask.
- [x] Define explicit regime encoding for kernel consumption (long/short/range/none).

### Phase 1: Kernel Data Plumbing
- [x] Wire `compute_combined_regime_filter` output into kernel dispatch path.
- [x] Allocate/copy regime buffer to device for kernel runs.
- [x] Ensure no additional copies in inner tile loops (copy once per optimization run).

### Phase 2: Kernel Signal Logic
- [x] Extend kernel signal evaluation to apply regime mask for:
- [x] CompositeTrend gating behavior.
- [x] RegimeHybrid switching behavior (trend vs range handling).
- [x] BreakoutMomentum gating behavior.
- [x] Preserve existing exit-mode constraints where kernel does not support overlays.

### Phase 3: Backend Routing + Gating
- [x] Relax kernel eligibility checks for the three strategy flags.
- [x] Keep unsupported combos explicitly rejected with clear errors.
- [x] Ensure `GpuVdps` route uses kernel path when eligible.

### Phase 4: Tests
- [x] Add unit/integration tests for each strategy under kernel backend:
- [x] Smoke run returns valid result.
- [x] Same-seed deterministic stability check for VDPS.
- [x] CPU vs GPU sanity/parity tolerance checks (non-identical but bounded deltas).
- [x] Add negative tests for unsupported combinations.

### Phase 5: Bench + Validation
- [x] Benchmark/report harness implemented for representative search spaces.
- [x] Runtime and top-k overlap capture implemented vs prior `GpuOnly`/CPU paths.
- [x] Baseline `DoubleMa` kernel regression coverage included in report harness.

## Risk Register

1. Regime semantics mismatch between CPU and kernel paths.
- Mitigation: shared encoding helpers + parity tests on fixed datasets.

2. Performance regression from additional per-bar branching.
- Mitigation: branch-minimized encoding, precomputed masks, benchmark each strategy separately.

3. Determinism drift in VDPS with added strategy logic.
- Mitigation: same-seed repeated-run tests and stable tie-break ordering checks.

## Tracking Checklist (Execution)

- [x] Phase 0 complete
- [x] Phase 1 complete
- [x] Phase 2 complete
- [x] Phase 3 complete
- [x] Phase 4 complete
- [x] Phase 5 complete

## Notes

- Prioritize speed and feature parity.
- Do not remove existing strategies or CPU workflows.
- Keep `CompositeMo` independent from this extension track.
- Unit tests/benchmarks are now fully wired for this stage; execution is intentionally deferred in this pass.
- Stage 5 report command (when ready to run): `cargo run -p ta_app_core --features cuda,cuda-backtest-kernel --bin stage6b_kernel_report -- dist/stage6b_kernel_report.json`
