# Phase 2 Lite Upload Bundle

Date: 2026-04-03

This folder is the trimmed non-code upload bundle for a GPT-5.4 Pro review of the `double_crossover` Phase 2 work.

It is derived from the full bundle in `SASS research/Phase 2`, but removes heavyweight profiler binaries and keeps only upload-friendly text, Markdown, JSON, JSONL, CSV, and log files.

## Core Upload Set

If you want the smallest high-signal set, start with these:

### Repo context

- `repo-docs/AGENTS.md`
- `repo-docs/REPO_MAP.md`
- `repo-docs/README.md`
- `repo-docs/RELEASE.md`

### Current research framing

- `research-docs/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `research-docs/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `research-docs/double_crossover_phase2_gpt54_pro_prompt_pack_2026-04-03.md`
- `research-docs/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`
- `research-docs/double_crossover_performance_findings_and_code_suggestions.md`
- `research-docs/double_crossover_runtime_verification_summary_2026-04-02.txt`

### Fresh SASS evidence

- `sass/double_crossover_phase2_hot_kernels_sass_excerpt_2026-04-03.txt`
- `sass/double_crossover_sm89_cuobjdump_symbols_2026-04-02.txt`
- `sass/double_crossover_targeted_store_counts_2026-04-02.txt`

### Fresh runtime evidence

- `profiling/runner/double_crossover_phase2_large_full17_row_first_2026-04-03.jsonl`
- `profiling/runner/double_crossover_phase2_large_trade22_row_first_2026-04-03.jsonl`
- `profiling/runner/double_crossover_phase2_xlarge_shortlist_full17_row_first_2026-04-03.jsonl`
- `profiling/runner/double_crossover_phase2_xlarge_shortlist_trade22_row_first_2026-04-03.jsonl`
- `profiling/runner/double_crossover_phase2_large_full17_pipeline_2026-04-03.log`
- `profiling/runner/double_crossover_phase2_large_trade22_pipeline_2026-04-03.log`
- `profiling/nsight-systems/double_crossover_phase2_large_full17_stats.txt`
- `profiling/nsight-systems/double_crossover_phase2_large_trade22_stats.txt`
- `profiling/nsight-compute/double_crossover_phase2_ncu_summary_2026-04-03.txt`
- `profiling/nsight-compute/double_crossover_phase2_ncu_summary_2026-04-03.json`

## Useful Detail Files

These are still small enough to upload and may help if you want more direct profiler evidence without shipping binary `.rep` files:

- `research-docs/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `research-docs/double_ma_backtest_performance_unexplored_research_2026-04-01.md`
- `profiling/nsight-systems/*.provenance.json`
- `profiling/nsight-compute/*_backtest.provenance.json`
- `profiling/nsight-compute/*_reducers.provenance.json`
- `profiling/nsight-compute/*_raw.csv`

## What Was Removed Relative To The Full Bundle

The lite bundle intentionally excludes:

- all `.ncu-rep` files
- all `.nsys-rep` files
- the full `32 MB` April 2 SASS dump
- local `nsys` SQLite exports
- transient stderr or helper logs
- all code files
- all helper scripts

The reason is simple:

- `.ncu-rep` and `.nsys-rep` are the least model-friendly files in the bundle
- the full April 2 SASS dump is replaced here by a smaller hot-kernel excerpt covering the Phase 2-relevant symbols
- the text summaries, JSON provenance, raw CSV exports, and Markdown findings capture the useful content in a much smaller form

## Recommended Upload Order

1. Core upload set first.
2. Your code files second.
3. Useful detail files only if the model needs more profiler granularity.

## Folder Intent

This folder is meant to be the practical upload set, not the archival one.
