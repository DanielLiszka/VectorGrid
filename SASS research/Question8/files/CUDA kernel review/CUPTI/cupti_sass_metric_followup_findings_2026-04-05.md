# CUPTI SASS-Metric Follow-up Findings

Date: 2026-04-05

## Outcome

This branch is viable and interpreted after correcting a helper-side ABI bug.

Final classification:
- `useful but only for research/tooling`

The repo now has:
- a positive local CUPTI SASS-metric inventory result on the active `RTX 4090` / `AD102` surface
- a positive control result from NVIDIA's bundled `sass_metrics` sample
- successful repo-local captures on both retained large exact lockstep surfaces:
  - [large/full17 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_full17_none_region_metrics.json)
  - [large/trade22 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_trade22_none_region_metrics.json)
- an interpreted region summary for those surfaces:
  - [large lockstep region summary](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_lockstep_region_summary_2026-04-05.md)

## Root Cause Of The Earlier Failure

The earlier repo-local failure was caused by a helper-side `CUpti_SassMetricsFlushData_Params` ABI mismatch.

Direct evidence:
- the local CUPTI header defines `CUpti_SassMetricsFlushData_Params_STRUCT_SIZE` using the `numOfInstances` field
- the repo helper originally computed `struct_size` through `p_metrics_data`
- on this 64-bit build that meant the helper passed `48` instead of the expected `40`
- the failing call was `cuptiSassMetricsFlushData(...): CUPTI_ERROR_INVALID_PARAMETER`

The fixed helper now sets:
- `struct_size = offset_of!(CUpti_SassMetricsFlushData_Params, num_of_instances) + size_of::<usize>()`

After that correction, the repo-local capture path completed successfully.

## What Was Implemented

Branch-local tooling:
- [cupti_sass_probe.rs](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/cupti_sass_probe.rs)
- [run_cupti_sass_probe.ps1](C:/Rust%20Projects/ta_desktop_demo/scripts/run_cupti_sass_probe.ps1)
- [prepare_cuda_kernel_runtime_for_probe](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs#L1907)

Branch-local durable docs:
- [implementation plan](C:/Rust%20Projects/ta_desktop_demo/CUDA%20kernel%20review/CUPTI/cupti_sass_metric_followup_implementation_plan_2026-04-05.md)
- [inventory notes](C:/Rust%20Projects/ta_desktop_demo/CUDA%20kernel%20review/CUPTI/cupti_sass_metric_inventory_notes_2026-04-05.md)

Generated artifacts:
- [inventory JSON](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/cupti_metric_inventory.json)
- [large/full17 region map](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/static_region_map_large_full17_lockstep_m17.md)
- [large/trade22 region map](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/static_region_map_large_trade22_lockstep_m22.md)
- [large/full17 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_full17_none_region_metrics.json)
- [large/trade22 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_trade22_none_region_metrics.json)
- [sample inventory](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/sample/sample_inventory.txt)
- [sample capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/sample/sample_capture.txt)

## What Was Proven

Positive results:
- the workstation/toolkit/driver combination exposes CUPTI SASS metrics for `AD102`
- the local inventory surface is stable enough to enumerate supported metrics
- the local toolkit can complete a control capture through NVIDIA's own sample
- the repo-local helper can now complete capture on the retained exact route for both `full17` and `trade22`

Observed large-surface capture facts:
- `full17` capture status: `ok`
- `full17` matched instruction records: `1986`
- `trade22` capture status: `ok`
- `trade22` matched instruction records: `1168`

Observed large-surface interpretation facts:
- `large/full17` dynamic mass collapses into the later `m17` control-heavy numeric/call cluster (`m17_r6`, `0x3770-0x61a0`), especially the hot `0x3c70-0x4550` span with repeated `CALL.REL.NOINC 0x61b0`
- `large/trade22` dynamic mass remains in the expected `m22_r3` main-loop region (`0x0640-0x34f0`)
- neither large exact surface points to entry/setup or writeback as the dominant explanation

## What Still Remains To Do

This branch has crossed the feasibility barrier and now has a first interpreted large-surface summary.

Remaining work only if the branch is deliberately reopened:
- split the hot `m17_r6` cluster into finer subregions
- extend the same approach to replay kernels if a future question really needs it
- otherwise keep the current branch artifacts as the stopping point

## Product Impact

No product-facing optimization priority changed.

The branch is now useful because it provides real repo-local instruction-level data and a tighter intra-symbol story for `full17` / `trade22`, not because it identified a new retained optimization.
