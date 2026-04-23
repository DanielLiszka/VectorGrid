# `double_crossover` Phase 2 Replay-Dataflow Architecture Plan

Date: 2026-04-03

## Purpose

Define the next concrete planning branch after the retained shortlist replay-batching prototype.

This plan is narrower than a full `compact_exact_search_stage` redesign. It treats the current state as:

1. the retained lockstep-body pass is complete
2. the retained shortlist replay-batching pass is complete
3. the next evidence-backed question is whether more meaningful headroom remains in replay/dataflow, not in local shortlist finalize cleanup or another tiny-`windowed` kernel pass

## Research Objective

This branch is not just another SASS-to-code cycle. The research question is whether static-plus-dynamic attribution can identify replay/dataflow boundaries that are worth architectural work after the obvious kernel-local wins are already retained.

The branch should therefore produce:

1. one measurement-backed causal story for the remaining shortlist replay cost
2. one explicit decision about whether replay/dataflow architecture work is justified
3. reusable artifacts that support a broader methodology paper or internal research memo, even if the final decision is to defer further product work

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

1. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=full17`, `shortlist_slice_mode=none`
2. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=trade22`, `shortlist_slice_mode=none`
3. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=full17`, `shortlist_slice_mode=halves`
4. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=trade22`, `shortlist_slice_mode=halves`
5. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=full17`, `shortlist_slice_mode=quartiles`
6. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=trade22`, `shortlist_slice_mode=quartiles`

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

## Phase A0: Hygiene And Baseline Readiness

Objective: make the replay branch reproducible before more profiling or code changes.

Required preconditions:

1. verify there is no other active benchmark or profiler process
2. record the current commit, GPU model, driver version, CUDA version, and Nsight tool versions in the phase artifact manifest
3. run a compile pass for `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
4. run the retained targeted replay-path tests before collecting new measurement artifacts

Expected output:

1. `SASS research/double_crossover_phase2_replay_artifact_manifest_2026-04-03.md`

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

1. one repo note that attributes the retained replay bucket into the subcomponents above:
   - `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
2. runner-visible or log-visible timing fields that can be compared across `none`, `halves`, and `quartiles`
3. one short decision memo stating what dominates the remaining replay cost:
   - `SASS research/double_crossover_phase2_replay_branch_decision_2026-04-03.md`
4. one artifact manifest listing the exact rerun JSONL files and any profiler or log outputs kept for this phase:
   - `SASS research/double_crossover_phase2_replay_artifact_manifest_2026-04-03.md`

Exit condition for Phase A:

1. at least `80%` of the retained replay bucket is attributed to named subcomponents
2. one dominant next target is clear enough to justify a bounded prototype

## Phase B: Branch Decision

After Phase A, choose exactly one of these branches:

### Branch B1: Replay Descriptor Reuse

Choose this if host replay descriptor build or upload is a large share of the remaining replay cost.

Promotion rule:

1. descriptor build plus upload is at least `8 ms` absolute on a retained `halves` truth surface
2. or descriptor build plus upload is at least `35%` of the replay bucket on that surface

Candidate ideas:

1. keep replay descriptors device-resident across active and diagnostics replay passes
2. reuse pair maps, period-row indices, and local-row indices for diagnostics subsets instead of rebuilding them
3. upload only the changed eval-range payload when the row identity payload is unchanged

### Branch B2: Replay Payload Compaction

Choose this if the remaining cost is still mostly replay compute or replay payload size rather than descriptor rebuild.

Promotion rule:

1. replay kernel work plus replay payload movement is at least `8 ms` absolute on a retained `halves` truth surface
2. or replay kernel work plus replay payload movement is at least `35%` of the replay bucket on that surface
3. and one targeted `ncu` or `nsys` refresh on `windowed_m17` / `windowed_m22` confirms that the remaining cost is not primarily host descriptor work

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
4. no single subcomponent clears the numeric promotion gates for `B1` or `B2`

## Preferred Immediate Candidate

Based on current repo evidence, the first branch to evaluate is:

1. `Branch B1: Replay Descriptor Reuse`

Why:

1. the first batching step already removed the obvious launch-fan-out waste
2. the remaining work is now more likely to be replay descriptor preparation, replay input movement, or duplicate diagnostics replay setup than another raw kernel-local inefficiency
3. this stays inside `gpu_backtest_kernel.rs` and preserves the current exact-row contract better than jumping straight to full compact-search redesign

Phase A update:

1. The first replay micro-attribution pass is now complete.
2. `B1` did not clear its numeric promotion rules.
3. The current no-new-code branch outcome is `B3: Clean Deferral` pending one more attribution layer inside `replay_other`.

## Concrete Checklist

### A. Measurement

- [x] Confirm A0 readiness: no active benchmark/profiler processes, repo and toolchain versions recorded, compile pass green, and targeted replay-path tests green.
- [x] Surface replay micro-timings for descriptor build, descriptor upload, replay launch, replay kernel work, and decode/writeback in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [x] Distinguish ranking replay from diagnostics-only replay in the surfaced timing output.
- [x] Re-run shortlist-heavy `xlarge` `full17` and `trade22` on `none`, `halves`, and `quartiles` with the new timing fields.
- [ ] If the repo has a stable non-synthetic shortlist-slice surface on the exact `GpuKernel` route, add one retained rerun there as a product-relevance cross-check.
- [x] Capture retained replay launch counts and launch-pair totals per mode alongside the new timing fields so dispatch-shape evidence stays visible after this phase.
- [x] Write a short attribution note that states which replay subcomponent is now dominant.

### B. Decision Gate

- [x] Choose exactly one immediate branch: descriptor reuse, payload compaction, or clean deferral.
- [x] Apply the numeric branch-promotion rules before promoting `B1` or `B2`.
- [x] Write one short branch-decision memo that states why the two non-chosen branches were not promoted.
- [x] Reject broader `compact_exact_search_stage` work unless the new attribution clearly points to finalist materialization or replay setup as the real next frontier.

### C. If Descriptor Reuse Wins

- [ ] Define the replay-descriptor reuse key explicitly:
  - param identity
  - pair-index rows
  - fast and slow period rows
  - local-row indices
  - eval-range payload
- [ ] Define the replay descriptor identity that can be reused safely across active and diagnostics replay.
- [ ] Define what can stay device-resident and what must still be rebuilt per mode.
- [ ] Prototype exactly one bounded descriptor-reuse change in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [ ] Add targeted tests for descriptor reuse on shortlist replay, including `trade22` tail-metric preservation.

### D. Verification

- [ ] Re-run shortlist-heavy `xlarge` truth surfaces after any retained change.
- [ ] Re-run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [ ] Re-run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`.
- [ ] Save explicit parity artifacts for reranked ordering, deterministic tie behavior, `best_params`, `best_metrics`, and `trade22` tail metrics on the retained surfaces.
- [ ] Save explicit parity artifacts proving no change to exact `full17` / `trade22` row semantics.
- [ ] Save explicit parity artifacts proving no change to shortlist diagnostics semantics.
- [ ] Save explicit parity artifacts showing `halves` and `quartiles` candidate diagnostics counts and slice labels still match current semantics.
- [ ] Save dispatch artifacts showing replay launch counts and `launch_pairs` on the retained implementation.

### E. Research Outputs

- [x] Write one replay micro-attribution memo after Phase A.
- [x] Write one retained-vs-deferred branch decision memo after Phase B.
- [x] Save one before/after artifact manifest for any retained next-step prototype.
- [x] Write one short methodology note stating what this branch adds to the repo’s broader static-plus-dynamic CUDA research story.

### F. Mergeback

- [x] Update `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`.
- [x] Update `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`.
- [x] Update `SASS research/double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md`.
- [x] If no new prototype is retained, still write a clean deferral memo so this branch remains a research artifact rather than dead work.

## Success Criteria

For a retained next-step prototype, require all of the following:

1. clear improvement on shortlist-heavy `xlarge` `halves` for at least one of `full17` or `trade22`
2. no regression on the `large` contract-row control surfaces beyond normal gate tolerance
3. exact `trade22` tail metrics remain correct
4. shortlist diagnostics outputs remain present and aligned with the current semantics

Suggested practical performance bar:

1. at least `8-10 ms` wall-time improvement on the shortlist-heavy `halves` surface
2. or at least `20%` reduction in the newly attributed dominant replay subcomponent
3. no meaningful regression on the matching `quartiles` surface when diagnostics semantics are preserved

## Rejection Criteria

Reject the next prototype if any of the following is true:

1. the apparent win only comes from reducing diagnostics semantics
2. the win only appears on `quartiles` and not on the real `halves` ranking path
3. the control surfaces regress materially
4. the code complexity expands toward full compact-search redesign without enough new evidence
5. the measured gain is too small to justify the added maintenance burden
6. the newly attributed dominant subcomponent is too small in absolute time to justify architecture work even if a percentage delta looks attractive

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
