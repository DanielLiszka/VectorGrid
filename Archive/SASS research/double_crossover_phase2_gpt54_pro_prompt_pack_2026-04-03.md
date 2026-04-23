# `double_crossover` Phase 2 GPT-5.4 Pro Prompt Pack

Date: 2026-04-03

Purpose: a repo-grounded prompt pack for handing the completed Phase 2 profiling bundle to GPT-5.4 Pro before any new `double_crossover` optimization work.

This pack is intentionally profiler-first. It is not a request for generic CUDA ideas.

## How To Use This Pack

- Use one prompt per chat.
- Always attach the shared baseline files first.
- Then attach the prompt-specific files.
- Do not ask the model to redesign everything. Ask it to pick the next bounded move from the attached evidence.
- Require the model to distinguish:
  - what the repo already proved
  - what the fresh artifacts do not prove
  - what it would measure before recommending code changes

## Shared Baseline Files

Attach these to every prompt:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `RELEASE.md`
- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`
- `SASS research/double_crossover_performance_findings_and_code_suggestions.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`

Recommended shared framing:

- the live product route is still row-first exact `GpuKernel`
- the repo already did a meaningful SASS-guided wrapper/store pass on April 2
- the next step must be chosen from profiler evidence, not from generic CUDA folklore

## Shared Answer Shape

Ask for this shape on every thread:

1. What the repo already proved.
2. What the attached profiling bundle does not prove.
3. The best next one or two measurements or interventions.
4. The exact file and symbol touch points.
5. The exact commands and artifacts to collect.
6. The perf and parity verification plan.
7. The rejection criteria.

## Prompt 1: Pick The Next Product Intervention

### Research Question

Given the completed Phase 2 profiling bundle, what is the strongest next product-facing optimization to attempt?

### Attach These Files

Required:

- all shared baseline files
- `target/double_crossover_phase2_large_full17_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_large_trade22_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_xlarge_shortlist_full17_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_xlarge_shortlist_trade22_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_large_full17_pipeline_2026-04-03.log`
- `target/double_crossover_phase2_large_trade22_pipeline_2026-04-03.log`
- `target/nsight/double_crossover_phase2_large_full17_stats.txt`
- `target/nsight/double_crossover_phase2_large_trade22_stats.txt`
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`

Optional:

- `target/nsight/double_crossover_phase2_large_full17_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_large_trade22_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_xlarge_reducers_raw.csv`

### Prompt

```text
You are reviewing the next optimization step for the `double_crossover` exact `double_ma` CUDA route in this repo.

Using only the attached files, answer this repo-specific question:

Given the completed Phase 2 runtime profiling bundle, what is the strongest next product-facing optimization to attempt?

Constraints:
- Do not give generic CUDA advice.
- Do not propose blanket hand-SASS work.
- Do not re-suggest ideas the attached docs already tried and rejected.
- Keep CPU fallback unchanged.
- Keep exact finalist semantics authoritative.
- The answer must be driven by the attached direct-runner, built-in timing, Nsight Systems, and Nsight Compute artifacts.

Deliverables:
1. Summarize what the repo already proved before this profiling pass.
2. Summarize what the new profiling bundle now proves.
3. Identify the strongest next one or two product interventions.
4. Rank those interventions and explain why the top choice wins.
5. Point to exact files and symbols to change.
6. Give the exact commands and artifacts you would require after implementation.
7. State what evidence would cause you to reject your own recommendation.

Be concrete and repo-specific. Cite attached files and kernel symbols rather than giving broad optimization heuristics.
```

## Prompt 2: Backtest-Body Dataflow Or Layout Follow-On

### Research Question

Do the current profiling artifacts justify one more bounded backtest-body source/layout pass, and if so what exact pass?

### Attach These Files

Required:

- all shared baseline files
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`
- `target/nsight/double_crossover_phase2_large_full17_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_large_trade22_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_large_full17_stats.txt`
- `target/nsight/double_crossover_phase2_large_trade22_stats.txt`
- `SASS research/double_crossover_sm89_cuobjdump_2026-04-02.sass.txt`
- `SASS research/double_crossover_targeted_store_counts_2026-04-02.txt`

### Prompt

```text
You are reviewing whether the current `double_crossover` exact row-first path still justifies one more bounded backtest-body source/layout pass.

Using only the attached files, answer this question:

Do the current Phase 2 profiler artifacts justify one more bounded backtest-body source/layout pass, and if so what exact pass should this repo try next?

Constraints:
- Focus on the retained hot exact kernels, not a broad architecture rewrite.
- Treat the April 2 SASS/store recovery work as already completed.
- Do not propose already rejected shuffle, broadcast, generic launch-tuning, or broad occupancy-only ideas.
- If you think no bounded body pass remains credible, say so and defend it.

Deliverables:
1. State whether the current profiler evidence points to dataflow/layout, schedule-only tuning, or neither.
2. Explain how the current long-scoreboard and bytes-per-sector evidence should be interpreted for this repo.
3. Propose the single best bounded body pass, if any.
4. Explain why it is meaningfully different from the rejected ideas in the attached docs.
5. Point to exact code regions and wrapper families.
6. Define a narrow A/B plan, including what would count as a keep vs reject result.

Keep the answer repo-specific and tied to the attached symbols and artifacts.
```

## Prompt 3: Research Framing From The Same Evidence

### Research Question

What is the strongest publishable research framing supported by this repo after the completed Phase 2 profiling bundle?

### Attach These Files

Required:

- all shared baseline files
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`
- `target/nsight/double_crossover_phase2_large_full17_stats.txt`
- `target/nsight/double_crossover_phase2_large_trade22_stats.txt`

Optional:

- `target/nsight/double_crossover_phase2_large_full17_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_large_trade22_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_xlarge_reducers_raw.csv`

### Prompt

```text
You are evaluating the strongest publishable research framing for this repo after the completed Phase 2 profiling bundle.

Using only the attached files, answer this repo-specific question:

What is the strongest publishable research framing now supported by this repo and its new profiling evidence?

Constraints:
- Do not answer with generic “CUDA optimization is interesting”.
- Distinguish product optimization priorities from research-paper framing.
- Treat the profiling bundle as the current truth source, not the earlier static-only hypothesis set.

Deliverables:
1. Explain what the repo already proved before Phase 2.
2. Explain what the new profiling bundle adds that was previously missing.
3. Rank the strongest research directions now supported by the evidence.
4. For the top-ranked direction, give:
   - the claim
   - the minimal experiment set
   - the required artifact set
   - the likely baseline comparisons
   - the obvious rejection criteria
5. Explain whether the evidence now supports:
   - a SASS-guided source-refactor oracle paper
   - a static+dynamic attribution tool paper
   - a narrow store-width recovery study
   - a distribution-aware specialization study
   - some other framing
6. State which framing you would not prioritize and why.

Keep the answer grounded in the attached repo files, kernels, and profiler outputs.
```

## Prompt 4: Follow-On Measurements Before Any Rewrite

### Research Question

If no code changes are made yet, what exact follow-on measurements should be collected next?

### Attach These Files

Required:

- all shared baseline files
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`
- `target/nsight/double_crossover_phase2_large_full17_stats.txt`
- `target/nsight/double_crossover_phase2_large_trade22_stats.txt`

### Prompt

```text
You are not allowed to propose code changes yet.

Using only the attached files, answer this question:

If we collect more measurements before touching the code again, what exact follow-on measurements should come next?

Constraints:
- The answer must be measurement-only.
- Stay within the repo’s existing profiling and perf-gate tooling unless you can justify a new tool.
- Distinguish must-have from nice-to-have measurements.

Deliverables:
1. List the minimum next measurements in priority order.
2. Explain what each measurement would resolve that the current bundle still leaves ambiguous.
3. Give the exact commands, workload surfaces, and artifact paths you would use.
4. State what outcomes would allow us to skip a measurement as unnecessary.
5. State what outcomes would justify moving from product optimization into a more research-oriented tooling phase.

Keep the answer concrete and repo-specific.
```
