# Performance Audit Program

Date: 2026-05-06
Status: implemented baseline-pack infrastructure for repo-wide performance audit

Update 2026-05-07: added lightweight support and campaign authority gates for non-credential data/options/report/artifact/portfolio/dynamic/live-status/execution/campaign surfaces. The current strict inventory reports full command coverage with no missing automated authority gaps; actual provider stream start and live broker/API calls remain manual exclusions.

## Purpose

Performance is VectorGrid's objective differentiator. This folder defines a focused audit program for improving existing features by measuring, attributing, and optimizing the workflows users already rely on.

This is not a feature-expansion plan. It does not introduce new product surfaces, new strategy families, or broker/live behavior. It creates a performance authority map and implementation backlog for existing workflows.

## Guiding Principle

Optimize only after the timing boundary and bottleneck are known.

The audit must distinguish:

- cold load versus warm app path;
- kernel-only versus full workflow;
- backend compute versus Tauri command overhead;
- GPU residency versus hidden host materialization;
- artifact/report serialization versus strategy execution;
- frontend render latency versus backend latency;
- candidate-mode evidence versus baseline refresh authority.

## Documents

0. [Command Surface Coverage Map](00_command_surface_coverage_map.md)
1. [Performance Authority And Baseline Plan](01_performance_authority_baseline_plan.md)
2. [Workflow Benchmark Coverage Matrix](02_workflow_benchmark_coverage_matrix.md)
3. [CPU Backend Hot Path Audit](03_cpu_backend_hot_path_audit.md)
4. [GPU CUDA Residency And Kernel Audit](04_gpu_cuda_residency_kernel_audit.md)
5. [Frontend UI Performance Audit](05_frontend_ui_performance_audit.md)
6. [Data Artifact Report Performance Audit](06_data_artifact_report_performance_audit.md)
7. [Optimization Backlog And Execution Program](07_optimization_backlog_execution_program.md)
8. [ML Meta Portfolio And Execution Performance Audit](08_ml_meta_portfolio_execution_performance_audit.md)
9. [Runtime Shell Tooling And Support Surface Performance Audit](09_runtime_shell_tooling_performance_audit.md)
10. [Repo-Wide Performance File Checklist](10_repo_wide_performance_file_checklist.md)
11. [Second-Pass Performance Tracking Review](11_second_pass_tracking_review.md)
12. [Third-Pass Exhaustive Performance Review](12_third_pass_exhaustive_review.md)
13. [Follow-Up Performance Implementation Pass](13_followup_performance_implementation.md)

## Existing Authority Anchors

- `docs/verification_benchmark_release_authority.md`
- `docs/NUMERICAL_REPRODUCIBILITY.md`
- `README.md`
- `docs/REPO_MAP.md`
- `scripts/benchmark_lock.ps1`
- `scripts/performance_audit_case_registry.json`
- `scripts/run_performance_audit_baseline_pack.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_gpu_followup_perf_gate.ps1`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/run_support_perf_gate.ps1`
- `scripts/run_campaign_perf_gate.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_options_expression_nsys.ps1`
- `scripts/profile_options_expression_ncu.ps1`
- `scripts/profile_options_expression_compute_sanitizer.ps1`

## Non-Goals

- No new strategy zoo.
- No live/broker expansion.
- No public-release polish except where performance evidence depends on release packaging.
- No baseline refresh without explicit baseline-mode process.
- No CUDA kernel rewrite before Nsight evidence.
- No frontend dependency adoption before bundle/render profiling.
- No claim that a single benchmark number proves product performance.

## First Audit Deliverable

The first concrete deliverable should be a `Performance Authority Baseline Pack`:

- workflow case registry;
- cold/warm timing definitions;
- current gate coverage;
- missing benchmark coverage;
- top known bottleneck hypotheses;
- profiling commands to run;
- output artifact locations under `output/performance_audit/`;
- ranked optimization queue.

## Implemented Tooling

The first baseline-pack implementation is now available:

- `scripts/performance_audit_case_registry.json`
  - machine-readable registry for performance audit cases;
  - covers all current Tauri handlers from `src-tauri/src/lib.rs`;
  - covers all command modules from `src-tauri/src/commands.rs`;
  - records claim class, timing boundary, automation mode, fixture class, output authority, baseline path, and gate ownership.
- `scripts/run_performance_audit_baseline_pack.ps1`
  - default `inventory` mode writes an audit pack without running benchmarks;
  - extracts live command-handler and command-module coverage from source;
  - fingerprints environment, Git state, toolchain, GPU identity, active benchmark/profiler processes, script paths, baselines, and existing outputs;
  - writes `summary.json`, `summary.md`, `case_registry_snapshot.json`, `command_surface_coverage.json`, `case_health.json`, `workflow_matrix.json`, `missing_authority_actions.json`, `active_perf_processes.json`, and `gate_run_manifest.json`;
  - can optionally run existing gates serially under `scripts/benchmark_lock.ps1`;
  - times out individual gate subprocesses with `-GateTimeoutSec` so a hung child cannot hold the audit pack indefinitely.
- `scripts/run_support_perf_gate.ps1`
  - runs `ta_perf_authority_support` in release mode by default;
  - covers non-credential data/cache, Polygon live idle status/stop, options dataset import/manifest, report export, artifact library, portfolio feasibility, execution local-paper/status scaffolding, and dynamic/GPU-status tooling timing;
  - candidate mode defaults to `scripts/baselines/support_perf_baseline.json` and fails if the baseline is missing.
- `scripts/run_campaign_perf_gate.ps1`
  - wraps `ta_campaign_perf` with a deterministic synthetic two-dataset fixture;
  - provides a first-class campaign authority script for the audit registry;
  - candidate mode defaults to `scripts/baselines/ta_campaign_perf_baseline.json` and fails if the baseline is missing.

Default safe inventory command:

```powershell
powershell -File scripts/run_performance_audit_baseline_pack.ps1 -FailOnCoverageGap
```

Plan-only GPU validation inventory:

```powershell
powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode plan-only -IncludeGpuValidationHeavy
```

Core candidate gate pack:

```powershell
powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode candidate
```

Support candidate gate pack:

```powershell
powershell -File scripts/run_performance_audit_baseline_pack.ps1 -Mode candidate -IncludeSupport
```

## Full Coverage Rule

The audit is not complete unless both coverage layers are green:

- `00_command_surface_coverage_map.md` maps every command module in `src-tauri/src/commands.rs` to a primary audit document.
- `scripts/performance_audit_case_registry.json` maps every Tauri handler in `src-tauri/src/lib.rs` to at least one performance audit case.

Low-priority support commands still need a timing boundary and an explicit reason they are not part of the primary performance identity path.

## Global Rules

- Never run multiple benchmarks or profiling sessions at the same time.
- Use `scripts/benchmark_lock.ps1` when using repo benchmark gates.
- Check for existing benchmark/profiler processes before manual profiling.
- Candidate mode must not update baselines.
- Record hardware, OS, Rust toolchain, build profile, feature flags, route, dataset hash, and timing boundary with every result.
- CPU-only build health is mandatory.
- GPU results must report device identity and route/fallback metadata.
- Frontend timings must name whether Tauri/backend is mocked or real.
