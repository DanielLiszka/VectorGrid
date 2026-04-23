# Bundle 7 Validated Tracking Checklist

This is the authoritative execution tracker for Bundle 7:

- `gpt54_feature_review_map.md` Section `## 7. Benchmark harnesses, perf gates, and timing boundaries`
- `bundle7_benchmark_harness_impl_plan.md`
- `bundle7_benchmark_harness_impl_plan_addendum_v2.md`

This checklist has already been validated against the current repo state.

Rules:

- Only implement items in the unchecked required sections below.
- Treat the older Bundle 7 plan/addendum text as background only.
- Do not re-implement items marked `[x]` in the preserve section.
- Use this file, not the older plan prose, as the progress tracker.

## Validation Completed

- [x] Re-read `gpt54_feature_review_map.md` Section 7.
- [x] Re-read `bundle7_benchmark_harness_impl_plan.md`.
- [x] Re-read `bundle7_benchmark_harness_impl_plan_addendum_v2.md`.
- [x] Compare the Bundle 7 docs against the current scripts, benches, binaries, and existing perf artifacts.
- [x] Identify stale assumptions and already-implemented surfaces that should be preserved rather than re-implemented.
- [x] Update `codex_bundle7_full_implementation_prompt.md` so it only directs valid current-repo work.

## Already Present: Preserve, Extend, Do Not Re-implement

- [x] `scripts/workflow_benchmark_pack.json` already exists and is the workflow-pack source for `scripts/run_workflow_gate.ps1`.
- [x] `scripts/run_gpu_perf_gate.ps1` already builds and runs `profile_gpu_dynamic_runner` in release mode.
- [x] `scripts/run_gpu_perf_gate.ps1` already emits a GPU CI-width instability warning.
- [x] `scripts/run_gpu_alma_contract_rows_perf_gate.ps1` already exists as a separate extended contract-row perf surface.
- [x] `scripts/profile_gpu_backtest_nsys.ps1` already supports `-NsysExePath` and already has bounded-capture fallback behavior.
- [x] `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` already persists detailed per-trial artifacts.
- [x] `src-tauri/src/bin/ta_app_path_faceoff.rs` already records per-trial total/harness/state/engine timing, GPU/hybrid subphase timing, and `timing_boundary_note`.
- [x] `src-tauri/src/bin/ta_double_ma_validation_cost.rs` already records requested backend, backend used/route/fallback, and separate main-validation vs cross-ticker wall timings.
- [x] `src-tauri/src/bin/ta_profile_route_matrix.rs` already records requested backend, actual route/fallback, runtime, wall, and best-result fields.
- [x] `crates/app_core/benches/gpu_native_expansion_opt.rs` already contains exact GPU route assertions / route probe behavior.
- [x] `crates/app_core/benches/composite_validation_workflow.rs` already contains exact GPU route assertions / route probe behavior.
- [x] `crates/app_core/benches/double_ma_validation_workflow.rs` already prints route/profile diagnostics.
- [x] `crates/app_core/benches/double_ma_opt.rs` already has route-probe logging and env-shaped contract-split helpers.
- [x] `crates/app_core/benches/dynamic_strategy_cpu.rs` already keeps its dynamic env toggles outside measured `b.iter(...)` loops.
- [x] `crates/app_core/benches/e2e_workflow_ml.rs` already supports env-configurable candle lengths via `VTA_E2E_BENCH_LENS`.

## Required Implementation Scope

### Scripts and Gates

- [x] Fix `scripts/run_workflow_gate.ps1` so baseline mode keeps `OutPath` stable, writes the fresh run to `OutPath`, copies that summary to `BaselinePath`, asserts the requested output exists, and supports explicit release-profile control suitable for gate use.
- [x] Fix `scripts/run_cpu_perf_gate.ps1` so it passes an explicit workflow summary path into `run_workflow_gate.ps1`, reads back that exact path, and stops assuming baseline mode also writes the candidate path.
- [x] Expand `scripts/run_cpu_perf_gate.ps1` from a single Criterion CPU case to a deterministic two-case matrix using the current live surfaces: `double_ma/cpu/sma_sma/200k/small` and `double_ma/cpu/alma_alma/200k/large`.
- [x] Persist explicit timing-boundary and build-profile metadata for the CPU workflow block, each CPU Criterion case, and the dynamic CPU ratio block in `scripts/run_cpu_perf_gate.ps1`.
- [x] Add baseline compatibility validation to CPU gate comparisons before numeric pass/fail checks.
- [x] Fix `scripts/run_gpu_perf_gate.ps1` so it no longer uses latest/glob estimate fallback for pass/fail logic and instead resolves exact Criterion estimate paths deterministically per named case.
- [x] Expand `scripts/run_gpu_perf_gate.ps1` from a single GPU Criterion case to a deterministic two-case matrix using the current live surfaces: `double_ma/gpu_kernel/sma_sma/200k/small` and `double_ma/gpu_kernel/alma_alma/200k/large`.
- [x] Keep contract-split / contract-row GPU surfaces available as extended or optional evidence instead of deleting them or forcing them into the main per-PR gate.
- [x] Change the GPU dynamic gate in `scripts/run_gpu_perf_gate.ps1` from candidate `on` vs candidate `off` to candidate ratio vs baseline ratio.
- [x] Keep an additional absolute baseline guard on static/off GPU latency in `scripts/run_gpu_perf_gate.ps1`.
- [x] Add baseline compatibility validation to GPU gate comparisons, including GPU hardware compatibility checks where relevant.
- [x] Add provenance sidecar JSON plus `try/finally`-style environment restoration to `scripts/profile_gpu_backtest_nsys.ps1`.
- [x] Add `-NcuExePath`, provenance sidecar JSON, and `try/finally`-style environment restoration to `scripts/profile_gpu_backtest_ncu.ps1`.
- [x] Update `scripts/run_vectorgrid_double_ma_faceoff.ps1` to read structured Criterion `estimates.json` instead of parsing human-readable CLI timing text.
- [x] Update `scripts/run_vectorgrid_double_ma_faceoff.ps1` to derive metadata from benchmark ID where practical and label artifacts with `timing_boundary = "Criterion"` and `build_profile = "bench"`.
- [x] Update `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` so persisted artifacts are labeled by timing boundary and build profile.
- [x] Update `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` so grouped summary rows use the canonical median-time trial instead of the first artifact.
- [x] Update `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` so parity uses the canonical trial and the summary preserves app-path-specific stats instead of mixing timing boundaries.

### Binaries and Harness Artifacts

- [x] Add metadata-envelope fields to `src-tauri/src/bin/ta_workflow_regression.rs`, including at least `schema_version`, `timing_boundary`, `timing_boundary_note`, and `build_profile`.
- [x] Keep the existing total `runtime_ms` in `src-tauri/src/bin/ta_workflow_regression.rs`.
- [x] Add stage breakdowns to `src-tauri/src/bin/ta_workflow_regression.rs` for at least optimize total, validate total, meta-select, and holdout.
- [x] Add additive metadata-envelope fields to `src-tauri/src/bin/ta_double_ma_validation_cost.rs`.
- [x] Preserve existing route/fallback fields in `src-tauri/src/bin/ta_double_ma_validation_cost.rs`.
- [x] Add separate total-vs-optimization, main-validation-vs-optimization, and cross-ticker-vs-optimization ratios to `src-tauri/src/bin/ta_double_ma_validation_cost.rs`.
- [x] Add normalized validation-cost metrics to `src-tauri/src/bin/ta_double_ma_validation_cost.rs` if they remain low-risk and useful in the current shape.
- [x] Add explicit `schema_version`, `timing_boundary`, `build_profile`, and `requested_backend` fields to `src-tauri/src/bin/ta_app_path_faceoff.rs`.
- [x] Preserve the current same-thread app-path boundary and existing medians/subphase timing fields in `src-tauri/src/bin/ta_app_path_faceoff.rs`.
- [x] Add a measured-trial route-summary block to `src-tauri/src/bin/ta_app_path_faceoff.rs`.
- [x] Add additive envelope/build-profile fields to `src-tauri/src/bin/ta_profile_route_matrix.rs`.
- [x] Add `expected_backend_route` and `route_ok` fields to `src-tauri/src/bin/ta_profile_route_matrix.rs`.
- [x] Add optional `--strict` handling to `src-tauri/src/bin/ta_profile_route_matrix.rs` without redesigning the one-shot default surface.

### Bench Surfaces

- [x] In `crates/app_core/benches/double_ma_opt.rs`, keep current benchmark IDs and the existing route-probe path.
- [x] In `crates/app_core/benches/double_ma_opt.rs`, move env-var mutation out of measured `b.iter(...)` loops where the mode is fixed for the whole benchmark.
- [x] In `crates/app_core/benches/double_ma_opt.rs`, add broader benchmark-shape metadata for env-shaped and contract-split variants without renaming benchmark IDs.
- [x] Add additive metadata banners to `crates/app_core/benches/double_ma_metrics.rs`.
- [x] Add additive metadata banners to `crates/app_core/benches/double_ma_followup_workflow.rs` without renaming the existing cold/cached benchmark IDs.
- [x] Add additive metadata banners to `crates/app_core/benches/dynamic_strategy_cpu.rs`.
- [x] Add an optional quick local-rerun helper/env to `crates/app_core/benches/dynamic_strategy_cpu.rs` without changing default gate numbers.
- [x] Add additive metadata banners to `crates/app_core/benches/gpu_native_expansion_opt.rs`.
- [x] Add additive metadata banners to `crates/app_core/benches/double_ma_validation_workflow.rs`.
- [x] Add optional strict exact-route behavior to `crates/app_core/benches/double_ma_validation_workflow.rs` only if it stays additive and low-risk.
- [x] Add additive metadata banners to `crates/app_core/benches/composite_validation_workflow.rs`.
- [x] Add additive metadata banners to `crates/app_core/benches/e2e_workflow_ml.rs`.
- [x] Add env-configurable train/holdout asset counts to `crates/app_core/benches/e2e_workflow_ml.rs`.
- [x] Add optional stage diagnostics to `crates/app_core/benches/e2e_workflow_ml.rs` while keeping the benchmark practical for normal use.

### Verification and Rollout

- [x] Run `cargo check --all-targets` or the closest practical compile-equivalent for the touched Rust surfaces.
- [x] Run relevant `cargo test` invocations for touched binaries/helpers where practical.
- [x] Run `cargo bench --no-run` for touched bench targets where practical.
- [x] Do PowerShell syntax or smoke validation for the touched scripts where practical.
- [x] Add helper/unit coverage where it is practical and low-risk for workflow stage breakdowns, validation-cost ratios, canonical trial selection, and structured Criterion-estimate reading.
- [x] Regenerate affected baseline files if schema shape, case matrix, or comparison identity changes require it.
- [x] Document exact GPU / Nsight validation commands if the current environment cannot run them.

Verification notes:

- Closest practical compile/test coverage run for touched Rust surfaces:
  - `cargo check -p ta_desktop_demo_app --bin ta_workflow_regression --bin ta_double_ma_validation_cost --bin ta_app_path_faceoff --bin ta_profile_route_matrix`
  - `cargo test -p ta_desktop_demo_app --bin ta_workflow_regression --bin ta_double_ma_validation_cost --bin ta_app_path_faceoff --bin ta_profile_route_matrix`
  - `cargo check -p ta_app_core --benches --features cuda,cuda-backtest-kernel`
  - `cargo bench -p ta_app_core --no-run --features cuda,cuda-backtest-kernel`
  - `cargo bench -p ta_app_core --bench double_ma_validation_workflow --no-run --features cuda,cuda-backtest-kernel`
- Gate and script validation run:
- `powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode baseline`
- `powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode baseline`
- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode baseline`
- `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode candidate`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode baseline`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
  - PowerShell parser validation on the touched scripts after edits.
- Helper/unit coverage added where repo-local tests were practical:
  - `ta_workflow_regression.rs` legacy-envelope/stage-shape coverage
  - `ta_double_ma_validation_cost.rs` ratio helper coverage
  - `ta_app_path_faceoff.rs` route-summary/additive-field coverage
  - `ta_profile_route_matrix.rs` route-ok helper coverage
  - No dedicated PowerShell test harness exists in-repo, so canonical-trial and structured-estimate script paths were validated by deterministic helper structure, parser validation, and the live gate runs above rather than standalone unit tests.
- Regenerated baselines:
  - `scripts/baselines/workflow_regression_baseline.json`
  - `scripts/baselines/cpu_perf_baseline.json`
  - `scripts/baselines/gpu_perf_baseline.json`
  - `scripts/baselines/gpu_alma_contract_rows/auto.json`
  - `scripts/baselines/gpu_alma_contract_rows/full17.json`
  - `scripts/baselines/gpu_alma_contract_rows/trade22.json`
- Additional optional-scope runtime validation:
  - `cargo run --release -p ta_desktop_demo_app --bin ta_app_path_faceoff -- --csv data/benchmarks/AAPL.csv --timeframe 15m --output-json target/app_path_faceoff_optional.json --backend cpu --fast-ma sma --slow-ma sma --fast-period 5:7:1 --slow-period 10:14:2 --top-k 3 --warmup-trials 0 --measured-trials 1 --cold-state-trials 1`
  - `cargo run --release -p ta_desktop_demo_app --bin ta_profile_route_matrix -- --csv data/benchmarks/AAPL.csv --timeframe 15m --out target/profile_route_matrix_optional.json --warmup-runs 0 --measured-runs 2 --strict`
  - The app-path optional smoke passed and emitted the new fresh-state boundary fields.
  - The route-matrix optional smoke wrote the repeated-run artifact and then failed `--strict` as expected on GPU/HYBRID cases because the binary was intentionally run without `--features cuda,cuda-backtest-kernel`, so those requests honestly fell back to CPU.
- Deferred long-running / admin-only validation commands:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_vectorgrid_double_ma_faceoff.ps1 -BenchmarkId "double_ma/gpu_kernel/sma_sma/200k/small"`
  - `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target bench -BenchFilter "double_ma/gpu_kernel/sma_sma/200k/small" -NvtxMode on -CaptureMode nvtx -OutputPrefix "target/nsight/gpu_kernel_small"`
  - `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target bench -BenchFilter "double_ma/gpu_kernel/sma_sma/200k/small" -OutputPrefix "target/nsight/gpu_kernel_small_ncu"`
- Live `VectorGrid` vs `VectorTA` faceoff execution was intentionally skipped at user request because it is long-running; the faceoff scripts were still updated and parser-validated.

## Optional Follow-up Only

- [x] Optional: add repeat/median support to `src-tauri/src/bin/ta_profile_route_matrix.rs` while preserving one-shot default behavior.
- [x] Optional: add stage-specific candidate-vs-baseline budgets to the workflow gate path after stage timing exists.
- [ ] Optional: add normalized throughput metrics to route/app-path surfaces if they are clearly additive and low-risk.
- [x] Optional: add an explicit warm/cold split to `src-tauri/src/bin/ta_app_path_faceoff.rs` only as a separate named boundary.
- [ ] Optional: add a manifest / registry file only if deterministic per-script case maps prove insufficient.
- [x] Optional: add environment fingerprinting or a stronger instability policy only if it stays additive and low-risk.

## Final Acceptance

- [x] No gate reads a file path it did not explicitly request.
- [x] No gate uses latest-matching artifact fallback for pass/fail logic.
- [x] CPU and GPU gates validate baseline compatibility before numeric comparisons.
- [x] Workflow-gate baseline mode no longer breaks candidate-summary readback.
- [x] GPU dynamic gate compares candidate ratio to baseline ratio, not candidate `on` vs candidate `off`.
- [x] Persisted summaries/artifacts clearly identify timing boundary and build profile.
- [x] `ta_workflow_regression.rs` emits stage breakdown plus total runtime.
- [x] `ta_app_path_faceoff.rs` preserves existing medians, adds route summary, and labels the artifact boundary/build profile explicitly.
- [x] `ta_double_ma_validation_cost.rs` separates total/main/cross ratios.
- [x] `ta_profile_route_matrix.rs` supports strict route checks.
- [x] `scripts/run_vectorgrid_double_ma_faceoff.ps1` uses structured Criterion JSON instead of CLI text parsing.
- [x] `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` uses canonical trials for summary rows/parity and keeps boundaries explicit.
- [x] `crates/app_core/benches/double_ma_opt.rs` no longer mutates fixed-mode env vars inside measured loops.
- [x] Existing features and execution paths are preserved.
- [x] Benchmark IDs remain stable unless there is no safe alternative.
- [x] Optional items intentionally skipped are called out with reasons.
