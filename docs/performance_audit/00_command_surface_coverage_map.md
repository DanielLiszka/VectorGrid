# Command Surface Coverage Map

Date: 2026-05-06
Status: coverage map plus machine-readable registry

## Purpose

This document maps every current Tauri command module to the performance-audit document that owns it. The goal is explicit full coverage: every existing feature surface either has a primary performance audit lane or is classified as low-priority/system-support with a defined measurement boundary.

Source anchors:

- module coverage: `src-tauri/src/commands.rs`;
- command-handler coverage: `src-tauri/src/lib.rs`;
- machine-readable registry: `scripts/performance_audit_case_registry.json`;
- verification runner: `scripts/run_performance_audit_baseline_pack.ps1 -FailOnCoverageGap`.

## Coverage Map

| Command module / surface | Primary audit doc | Performance boundary | Priority |
|---|---|---|---|
| `commands_optimize.rs` | `03_cpu_backend_hot_path_audit.md`, `04_gpu_cuda_residency_kernel_audit.md` | warm app path, CPU/GPU gates, engine call | highest |
| `commands_validation.rs` | `03_cpu_backend_hot_path_audit.md`, `04_gpu_cuda_residency_kernel_audit.md` | validation module timing, GPU validation gate | highest |
| `commands_analysis.rs` | `03_cpu_backend_hot_path_audit.md`, `04_gpu_cuda_residency_kernel_audit.md`, `05_frontend_ui_performance_audit.md` | candidate drilldown/local stability/follow-up latency | high |
| `commands_campaign.rs` | `02_workflow_benchmark_coverage_matrix.md`, `03_cpu_backend_hot_path_audit.md` | campaign throughput, preload, skip-existing, validation follow-up | high |
| `commands_options_expression.rs` | `03_cpu_backend_hot_path_audit.md`, `04_gpu_cuda_residency_kernel_audit.md`, `06_data_artifact_report_performance_audit.md` | options run, validation replay, event-path inspection, save/export | high |
| `commands_options_data.rs` | `06_data_artifact_report_performance_audit.md` | options dataset import/download/load/list | high |
| `commands_data.rs` | `06_data_artifact_report_performance_audit.md`, `09_runtime_shell_tooling_performance_audit.md` | CSV/provider/cache/resample/live-stream status | high |
| `commands_artifacts.rs` / `run_artifacts.rs` | `06_data_artifact_report_performance_audit.md` | save/load/list/hydration/repair/catalog scan | high |
| `commands_reports.rs` / `commands_reports_api.rs` | `06_data_artifact_report_performance_audit.md` | report model parse, XLSX build, CSV shortlist, save/open/reveal | medium-high |
| `commands_meta.rs` | `08_ml_meta_portfolio_execution_performance_audit.md` | meta selection, Stage 4/5, ML confidence training/eval/apply | medium-high |
| `commands_ml_signal.rs` | `08_ml_meta_portfolio_execution_performance_audit.md` | ML Signal dataset export, score cache, validation, policy sweep | medium-high |
| `commands_portfolio.rs` | `08_ml_meta_portfolio_execution_performance_audit.md` | portfolio feasibility analysis | medium |
| `commands_execution.rs` | `08_ml_meta_portfolio_execution_performance_audit.md` | local/paper/live scaffolding latency, snapshots, runner overhead | medium |
| `commands_dynamic.rs` | `09_runtime_shell_tooling_performance_audit.md` | strategy IR preview/format/constant-bit tooling | low-medium |
| `commands_simple_summary.rs` | `06_data_artifact_report_performance_audit.md`, `09_runtime_shell_tooling_performance_audit.md` | Markdown summary generation/save | low-medium |
| `commands_system.rs` | `09_runtime_shell_tooling_performance_audit.md` | GPU status, window effect, app exit | low |
| `metric_surface.rs` | `03_cpu_backend_hot_path_audit.md`, `05_frontend_ui_performance_audit.md` | metric table/surface preparation and render | medium |
| `options_parent_adapters.rs` | `03_cpu_backend_hot_path_audit.md`, `06_data_artifact_report_performance_audit.md` | parent artifact replay/adaptation overhead | medium |
| `validation_common.rs` | `03_cpu_backend_hot_path_audit.md` | cross-ticker candidate key normalization and gate checks | high |

## Frontend Coverage Map

| Frontend surface | Primary audit doc | Priority |
|---|---|---|
| Workbench shell, workflow state, startup | `05_frontend_ui_performance_audit.md`, `09_runtime_shell_tooling_performance_audit.md` | medium-high |
| Data workspace | `06_data_artifact_report_performance_audit.md`, `05_frontend_ui_performance_audit.md` | high |
| Optimize/top/heatmap workspaces | `03_cpu_backend_hot_path_audit.md`, `04_gpu_cuda_residency_kernel_audit.md`, `05_frontend_ui_performance_audit.md` | highest |
| Validation workspace | `03_cpu_backend_hot_path_audit.md`, `04_gpu_cuda_residency_kernel_audit.md`, `05_frontend_ui_performance_audit.md` | highest |
| Candidate detail/drilldown | `03_cpu_backend_hot_path_audit.md`, `04_gpu_cuda_residency_kernel_audit.md`, `05_frontend_ui_performance_audit.md` | high |
| Options expression panel | `03_cpu_backend_hot_path_audit.md`, `06_data_artifact_report_performance_audit.md`, `05_frontend_ui_performance_audit.md` | high |
| History/library | `06_data_artifact_report_performance_audit.md`, `05_frontend_ui_performance_audit.md` | high |
| Meta/campaign workspace | `08_ml_meta_portfolio_execution_performance_audit.md`, `05_frontend_ui_performance_audit.md` | medium-high |
| ML Signal shell | `08_ml_meta_portfolio_execution_performance_audit.md`, `05_frontend_ui_performance_audit.md` | medium-high |
| Portfolio workspace | `08_ml_meta_portfolio_execution_performance_audit.md`, `05_frontend_ui_performance_audit.md` | medium |
| Live execution workspace | `08_ml_meta_portfolio_execution_performance_audit.md`, `05_frontend_ui_performance_audit.md` | medium |
| Settings, command palette, diagnostics | `09_runtime_shell_tooling_performance_audit.md`, `05_frontend_ui_performance_audit.md` | low-medium |

## Remaining Weak/Partial Performance Coverage

Strict command-handler inventory is covered by `scripts/performance_audit_case_registry.json`.
The remaining gaps are workflow-depth gaps, fixture gaps, or manual live/provider exclusions:

1. ML/meta and ML Signal timing cases still need stronger reusable timing artifacts.
2. Options validation/replay/event-path timing is fixture-dependent and remains weak until reusable evidence is recorded.
3. Frontend real-backend latency cases are still weaker than mocked browser workflow behavior.
4. Runtime startup and shell mount timing still needs a dedicated timing boundary.
5. Credentialed provider/live-broker checks remain manual unless mocked fixtures are introduced.

## Definition Of Full Coverage

Full coverage means:

- every command module has a primary audit doc;
- every Tauri handler has at least one registry case in `scripts/performance_audit_case_registry.json`;
- every user-facing workflow has at least one proposed timing case;
- every high-priority workflow has a primary authority or a documented missing-authority action;
- every performance result names a timing boundary;
- every benchmark/profiling run is serialized;
- CPU-only support remains mandatory.
