# Options Expression CUDA Profiling Playbook

Date: 2026-04-13  
Status: active playbook for profiling the current options-expression CUDA path and its adjacent CPU bottlenecks on Windows

## Purpose

This document explains how to profile the current options backtest optimization lane in a way that is consistent with the active iteration guide:

- [options_expression_performance_iteration_implementation_guide_2026-04-13.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_performance_iteration_implementation_guide_2026-04-13.md)

Use this when you need to answer one of these concrete questions:

- where does end-to-end options preview/run time go now?
- is host replay/build or prepare still dominating?
- is the CUDA scorer kernel actually inefficient?
- are Windows scheduling or host CPU hot paths masking CUDA gains?
- are CUDA-side changes still correct after widening work on device?

## Scope guard

This playbook is about:

- profiling the current options-expression workflow
- profiling the existing CUDA scorer path
- profiling adjacent CPU work that affects the practical value of the CUDA route

This playbook is not for:

- generic GPU tuning detached from repo evidence
- broad architecture changes
- product-scope expansion
- SASS/PTX work without profiler evidence

---

## 1. Recommended profiling order

Use this order unless the current evidence clearly says otherwise.

1. **Phase05 / feasibility evidence**
   - establish the current timing boundary and current CPU/CUDA gap

2. **Nsight Systems**
   - use first for timeline truth

3. **Nsight Compute**
   - use only after Systems identifies a kernel/range worth deeper inspection

4. **Compute Sanitizer**
   - use before keeping aggressive CUDA-side changes

5. **WPA / ETW CPU sampling**
   - use when host-side work is still large or the CPU/CUDA gap is narrow

6. **GPUView**
   - use when Windows queueing / WDDM scheduling is suspected

7. **CUPTI**
   - use only when repeated scripted profiling is valuable enough to justify custom automation

---

## 2. Current options NVTX ranges

The options lane now exposes stable NVTX ranges intended for bounded capture.

### Current cold-start interpretation on `HEAD`

On the current dense SPY Phase05 workload, the most useful current reading is:

- without prewarm, cold preview is about `113ms`
- with the landed process-wide runtime reuse plus prewarm path, the same workload drops to about `16ms`
- `prepare_us` is still only about `1.8ms`
- `cpu_ranking_us` is still only about `1.7ms`
- `gpu_ranking_us` is still only about `1.7ms`
- the old dominant cold bucket was the first CUDA device/runtime bring-up:
  - `gpu_device_resolution_us` was about `11.9ms`
  - `gpu_plan_probe_us` was about `85.0ms`
- the prewarmed path hides that bucket almost completely:
  - `gpu_device_resolution_us` is `0`
  - `gpu_plan_probe_us` is about `45us`

Interpret this carefully:

- the options scorer math is not the first cold bottleneck on the canonical dense workload
- the main architectural cold-start win has already been taken
- a pure kernel-micro-tuning pass is unlikely to move end-to-end preview time materially by itself on this workload
- if you reopen CUDA work, first decide whether you are measuring:
  - raw first-run bring-up with prewarm intentionally disabled
  - startup-hidden / steady-state scorer throughput with prewarm enabled
  - or a decomposition change that increases useful work per launch on a larger workload

Reference artifacts:

- `output/options_expression_phase05_dense_device_timing/2026-04-13T16-00-00/phase05_summary.json`
- `output/options_expression_phase05_dense_device_resolver_cubin/2026-04-13T17-05-00/phase05_summary.json`
- `output/options_expression_phase05_dense_prewarm_runtime/2026-04-13T16-24-40/phase05_summary.json`

### Top-level capture ranges

- `options_expression_feasibility_total`
- `options_expression_run_total`
- `tauri_options_expression_feasibility_request`
- `tauri_options_expression_request`

### High-value nested ranges

- replay/build:
  - `options_replay_build_total`
  - `options_parent_artifact_validation`
  - `options_source_resolution`
  - `options_parent_signal_process_cache_lookup`
  - `options_persisted_signal_manifest_cache_lookup`
  - `options_candle_replay_load`
  - `options_parent_replay_and_drilldown`
  - `options_signal_manifest_materialization`
  - `options_dataset_manifest_validation`
  - `options_dataset_process_cache_lookup`
  - `options_dataset_persisted_cache_lookup`
  - `options_dataset_csv_load`
  - `options_request_normalization`

- prepare:
  - `options_prepare`
  - `options_tuple_compile`
  - `options_prepare_baseline_candidates`
  - `options_prepare_baseline_exit_summaries`
  - `options_prepare_conservative_candidates`
  - `options_prepare_conservative_exit_summaries`
  - `options_prepare_harsher_candidates`
  - `options_prepare_harsher_exit_summaries`

- ranking and finalist work:
  - `options_cpu_rank_total`
  - `options_gpu_rank_total`
  - `options_gpu_tensor_score`
  - `options_gpu_host_shortlist`
  - `options_gpu_host_materialize_topk`
  - `options_gpu_host_materialize_full`
  - `options_exactify_total`
  - `options_exactify_summary_overwrite`
  - `options_exactify_enrichment`
  - `options_exactify_extreme`
  - `options_sealed_stage_total`
  - `options_sealed_stage_non_extreme`
  - `options_sealed_stage_extreme`

- feasibility probes:
  - `options_feasibility_cpu_rank`
  - `options_feasibility_gpu_plan_probe`
  - `options_feasibility_gpu_rank_probe`

These names are intended to stay stable enough for repeated Nsight capture and findings-log references.

---

## 3. Fastest current profiling entrypoints

### 3.1 Baseline summary

Use this before any deep profiler session:

```powershell
powershell -File scripts/run_options_expression_phase05.ps1 -Cuda -CompareBackends
```

To measure the landed startup-hidden path instead of the raw first-run CUDA bring-up:

```powershell
powershell -File scripts/run_options_expression_phase05.ps1 -Cuda -CompareBackends -PrewarmGpuRuntime
```

This answers:

- current replay/build timing
- current prepare timing
- current CPU vs accelerated comparison
- candidate density
- invalid-slot rate
- current backend recommendation

### 3.2 Nsight Systems

Use this first for timeline truth:

```powershell
powershell -File scripts/profile_options_expression_nsys.ps1
```

Common variants:

```powershell
powershell -File scripts/profile_options_expression_nsys.ps1 -BackendPreference gpu_compiled_tensor

powershell -File scripts/profile_options_expression_nsys.ps1 -RunFull
```

This emits:

- `.nsys-rep` or `.qdrep`
- a summary JSON and markdown file from the same headless options run
- a provenance sidecar JSON

Current repo-grounded notes:

- the existing `sm_89` host now has an options-expression cubin fast path, so if Systems still shows a large cold gap on the first feasibility run, treat it as broader CUDA runtime/module bring-up rather than assuming all of it is PTX JIT
- the headless feasibility binary now supports `--prewarm-gpu-runtime`, and the Phase05 wrapper exposes that as `-PrewarmGpuRuntime`
- use prewarm when you want to profile the landed desktop-like path after startup-hidden runtime warm-up; omit it when you explicitly want the raw first-run bring-up cost

All current profiling wrappers accept `-RequestArgs` and forward those flags directly to
`ta_options_expression_feasibility`. Use that passthrough when the default Phase05 grid is too
small to represent the workload you actually want to profile.

Important invocation note:

- when `-RequestArgs` contains values that begin with `--`, prefer invoking the script directly
  from PowerShell with `&` and parameter splatting instead of `powershell -File ... -RequestArgs '--foo' ...`
- example:

```powershell
$params = @{
  RequestArgs = $denseArgs
}

& 'C:\Rust Projects\ta_desktop_demo\scripts\profile_options_expression_nsys.ps1' @params
```

Windows direct-launch note:

- the repo-local profiler wrappers now stage Cargo-style runtime DLL search paths before launching
  the release feasibility binary directly
- keep that behavior intact: `cargo run` adds those paths automatically, but direct Nsight /
  Compute Sanitizer launches do not
- when a run is pinned to a specific options dataset, prefer passing `-ArtifactPath` explicitly if
  the newest saved artifact on disk may belong to a different root symbol

Dense GPU-probe example:

```powershell
$denseArgs = @(
  '--delay-minutes', '0,1,2,3',
  '--dte-buckets', '1',
  '--moneyness-buckets', 'atm',
  '--hold-minutes', ((1..32) -join ','),
  '--target-return-buckets', '0.1,0.2,0.3,0.4',
  '--stop-return-buckets', '0.1,0.2,0.3,0.4'
)

& 'C:\Rust Projects\ta_desktop_demo\scripts\profile_options_expression_nsys.ps1' `
  -RequestArgs $denseArgs
```

### 3.3 Nsight Compute

Use this only after Systems shows that the scorer kernel is worth deeper inspection:

```powershell
powershell -File scripts/profile_options_expression_ncu.ps1
```

Default kernel target:

- `options_expression_score_split_f64`

Default target-process mode:

- `application-only`
- this is the correct default for the current single-process options feasibility binary on this host
- `all` may fail to attach or fail to emit a report on this Windows setup

Common variant:

```powershell
& 'C:\Rust Projects\ta_desktop_demo\scripts\profile_options_expression_ncu.ps1' -LaunchCount 50
```

Dense GPU-probe variant:

```powershell
$denseArgs = @(
  '--delay-minutes', '0,1,2,3',
  '--dte-buckets', '1',
  '--moneyness-buckets', 'atm',
  '--hold-minutes', ((1..32) -join ','),
  '--target-return-buckets', '0.1,0.2,0.3,0.4',
  '--stop-return-buckets', '0.1,0.2,0.3,0.4'
)

& 'C:\Rust Projects\ta_desktop_demo\scripts\profile_options_expression_ncu.ps1' `
  -RequestArgs $denseArgs `
  -LaunchCount 50
```

Practical artifact note:

- if the latest saved run artifact is not for the same root symbol as the profiled options dataset,
  pass `-ArtifactPath` explicitly or the feasibility binary can exit before the profiler reaches the
  scorer kernel

### 3.4 Compute Sanitizer

Use this before keeping aggressive CUDA-side changes:

```powershell
powershell -File scripts/profile_options_expression_compute_sanitizer.ps1
```

Common variants:

```powershell
powershell -File scripts/profile_options_expression_compute_sanitizer.ps1 -Tool memcheck
powershell -File scripts/profile_options_expression_compute_sanitizer.ps1 -Tool racecheck
powershell -File scripts/profile_options_expression_compute_sanitizer.ps1 -Tool synccheck
```

Dense workload variant:

```powershell
$denseArgs = @(
  '--delay-minutes', '0,1,2,3',
  '--dte-buckets', '1',
  '--moneyness-buckets', 'atm',
  '--hold-minutes', ((1..32) -join ','),
  '--target-return-buckets', '0.1,0.2,0.3,0.4',
  '--stop-return-buckets', '0.1,0.2,0.3,0.4'
)

& 'C:\Rust Projects\ta_desktop_demo\scripts\profile_options_expression_compute_sanitizer.ps1' `
  -RequestArgs $denseArgs `
  -Tool memcheck
```

---

## 4. Current 2026-04-13 findings snapshot

These findings are the current local evidence baseline for further options CUDA work.

- cold replay/build:
  - persisted dataset replay-cache lookup on the dense Phase05 workload dropped from about `163287us`
    to about `47389us` after switching the persisted dataset cache to a binary-safe replay format
  - cold replay/build dropped from about `167ms` to about `50ms`
  - cold total preview dropped from about `314ms` to about `236ms`
  - after the accepted LongSingle prepare-path optimization, the current dense current-`HEAD`
    preview dropped further from about `211ms` to about `193ms` while warm preview held at about
    `11ms`
  - the prepare-side LongSingle exit-summary compile bucket dropped from about `30ms` total
    (`10ms + 10ms + 10ms`) to about `25ms` total (`8ms + 8ms + 9ms`)
  - after the accepted all-holds LongSingle prepare-path optimization, the same dense workload
    dropped again from about `193ms` to about `170ms` while warm preview still held at about `11ms`
  - the visible LongSingle exit-summary compile bucket is now about `9ms` total (`3ms + 3ms + 3ms`)

- Nsight Compute on `options_expression_score_split_f64`:
  - launch shape on the dense workload was `grid=32`, `block=64`
  - the profiler reported only about `0.01` waves per SM on a `128` SM device
  - theoretical occupancy was `100%`, but achieved occupancy was only about `4.2%`
  - scheduler data showed `No Eligible` about `97.18%` and `Eligible Warps Per Scheduler` about `0.03`
  - memory throughput was only about `33.51 GB/s` / `3.15%` of peak and compute throughput only about `0.28%` of peak
  - top stall guidance pointed to L1/TEX scoreboard dependency stalls dominating the issue gap

- Compute Sanitizer:
  - `memcheck` completed with `ERROR SUMMARY: 0 errors`

- repo-local wrapper status:
  - the repo-local Nsight Compute wrapper is now working again on this Windows host
  - root cause was a direct-launch environment gap, not a kernel/profiler bug: `cargo run` succeeded
    while direct executable launches were missing Cargo-style runtime DLL search paths
  - after staging those runtime DLL paths in the wrappers, the following smoke captures all emitted
    their expected artifacts:
    - `target/nsight/options_expression_ncu_script_smoke_fixed_spy_2026-04-13.ncu-rep`
    - `target/nsight/options_expression_nsys_script_smoke_fixed_spy_2026-04-13.nsys-rep`
    - `target/nsight/options_expression_compute_sanitizer_script_smoke_fixed_spy_2026-04-13.memcheck.log`

- practical implication:
  - the next meaningful CUDA step is not broad SASS/PTX tuning
  - it is a bounded change that increases useful work per launch or changes the launch/decomposition shape enough to raise eligible warps and launch occupancy
  - the current accepted CPU-side prepare win lowers end-to-end preview cost without weakening the
    CUDA route, so reopening profiler work still needs a decomposition or candidate-coverage
    hypothesis rather than generic scorer micro-tuning
  - because the visible prepare bucket is now much smaller, the next evidence-backed pass should
    first isolate the remaining uninstrumented feasibility/report-finalization share before assuming
    more prepare work is still first-order

---

## 5. How to interpret each tool

### 5.1 Nsight Systems

Use Systems to answer:

- Is the outer options request still dominated by replay/build?
- Is prepare still large enough to matter?
- Is the GPU scorer share actually first-order?
- Are there suspicious H2D/D2H copies, synchronizations, or launch gaps?

If Systems says host work still dominates, do not jump to Nsight Compute first.

### 5.2 Nsight Compute

Use Compute to answer:

- Is `options_expression_score_split_f64` occupancy-limited?
- Is the kernel memory-throughput limited?
- Is the kernel instruction mix obviously poor?
- Is there a strong reason to revisit launch shape or kernel internals?

If Compute shows only modest kernel issues while Systems still shows host-side dominance, stay out of SASS/PTX work.

### 5.3 Compute Sanitizer

Use it to protect profiler conclusions from correctness bugs.

Relevant tools:

- `memcheck`
- `racecheck`
- `initcheck`
- `synccheck`

This is especially important if:

- output buffers are widened
- more work is moved into CUDA
- new synchronization or device-state reuse is introduced

### 5.4 WPA / ETW CPU sampling

Use WPA when:

- replay/build is still large
- prepare or exactify is still CPU-heavy
- CPU/CUDA speedup remains narrow and the host side may be the real blocker

What to collect:

- CPU sampling with stack walking
- optional file I/O context if replay/build is suspected to be storage-heavy

What to look for:

- hot functions in `commands_options_expression.rs`
- hot functions in `options_expression.rs`
- heavy JSON/dataset parsing
- heavy cache-key/hash/materialization work

### 5.5 GPUView

Use GPUView when:

- you suspect WDDM queueing or Windows scheduling effects
- kernels are short and frequent
- the measured CUDA share looks distorted by queue or submit behavior

This is especially useful if Nsight Systems suggests queueing gaps but does not fully explain them.

---

## 6. Manual Windows host-side workflow

This part is intentionally manual because it is environment-specific.

### 6.1 WPA / ETW

Suggested approach:

1. Start an ETW trace with CPU sampling and stack walking enabled.
2. Run one bounded options workload only.
3. Stop the trace immediately after the run.
4. Open in WPA and isolate:
   - process CPU usage
   - call stacks
   - file I/O if relevant
   - GPU activity only if it helps correlate host gaps

### 6.2 GPUView

Suggested approach:

1. Generate a bounded GPUView trace.
2. Run one options workload only.
3. Inspect queue submission, packet timing, and CPU/GPU overlap.

Use this only after Systems indicates that Windows scheduling may matter.

---

## 7. When to reopen CUPTI

Do not start with CUPTI.

Reopen it only if:

- repeated scripted profiling is becoming routine,
- you need automated range-based or PC-sampling capture,
- and the Nsight GUI/manual loop is no longer sufficient

Until then, the current repo should prefer:

- summary JSON
- Nsight Systems
- Nsight Compute
- Compute Sanitizer
- WPA / GPUView

---

## 8. Findings discipline

Every profiler result should be translated back into the active findings log using this shape:

- workload:
- profiler / evidence source:
- timing boundary:
- key numbers:
- interpretation:
- implication for lane priority:
- next action:

Examples:

- workload: canonical SPY options feasibility preview
- profiler / evidence source: Nsight Systems
- timing boundary: end-to-end preview wall time
- key numbers: replay/build visibly dominates; `options_gpu_tensor_score` remains small
- interpretation: host-side replay/build is still first-order
- implication for lane priority: keep Lane A above Lane C and runtime work
- next action: inspect replay/build subranges and choose one cold-path reduction

---

## 9. Stop conditions

Stop profiling escalation when:

- phase05 evidence already answers the lane-priority question clearly
- Nsight Systems shows host work dominates and there is no kernel-first case
- Nsight Compute does not reveal a first-order kernel problem
- Windows host-side sampling identifies the real bottleneck before deeper CUDA work is justified

Stop before SASS/PTX work unless:

- Systems says the kernel/runtime layer is materially first-order
- Compute says the scorer kernel has a clear stable bottleneck
- and the expected end-to-end win is large enough to matter
