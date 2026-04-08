# `double_crossover` Phase 2 Research Findings

Date: 2026-04-03

## Purpose

Summarize the retained Phase 1 and Phase 2 evidence for `double_crossover` as a research artifact rather than as an open-ended optimization log.

## Scope

This memo covers:

1. the exact `double_ma` GPU route for `double_crossover`
2. static SASS-guided source and layout work
3. runtime attribution on the retained exact path
4. one retained replay-batching intervention
5. one measured rejection of descriptor reuse as the next branch

It does not claim:

1. that the whole kernel family should be rewritten in SASS
2. that all meaningful performance headroom has been exhausted
3. that a full compact-search architecture redesign is already justified

## Repo Constraints

The retained work preserved:

1. exact `full17` / `trade22` row authority
2. shortlist diagnostics semantics
3. CPU fallback
4. the existing row-first exact route as the authoritative product path

## Main Research Claim

The strongest current artifact is:

1. static SASS plus runtime attribution can identify the correct optimization layer for an irregular exact CUDA kernel pipeline more reliably than continuing with generic low-level tuning

In this repo, that methodology already produced:

1. one retained lockstep-body source/runtime improvement
2. one retained replay/dataflow improvement
3. one measured negative result that ruled out descriptor reuse as the next justified branch

## Evidence Sequence

### 1. Static SASS Identified The Right Early Targets

The initial SASS work pointed to:

1. wrapper and stride contract issues
2. exact-row store shape issues
3. reducer and output-path questions

That work is recorded in:

1. `SASS research/double_crossover_performance_findings_and_code_suggestions.md`
2. `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`

## 2. Runtime Profiling Ruled Out A Reducer-First Story

The retained Phase 2 runtime pass showed:

1. `large/full17`: about `74.8 ms`
2. `large/trade22`: about `77.0 ms`
3. `tile_score` consuming about `95%+` of pipeline time on the large exact surfaces
4. Nsight Systems dominated by `lockstep_time_m17` / `lockstep_time_m22`
5. Nsight Compute showing scoreboard and access-shape pressure rather than spill-driven behavior

This is recorded in:

1. `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`

## 3. A Bounded Lockstep-Body Pass Produced A Retained Win

The retained lockstep row-base change:

1. stayed inside the exact lockstep path
2. passed both authoritative GPU perf gates
3. preserved the exact contracts

This established that the methodology could still produce a bounded product-facing win after the initial SASS wave.

## 4. Shortlist-Slice Attribution Identified Replay As The Next Layer

The shortlist-heavy `xlarge/alma/alma/synth` attribution work showed:

1. `full17`: `none 297.40 ms`, `halves 379.20 ms`, `quartiles 986.80 ms`
2. `trade22`: `none 319.60 ms`, `halves 396.20 ms`, `quartiles 599.00 ms`
3. replay dominated the surfaced shortlist slice bucket
4. host-side record assembly, sort, and diagnostics bookkeeping were too small to promote first

This is recorded in:

1. `SASS research/double_crossover_phase2_shortlist_slice_attribution_log_2026-04-03.md`

## 5. Replay Batching Produced A Retained Replay/Dataflow Win

The retained replay-batching change reduced replay fan-out by batching the replay ranges instead of replaying one launch per range.

Retained shortlist-heavy reruns moved to:

1. `full17`: `none 296.00 ms`, `halves 342.80 ms`, `quartiles 349.00 ms`
2. `trade22`: `none 308.20 ms`, `halves 359.00 ms`, `quartiles 365.00 ms`

That is a real win on replay-enabled surfaces while preserving exact row and diagnostics semantics.

This is recorded in:

1. `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`

## 6. Descriptor Reuse Was Measured And Rejected

After the retained replay-batching step, the replay micro-attribution pass showed:

1. `full17 / halves`: replay `31.33 ms`, descriptor prep `0 ms`, descriptor upload `0 ms`
2. `trade22 / halves`: replay `32.00 ms`, descriptor prep `0 ms`, descriptor upload `0 ms`
3. `none` control surfaces still showed no replay bucket

So descriptor reuse did not clear the branch’s numeric promotion rule and is not justified as the next prototype.

This is recorded in:

1. `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
2. `SASS research/double_crossover_phase2_replay_branch_decision_2026-04-03.md`

## What The Repo Now Proves

Direct evidence:

1. static SASS was useful for picking early source/layout targets
2. runtime profiling was necessary to distinguish backtest-body costs from replay/dataflow costs
3. one bounded lockstep-body change and one bounded replay-dataflow change both produced retained wins
4. a plausible next branch, descriptor reuse, can be rejected with direct measurement rather than opinion

## What Remains Ambiguous

The current evidence does not yet split the retained replay `replay_other` bucket into:

1. launch-submit overhead
2. replay kernel execution
3. replay decode or writeback

That is now the main unresolved technical ambiguity.

## Product Conclusion

The repo should not start another replay-side code prototype by default.

The current evidence supports:

1. keeping the retained lockstep-body and replay-batching changes
2. freezing further code changes unless one more measurement-only pass isolates a new dominant replay subcomponent

## Research Conclusion

The branch already supports a real artifact:

1. a static-plus-dynamic attribution case study for irregular exact CUDA kernels

The strongest research contribution is not “we hand-tuned everything.” It is:

1. the repo used static SASS to find the right early source/runtime layer
2. the repo used runtime attribution to identify replay as the next layer
3. the repo retained one replay/dataflow win
4. the repo then explicitly rejected an unjustified next branch

## Current Stop Rule

Do not start another optimization prototype unless:

1. the next measurement-only pass splits `replay_other` and reveals a dominant subcomponent that clears the branch’s numeric promotion rules, or
2. the research artifact explicitly needs one more bounded intervention to close a causal gap

Otherwise, the correct next move is artifact consolidation, not more code.
