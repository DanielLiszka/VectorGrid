# Double-MA ALMA GPU Full Optimization Pass
## Date: 2026-03-21
## Target benchmarks

- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only`
- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`
- Secondary reference:
  `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`

## Purpose

This document is the execution plan for a complete optimization pass on the ALMA GPU benchmark path for the plain Double-MA optimizer.

The goal is to improve performance without removing useful work, reducing user-visible fidelity, or reverting product-oriented behavior that benefits real app usage.

This plan assumes the current code is doing more useful work than the older `~76.919 ms` historical benchmark path. The optimization problem is therefore:

- keep the current product contract,
- measure where the time now goes,
- optimize the current work rather than backing out the work.

## Current understanding

- The old `~76.919 ms` note is a historical reading for `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`.
- That old reading is not directly apples-to-apples with the current full product path.
- Current code adds more product-like work, including finalist full-metric recompute and broader compact/top-k plumbing.
- The split benchmark family is the cleanest current way to separate:
  - narrow search engine cost:
    `search_only`
  - full current product-contract cost:
    `end_to_end_product`
- Fresh repo-local runs on `2026-03-21` measured approximately:
  - initial `search_only`: `89.074 ms .. 89.277 ms`
  - initial `end_to_end_product`: `181.95 ms .. 182.40 ms`
  - `gpu_kernel_pnl_full_metrics/large`: `164.83 ms .. 167.64 ms`
- Current retained optimization state on `2026-03-21` measures approximately:
  - `search_only`: `85.599 ms .. 86.834 ms .. 90.651 ms`
  - `end_to_end_product`: `156.06 ms .. 158.28 ms`
  - plain `gpu_kernel`: `178.43 ms .. 183.16 ms .. 185.30 ms`
- A rerun of older code from commit `1fccb9d` on the same machine still measured about:
  - `search_only`: `90.593 ms .. 91.203 ms`
  - `end_to_end_product`: `183.77 ms .. 187.35 ms`
- That strongly suggests the recent gap versus the `~156 ms` note is not primarily a new code regression after `1fccb9d`.
- The repo notes already indicate that normalized GPU clocks materially affect the measured outcome.

## Guiding principles

- Do not remove useful user-facing metric work just to improve a benchmark.
- Do not turn `search_only` into the product target.
- Do not optimize only for one GPU unless the specialization is explicitly gated by compute capability.
- Use benchmark-first, profile-second, patch-third, verify-fourth workflow.
- Keep each experiment narrow enough that wins and regressions are attributable.
- Preserve correctness, parity, and route stability throughout.

## Non-goals

- Reverting finalist recompute purely to make `end_to_end_product` look faster.
- Reverting compact-output support, top-k support, or heatmap support if they are part of the intended product contract.
- Chasing old historical numbers without matching their exact timing boundary and machine state.
- Making a universal default launch policy based only on one RTX 4090.

## Benchmark semantics

These distinctions must stay explicit throughout the pass.

- `search_only`
  - objective: `Sharpe`
  - finalist recompute: disabled
  - `heatmap_bins=0`
  - purpose: isolate the search engine cost as closely as possible while still using the current search contract
- `end_to_end_product`
  - objective: `Sharpe`
  - finalist recompute: enabled
  - `heatmap_bins=64`
  - purpose: measure the current product-oriented optimizer contract
- `gpu_kernel_pnl_full_metrics`
  - objective: `Pnl`
  - full metric row contract
  - use only as a supporting reference, not as a replacement for the Sharpe benchmark pair

## Code surfaces in scope

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- Secondary supporting files when required:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/build.rs`

## Working hypotheses

These are the leading hypotheses to test, in order.

### H1. Environment variance is still masking code-level conclusions

- GPU clocks are likely not normalized during many ad hoc runs.
- The same code has already shown materially different timings under different clock conditions.
- Baseline stability must be established before code changes are evaluated.

### H2. Finalist recompute is the largest remaining product-path cost

- The current product path adds finalist full-metric recompute after the search stage.
- This is the most direct explanation for the gap between `search_only` and `end_to_end_product`.
- If the goal is sub-160 ms product timing without reducing work, this path is the highest-priority target.

### H3. Launch shape and codegen still leave performance on the table

- The current chooser only considers `128/256/512`.
- Historical notes already showed `256` can be materially worse.
- The correct winner may be hardware-specific and may lie outside the current candidate set.

### H4. Compact-output and top-k plumbing still has fixed overhead worth reducing

- Current code keeps compact valid-pair output on and adds packed top-k host copy.
- This is probably the right product architecture, but it adds branch paths and copy/pack work.
- There may still be waste in merge cadence, row packing, row decoding, or host/device transfer boundaries.

### H5. The current kernel body may be leaving occupancy or instruction-level wins on the table

- The backtest path now carries `ret1`, compact-output logic, and broader metric plumbing.
- Even if that work is correct and useful, the generated code may still be improvable.
- Nsight metrics and ptxas output should decide whether the right next move is block size, register pressure reduction, branch reduction, memory layout work, or shared-memory tiling.

## Success criteria

Primary targets:

- `search_only`:
  - recover meaningful speed versus the current `~89-91 ms` band
  - aspirational target: low-to-mid `70 ms` range, only if achieved without reducing current search semantics
- `end_to_end_product`:
  - recover meaningful speed versus the current `~182-185 ms` band
  - aspirational target: sub-`160 ms`, only if achieved without removing useful metric work

Secondary targets:

- reduce variance across repeated runs under normalized clocks
- preserve top-1 identity and top-k overlap
- preserve current route selection and avoid fallback
- preserve product semantics for `best`, `top_k`, and heatmap behavior

## Measurement discipline

Every experiment in this plan must record:

- exact commit hash
- benchmark id
- date and local time
- GPU model
- driver version
- whether GPU clocks were normalized or pinned
- relevant env vars
- Criterion range and median/mean
- whether the benchmark ran alone with no concurrent GPU work
- whether the run was done on the current tree or on a temporary comparison worktree

## Baseline protocol

No optimization work should begin until this baseline is complete.

### Environment stabilization checklist

- [ ] Reboot or otherwise clear stale GPU load if the machine state is noisy.
- [ ] Confirm no concurrent `cargo bench`, game, render, browser-WebGPU, or CUDA workloads are active.
- [ ] Record `nvidia-smi` GPU name, driver version, current clocks, P-state, power draw, and temperature.
- [ ] Reuse the same clock-normalization workflow for every comparison run.
- [ ] Record whether the clock workflow is:
  - pinned/locked clocks,
  - application clocks,
  - a repeatable warmup-only normalization routine.
- [ ] If no stable clock workflow exists, create and document one before profiling.
- [ ] Use the same terminal/session style and avoid running benchmarks in parallel.
- [ ] Keep the same `target` directory strategy across comparison runs unless the experiment specifically tests build/codegen differences.

### Baseline benchmark checklist

- [ ] Run the current `search_only` benchmark three times under the stabilized environment.
- [ ] Run the current `end_to_end_product` benchmark three times under the stabilized environment.
- [ ] Run the current plain benchmark `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs` as a reference.
- [ ] Run the current batched benchmark `double_ma/gpu_kernel/alma_alma/200k/large_batched_1gb/58300pairs` as a secondary reference.
- [ ] Record throughput in backtests/second for each.
- [ ] Reject further interpretation if repeated runs differ too much to establish a stable baseline.

### Baseline commands

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only"
```

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"
```

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs"
```

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/alma_alma/200k/large_batched_1gb/58300pairs"
```

## Comparison baseline against known reference points

These are sanity anchors, not acceptance criteria.

- Current split benchmark pair under normalized clocks
- Current split benchmark pair without normalized clocks
- Current tree vs old reference logic
- Current tree vs commit `1fccb9d`

### Reference comparison checklist

- [ ] Re-run `search_only` and `end_to_end_product` on current `HEAD` under normalized clocks.
- [ ] Re-run the same two benchmarks on a comparison worktree at `1fccb9d` under the same clock conditions.
- [ ] If possible, create a throwaway worktree using the old reference versions of:
  - `gpu_backtest_kernel.rs`
  - `double_crossover.cu`
- [ ] Re-run the same benchmark pair there under the same conditions.
- [ ] Use those results to separate:
  - benchmark-contract drift,
  - environment drift,
  - code-path drift.

## Nsight profiling plan

Nsight profiling is mandatory before structural kernel changes.

### Profiling targets

- `search_only`
  - this isolates the search engine and hottest kernel loop
- `end_to_end_product`
  - this identifies how much time sits outside the core search pass
- optionally:
  - plain `gpu_kernel` benchmark
  - `gpu_kernel_pnl_full_metrics` benchmark for cross-checking row-width and full-metric costs

### Profiling goals

- identify the exact dominant kernel(s)
- confirm which dispatch path is active
- confirm chosen block size
- inspect registers per thread
- inspect theoretical vs achieved occupancy
- inspect issue stall reasons
- inspect branch divergence
- inspect DRAM throughput and memory-latency behavior
- inspect whether the path is compute-bound, register-bound, or memory-bound

### Nsight checklist

- [ ] Capture a clean Nsight Compute report for `search_only`.
- [ ] Capture a clean Nsight Compute report for `end_to_end_product`.
- [ ] Record the exact kernel names hit for both.
- [ ] Confirm whether the hot kernel is:
  - `lockstep_time`
  - `lockstep_time_m7`
  - another specialization
- [ ] Record per-kernel registers/thread.
- [ ] Record block size and grid size.
- [ ] Record achieved occupancy.
- [ ] Record top stall reasons.
- [ ] Record memory throughput and cache behavior.
- [ ] Compare `search_only` vs `end_to_end_product` timeline shape.
- [ ] Quantify time spent specifically in finalist recompute.
- [ ] Save the `.ncu-rep` artifacts and note their path.

### Profiling commands

Use the repo scripts if they already fit the workload; otherwise run a direct benchmark command with the same filter while capturing Nsight data.

- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`

If direct manual capture is required, keep the command aligned with the benchmark filter being studied.

## Launch-shape experiment plan

This is the first optimization experiment to run after the baseline and first Nsight capture.

### Rationale

- Launch choice is one of the few levers that can materially improve the narrow search path without changing semantics.
- Current chooser candidates are conservative.
- The right value may differ by hardware.

### Experimental candidate sweep

Treat this as an experimental sweep only, not a default-policy change.

- [ ] `64`
- [ ] `96`
- [ ] `128`
- [ ] `160`
- [ ] `192`
- [ ] `224`
- [ ] `256`
- [ ] `320`
- [ ] `384`
- [ ] `512`

### Launch sweep checklist

- [ ] Sweep block sizes on `search_only` first.
- [ ] Sweep the same block sizes on `end_to_end_product`.
- [ ] Record per-size timings under normalized clocks.
- [ ] Re-run the best two candidates to reject flukes.
- [ ] Use Nsight on the top candidate and on the current default for comparison.
- [ ] Record whether the winner is stable only on `sm_89` or looks portable.
- [ ] If a non-current value wins clearly, consider a compute-capability gate instead of a universal default.

### Launch sweep command shape

```powershell
$env:VECTORBT_KERNEL_BLOCK_X="128"
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only"
Remove-Item Env:VECTORBT_KERNEL_BLOCK_X
```

Repeat for each candidate block size and for `end_to_end_product`.

## End-to-end product-path optimization plan

This is the highest-priority product-path investigation area.

### Focus area 1: finalist recompute

Questions to answer:

- How much of `end_to_end_product` time is spent before finalist recompute?
- How much time is spent inside finalist recompute itself?
- Are finalists being recomputed over ranges larger than necessary?
- Is recompute re-uploading, re-packing, or re-launching more work than needed?
- Can work from the search pass be reused safely?

### Finalist recompute checklist

- [ ] Measure product path with finalist recompute on and off under controlled conditions for diagnosis only.
- [ ] Instrument the finalist recompute helper with timing boundaries if current logging is insufficient.
- [ ] Record candidate count, finalist count, unique fast periods, and unique slow periods used during recompute.
- [ ] Verify whether recompute still allocates or uploads buffers that could be reused.
- [ ] Verify whether recompute launches full-grid kernels where a much smaller finalist-grid kernel would suffice.
- [ ] Verify whether the current finalist path is bandwidth-bound or launch-bound.
- [ ] Check whether packed top-k data already contains enough information to avoid part of the finalist rerun setup.
- [ ] Check whether precomputed `ret1`, MA matrices, or compact mappings are rebuilt unnecessarily.
- [ ] Prototype only one narrowing change at a time.
- [ ] Reject any change that weakens best/top-k metric fidelity.

### Focus area 2: heatmap and product-contract overhead

Questions to answer:

- Is heatmap update cost material in the current product path?
- Is any D2H or D2D work still happening more often than required?
- Is the packed top-k path optimal in cadence and row width?

### Product-contract overhead checklist

- [ ] Measure `end_to_end_product` with heatmap enabled as-is.
- [ ] Measure a diagnosis-only variant with heatmap disabled to bound its share of the cost.
- [ ] Measure packed top-k copy cost separately if possible.
- [ ] Check if the current rolling `2 * top_k` merge window is still optimal.
- [ ] Check if row packing is done more frequently than necessary.
- [ ] Check if host-side row decode or constraint handling is still on the critical path.

## Search-only optimization plan

This is the highest-priority narrow-path investigation area.

### Focus area 1: lockstep kernel body

Questions to answer:

- Is the hot path compute-bound or memory-bound?
- Is register pressure suppressing occupancy?
- Is branch logic from compact-output or optional metric plumbing materially hurting the narrow path?
- Are `ret1` reads better than the old `expm1f(lr_t)` path in practice on this exact kernel shape?

### Search kernel checklist

- [ ] Confirm the exact hot kernel specialization used on the target benchmark.
- [ ] Record registers/thread and occupancy for the current best block size.
- [ ] Compare the current kernel body to the old reference implementation with Nsight-backed evidence rather than source inspection alone.
- [ ] Check whether compact-output branches are taken uniformly or divergently.
- [ ] Check whether additional row-width logic changes codegen even when `search_only` uses reduced metrics.
- [ ] Check instruction mix and issue stall reasons around:
  - return computation,
  - objective score path,
  - branch-heavy crossover logic,
  - compact-output row addressing.

### Focus area 2: selection, gather, and packed-row plumbing

- [ ] Measure time spent in selection kernels vs backtest kernels.
- [ ] Measure time spent in gather and packed-row copy kernels.
- [ ] Check whether compact fast/slow row arrays introduce measurable overhead.
- [ ] Check whether selection kernels still do avoidable pair decode work.
- [ ] Check whether a lighter packed representation or a different merge cadence helps.

## Candidate code experiments

These are ordered from least risky to most structural.

### Tier 1: measurement-oriented and low-risk experiments

- [ ] Expand launch-size sweep beyond `128/256/512`.
- [ ] Re-run ptxas register reporting for the hot kernels at the best candidate launch size.
- [ ] Re-check `__launch_bounds__` settings after launch-size selection.
- [ ] Add or tighten profiling instrumentation around finalist recompute and top-k/pack steps.

### Tier 2: moderate-risk implementation experiments

- [ ] Narrow finalist recompute inputs further.
- [ ] Reuse more search-pass allocations or uploaded buffers in finalist recompute.
- [ ] Reduce redundant compact-row decode work in selection/gather kernels.
- [ ] Reduce pack/unpack overhead in packed-row top-k copy.
- [ ] Evaluate whether some compact-output support code can be made cheaper in the narrow search case without changing behavior.

### Tier 3: deeper structural experiments

- [ ] Re-evaluate warp-per-fast-row or related mapping strategies only if Nsight still shows the hot loop as the dominant remaining cost after Tier 1 and Tier 2.
- [ ] Re-evaluate shared-memory time tiling only if profiling still shows memory-stall dominance after launch/codegen tuning.
- [ ] Consider compute-capability-specific launch policy if a clear `sm_89` winner emerges and portability is preserved via gating.

## Verification plan

Every accepted optimization change must pass both performance and correctness checks.

### Correctness checklist

- [ ] Verify `best` parameter identity still matches the prior baseline where expected.
- [ ] Verify top-k overlap remains acceptable.
- [ ] Verify `search_only` vs `end_to_end_product` semantic boundaries remain unchanged.
- [ ] Verify no route fallback occurs.
- [ ] Run targeted CUDA tests for the affected path.
- [ ] Re-run representative strategy-profile GPU benchmarks if the change touches shared GPU plumbing.

### Suggested verification commands

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
```

```powershell
cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture
```

```powershell
cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel -- --nocapture
```

```powershell
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

If a change touches strategy or finalist recompute behavior more broadly, add the relevant profile-specific tests and validation reruns for those surfaces too.

## Iterative execution loop

Each optimization cycle should follow this exact sequence.

1. Record the current baseline under stabilized conditions.
2. Form one narrow hypothesis.
3. Capture enough profiling evidence to justify touching code.
4. Make one small change set.
5. Run targeted correctness tests.
6. Re-run the same benchmark(s) under the same clock conditions.
7. Compare against the baseline and the prior iteration.
8. Keep the change only if the gain is repeatable and the added complexity is justified.
9. Update this document with:
   - date
   - commit
   - hypothesis
   - commands
   - result
   - keep/revert decision

## Experiment log template

Use this template for every measured attempt.

```md
### Experiment N - <short title>

- Date:
- Commit:
- Hypothesis:
- Files changed:
- Env:
- GPU clocks normalized:
- Benchmark commands:
- Tests run:
- Nsight artifact:
- Result:
- Decision:
- Notes:
```

## 2026-03-21 measured progress

### Experiment 1 - Baseline and launch sweep

- Date: 2026-03-21
- Commit: working tree before first optimization patch
- Hypothesis: the current ALMA search and product paths still have launch-shape headroom, and the default chooser is not selecting the best block size for the `58,300`-pair workload.
- Files changed: none
- Env: `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel`
- GPU clocks normalized: no
- Benchmark commands:
  - `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only`
  - `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`
  - `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
  - repeated `search_only` and `end_to_end_product` with `VECTORBT_KERNEL_BLOCK_X` sweeps
- Tests run: targeted benchmark verification only
- Nsight artifact: `target/nsight/alma_search_only_baseline.ncu-rep`
- Result:
  - Baseline `search_only`: `89.010 .. 89.370 ms`
  - Baseline `end_to_end_product`: `158.68 .. 162.89 ms`
  - Baseline plain `gpu_kernel`: `163.32 .. 166.88 ms`
  - `search_only` block sweep:
    - `64`: `87.696 .. 87.913 ms`
    - `96`: `87.751 .. 88.047 ms`
    - `128`: `87.745 .. 87.850 ms`
    - `160`: `87.666 .. 88.097 ms`
    - `192`: `86.344 .. 86.423 ms`
    - `224`: `86.770 .. 86.849 ms`
    - `256`: `86.751 .. 87.710 ms`
    - `320`: `86.766 .. 86.803 ms`
    - `384`: `86.688 .. 86.780 ms`
    - `512`: `86.310 .. 86.370 ms`
  - `end_to_end_product` block sweep:
    - `128`: `156.99 .. 158.29 ms`
    - `192`: `156.59 .. 159.58 ms`
    - `320`: `156.62 .. 158.96 ms`
    - `384`: `156.00 .. 158.19 ms`
    - `512`: `156.12 .. 156.45 ms`
- Decision: keep a launch-shape optimization in scope; the data clearly shows the old `128/256/512` candidate set was leaving performance on the table for this workload.
- Notes:
  - Nsight Compute on `search_only` showed low achieved occupancy and a dominant L1TEX scoreboard stall pattern in `double_cross_backtest_tm_flex_f32_lockstep_time`.
  - The best block size was not the original default candidate winner.

### Experiment 2 - Widen backtest launch candidates

- Date: 2026-03-21
- Commit: working tree with launch chooser update
- Hypothesis: expanding the backtest-only launch candidate set will let the existing chooser land on a faster block shape without changing benchmark semantics.
- Files changed:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Env: default path, no `VECTORBT_KERNEL_BLOCK_X` override
- GPU clocks normalized: no
- Benchmark commands:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only"`
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"`
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs"`
- Tests run: targeted benchmark verification only
- Nsight artifact: none yet for the post-change default path
- Result:
  - Post-change `search_only`: `86.637 .. 87.116 ms`
  - Post-change `end_to_end_product`: `160.05 .. 165.79 ms`
  - Post-change plain `gpu_kernel`: `157.94 .. 161.84 ms`
- Decision: keep
- Notes:
  - This is a low-risk improvement because it only affects the backtest launch chooser, not benchmark semantics or result correctness.
  - The plain `gpu_kernel` benchmark id is now effectively on the `~160 ms` threshold again under the current session conditions.

### Experiment 3 - Pipeline cost isolation

- Date: 2026-03-21
- Commit: working tree with launch chooser update
- Hypothesis: the remaining `end_to_end_product` gap is dominated by finalist recompute rather than heatmap or top-k finalize overhead.
- Files changed: none
- Env:
  - `VECTORBT_KERNEL_PROFILE_PIPELINE=1`
  - `cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel`
- GPU clocks normalized: no
- Benchmark commands:
  - `search_only` equivalent: `--backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 0` with `VECTORBT_KERNEL_FINALIST_RECOMPUTE=0`
  - `end_to_end_product` equivalent: same command with `--heatmap-bins 64` and default finalist recompute
  - isolation run: same product command with `VECTORBT_KERNEL_FINALIST_RECOMPUTE=0`
- Tests run: single-run pipeline instrumentation only
- Nsight artifact: pipeline logs
  - `target/alma_search_only_pipeline_profile.txt`
  - `target/alma_end_to_end_pipeline_profile.txt`
  - `target/alma_end_to_end_no_recompute_pipeline_profile.txt`
- Result:
  - `search_only` pipeline:
    - total `98.889 ms`
    - `tile_score`: `93.784 ms`
    - `final_finalize`: `0.129 ms`
  - `end_to_end_product` pipeline:
    - total `177.634 ms`
    - `tile_score`: `94.616 ms`
    - `final_finalize`: `78.172 ms`
  - `end_to_end_product` with finalist recompute disabled:
    - total `97.875 ms`
    - `tile_score`: `92.338 ms`
    - `final_finalize`: `0.148 ms`
- Decision: focus the next optimization pass on finalist recompute rather than on heatmap or host-side top-k packaging.
- Notes:
  - The extra product cost is almost entirely finalist recompute.
  - Heatmap cost on this workload is negligible.
  - Search-path optimization beyond the launch fix now looks like a deeper kernel/codegen problem rather than host finalize overhead.

### Experiment 4 - Exact finalist-pair recompute attempt

- Date: 2026-03-21
- Commit: temporary working tree experiment, reverted
- Hypothesis: finalist recompute was wasting time by evaluating the dense unique-fast by unique-slow grid and then filtering back to finalists on the host; using the kernel pair-map path to launch only exact finalist pairs should reduce `final_finalize`.
- Files changed:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Env:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"`
  - `VECTORBT_KERNEL_PROFILE_PIPELINE=1`
  - `cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel -- --backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 64`
- GPU clocks normalized: no
- Tests run: targeted benchmark verification and single-run pipeline instrumentation
- Nsight artifact: none
- Result:
  - Criterion `end_to_end_product`: `161.63 .. 166.00 .. 168.19 ms`
  - Single-run pipeline after the patch:
    - total `182.723 ms`
    - `tile_score`: `92.604 ms`
    - `final_finalize`: `85.255 ms`
- Decision: revert
- Notes:
  - The pair-map/compact-output path for this workload did not offset its own overhead.
  - The intuitive “fewer pairs must be faster” assumption was wrong here; the denser path appears to codegen or schedule better for the finalist rerun.

### Experiment 5 - Candidate perf gate validation

- Date: 2026-03-21
- Commit: working tree with launch chooser update only
- Hypothesis: the retained launch-chooser change should pass the repo’s GPU perf gate when measured through the project script.
- Files changed:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Env:
  - `& .\scripts\run_gpu_perf_gate.ps1 -Mode baseline ...`
  - `& .\scripts\run_gpu_perf_gate.ps1 -Mode candidate ...`
- GPU clocks normalized: no
- Benchmark commands:
  - bench filter `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
- Tests run: `run_gpu_perf_gate.ps1` baseline/candidate flow with `-CheckDynamicStrategyRatio $false`
- Nsight artifact: none
- Result:
  - baseline summary mean: `168.866462 ms`
  - candidate summary mean: `169.159925 ms`
  - script verdict: `[PASS] gpu_criterion_mean_not_regressed_10pct`
- Decision: keep
- Notes:
  - This gate run was used as a workflow validation for the retained change, not as a stabilized absolute-performance reference.

### Experiment 6 - Force low-pair-count lockstep_time for finalist recompute

- Date: 2026-03-21
- Commit: working tree with launch chooser update only
- Hypothesis: the finalist full-metrics recompute might be slow because it falls onto the generic `double_cross_backtest_tm_flex_f32` kernel at very low pair count; lowering `VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIR_BARS` should force lockstep dispatch and improve the product path.
- Files changed: none
- Env:
  - `VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIR_BARS=1000000`
- GPU clocks normalized: no
- Benchmark commands:
  - `cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel -- --backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 64`
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"`
- Tests run: single-run pipeline instrumentation and Criterion rerun
- Nsight artifact: previous `target/nsight/alma_end_to_end_product_baseline.ncu-rep` used to motivate the hypothesis
- Result:
  - single-run pipeline with override:
    - total `179.168 ms`
    - `tile_score`: `84.234 ms`
    - `final_finalize`: `89.955 ms`
  - Criterion with override:
    - `end_to_end_product`: `386.07 .. 406.65 .. 421.62 ms`
- Decision: reject
- Notes:
  - This confirms that simply forcing low-pair-count lockstep is the wrong fix.
  - The finalist recompute kernel-selection problem is more subtle than “always use lockstep for tiny pair counts.”

### Experiment 7 - Exact sparse explicit-combo finalist recompute

- Date: 2026-03-21
- Commit: current working tree after the retained finalist-recompute patch
- Hypothesis: finalist recompute can still be narrowed successfully if the helper uses the exact explicit combo list, builds a tile-local sparse pair map, and copies only the compact output prefix instead of decoding a denser temporary grid on the host.
- Files changed:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Env:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"`
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only"`
- GPU clocks normalized: no
- Tests run:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run`
  - Criterion reruns only
- Nsight artifact: captured in the follow-up profiling experiment below
- Result:
  - `end_to_end_product`: `156.06 .. 158.28 ms`
  - `search_only`: `85.599 .. 86.834 .. 90.651 ms`
  - Criterion reported no statistically significant `search_only` change, which is the expected outcome because the patch only changes finalist recompute.
- Decision: keep
- Notes:
  - This recovered the product benchmark into the sub-`160 ms` band without reducing useful work.
  - The winning shape was not the earlier reverted “dense unique-fast x unique-slow but filter later” attempt.
  - The retained version works because the recompute helper now stays sparse all the way through tile mapping and compact-prefix copy-out.

### Experiment 8 - Nsight Compute deep dive on retained exact-sparse product path

- Date: 2026-03-21
- Commit: current working tree with widened launch candidates and retained exact-sparse finalist recompute
- Hypothesis: after the retained recompute narrowing, the main remaining product-path cost is a kernel-dispatch mismatch inside finalist recompute rather than the main search pass.
- Files changed: none
- Env:
  - `scripts/profile_gpu_backtest_ncu.ps1`
  - benchmark filter `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`
  - `LaunchCount=80`
- GPU clocks normalized: no
- Tests run: none; profiling only
- Nsight artifact:
  - `target/nsight/alma_end_to_end_product_sparse_exact.ncu-rep`
- Result:
  - The current search-stage kernel inside the product benchmark is `double_cross_backtest_tm_flex_f32_lockstep_time` at block `(64, 1, 1)`, grid `(911, 1, 1)`.
  - Typical profiled search-stage launches were `104-108 ms`, with:
    - achieved occupancy about `29.65-29.67%`
    - theoretical occupancy `83.33%`
    - SM busy about `31-33%`
    - memory throughput about `10-12%`
    - L1/TEX hit rate about `61-65%`
    - L2 hit rate about `96-97%`
  - The retained finalist recompute repeatedly falls onto `double_cross_backtest_tm_flex_f32` at block `(64, 1, 1)`, grid `(1, 1, 1)`.
  - Typical profiled finalist-recompute launches were `104-118 ms`, with:
    - achieved occupancy `4.17%`
    - SM busy about `0.03-0.04%`
    - memory throughput about `0.72-0.78%`
    - L2 hit rate often only `4-6%`
  - This is the strongest remaining evidence that product-path time is still being lost to a tiny-grid generic recompute kernel, not to the main search-stage lockstep kernel.
- Decision: keep profiling conclusion and shift the next optimization target to a recompute-specific dispatch or kernel family.
- Notes:
  - The report also contains larger `(128, 1, 1)` / `(3841, 1, 1)` launches. Those appear to be unrelated harness noise or other benchmark activity inside the captured run and should not be used as the ALMA `58,300`-pair target signal.
  - The actionable Nsight signal is the alternating pair of current target kernels:
    - search stage: `lockstep_time`, grid `911`
    - finalist recompute: generic kernel, grid `1`
  - An attempted Nsight Systems follow-up capture produced a `.nsys-rep`, but the exported report did not contain usable CUDA trace data, so it should not be treated as evidence.

### Experiment 9 - Single-process reference rerun for the unsplit benchmark id

- Date: 2026-03-21
- Commit: current working tree with retained optimizations
- Hypothesis: the historical unsplit benchmark id should be rerun in isolation after the profiling work so the guide records its current meaning and current timing, even though it is no longer the cleanest product-contract reference.
- Files changed: none
- Env:
  - verified no live `cargo`, `double_ma_opt`, `ncu`, or `nsys` processes
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs"`
- GPU clocks normalized: no
- Tests run: Criterion only
- Nsight artifact: none
- Result:
  - plain `gpu_kernel`: `178.43 .. 183.16 .. 185.30 ms`
- Decision: keep as a secondary reference only
- Notes:
  - This remains slower than the split `end_to_end_product` benchmark and should not be treated as the authoritative product-path optimization target.
  - The split benchmark pair remains the clearest signal for continued work because it isolates search-stage cost from finalist-recompute cost.

### Experiment 10 - Global small-block override check for the product benchmark

- Date: 2026-03-21
- Commit: current working tree with retained optimizations
- Hypothesis: because finalist recompute collapses to a one-CTA generic kernel at `block=64`, a smaller global override might improve `end_to_end_product` by turning the finalist pass into multiple CTAs even before introducing a recompute-specific dispatch path.
- Files changed: none
- Env:
  - isolated Criterion reruns only
  - `VECTORBT_KERNEL_BLOCK_X=32`
  - `VECTORBT_KERNEL_BLOCK_X=64`
- GPU clocks normalized: no
- Benchmark commands:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"`
- Tests run: Criterion only
- Nsight artifact: none
- Result:
  - with `VECTORBT_KERNEL_BLOCK_X=32`: `159.30 .. 160.90 .. 162.32 ms`
  - with `VECTORBT_KERNEL_BLOCK_X=64`: `156.91 .. 158.43 .. 160.32 ms`
- Decision: reject as a global policy change
- Notes:
  - `32` did not beat `64` on the full product benchmark under the same session conditions.
  - This does not rule out a recompute-specific small-block policy, but it does rule out the simpler “make the whole benchmark smaller-block globally” fix.
  - The remaining likely win still points toward a dedicated finalist-recompute dispatch or kernel, not another broad launch-candidate expansion.

### Experiment 11 - Recompute-only small-block launch policy trial

- Date: 2026-03-21
- Commit: temporary working tree experiment, reverted
- Hypothesis: if finalist recompute is the only place suffering from the one-CTA generic kernel, then allowing `32`-thread CTAs only for the low-pair generic recompute path might improve `end_to_end_product` without perturbing the normal search path.
- Files changed:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Env:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run`
  - isolated Criterion rerun of `end_to_end_product`
- GPU clocks normalized: no
- Benchmark commands:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"`
- Tests run:
  - benchmark compile check only
- Nsight artifact: none
- Result:
  - patched `end_to_end_product`: `156.86 .. 157.39 .. 158.23 ms`
  - Criterion reported no statistically significant change
- Decision: revert
- Notes:
  - The scoped small-block trial was directionally fine, but not strong enough to justify extra launch-policy complexity.
  - This narrows the remaining search space further: the real next gain likely requires changing how finalist recompute is parallelized, not just how its existing generic pair-parallel kernel is blocked.

### Experiment 12 - Recompute-only low-pair M7 dispatch

- Date: 2026-03-21
- Commit: working tree, retained
- Hypothesis: the finalist recompute path is spending most of its time in the generic full-metrics kernel on a tiny pair count. If the existing full-metric `M7` specialization is a better fit for small finalist sets, then a recompute-only low-pair dispatch should materially reduce `end_to_end_product` without perturbing `search_only`.
- Files changed:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Env:
  - isolated `ta_cli` pipeline-profile run with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`
  - exploratory proof run with `VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIR_BARS=1`
  - isolated Criterion reruns of `end_to_end_product` and `search_only`
  - targeted unit tests for the new dispatch predicate
- GPU clocks normalized: no
- Benchmark commands:
  - `cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel -- --backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 64`
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product"`
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only"`
- Tests run:
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run`
  - `cargo test -p ta_app_core low_pair_recompute_m7_force --features cuda,cuda-backtest-kernel -- --nocapture`
- Nsight artifact: none
- Result:
  - pre-change pipeline profile:
    - total `405.288 ms`
    - `tile_score`: `226.535 ms`
    - `final_finalize`: `168.770 ms`
  - proof run with forced low pair-bar M7 threshold:
    - total `153.412 ms`
    - `tile_score`: `92.180 ms`
    - `final_finalize`: `56.237 ms`
  - retained scoped patch pipeline profile:
    - total `152.633 ms`
    - `tile_score`: `92.074 ms`
    - `final_finalize`: `55.668 ms`
  - retained Criterion `end_to_end_product`:
    - first rerun: `148.13 ms .. 149.21 ms .. 150.49 ms`
    - gate-respecting verification rerun: `147.00 ms .. 148.62 ms .. 152.14 ms`
  - retained Criterion `search_only`:
    - `86.620 ms .. 86.656 ms .. 86.690 ms`
- Decision: retain
- Notes:
  - This is the first high-confidence optimization in this pass that directly attacks the disproportionate finalist-recompute cost instead of shaving a few milliseconds from the broad search path.
  - The implementation is intentionally scoped: only finalist recompute is allowed to bypass the normal low-pair generic full-metric path, and only when the existing `M7` kernel is available and globally enabled.
  - A follow-up review caught an important control-path bug in the first draft: the forced recompute route initially ignored `VECTORBT_KERNEL_M7_SPECIALIZE=0`. The retained version now respects the global M7 enable flag, and that behavior is covered by unit tests.
  - The new measured gap is much more defensible:
    - `search_only`: about `86.6 ms`
    - `end_to_end_product`: about `147-152 ms`
    - extra product cost above `search_only`: about `60-65 ms`, down materially from the earlier `~70+ ms` band
  - This does not mean finalist recompute is now “solved.” It means the cheapest high-confidence dispatch win has been captured. Further gains will likely require either normalized-clocks validation or a genuinely different small-finalist execution shape.

### Experiment 13 - Current retained-code slowdown isolation

- Date: 2026-03-21
- Commit: current working tree after retained low-pair `M7` recompute dispatch
- Hypothesis: after the retained recompute dispatch win, the remaining `end_to_end_product` slowdown might still be mostly finalist recompute, but it is worth re-checking whether heatmap, top-k finalize, or other host-side work has become the new dominant cost.
- Files changed: none
- Env:
  - isolated `ta_cli` pipeline-profile runs only
  - no concurrent benchmark, `cargo`, `ncu`, `nsys`, or `ta_cli` processes
- GPU clocks normalized: no
- Benchmark commands:
  - product contract:
    - `VECTORBT_KERNEL_PROFILE_PIPELINE=1 cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel -- --backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 64`
  - search-only equivalent:
    - `VECTORBT_KERNEL_PROFILE_PIPELINE=1 VECTORBT_KERNEL_FINALIST_RECOMPUTE=0 cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel -- --backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 0`
  - heatmap-on but recompute-off control:
    - `VECTORBT_KERNEL_PROFILE_PIPELINE=1 VECTORBT_KERNEL_FINALIST_RECOMPUTE=0 cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel -- --backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 64`
- Tests run: none; profiling only
- Nsight artifact: none
- Result:
  - product contract:
    - total `157.235 ms`
    - `tile_score`: `91.559 ms`
    - `final_finalize`: `60.737 ms`
  - search-only equivalent:
    - total `88.546 ms`
    - `tile_score`: `82.676 ms`
    - `final_finalize`: `0.101 ms`
  - heatmap-on but recompute-off control:
    - total `98.319 ms`
    - `tile_score`: `92.353 ms`
    - `final_finalize`: `0.151 ms`
- Decision: diagnosis retained
- Notes:
  - The heatmap itself is negligible:
    - `heatmap_bins=64` with recompute disabled still leaves `final_finalize` at only `0.151 ms`
    - the measured heatmap work inside `tile_score` was only about `0.065 ms`
  - That means the remaining product premium is still overwhelmingly the finalist full-metrics rerun.
  - The relevant code region is [gpu_backtest_kernel.rs](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs):
    - `final_finalize` begins around the global top-k pack / decode and result finalize path near line `7822`
    - finalist recompute is gated near line `7913`
    - the actual recompute helper is `recompute_optimization_result_finalists_with_full_gpu_pass` near line `5239`
  - The benchmark contract itself still differs only in two meaningful ways:
    - `search_only` sets `heatmap_bins = 0`
    - `search_only` disables finalist recompute, while `end_to_end_product` leaves it enabled
  - So if more gain is still required, the next serious target remains the same:
    - optimize or redesign the finalist full-metrics rerun
    - do not spend more time blaming heatmap or broad search-stage overhead

## Complete checklist

### Phase 0 - Ground truth

- [x] Confirm benchmark semantics for `search_only`, `end_to_end_product`, and `gpu_kernel_pnl_full_metrics`.
- [x] Confirm the target benchmark ids have not changed.
- [x] Confirm the old `~76.919 ms` reference is historical and not a current expectation for the full product path.
- [x] Confirm the current optimization goal is to improve the present product contract, not revert to the old contract.

### Phase 1 - Stable baseline

- [ ] Stabilize GPU clocks and run conditions.
- [x] Capture repeated baseline timings for `search_only`.
- [x] Capture repeated baseline timings for `end_to_end_product`.
- [x] Capture repeated baseline timings for plain `gpu_kernel`.
- [x] Record all environment details.

### Phase 2 - Profiling baseline

- [x] Capture Nsight Compute for `search_only`.
- [x] Capture Nsight Compute for `end_to_end_product`.
- [x] Record hot kernels, registers, occupancy, stall reasons, and throughput.
- [x] Determine how much of the product gap is finalist recompute.

### Phase 3 - Launch tuning

- [x] Sweep block sizes on `search_only`.
- [x] Sweep block sizes on `end_to_end_product`.
- [x] Re-run the best two candidates.
- [ ] Decide whether any winning policy is generic or `sm_89`-specific.

### Phase 4 - Search-only path optimization

- [ ] Profile the backtest kernel body.
- [ ] Profile selection/gather/packed-row overhead.
- [x] Implement the smallest high-confidence search-path optimization.
- [x] Verify correctness.
- [x] Re-measure under identical conditions.

### Phase 5 - Product-path optimization

- [x] Profile finalist recompute in detail.
- [x] Narrow or reuse work inside finalist recompute where possible.
- [x] Measure heatmap contribution.
- [x] Measure top-k/pack contribution.
- [x] Implement one product-path optimization at a time.
- [x] Verify correctness.
- [x] Re-measure under identical conditions.

### Phase 6 - Comparison and rollout

- [ ] Compare final timings against the stabilized baseline, not against unmatched historical notes.
- [ ] Compare current optimized tree against `1fccb9d` under the same conditions if needed.
- [x] Run the relevant perf gate or targeted candidate perf scripts.
- [x] Update this document and any related benchmark records with the final measured deltas.

## Expected outcome

The expected result of this pass is not “get back to `76.919 ms` at all costs.”

The expected result is:

- understand the current performance envelope,
- measure the real cost of product-oriented work,
- reduce that cost as much as possible without reducing usefulness,
- and leave the repo with a defensible, repeatable benchmark and profiling workflow for future iterations.

## Current recommendation

The next implementation step should not be more global lockstep-threshold tuning.

The retained optimization work already recovered:

- `search_only` to the `~86-90 ms` band
- `end_to_end_product` to the `~147-152 ms` band

The current evidence says the biggest easy win was not a new kernel at all, but a better recompute dispatch:

- keep the current exact sparse combo mapping,
- keep full finalist metric fidelity,
- route tiny finalist reruns onto the existing full-metric `M7` specialization when it is available and enabled.

The earlier override checks still say that simply shrinking the global backtest block size is not enough:

- `block=32` did not beat `block=64` on `end_to_end_product`
- so the next win is unlikely to come from another global launch-candidate tweak by itself

The recompute-only small-block launch-policy trial also failed to produce a meaningful gain:

- allowing `32` only for the low-pair generic finalist rerun stayed within noise
- so the next likely win is a different recompute execution shape, not just a different CTA size

The most promising next design, if more product-path gain is still needed after stabilized-clock reruns, is a recompute-only route for very small finalist sets that parallelizes over time within each finalist pair instead of relying on pair-count parallelism.
