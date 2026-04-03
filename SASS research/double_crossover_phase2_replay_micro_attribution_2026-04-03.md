# `double_crossover` Phase 2 Replay Micro-Attribution

Date: 2026-04-03

## Purpose

Attribute the retained shortlist replay bucket after the first replay-batching prototype, using the new runner-visible replay sub-buckets and fresh `xlarge/alma/alma/synth` reruns.

This note is intentionally focused on the replay/dataflow boundary, not on another SASS pass.

## Preconditions

Phase A0 readiness was established in:

- `SASS research/double_crossover_phase2_replay_artifact_manifest_2026-04-03.md`

That manifest records:

- compile success for `profile_gpu_dynamic_runner`
- targeted replay-path tests passing
- toolchain and GPU metadata
- clean process hygiene before the reruns

## Measured Surfaces

Fresh reruns were captured with:

- `double_ma`
- `alma/alma`
- `xlarge`
- `candle_shape=synth`
- `dynamic_mode=off`
- `cuda_graphs=off`
- `measure_iterations=3`
- `warmup_iterations=1`

Artifacts:

- `target/double_crossover_phase2_replay_micro_full17_none_2026-04-03.jsonl`
- `target/double_crossover_phase2_replay_micro_full17_halves_2026-04-03.jsonl`
- `target/double_crossover_phase2_replay_micro_full17_quartiles_2026-04-03.jsonl`
- `target/double_crossover_phase2_replay_micro_trade22_none_2026-04-03.jsonl`
- `target/double_crossover_phase2_replay_micro_trade22_halves_2026-04-03.jsonl`
- `target/double_crossover_phase2_replay_micro_trade22_quartiles_2026-04-03.jsonl`

Dispatch-shape artifacts:

- `target/double_crossover_phase2_replay_dispatch_full17_halves_2026-04-03.log`
- `target/double_crossover_phase2_replay_dispatch_full17_quartiles_2026-04-03.log`
- `target/double_crossover_phase2_replay_dispatch_trade22_halves_2026-04-03.log`
- `target/double_crossover_phase2_replay_dispatch_trade22_quartiles_2026-04-03.log`

## Surfaced Replay Sub-Buckets

New runner-visible fields:

- `host_finalize_shortlist_slice_replay_ms`
- `host_finalize_shortlist_slice_replay_descriptor_prep_ms`
- `host_finalize_shortlist_slice_replay_descriptor_upload_ms`
- `host_finalize_shortlist_slice_record_assembly_ms`
- `host_finalize_shortlist_slice_sort_ms`
- `host_finalize_shortlist_slice_diagnostics_ms`

## Averaged Results

Measure-phase means across three iterations:

| Surface | Mean wall | Mean replay | Descriptor prep | Descriptor upload | Record assembly | Sort | Diagnostics |
|---|---:|---:|---:|---:|---:|---:|---:|
| `full17 / none` | `323.33 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` |
| `full17 / halves` | `396.00 ms` | `31.33 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` |
| `full17 / quartiles` | `379.33 ms` | `30.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` |
| `trade22 / none` | `336.33 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` |
| `trade22 / halves` | `394.00 ms` | `32.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` |
| `trade22 / quartiles` | `392.00 ms` | `30.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` | `0.00 ms` |

## Dispatch Shape

The retained replay path still runs two replay launches per replay-enabled mode, but each launch is already batched across the contributing slice ranges:

- `full17 / halves`
  - `range_count=2`, `launch_pairs=100`, `dispatch=windowed_m17`
  - `range_count=4`, `launch_pairs=32`, `dispatch=windowed_m17`
- `full17 / quartiles`
  - `range_count=4`, `launch_pairs=200`, `dispatch=windowed_m17`
  - `range_count=2`, `launch_pairs=16`, `dispatch=windowed_m17`
- `trade22 / halves`
  - `range_count=2`, `launch_pairs=100`, `dispatch=windowed_m22`
  - `range_count=4`, `launch_pairs=32`, `dispatch=windowed_m22`
- `trade22 / quartiles`
  - `range_count=4`, `launch_pairs=200`, `dispatch=windowed_m22`
  - `range_count=2`, `launch_pairs=16`, `dispatch=windowed_m22`

## Direct Evidence

1. The replay-enabled surfaces still carry a real `~30-32 ms` shortlist replay bucket after the retained batching prototype.
2. The `none` control surfaces continue to show no shortlist replay bucket.
3. The newly surfaced host-side replay descriptor prep and replay descriptor upload buckets round to `0 ms` on all six truth surfaces.
4. Record assembly, sort, and diagnostics also round to `0 ms` on all six truth surfaces.
5. The retained replay path is no longer a many-launch fan-out problem, but it is still not a single monolithic replay launch either. Each replay-enabled mode currently executes two batched replay launches.

## Interpretation

The new measurement pass narrows the remaining replay problem substantially:

1. The remaining replay bucket is not explained by host descriptor prep at millisecond resolution.
2. It is not explained by host descriptor upload at millisecond resolution.
3. It is not explained by record assembly, sort, or diagnostics bookkeeping at millisecond resolution.
4. The remaining cost is therefore concentrated in the currently unsplit `replay_other` bucket.

Inference:

- `replay_other` most likely includes replay launch submission below millisecond resolution, GPU replay kernel execution, explicit synchronization, and replay decode/writeback that are still being aggregated together in the current surfaced timer layout.

## What This Rules Out

This pass does rule out one previously plausible follow-on:

1. A descriptor-reuse branch justified by large host descriptor build or upload cost is not supported by the current evidence.

## What Remains Ambiguous

This pass does not yet separate the retained `replay_other` bucket into:

1. replay launch submit overhead
2. replay kernel execution time
3. replay-side synchronization or decode/writeback

That ambiguity is now the only meaningful unresolved part of the replay follow-on.

## Outcome

The replay-dataflow branch now has a measurement-backed causal story:

1. the first batching step removed the gross replay fan-out waste
2. the remaining replay bucket is real
3. the remaining replay bucket is not a host descriptor prep/upload problem at the current measurement resolution
4. any further retained work should target `replay_other`, not descriptor reuse first
