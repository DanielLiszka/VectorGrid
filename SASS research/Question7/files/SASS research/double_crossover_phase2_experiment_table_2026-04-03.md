# `double_crossover` Phase 2 Experiment Table

Date: 2026-04-03

## Purpose

Provide one compact table of the retained and rejected Phase 2 branches.

| Branch | Hypothesis | Main surfaces | Before | After / Outcome | Verification | Result |
|---|---|---|---|---|---|---|
| Lockstep row-base pass | The exact lockstep path still has a bounded source/runtime addressing improvement available. | `large/full17`, `large/trade22`, shortlist-heavy `xlarge` checks | Stage C pre-change direct-runner and profiler baselines | Retained bounded lockstep-body change; authoritative GPU gates passed. | `run_gpu_perf_gate.ps1`, `run_gpu_alma_contract_rows_perf_gate.ps1`, narrow cuobjdump refresh | Retained |
| Shortlist-slice attribution | The next shortlist-facing cost is replay, not host-side bookkeeping. | `xlarge/alma/alma/synth`, `full17/trade22`, `none/halves/quartiles` | `full17 none 297.40 ms`, `halves 379.20 ms`; `trade22 none 319.60 ms`, `halves 396.20 ms` | Replay dominated surfaced shortlist slice cost; record assembly, sort, diagnostics too small to promote. | instrumentation in runner + pipeline logs + focused Nsight Systems | Retained evidence |
| Replay-kernel study | The retained replay family is still a tiny-launch replay problem, not a large throughput-heavy kernel problem. | shortlist-heavy `xlarge`, `windowed_m17`, `windowed_m22` | replay family only loosely attributed before focused study | `windowed_m17/m22` profiled as tiny `grid=(1,1,1)` kernels with very low achieved occupancy; old `halves` route still issued `6` replay launches. | targeted Nsight Compute + dispatch logs | Retained evidence |
| Replay batching | Multi-range replay can be packed into fewer launches without changing exact row or diagnostics semantics. | `xlarge/alma/alma/synth`, `full17/trade22`, `none/halves/quartiles` | `full17 halves 379.20 ms`, `quartiles 986.80 ms`; `trade22 halves 396.20 ms`, `quartiles 599.00 ms` | `full17 halves 342.80 ms`, `quartiles 349.00 ms`; `trade22 halves 359.00 ms`, `quartiles 365.00 ms`; replay bucket collapsed to about `28 ms`. | targeted replay-path tests + both GPU perf gates | Retained |
| Replay descriptor reuse | Descriptor prep/upload is large enough to justify a bounded descriptor-reuse prototype. | `xlarge/alma/alma/synth`, `full17/trade22`, `none/halves/quartiles` | Retained replay bucket after batching still around `30-32 ms` on replay-enabled surfaces | Descriptor prep and descriptor upload both rounded to `0 ms`; did not clear numeric promotion rules. | replay micro-attribution reruns + dispatch logs | Rejected |

## Notes

1. The retained branches produced real measured wins or stable attribution improvements.
2. The rejected branch is still useful because it narrows the remaining replay problem.
3. The unresolved technical target is now the unsplit `replay_other` bucket, not descriptor reuse first.
