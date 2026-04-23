# `double_crossover` Phase 2 Research Artifact Index

Date: 2026-04-03

## Purpose

Provide one entry point for the full Phase 2 research bundle.

## Canonical Reading Order

1. `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
2. `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`
3. `SASS research/double_crossover_phase2_research_artifact_plan_2026-04-03.md`

## Static SASS And Early Contract Work

1. `SASS research/double_crossover_performance_findings_and_code_suggestions.md`
2. `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`

## Runtime Attribution And Retained Phase 2 Branches

1. `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
2. `SASS research/double_crossover_phase2_shortlist_slice_attribution_log_2026-04-03.md`
3. `SASS research/double_crossover_phase2_windowed_replay_kernel_study_2026-04-03.md`
4. `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`

## Replay Follow-On Measurement Artifacts

1. `SASS research/double_crossover_phase2_replay_artifact_manifest_2026-04-03.md`
2. `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
3. `SASS research/double_crossover_phase2_replay_branch_decision_2026-04-03.md`
4. `SASS research/double_crossover_phase2_replay_methodology_note_2026-04-03.md`

## Planning And Tracking

1. `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
2. `SASS research/double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md`
3. `SASS research/double_crossover_phase2_replay_dataflow_architecture_plan_2026-04-03.md`

## Key Verification Outputs

1. `target/gpu_perf_summary.json`
2. `target/gpu_alma_contract_rows_perf_summary.json`

## Representative Code Surfaces

1. `crates/app_core/src/kernels/double_crossover.cu`
2. `crates/app_core/src/gpu_backtest_kernel.rs`
3. `crates/app_core/src/double_ma.rs`
4. `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

## Key Quantitative Anchors

1. retained shortlist-heavy replay-batching result:
   - `full17`: `none 296.00 ms`, `halves 342.80 ms`, `quartiles 349.00 ms`
   - `trade22`: `none 308.20 ms`, `halves 359.00 ms`, `quartiles 365.00 ms`
2. replay micro-attribution result:
   - replay-enabled surfaces still carry `~30-32 ms` replay buckets
   - descriptor prep and upload round to `0 ms`
3. authoritative control surface:
   - `target/gpu_alma_contract_rows_perf_summary.json`

## Suggested External Review Bundle

If sharing this with an external reviewer or model, the minimum high-signal subset is:

1. `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
2. `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`
3. `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
4. `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`
5. `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
6. `SASS research/double_crossover_phase2_replay_branch_decision_2026-04-03.md`
7. `target/gpu_perf_summary.json`
8. `target/gpu_alma_contract_rows_perf_summary.json`
