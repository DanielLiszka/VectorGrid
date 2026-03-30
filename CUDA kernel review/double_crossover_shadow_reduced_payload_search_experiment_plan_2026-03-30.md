# Double Crossover Shadow Reduced-Payload Search Experiment Plan

Date: 2026-03-30

Scope: repo-grounded implementation plan for a dev-only, classic `double_ma` GPU optimization experiment that tests whether a lighter internal search payload can beat the current row-first `auto` / reduced-row baseline without affecting shipped application behavior.

This document is intentionally separate from:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md`

Those documents track the shipped path and the completed M17 / M22 phase. This document tracks an isolated experimental architecture only.

Primary code-facing sources:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`

## Purpose

The current evidence says:

- the shipped row-first path is still the best option for real `large` / `xlarge` workloads
- the current score-first prototype does not beat row-first where it matters
- the remaining credible win is not another local tuning pass, but a different search-stage dataflow

This experiment is meant to test one specific claim:

- can a lighter internal search payload outperform the current row-first reduced search path on classic `double_ma` exact GPU optimization?

This is not a product change. It is a falsifiable experiment.

## Why This Experiment Exists

The current row-first architecture still makes the all-pairs search stage do expensive work that may not be needed for ranking:

- write search rows for every evaluated pair
- reread those rows during shortlist selection
- keep full-row search artifacts even when final exact rows are only needed for finalists

At the same time, current exact large-benchmark timings show that `auto` / reduced-row, `full17`, and `trade22` are already in roughly the same band on the current architecture. That means row-count alone is not the dominant cost today. The likely remaining gain is not "fewer rows in the same pipeline." It is "lighter search payload in a different pipeline."

This experiment therefore targets:

- a reduced-payload search stage
- exact replay kept authoritative for finalist materialization
- no changes to the shipped route or request behavior

## Baseline

Current exact GPU benchmark reference surface for this experiment:

- strategy: `double_ma`
- fast/slow MA: `alma/alma`
- workload: `large` (`58,300` valid pairs)
- objective: `Sharpe`
- route: exact `GpuKernel`
- `include_all=false`
- `heatmap_bins=0`

Recent serial measurements on the current row-first tree:

- `metric_contract=auto`:
  - route diagnostics confirm `metrics_count=10` on this surface
  - recent run: `73 / 74 / 86 ms`
  - earlier near-idle run: `73 / 73 / 72 ms`
- `metric_contract=full17`:
  - recent run: `78 / 78 / 77 ms`
- `metric_contract=trade22`:
  - recent run: `76 / 75 / 77 ms`

Interpretation:

- the current `10`-row-equivalent exact surface is not dramatically ahead of `17` or `22`
- the experiment must therefore beat the current `auto` / `10`-row-equivalent row-first baseline, not just `full17` or `trade22`

## Hypothesis

The best remaining experiment is:

- keep the search stage on a compact payload
- avoid writing full search metric rows for every pair
- shortlist on compact scored candidates
- replay finalists through the current authoritative exact path

If that cannot beat the current row-first reduced path, the architecture claim is probably wrong or at least not worth pursuing in this codebase.

## Experiment Contract

This experiment must stay isolated from shipped behavior.

That means:

- no wiring into the main app request path
- no UI changes
- no Tauri command changes
- no change to the meaning of `auto`, `full17`, or `trade22`
- no change to CPU fallback behavior
- no change to current exact replay semantics
- no broadening to `include_all`, export, heatmap, shortlist union, shortlist confidence, shortlist slice, or DynamicDoubleMa while the experiment is still unproven

The experiment should be reachable only through a dev-only surface such as:

- a dedicated runner flag in `profile_gpu_dynamic_runner.rs`
- or a narrow environment flag used only by the runner / test harness

## Recommended First Slice

The first slice should be intentionally narrow.

Supported experiment surface:

- classic non-dynamic `double_ma`
- exact `GpuKernel`
- `objective=Sharpe`
- `include_all=false`
- `constraints=None`
- `heatmap_bins=0`
- no extra shortlist heads
- no shortlist union / confidence / slice rerank
- `metric_contract=auto` benchmark target first

Important detail:

- on this exact surface, `auto` currently resolves to `metrics_count=10`
- this does not mean the experiment is "the 10-row product path"
- it means the experiment competes against the current reduced search contract on a controlled exact surface

## Architecture Direction

The first experiment should not try to improve the existing row-first reducer chain.

It should instead introduce a parallel shadow architecture:

1. backtest search stage computes only the ranking payload needed for shortlist competition
2. search stage emits compact scored candidates, not full search rows
3. reducers merge compact scored candidates only
4. finalists are replayed through the existing exact path
5. final returned metrics remain authoritative and unchanged

Preferred mental model:

- search payload: reduced and score-oriented
- final output payload: current exact replay rows

## Candidate Payload Shapes

Preferred first candidate:

- `(score, pair_idx)` or `(score, compact_pair_idx)`

Possible follow-up variants:

- `(score, pair_idx, tiny metadata needed for exact replay routing)`
- tile-local shortlist emission consumed by a host/device merge stage

Not recommended for the first slice:

- attached full metrics rows
- score side-buffers threaded through the current row reducers
- score plus feasibility fusion
- broad pair-code packing experiments before the simpler candidate flow proves out

## Implementation Boundaries

### Must Reuse

- current exact replay path in `gpu_backtest_kernel.rs`
- current final metric materialization semantics
- current route diagnostics / timing output
- current runner-based benchmark surfaces

### Must Not Disturb

- row-first shipped path
- current `VECTORBT_KERNEL_SCORE_FIRST_TOPK` experimental branch semantics
- main application request routing
- existing M17 / M22 exact path behavior

### Likely Files

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - shadow scratch buffers
  - dev-only shadow route gate
  - shadow shortlist merge flow
  - replay handoff
- `crates/app_core/src/kernels/double_crossover.cu`
  - compact scored-candidate emission or tile-local shortlist kernels
- `crates/app_core/src/double_ma.rs`
  - narrow parity or route-diagnostic tests if needed
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
  - runner-only opt-in flag for the experiment

## Verification Strategy

The experiment should be judged in this order:

1. Does it return?
2. Does it preserve winner identity and final exact metrics?
3. Does it beat current row-first `auto` on `large` and `xlarge`?
4. Does it avoid regressing `small` materially?

Minimum verification surfaces:

- `small`, `large`, `xlarge`
- `alma/alma`
- `objective=Sharpe`
- exact `GpuKernel`
- `metric_contract=auto`

Only after that:

- `full17`
- `trade22`
- replay-backed shortlist surfaces

## Acceptance Criteria

The experiment is worth keeping only if all of the following are true:

- same winning params as the current row-first baseline on the controlled exact surface
- same final exact metrics after replay
- measurable win on `large`
- measurable win on `xlarge`
- no material regression on `small`
- no contamination of shipped app behavior

If any of those fail, the experiment should remain runner-only or be removed.

## Rejection Criteria

Reject the experiment immediately if:

- it only wins on `small`
- it loses on `large` or `xlarge`
- it needs broad route/product wiring just to function
- it reproduces the old pattern of looking promising on one narrow surface while regressing the realistic surfaces
- it starts mutating the current row-first path instead of staying parallel and isolated

## Suggested Phase Order

### Phase 0: Scaffolding

Goal:

- create a runner-only shadow path that cannot affect shipped behavior

Done when:

- the experiment is opt-in only
- the current app route never selects it
- diagnostics clearly identify when the shadow path is active

### Phase 1: Minimal Compact Search Payload

Goal:

- prove the experiment can search and shortlist on a compact payload for the controlled `auto` / reduced exact surface

Done when:

- the shadow path runs end to end
- finalists replay through the current exact path
- parity is proven on the controlled surface

### Phase 2: Benchmark Decision

Goal:

- decide whether the shadow path actually beats the current row-first reduced path

Done when:

- `small`, `large`, and `xlarge` are benchmarked serially
- the keep/reject decision is evidence-backed

### Phase 3: Optional Broadening

Goal:

- only if Phase 2 wins, decide whether to extend to `full17`, `trade22`, or replay-backed shortlist surfaces

Done when:

- broadening is either explicitly accepted with evidence or explicitly deferred

## Checklist

### Phase 0 Checklist

- [ ] Create a new shadow experiment doc/ledger entry for implementation progress.
- [ ] Add a runner-only opt-in flag for the shadow reduced-payload experiment.
- [ ] Ensure the main application path cannot activate the experiment.
- [ ] Add route diagnostics that clearly print when the shadow path is active.
- [ ] Record the current row-first `auto` / reduced baseline before landing experiment code.

### Phase 1 Checklist

- [ ] Keep the first slice limited to classic `double_ma`, exact `GpuKernel`, `objective=Sharpe`, `include_all=false`, `constraints=None`, `heatmap_bins=0`.
- [ ] Add shadow scratch buffers for compact search payloads only.
- [ ] Add a parallel shadow search flow instead of mutating the current row-first reducer path.
- [ ] Emit compact scored candidates without requiring full search rows as the primary shortlist payload.
- [ ] Reuse the current exact replay path for finalist materialization.
- [ ] Confirm the shadow path returns valid winners on the controlled surface.
- [ ] Confirm route diagnostics identify the shadow path correctly.

### Phase 2 Checklist

- [ ] Verify best-params parity against the current row-first `auto` baseline.
- [ ] Verify final exact metrics parity after replay.
- [ ] Benchmark `small` serially.
- [ ] Benchmark `large` serially.
- [ ] Benchmark `xlarge` serially.
- [ ] Reject immediately if `large` or `xlarge` regresses.
- [ ] Keep the experiment only if it beats the current row-first reduced baseline on the realistic surfaces.

### Phase 3 Checklist

- [ ] Decide whether `full17` broadening is justified.
- [ ] Decide whether `trade22` broadening is justified.
- [ ] Decide whether replay-backed shortlist surfaces are justified.
- [ ] Do not broaden the experiment if the reduced baseline is not clearly beaten first.

### Explicit Guardrail Checklist

- [ ] Do not wire this experiment into the UI.
- [ ] Do not wire this experiment into the default app request path.
- [ ] Do not treat this as a product change from `auto` to a new public mode.
- [ ] Do not regress current row-first shipped behavior.
- [ ] Do not run multiple benchmarks at the same time.
- [ ] Do not treat a `small`-only win as success.

## Definition Of Done

This experiment is done when one of these outcomes is reached:

1. It clearly beats the current row-first reduced exact path on `large` and `xlarge`, while preserving exact replay semantics and staying isolated from shipped behavior.
2. It fails to do so, is rejected with evidence, and the repo keeps row-first as the clear winner.

Either outcome is useful. The point of this experiment is to answer the architecture question cleanly, not to force a win.
