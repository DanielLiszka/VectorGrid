# Repo-Wide Deep Dive And Implementation Planning Package

Date: 2026-04-30

This folder is the working package for a comprehensive repo-wide audit of VectorGrid. It is meant to be actionable rather than archival: each plan names the feature surface, the files to inspect first, the improvement candidates, the expected risk, and the verification lane.

## Scope

The audit covers the full current application surface:

- Tauri command boundary and desktop runtime.
- Rust app-core optimization, validation, CUDA, strategy, ML, options, and portfolio modules.
- Vue/Pinia frontend, Simple Mode, Advanced workbench, visualization, stores, services, and E2E QA.
- Data ingestion, Polygon historical/live data, CSV/cache/timeframe lifecycle, local retention, artifacts, reports, and exports.
- ML Signal, meta selection, Stage 5 ML confidence, model artifacts, and evidence semantics.
- Options expression workflow, portfolio feasibility, local paper/live execution, broker risk controls.
- Release packaging, signing/updater readiness, public docs, support/privacy/security docs, benchmarks, and gates.

## Folder Contents

- `00_master_coverage_matrix.md`: feature inventory, coverage status, and plan index.
- `01_data_polygon_lifecycle.md`: CSV, cache, timeframe, Polygon, and live market-data robustness.
- `02_optimization_runtime_and_strategy_routes.md`: optimization request shaping, strategy capability routing, and backend execution routes.
- `03_validation_gpu_and_evidence_pipeline.md`: validation semantics, GPU/CPU residency, cross-ticker, reports, and evidence.
- `04_frontend_simple_mode_public_workflow.md`: public-facing Simple Mode workflow and user guidance.
- `05_frontend_advanced_workbench_and_visualizations.md`: Advanced workbench, command surfaces, tables, charts, and frontend performance.
- `06_artifacts_history_reports_and_exports.md`: artifact lifecycle, saved history, replay, report parity, and exports.
- `07_meta_stage5_ml_confidence.md`: meta selection, Stage 5 ML confidence, policy evaluation, and model lifecycle.
- `08_ml_signal_workflow.md`: ML Signal dataset export, model bundle, score cache, validation, and policy sweep.
- `09_options_expression_workflow.md`: options expression optimization, validation replay, Polygon options data, and GPU promotion.
- `10_portfolio_and_live_execution.md`: portfolio feasibility, local paper broker, Alpaca/live execution, and risk controls.
- `11_cuda_gpu_runtime_and_performance.md`: CUDA/PTX/runtime, GPU validation, benchmarks, profiling, and research lanes.
- `12_settings_privacy_security_and_local_state.md`: settings, local storage, secrets, privacy, redaction, and trust boundaries.
- `13_release_packaging_and_public_docs.md`: release readiness, installer packaging, signing, updater, bundled resources, and public docs.
- `14_verification_ci_benchmarking_and_qa.md`: test gates, benchmark authority, E2E coverage, CI posture, and evidence dashboards.
- `15_cross_cutting_prioritized_backlog.md`: consolidated backlog with sequencing.

## Audit Method

1. Started from `docs/REPO_MAP.md` and AGENTS policy.
2. Inventoried command modules, frontend services/stores/components, scripts, docs, crates, benches, and tests.
3. Split the audit into bounded subsystem reviews with sub-agents.
4. Classified improvement candidates by expected regression risk:
   - Low: additive metadata, clearer status, stricter frontend preflight, extra tests, docs/gates.
   - Medium: response schema additions, request policy changes with compatibility handling, UI flow changes.
   - High: algorithmic behavior, route policy rewrites, default policy changes, provider/lifecycle changes.
5. Preferred implementation plans that preserve CPU-only builds, CUDA fallback, old artifacts, and current public command names.

## Second-Pass Validity Rules

The 2026-04-30 follow-up pass applied these rules to keep the guides implementation-valid:

- Existing file references should point at current repo paths.
- Proposed future files are labeled as new in the relevant guide.
- Frontend QA commands should use current `web/package.json` scripts unless a new script is explicitly proposed.
- Options workflow work should start from the current shipped files: `options_expression.rs`, `options_expression_gpu.rs`, `options_types.rs`, `commands_options_expression.rs`, and `commands_options_data.rs`.
- Portfolio work should harden the existing replay-evidence contract rather than invent a second replay-matrix concept.
- Public release/security work should never print discovered secrets and should include ignored repo-local files in scans.
- CUDA work should preserve CPU-only builds and keep strict GPU residency policy opt-in.

## Execution Rule

Do not attempt this as one large change. Use the backlog in `15_cross_cutting_prioritized_backlog.md` to run small implementation batches:

1. Pick one feature plan.
2. Implement only the low-risk items first.
3. Add tests close to the changed surface.
4. Run the feature gate plus `scripts/run_repo_quality_gate.ps1`.
5. Update the plan with evidence and deferred items.
