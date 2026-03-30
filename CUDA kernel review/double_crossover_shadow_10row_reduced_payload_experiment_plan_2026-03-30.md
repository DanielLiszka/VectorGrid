# Double Crossover Shadow 10-Row Reduced-Payload Experiment Plan

Date: 2026-03-30

## Purpose

This document plans a new isolated experiment for `double_ma` GPU optimization work.

The goal is not to modify the shipped row-first path. The goal is to test, in a controlled and reversible way, whether a different search-stage architecture can beat the current reduced-payload exact GPU baseline that resolves to `10` rows on common `Sharpe` surfaces.

This experiment is intentionally:

- limited to `double_ma`
- limited to developer-only execution surfaces
- not exposed through the main application route
- not allowed to change product semantics, defaults, or supported surfaces

## Why A Separate Experiment Lane Exists

The active shipped path is currently the row-first path described in [double_crossover_next_phase_performance_implementation_plan_2026-03-30.md](C:/Rust%20Projects/ta_desktop_demo/CUDA%20kernel%20review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md).

Current evidence says:

- the current row-first architecture is the correct shipped choice
- the obvious low-risk tuning work is largely exhausted
- recent score-first extensions inside the current reducer shape regressed and were reverted
- on the current large `alma/alma` exact GPU surface, `10`, `17`, and `22` rows all land in roughly the same timing band, which means row width alone is not the main bottleneck

That means the next claim worth testing is not "tune the current path harder."

It is:

- can a different search-stage dataflow beat the current reduced `10`-row-equivalent row-first baseline?

## Experiment Thesis

The current reduced exact GPU baseline still pays for:

- all-pairs backtest execution
- all-pairs search-stage row writes
- all-pairs row reread / rescoring behavior in the selection pipeline

The experiment thesis is:

- if the search stage emits only compact scored candidates instead of full search rows
- and if exact replay remains authoritative for finalists
- then a shadow reduced-payload path may outperform the current row-first reduced baseline on `large` and `xlarge`

This must be proven against the current reduced baseline, not assumed.

## Success Criteria

This experiment is successful only if all of the following are true:

- it beats the current reduced `10`-row-equivalent row-first baseline on `large`
- it beats the current reduced `10`-row-equivalent row-first baseline on `xlarge`
- it does not lose materially on `small`
- it preserves best-params identity on the narrow benchmark surface
- it preserves top-k identity on the narrow benchmark surface
- finalist metrics after exact replay remain identical to the current authoritative path
- the shipped row-first path remains unchanged and callable

If those conditions are not met, the experiment should be rejected and kept out of the product path.

## Non-Goals

- changing any frontend or application default
- wiring the new path into the normal optimize command
- changing the meaning of `auto`, `full17`, or `trade22`
- broadening the experiment to `include_all`, export, heatmap, shortlist union, shortlist confidence, or shortlist slice before the narrow exact surface proves out
- changing CPU fallback behavior
- changing final exact metric semantics
- retrying the current rejected score-first global-head design inside the shipped reducer shape

## Initial Scope

The first implementation slice should be extremely narrow:

- strategy: classic `double_ma`
- backend family: exact GPU kernel path only
- objective: `Sharpe`
- metric contract: `auto`
- expected reduced search width on this surface: `10`
- `include_all=false`
- `top_k > 0`
- no constraints
- no heatmap
- no export
- no host filter mode
- no shortlist union/confidence/slice extras
- no dynamic strategy route
- no precomputed matrix route differences

This is the smallest useful falsifiable experiment.

## What "10-Row" Means Here

This plan is not about forcing the product to expose a new hardcoded `10` row option.

It is about building a shadow path that targets the current reduced exact GPU search surface where `metric_contract=auto` resolves to `metrics_count=10` on the benchmark route being tested.

The benchmark truth surface for this document is:

- `double_ma`
- `alma/alma`
- `objective=Sharpe`
- `workload=large`
- exact `GpuKernel`
- `heatmap_bins=0`
- no constraints

On that surface, `auto` is the current reduced `10`-row-equivalent baseline.

## Architecture Target

The experiment should not mutate the current row-first reducer path in place.

The shadow path should instead:

- keep the existing row-first path untouched
- create a separate reduced-payload search pipeline
- emit compact scored candidates during search
- keep exact replay authoritative for finalist materialization

Preferred first payload:

- `(score, pair_id)`

Where:

- `pair_id` is a stable identifier for a `(fast_len, slow_len)` candidate
- the reducer chain merges scored candidates, not full row payloads

Exact replay then:

- reruns shortlisted finalists on the existing authoritative path
- materializes final metrics using the existing path
- returns comparison-safe results against the current reduced baseline

## Primary Files

Required:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

Optional only if needed:

- `crates/app_core/src/double_ma.rs`

The preferred implementation is runner-only or env-gated and should avoid normal app-route wiring.

## Isolation Guardrails

The new experiment path must:

- remain unreachable from the normal application route
- be triggered only from a dev-only runner flag, explicit internal helper, or explicit env gate
- not replace or alter the current row-first path selection logic for shipped requests
- not change current UI behavior
- not alter current route diagnostics for shipped paths except to add explicit experiment diagnostics when the shadow path is chosen

Recommended naming shape:

- `shadow`
- `experimental`
- `reduced_payload`
- `score10`

Avoid names that imply production readiness.

## Phases

### Phase 0: Baseline Lock And Harness

Goal:

- lock the current reduced baseline and isolate the experiment trigger

Deliverables:

- document the exact benchmark command set
- confirm the reduced benchmark surface resolves to `metrics_count=10`
- add an explicit dev-only runner switch or env flag for the new path
- ensure the main app path cannot select it

Acceptance:

- the current baseline can still be run unchanged
- the shadow path can be toggled independently

### Phase 1: Shadow Entry Point

Goal:

- add a new reduced-payload search entry point without changing the existing shipped path

Deliverables:

- allocate shadow scratch buffers separately from current row-first flow
- add explicit route diagnostics for the shadow path
- keep the old row-first path fully intact

Acceptance:

- row-first behavior remains unchanged when the experiment is off
- the runner can force the experiment path on the narrow benchmark surface

### Phase 2: First Reduced-Payload Search Prototype

Goal:

- build the smallest scored-candidate search pipeline that can be measured honestly

Preferred first slice:

- compute ranking signal on-device
- emit compact `(score, pair_id)` candidates
- merge only compact candidates across tiles
- do not materialize full search rows in the experimental path unless strictly necessary for correctness of the prototype

Acceptance:

- the experiment returns a valid shortlist
- the shortlist can be exact-replayed on the authoritative path

### Phase 3: Exact Replay And Parity

Goal:

- prove the shadow path still returns correct results

Deliverables:

- finalist exact replay using the current authoritative path
- parity comparison against the reduced row-first baseline
- best-params parity
- top-k identity parity
- finalist metrics parity

Acceptance:

- parity passes on the narrow benchmark surface

### Phase 4: Benchmark Decision

Goal:

- decide keep-or-reject using serial measurements only

Required benchmark surfaces:

- `small`
- `large`
- `xlarge`

Required contract surface:

- reduced `auto` route that resolves to `10`

Optional after success:

- compare against `full17`
- compare against `trade22`

Acceptance:

- measurable win on `large`
- measurable win on `xlarge`
- no material regression on `small`

Reject immediately if:

- it loses on `large`
- it loses on `xlarge`
- it requires product-path changes to look good
- it only looks good under contaminated or parallel benchmark conditions

## Benchmark Commands

Baseline reduced path:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload large --objective sharpe --metric-contract auto --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

Shadow path:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload large --objective sharpe --metric-contract auto --iterations 3 --warmup-iterations 1 --heatmap-bins 0 --shadow-score10 1
```

The exact switch name can change, but the runner surface should stay this narrow.

All benchmark runs must remain serialized.

## Verification

Minimum verification:

- targeted unit tests for route gating
- targeted unit tests for parity on the narrow benchmark surface
- serial runner comparisons on `small`, `large`, `xlarge`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after any kept experiment slice

Nice to have after the prototype proves out:

- Nsight Systems capture on the shadow path
- Nsight Compute capture on the shadow path

## Rejection Criteria

Reject the experiment if any of the following become true:

- it cannot beat the current reduced baseline on `large` and `xlarge`
- it starts mutating the shipped reducer path instead of staying parallel
- it requires widening into unsupported surfaces before it proves out
- it depends on heuristic behavior that cannot preserve finalist parity

## Follow-On Variants Allowed Inside This Same Experimental Lane

Only after the first prototype is measured and understood:

- tile-local top-k emission instead of global compact candidate merge
- reduced metrics payload instead of pure score-only payload
- alternate stable `pair_id` encodings
- constrained reduced-payload experiments

Do not evaluate these until the first prototype has either won or failed clearly.

## Checklist

### Phase 0 Checklist

- [ ] Record the current reduced `auto` baseline for `small`, `large`, and `xlarge`.
- [ ] Confirm that the narrow benchmark surface resolves `auto` to `metrics_count=10`.
- [ ] Add a dev-only runner switch or env gate for the shadow path.
- [ ] Verify that the main application path cannot reach the shadow path.

### Phase 1 Checklist

- [ ] Add separate shadow scratch allocation without mutating the shipped row-first path.
- [ ] Add explicit route diagnostics for the shadow path.
- [ ] Keep the existing row-first route fully intact and callable.

### Phase 2 Checklist

- [ ] Add a compact scored-candidate payload for the shadow search path.
- [ ] Add a shadow candidate reducer path that merges compact candidates only.
- [ ] Keep finalist exact replay on the current authoritative path.

### Phase 3 Checklist

- [ ] Verify best-params parity against the reduced row-first baseline.
- [ ] Verify top-k identity parity against the reduced row-first baseline.
- [ ] Verify finalist metrics parity after exact replay.
- [ ] Reject the slice immediately if parity does not hold.

### Phase 4 Checklist

- [ ] Benchmark `small` serially.
- [ ] Benchmark `large` serially.
- [ ] Benchmark `xlarge` serially.
- [ ] Reject the slice immediately if `large` or `xlarge` loses.
- [ ] Keep the slice only if it beats the reduced row-first baseline where it matters.

### Guardrail Checklist

- [ ] Do not wire the shadow path into the shipped application route.
- [ ] Do not change any existing metric-contract semantics.
- [ ] Do not remove or narrow existing functionality.
- [ ] Do not run benchmarks in parallel.
- [ ] Do not treat early wins as real until they are repeated serially on an idle GPU.

## Definition Of Done

This experiment is done when one of these is true:

- the shadow reduced-payload path beats the current reduced row-first baseline with parity and remains isolated from the product path
- or the experiment is rejected cleanly with measurements, and the repo keeps the current row-first shipped path unchanged
