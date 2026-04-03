# `double_crossover` Phase 2 Replay Methodology Note

Date: 2026-04-03

## Purpose

Record what this replay-dataflow branch adds to the repo’s broader research methodology.

## Methodological Contribution

This branch extends the repo beyond a simple cycle of:

1. read SASS
2. make a code change
3. rerun a benchmark

The retained methodology is now:

1. use static SASS and coarse runtime profiling to identify the right optimization layer
2. retain one bounded code change that clearly improves the targeted path
3. add post-change micro-attribution inside that layer
4. use numeric branch-promotion rules to decide whether another implementation step is justified
5. treat negative results as useful evidence rather than as failed optimization attempts

## What This Repo Now Demonstrates

The Phase 2 replay branch now has all of the following:

1. a retained replay-batching prototype with real shortlist-heavy wins
2. preserved exact `full17` / `trade22` row authority and shortlist diagnostics semantics
3. a follow-on attribution pass that rules out descriptor reuse as the next step
4. a recorded decision to stay in measurement mode until the remaining `replay_other` bucket is split further

## Why This Matters

This is a stronger research story than blind iterative tuning:

1. the repo is using runtime evidence to choose the optimization layer deliberately
2. the repo is using bounded interventions rather than speculative architecture rewrites
3. the repo is explicitly documenting when a plausible next branch should not be promoted

## Practical Outcome

The next replay/dataflow branch should start from:

1. `SASS research/double_crossover_phase2_replay_artifact_manifest_2026-04-03.md`
2. `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
3. `SASS research/double_crossover_phase2_replay_branch_decision_2026-04-03.md`

That gives the repo a reusable evidence chain for future product work and for a broader static-plus-dynamic CUDA optimization research writeup.
