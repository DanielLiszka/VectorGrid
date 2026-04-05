# CUPTI SASS-Metric Follow-up Implementation Plan

Date: 2026-04-05

## Completion Status

Status: feasibility, large-surface capture, and first interpreted region summary complete

Final classification:
- `useful but only for research/tooling`

Durable repo result:
- [cupti_sass_metric_followup_findings_2026-04-05.md](C:/Rust%20Projects/ta_desktop_demo/CUDA%20kernel%20review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md)

Primary generated artifacts:
- [inventory JSON](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/cupti_metric_inventory.json)
- [large/full17 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_full17_none_region_metrics.json)
- [large/trade22 capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_trade22_none_region_metrics.json)
- [large lockstep region summary](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/large_lockstep_region_summary_2026-04-05.md)
- [sample inventory](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/sample/sample_inventory.txt)
- [sample capture](C:/Rust%20Projects/ta_desktop_demo/target/cupti_sass_probe/sample/sample_capture.txt)

## Purpose

This document turns the optional CUPTI branch from the Phase 2 `double_crossover` work into a standalone implementation plan.

This is a **measurement-first tooling branch**, not a product optimization branch and not a commitment to post-compilation editing.

## Why This Branch Still Exists

The retained repo evidence already established:

1. static SASS was useful for selecting early source/runtime fixes
2. runtime profiling was necessary to identify replay and dataflow costs
3. one bounded lockstep-body change and one bounded replay-batching change were retained
4. at least one plausible next replay branch was measured and rejected

The remaining reason to pursue CUPTI is narrower:

1. Nsight Systems and Nsight Compute still summarize kernels well at the kernel level
2. they do **not** currently attribute cost to instruction regions inside the same control-heavy kernel
3. CUPTI SASS metrics are the cleanest remaining route if the repo wants one more serious attribution layer without reopening product code changes first

This branch should therefore answer:

1. whether live cost inside the retained hot kernels is dominated by the same static regions highlighted in the SASS review
2. whether that extra attribution would change any concrete product or follow-on tooling decision

## Current Repo Baseline

Treat the following as the current authoritative baseline:

1. the retained exact `double_ma` route is still the row-first `GpuKernel` path
2. the retained lockstep-body and replay-batching changes are already in place
3. the cache-fit probe was measured and rejected as a product-facing branch
4. the bounded `vwma/vwma` TM-native pilot was retained, but did not justify broader TM-native expansion
5. there is now a branch-local CUPTI helper/script, and the feasibility barrier is cleared for the large exact route

Primary baseline docs:

1. [double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md)
2. [double_crossover_phase2_runtime_profiling_findings_2026-04-03.md](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md)
3. [double_crossover_phase2_research_findings_2026-04-03.md](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/double_crossover_phase2_research_findings_2026-04-03.md)
4. [double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md)

## Scope

This plan covers only:

1. the exact `double_crossover` GPU route used by `double_ma`
2. static-region classification for already-profiled hot kernels
3. a minimal CUPTI SASS-metrics measurement helper
4. one bounded attribution note comparing static SASS expectations against runtime instruction-region evidence

This plan does **not** authorize:

1. kernel code changes
2. `double_crossover.cu` rewrites
3. post-compilation cubin editing
4. replay/dataflow architecture changes
5. frontend or Tauri product-surface changes
6. changing benchmark stories without new benchmark evidence

## Working Hypothesis

The working hypothesis is:

1. one or more retained hot kernels still contain distinct static instruction regions whose live contribution cannot be separated well enough with kernel-level Nsight summaries alone
2. CUPTI SASS metrics can separate at least some of these regions well enough to tighten the repo’s causal story

This branch is **not** trying to prove:

1. that CUPTI is required for product optimization
2. that the repo needs post-compilation editing
3. that the remaining headroom is necessarily low-level

## Primary Kernels And Surfaces

Start with exactly these kernels and surfaces.

### Large exact truth surfaces

1. `double_ma`, `alma/alma`, `large`, `metric_contract=full17`
2. `double_ma`, `alma/alma`, `large`, `metric_contract=trade22`

Primary target symbols:

1. `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
2. `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`

### Optional shortlist replay surfaces

Only use these if Phase 3 shows that large exact attribution is still insufficient.

1. `double_ma`, `alma/alma`, `xlarge`, `metric_contract=full17`, `shortlist_slice_mode=halves`
2. `double_ma`, `alma/alma`, `xlarge`, `metric_contract=trade22`, `shortlist_slice_mode=halves`

Primary optional symbols:

1. `double_cross_backtest_tm_flex_f32_windowed_m17*`
2. `double_cross_backtest_tm_flex_f32_windowed_m22*`

## Static Region Classes To Track

Use the same region taxonomy across all notes and tooling.

### Lockstep / row-first body

1. fast/slow MA load path
2. address generation and index arithmetic
3. regime / epsilon / signal-decision logic
4. trade-state transition logic
5. return accumulation and metric-stat updates
6. exact-row writeback / metric row store path

### Optional replay path

1. replay MA load path
2. range-window setup
3. signal / trade-state update logic
4. replay-row or metric writeback

The classification source should be:

1. [double_crossover_sm89_cuobjdump_2026-04-02.sass.txt](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/double_crossover_sm89_cuobjdump_2026-04-02.sass.txt)
2. [double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_2026-04-03.sass.txt](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_2026-04-03.sass.txt)
3. [double_crossover_phase2_hot_kernels_sass_excerpt_2026-04-03.txt](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/Phase%202%20Lite/sass/double_crossover_phase2_hot_kernels_sass_excerpt_2026-04-03.txt)

## Local Tooling Readiness

Current local readiness is good enough to start a tooling spike:

1. `cuobjdump`, `nvdisasm`, `ptxas`, and `ncu` are already present
2. CUPTI headers are present locally, including:
   - `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.0\extras\CUPTI\include\cupti_sass_metrics.h`
3. Nsight Systems is installed locally

Current repo/tooling gaps:

1. there is no existing repo helper for CUPTI SASS metrics
2. there is no current script under `scripts/` for this branch
3. there is no existing artifact schema for region-level CUPTI output

## Planned Touch Surfaces

### New branch-local helper surfaces

Preferred first-wave additions:

1. `crates/app_core/src/bin/cupti_sass_probe.rs`
2. `scripts/run_cupti_sass_probe.ps1`
3. `target/cupti_sass_probe/`

### Existing surfaces to reuse, not redesign

1. [profile_gpu_dynamic_runner.rs](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/profile_gpu_dynamic_runner.rs)
2. [double_crossover.cu](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/double_crossover.cu)
3. [double_crossover_phase2_runtime_profiling_findings_2026-04-03.md](C:/Rust%20Projects/ta_desktop_demo/SASS%20research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md)

First-wave rule:

1. do not add CUPTI wiring to the desktop app
2. do not add CUPTI wiring to the main product runtime
3. keep the branch isolated to a dedicated helper path until it proves value

## Artifact Layout

Suggested artifact root:

1. `target/cupti_sass_probe/`

Suggested saved outputs:

1. `static_region_map_<kernel>.md`
2. `cupti_metric_inventory.txt`
3. `large_full17_lockstep_region_metrics.json`
4. `large_trade22_lockstep_region_metrics.json`
5. optional `xlarge_full17_windowed_region_metrics.json`
6. optional `xlarge_trade22_windowed_region_metrics.json`
7. `cupti_sass_followup_findings_2026-04-05.md`

Suggested branch-local doc companions:

1. `CUDA kernel review/CUPTI/cupti_sass_region_map_template_2026-04-05.md`
2. `CUDA kernel review/CUPTI/cupti_sass_metric_inventory_notes_2026-04-05.md`

## Helper Contract

The first-wave helper should stay minimal.

Required behavior:

1. launch exactly one already-supported runner surface per invocation
2. accept enough CLI input to choose:
   - workload
   - metric contract
   - optional shortlist-slice mode
   - target kernel name filter
3. emit one machine-readable output file per invocation
4. log the exact kernel/module/symbol names it attempted to instrument
5. fail loudly if CUPTI setup succeeds but the requested SASS-metric surface is unsupported

Preferred first-wave output schema:

1. `run_id`
2. `gpu_name`
3. `driver_version`
4. `cuda_version`
5. `cupti_header_path`
6. `target_surface`
7. `target_kernel_filter`
8. `metric_inventory_used`
9. `capture_status`
10. `region_metrics`
11. `notes`

Non-goals for the helper:

1. general desktop-app integration
2. persistent daemon behavior
3. multi-surface orchestration in one run
4. automatic metric interpretation

## Phase 0: Preflight

- [x] Confirm no benchmark, profiler, or Nsight process is still running.
- [ ] Record branch and commit.
- [x] Record GPU, driver, CUDA version, and CUPTI header path used.
- [x] Record whether the PowerShell session is elevated.
- [x] Keep the retained code state fixed while the tooling spike runs.

## Phase 1: Static Region Map

Goal: define the exact region taxonomy before any runtime CUPTI capture.

- [x] Choose the first large-surface lockstep symbol for `full17`.
- [x] Choose the first large-surface lockstep symbol for `trade22`.
- [x] Use the current SASS dump or symbol-scoped excerpt to mark instruction regions for each target kernel.
- [x] Produce one short static region map per target kernel.
- [x] Keep the region classes stable so later captures can be compared apples-to-apples.

Minimum output:

1. one markdown note listing region name, intent, and approximate SASS span for each target kernel

Suggested static-region map columns:

1. `region_id`
2. `kernel_symbol`
3. `region_name`
4. `approx_sass_start`
5. `approx_sass_end`
6. `intent`
7. `primary data touched`
8. `why it may matter`

Classification rule:

1. keep regions coarse enough that they can be recognized repeatably after rebuilds
2. do not overfit to single instruction addresses if a semantic block is clearer
3. prefer semantically stable blocks over maximal granularity

## Phase 2: Minimal Tooling Spike

Goal: prove the workstation and helper path can query CUPTI SASS metrics at all.

- [x] Build a minimal standalone helper rather than wiring CUPTI into the main runtime.
- [x] Start from a single process that launches one already-supported runner surface.
- [x] Add just enough CUPTI setup to enumerate or query the available SASS-metric capabilities on this workstation.
- [x] Save the inventory to `target/cupti_sass_probe/cupti_metric_inventory.json`.
- [x] Record any architecture or permission limitations immediately.

Inventory output should capture:

1. supported metric names
2. unsupported or unavailable requested metrics
3. whether source correlation is available
4. whether per-instance or per-region collection is available
5. any required replay, patching, or privilege caveats reported by the runtime

Phase-2 success threshold:

1. the helper can enumerate the usable SASS-metric surface without crashing
2. at least one target metric family relevant to instruction attribution is available

Phase-2 failure threshold:

1. only inventory-level metadata is available but no collectable region metrics exist
2. collection fails repeatably on the current GPU/driver/toolkit combination
3. the only way forward would require invasive runtime integration before a first capture

Stop rule for Phase 2:

- [x] Stop rule evaluated: the inventory surface was available, so the branch continued to first capture.

## Phase 3: First Large-Surface Capture

Goal: collect one useful region-level runtime attribution result on the large exact route.

- [x] Reuse the existing large `full17` runner surface.
- [x] Reuse the existing large `trade22` runner surface.
- [x] Capture only one kernel family at a time.
- [x] Save machine-readable output under `target/cupti_sass_probe/`.
- [x] Correlate the CUPTI output with the static region map created in Phase 1.

Required interpretation questions:

1. does the runtime instruction evidence agree with the static suspicion about address generation, predication, or writeback?
2. does it contradict the static suspicion enough to change what the repo would optimize next?
3. does it reveal a region split that Nsight Compute could not already make clear?

Minimum branch-local comparison rule:

1. compare CUPTI output against the already-saved Nsight evidence for the same large surface
2. do not claim CUPTI value unless it changes the interpretation relative to existing Nsight summaries

Preferred first-pass artifact pair:

1. one `full17` lockstep capture
2. one `trade22` lockstep capture

Phase 3 completion note:

- The first `large/full17` repo-local capture attempt failed with `CUPTI_ERROR_INVALID_PARAMETER`, but the root cause was later traced to a helper-side `CUpti_SassMetricsFlushData_Params.struct_size` mismatch. After correcting that ABI bug, both `large/full17` and `large/trade22` repo-local captures succeeded.

## Phase 4: Optional Replay Capture

Only start this phase if the large exact capture remains ambiguous or if replay-specific ambiguity still matters.

- [ ] Reuse shortlist-heavy `xlarge/full17/halves` only if needed.
- [ ] Reuse shortlist-heavy `xlarge/trade22/halves` only if needed.
- [ ] Target only the retained replay kernel family, not the whole route.
- [ ] Save a separate replay note rather than mixing replay and lockstep conclusions in one artifact.

Phase 4 completion note:

- Not started. Large exact capture is now working, so replay follow-up remains optional rather than blocked.

## Phase 5: Findings Note

Goal: close the branch with a repo-owned note that answers whether CUPTI materially helped.

- [x] Write `target/cupti_sass_probe/cupti_sass_followup_findings_2026-04-05.md`.
- [x] State clearly what CUPTI added beyond existing Nsight evidence.
- [x] State clearly what remained ambiguous even after CUPTI.
- [x] State whether the branch changes any product-facing optimization priority.

Required final classifications:

1. `useful and changes the next decision`
2. `useful but only for research/tooling`
3. `not useful enough to justify maintaining the tooling`

Phase 5 completion note:

- the branch classifies as `useful but only for research/tooling`
- the retained large exact interpretation now says:
  - `large/trade22` remains dominated by the expected `m22_r3` main-loop region
  - `large/full17` is dominated by a later `m17` control-heavy numeric/call cluster rather than by entry/setup or writeback
- that is enough to justify keeping the branch as a tooling/attribution artifact, but not enough to promote a new product-facing optimization branch

Required findings-note sections:

1. `What Was Measured`
2. `What CUPTI Added Beyond Nsight`
3. `What Remains Ambiguous`
4. `Would This Change The Next Product Decision`
5. `Maintenance Cost And Tooling Risk`

## Verification Checklist

- [x] `cargo check -p ta_app_core`
- [x] keep all benchmark and profiling runs serialized
- [x] save exact commands used
- [x] save all generated artifacts under `target/cupti_sass_probe/`
- [x] confirm the helper branch does not change product runtime behavior

## Success Criteria

Treat this branch as successful if **any one** of the following happens:

1. the repo gets a repeatable CUPTI SASS-metrics helper plus at least one useful region-level finding on the retained large exact route
2. the repo gets a defensible negative result showing that CUPTI does not materially improve the next product decision beyond Nsight
3. the repo gets a clear feasibility note showing the practical blocker is tooling or architecture support, not lack of branch discipline

Practical promotion rule:

1. do not keep the helper as a maintained repo tool unless it produces at least one attribution result that is materially more specific than the current Nsight-based story

## Rejection Criteria

Reject the branch without promoting it further if any of the following becomes true:

1. CUPTI setup is too fragile to use repeatably on the current workstation
2. the helper requires invasive product-runtime integration just to get the first capture
3. the collected CUPTI data does not provide more actionable separation than Nsight Compute already does
4. the branch starts drifting toward post-compilation or kernel edits without a finished attribution note

## Decision Rule

Treat this plan as complete when it produces one of these outcomes:

1. a retained branch-local CUPTI helper plus one useful attribution memo, or
2. a clean no-go memo explaining why CUPTI is not worth pursuing further in this repo

Anything weaker should be treated as an exploratory dead end, not as the start of a larger rewrite.
