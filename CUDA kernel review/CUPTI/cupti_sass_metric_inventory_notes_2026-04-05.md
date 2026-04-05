# CUPTI SASS Metric Inventory Notes

Date: 2026-04-05

## Outcome

The local workstation supports CUPTI SASS metrics on the active RTX 4090 surface, and the repo-local helper now captures successfully after a helper-side ABI fix.

Key facts:
- `CUDA_PATH` resolves to `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2`
- inventory succeeded through the branch-local helper
- `cuptiProfilerDeviceSupported(api=CUPTI_PROFILER_SASS_METRICS)` reported supported
- the inventory found `48` SASS metrics on chip `AD102`

Primary artifact:
- [cupti_metric_inventory.json](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/cupti_metric_inventory.json)

Representative metrics present:
- `smsp__sass_inst_executed`
- `smsp__sass_inst_executed_op_branch`
- `smsp__sass_inst_executed_op_global`
- `smsp__sass_inst_executed_op_global_ld`
- `smsp__sass_inst_executed_op_global_st`
- `smsp__sass_inst_executed_op_shared`
- `smsp__sass_inst_executed_op_memory_32b`
- `smsp__sass_branch_targets_threads_uniform`
- `smsp__sass_branch_targets_threads_divergent`

## Control Check

The installed NVIDIA `sass_metrics` sample also succeeds locally on the same workstation.

Control artifacts:
- [sample inventory](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/sample/sample_inventory.txt)
- [sample capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/sample/sample_capture.txt)

That matters because it separates:
- local toolkit / driver support, which is present
- from repo-local helper correctness, which also now works after the `FlushData` struct-size fix

## Resolved Failure Cause

The earlier repo-local `CUPTI_ERROR_INVALID_PARAMETER` result was caused by the helper computing `CUpti_SassMetricsFlushData_Params.struct_size` against `p_metrics_data` instead of `num_of_instances`.

After correcting that field-size calculation, the repo-local captures succeeded on:
- [large/full17 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_full17_none_region_metrics.json)
- [large/trade22 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_trade22_none_region_metrics.json)

## Current Branch State

This is no longer an inventory-only feasibility note.

The branch is now capture-capable and should be treated as:
- feasible for static-plus-dynamic attribution work
- still not a product optimization result by itself
