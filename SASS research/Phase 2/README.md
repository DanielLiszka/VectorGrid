# Phase 2 Upload Bundle

Date: 2026-04-03

This folder is the non-code handoff bundle for the `double_crossover` Phase 2 GPT-5.4 Pro review.

## What Is Included

`repo-docs/`

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `RELEASE.md`

`research-docs/`

- the current Phase 2 plan
- the fresh Phase 2 profiling findings note
- the fresh GPT-5.4 Pro prompt pack
- the April 2 validated SASS handoff note
- the April 2 SASS findings note
- the April 2 runtime verification summary
- the March 30 next-phase performance plan
- the April 1 unexplored-research note

`sass/`

- fresh April 2 SASS dump
- fresh April 2 symbol dump
- fresh April 2 targeted store-count summary

`profiling/runner/`

- fresh April 3 direct-runner JSONL baselines
- fresh April 3 built-in pipeline logs

`profiling/nsight-systems/`

- fresh April 3 `nsys` reports
- fresh April 3 `nsys` provenance JSON
- fresh April 3 `nsys` text stats

`profiling/nsight-compute/`

- fresh April 3 `ncu` reports
- fresh April 3 `ncu` provenance JSON
- fresh April 3 raw CSV exports
- fresh April 3 summarized NCU findings in text and JSON form

## Primary Upload Targets

If you want the highest-signal upload set first, start with:

1. `repo-docs/AGENTS.md`
2. `repo-docs/REPO_MAP.md`
3. `repo-docs/README.md`
4. `repo-docs/RELEASE.md`
5. `research-docs/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
6. `research-docs/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
7. `research-docs/double_crossover_phase2_gpt54_pro_prompt_pack_2026-04-03.md`
8. `research-docs/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`
9. `research-docs/double_crossover_performance_findings_and_code_suggestions.md`
10. `sass/double_crossover_sm89_cuobjdump_2026-04-02.sass.txt`
11. `sass/double_crossover_sm89_cuobjdump_symbols_2026-04-02.txt`
12. `sass/double_crossover_targeted_store_counts_2026-04-02.txt`
13. `profiling/runner/*.jsonl`
14. `profiling/runner/*.log`
15. `profiling/nsight-systems/*.txt`
16. `profiling/nsight-systems/*.json`
17. `profiling/nsight-compute/double_crossover_phase2_ncu_summary_2026-04-03.txt`
18. `profiling/nsight-compute/double_crossover_phase2_ncu_summary_2026-04-03.json`
19. `profiling/nsight-compute/*_raw.csv`

The `.ncu-rep` and `.nsys-rep` files are included for completeness, but the text, JSON, CSV, and Markdown files are the most useful uploads for a model review.

## What Was Intentionally Excluded

- source files such as `.cu`, `.rs`, `.ts`, and similar code files
- helper scripts such as `.ps1`
- intermediate or derived profiler files that are not useful for model upload, such as local `nsys` SQLite exports and transient stderr logs

## Folder Intent

This folder is meant to save you from manually assembling the non-code evidence bundle before a GPT-5.4 Pro review.
