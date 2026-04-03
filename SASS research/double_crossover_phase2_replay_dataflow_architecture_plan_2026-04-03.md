# `double_crossover` Phase 2 Replay-Dataflow Architecture Plan

Date: 2026-04-03

## Purpose

Define the next concrete planning branch after the retained shortlist replay-batching prototype.

This plan is narrower than a full `compact_exact_search_stage` redesign. It treats the current state as:

1. the retained lockstep-body pass is complete
2. the retained shortlist replay-batching pass is complete
3. the next evidence-backed question is whether more meaningful headroom remains in replay/dataflow, not in local shortlist finalize cleanup or another tiny-`windowed` kernel pass

## Current State

What the repo now proves:

1. The admitted `large` exact route remains primarily a backtest-body problem, not a reducer-first problem.
2. The shortlist-heavy `xlarge` follow-on is real, but it is specific to `shortlist_slice_mode`, not a broad speedup for the normal contract-row benchmark path.
3. The first retained replay-batching step materially reduced replay fan-out:
   - old `halves` replay shape: `2x50` shortlist rows plus `4x8` diagnostics rows
   - retained shape: one launch per replay mode
4. The retained shortlist replay bucket is now about `28 ms` on both `full17` and `trade22`.
5. Exact `full17` / `trade22` row authority and shortlist diagnostics semantics were preserved.

What the repo does not yet prove:

1. Whether the remaining replay cost is now mostly:
   - host descriptor build
   - host-to-device replay input upload
   - actual replay kernel compute
   - duplicate active-vs-diagnostics replay setup
2. Whether the next meaningful gain comes from:
   - replay descriptor reuse
   - device-side finalist or replay-input emission
   - broader compact exact search-stage work

## Scope

Primary ownership surface:

1. `crates/app_core/src/gpu_backtest_kernel.rs`

Primary functions and code paths:

1. `compute_shortlist_slice_mode_records(...)`
2. `rerank_shortlist_by_slice_robustness(...)`
3. `eval_double_ma_batch_gpu_kernel_impl(...)`
4. the shortlist replay path under `if let Some(mode) = shortlist_slice_mode`

Secondary supporting surface:

1. `crates/app_core/src/double_ma.rs`

CUDA surface remains secondary for this branch:

1. `crates/app_core/src/kernels/double_crossover.cu`

Only touch CUDA again if the replay-dataflow evidence says the remaining cost is still dominated by replay launch compute rather than host-side replay setup or payload movement.

## Fixed Guardrails

These are non-negotiable for this branch:

1. Keep exact `full17` / `trade22` row authority unchanged.
2. Keep shortlist diagnostics semantics unchanged.
3. Do not remove `quartiles` diagnostics to claim a win.
4. Do not reopen blanket PTX or SASS work.
5. Keep the current row-first exact route available until parity and perf are both proven on any larger redesign.

## Primary Truth Surfaces

Use these exact surfaces unless a narrower micro-study is explicitly called out:

1. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=full17`, `shortlist_slice_mode=halves`
2. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=trade22`, `shortlist_slice_mode=halves`
3. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=full17`, `shortlist_slice_mode=quartiles`
4. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=trade22`, `shortlist_slice_mode=quartiles`

Use these as control surfaces to ensure the branch does not drift into fake wins:

1. `double_ma`, `alma/alma`, `large`, `metric_contract=auto`
2. `double_ma`, `alma/alma`, `large`, `metric_contract=full17`
3. `double_ma`, `alma/alma`, `large`, `metric_contract=trade22`

## Main Question

The next planning question is:

"Can replay work across shortlist rows, replay descriptors, and diagnostics modes be reduced further without changing exact result or diagnostics semantics?"

This is deliberately narrower than:

1. full compact exact search-stage redesign
2. route broadening to heatmap/include-all/export/union/confidence
3. multi-streaming or larger runtime orchestration changes

## Phase A: Replay Micro-Attribution

Objective: split the remaining `~28 ms` replay bucket into concrete subcomponents before choosing another implementation lane.

Add or surface timing for:

1. range generation
2. replay row preparation
3. replay descriptor build
4. replay descriptor upload
5. replay launch submit
6. replay kernel execution
7. replay decode or result writeback
8. diagnostics-only replay work vs ranking-critical replay work

Required outputs:

1. one repo note that attributes the retained replay bucket into the subcomponents above
2. runner-visible or log-visible timing fields that can be compared across `halves` and `quartiles`
3. one short decision memo stating what dominates the remaining replay cost

Exit condition for Phase A:

1. at least `80%` of the retained replay bucket is attributed to named subcomponents
2. one dominant next target is clear enough to justify a bounded prototype

## Phase B: Branch Decision

After Phase A, choose exactly one of these branches:

### Branch B1: Replay Descriptor Reuse

Choose this if host replay descriptor build or upload is a large share of the remaining replay cost.

Candidate ideas:

1. keep replay descriptors device-resident across active and diagnostics replay passes
2. reuse pair maps, period-row indices, and local-row indices for diagnostics subsets instead of rebuilding them
3. upload only the changed eval-range payload when the row identity payload is unchanged

### Branch B2: Replay Payload Compaction

Choose this if the remaining cost is still mostly replay compute or replay payload size rather than descriptor rebuild.

Candidate ideas:

1. use a more compact replay-descriptor representation
2. reduce replay payload width before the final authoritative row materialization
3. make replay consume device-side finalist descriptors more directly

### Branch B3: Clean Deferral

Choose this if the remaining replay cost is too small, too diffuse, or too risky relative to product value.

Valid reasons to stop:

1. remaining replay headroom is too small to justify architecture complexity
2. improvements would only help diagnostics-heavy paths and not real product usage
3. preserving exact row and diagnostics semantics would require too much redesign for too little measured return

## Preferred Immediate Candidate

Based on current repo evidence, the first branch to evaluate is:

1. `Branch B1: Replay Descriptor Reuse`

Why:

1. the first batching step already removed the obvious launch-fan-out waste
2. the remaining work is now more likely to be replay descriptor preparation, replay input movement, or duplicate diagnostics replay setup than another raw kernel-local inefficiency
3. this stays inside `gpu_backtest_kernel.rs` and preserves the current exact-row contract better than jumping straight to full compact-search redesign

## Concrete Checklist

### A. Measurement

- [ ] Surface replay micro-timings for descriptor build, descriptor upload, replay launch, replay kernel work, and decode/writeback in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [ ] Distinguish ranking replay from diagnostics-only replay in the surfaced timing output.
- [ ] Re-run shortlist-heavy `xlarge` `full17` and `trade22` on both `halves` and `quartiles` with the new timing fields.
- [ ] Write a short attribution note that states which replay subcomponent is now dominant.

### B. Decision Gate

- [ ] Choose exactly one immediate branch: descriptor reuse, payload compaction, or clean deferral.
- [ ] Reject broader `compact_exact_search_stage` work unless the new attribution clearly points to finalist materialization or replay setup as the real next frontier.

### C. If Descriptor Reuse Wins

- [ ] Define the replay descriptor identity that can be reused safely across active and diagnostics replay.
- [ ] Define what can stay device-resident and what must still be rebuilt per mode.
- [ ] Prototype exactly one bounded descriptor-reuse change in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [ ] Add targeted tests for descriptor reuse on shortlist replay, including `trade22` tail-metric preservation.

### D. Verification

- [ ] Re-run shortlist-heavy `xlarge` truth surfaces after any retained change.
- [ ] Re-run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [ ] Re-run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`.
- [ ] Confirm no change to exact `full17` / `trade22` row semantics.
- [ ] Confirm no change to shortlist diagnostics semantics.

## Success Criteria

For a retained next-step prototype, require all of the following:

1. clear improvement on shortlist-heavy `xlarge` `halves` for at least one of `full17` or `trade22`
2. no regression on the `large` contract-row control surfaces beyond normal gate tolerance
3. exact `trade22` tail metrics remain correct
4. shortlist diagnostics outputs remain present and aligned with the current semantics

Suggested practical performance bar:

1. at least `8-10 ms` wall-time improvement on the shortlist-heavy `halves` surface
2. or at least `20%` reduction in the newly attributed dominant replay subcomponent

## Rejection Criteria

Reject the next prototype if any of the following is true:

1. the apparent win only comes from reducing diagnostics semantics
2. the win only appears on `quartiles` and not on the real `halves` ranking path
3. the control surfaces regress materially
4. the code complexity expands toward full compact-search redesign without enough new evidence
5. the measured gain is too small to justify the added maintenance burden

## Explicitly Not Next

These are not the immediate next step from the current evidence:

1. another `windowed` kernel micro-pass
2. reducer-first work
3. broad SASS rewriting
4. route broadening to heatmap/include-all/export/union/confidence
5. full compact exact search-stage redesign before replay micro-attribution is complete

## Decision Output

This plan is complete when the repo has all of the following:

1. a replay micro-attribution note for the retained `~28 ms` replay bucket
2. one explicit branch decision: descriptor reuse, payload compaction, or clean deferral
3. if a new prototype is retained, fresh truth-surface reruns plus both authoritative GPU perf gates

At that point, fold the chosen result back into the main Phase 2 plan and continue from there rather than creating another parallel architecture plan.
