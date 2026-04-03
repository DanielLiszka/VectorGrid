# `double_crossover` Phase 2 Replay Branch Decision

Date: 2026-04-03

## Purpose

Choose the immediate next replay/dataflow branch after the first replay micro-attribution pass.

Inputs:

- `SASS research/double_crossover_phase2_replay_dataflow_architecture_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_artifact_manifest_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`

## Numeric Promotion Rules

The plan set these promotion rules:

- promote `B1: Replay Descriptor Reuse` only if descriptor build plus upload is at least `8 ms` absolute or at least `35%` of the replay bucket on a retained `halves` truth surface
- promote `B2: Replay Payload Compaction` only if replay kernel work plus replay payload movement is at least `8 ms` absolute or at least `35%` of the replay bucket, and a targeted profiler refresh confirms the cost is not primarily host descriptor work
- otherwise choose `B3: Clean Deferral`

## Branch Evaluation

### `B1: Replay Descriptor Reuse`

Decision: do not promote.

Direct evidence:

- `full17 / halves`: descriptor prep `0 ms`, descriptor upload `0 ms`, replay `31.33 ms`
- `trade22 / halves`: descriptor prep `0 ms`, descriptor upload `0 ms`, replay `32.00 ms`

Result against the gate:

- absolute gate: failed
- percentage gate: failed

Conclusion:

- the current evidence does not support descriptor reuse as the next bounded implementation lane

### `B2: Replay Payload Compaction`

Decision: not yet promoted.

Direct evidence:

- the retained replay bucket remains `~30-32 ms`
- the new host-side replay sub-buckets do not explain that cost
- existing replay dispatch logs still show two batched replay launches per replay-enabled mode on the retained path

Why promotion is still deferred:

1. the current measurement pass only proves that descriptor prep/upload is not the driver
2. it does not yet split `replay_other` into replay kernel execution, replay launch submit, or replay decode/writeback
3. promoting a CUDA-facing payload change before resolving that remaining ambiguity would violate the branch’s own measurement-first intent

### `B3: Clean Deferral`

Decision: promote as the immediate branch outcome for code changes.

Meaning here:

1. do not start another replay prototype yet
2. keep the branch in measurement and research mode
3. only reopen implementation if the next measurement layer isolates `replay_other` enough to justify it

This is a deferral of new code changes, not abandonment of the research branch.

## Final Decision

The immediate branch decision is:

1. `B1` rejected
2. `B2` deferred pending one more targeted attribution layer inside `replay_other`
3. `B3` selected as the current no-new-code branch outcome

## Exact Next Steps

1. Keep the retained replay-batching code unchanged.
2. Treat the current replay-dataflow checklist as still active, with the next measurement task narrowed to `replay_other`.
3. Add one more bounded attribution layer that isolates:
   - replay launch submit
   - replay kernel execution
   - replay decode or writeback
4. If necessary, pair that with one targeted profiler refresh on the `windowed_m17` / `windowed_m22` replay kernels.
5. Only reopen a new replay-side prototype if that next attribution layer shows a dominant subcomponent that clears the plan’s numeric promotion rules.

## Why This Is Still Useful Research

This is still a positive research outcome:

1. the repo now shows a full example where static and dynamic evidence identified the right optimization layer
2. a retained batching prototype produced a real improvement
3. a follow-on attribution pass then ruled out one plausible next branch instead of guessing
4. the methodology is getting stronger, not weaker, by refusing to promote a new architecture change without causal evidence
