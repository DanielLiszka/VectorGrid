# Master Coverage Matrix

This matrix is the coverage ledger for the repo-wide deep dive. It intentionally uses broad feature surfaces rather than individual files because the repo is cross-layer: most real features include Rust app-core, Tauri commands, Vue services, stores, components, artifacts, and tests.

## Coverage Legend

| Status | Meaning |
|---|---|
| Audited | Surface was inspected and has a plan in this folder. |
| Low-risk ready | Contains additive improvements that can be implemented immediately. |
| Needs design | Contains medium/high-risk work requiring a focused plan or user decision. |
| Gate gap | Existing code may be solid, but verification/release evidence is incomplete. |

## Feature Surface Matrix

| Surface | Primary files | Current posture | Improvement class | Plan |
|---|---|---|---|---|
| CSV/data cache/timeframe lifecycle | `src-tauri/src/commands_data.rs`, `src-tauri/src/state.rs`, `src-tauri/src/data_cache.rs`, `web/src/services/dataService.ts`, `web/src/stores/dataStore.ts` | Mature, with binary cache and derived dataset invalidation | Low-risk ready, gate gap | `01_data_polygon_lifecycle.md` |
| Polygon historical/live | `src-tauri/src/polygon.rs`, `src-tauri/src/commands_data.rs`, `web/src/components/DataWorkspacePanel.vue` | Strong edge-case repair and credential handling; options lane remains narrower | Low-risk ready, needs design | `01_data_polygon_lifecycle.md` |
| Optimization request/runtime | `web/src/services/optimizeRequestBuilder.ts`, `src-tauri/src/commands_optimize.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/composite_mo.rs` | Broad feature set, several strategy-specific routes | Low-risk ready, needs design | `02_optimization_runtime_and_strategy_routes.md` |
| Strategy capability/routing | `web/src/utils/strategyOptimizationCapabilities.ts`, `web/src/strategies/compositeProfileGpuPolicy.ts`, `crates/app_core/src/composite_profile_route.rs` | Product policy exists but can drift across frontend/backend | Low-risk ready | `02_optimization_runtime_and_strategy_routes.md` |
| Validation pipeline | `web/src/services/validationRequestBuilder.ts`, `web/src/services/validationService.ts`, `src-tauri/src/commands_validation.rs`, `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/validation_pipeline.rs` | Sophisticated validation stack with GPU route metadata | Low-risk ready, needs design | `03_validation_gpu_and_evidence_pipeline.md` |
| GPU validation/residency | `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/composite_validate.rs`, `scripts/run_gpu_validation_perf_gate.ps1` | Good metadata exists; user-facing residency could be clearer | Low-risk ready | `03_validation_gpu_and_evidence_pipeline.md` |
| Simple Mode public workflow | `web/src/components/GuidedWorkspacePanel.vue`, `web/src/utils/simpleWorkflowModel.ts`, `web/src/composables/useGuidedWorkflowReadiness.ts` | Public-facing path is close but still needs stronger end-to-end proof | Low-risk ready, gate gap | `04_frontend_simple_mode_public_workflow.md` |
| Advanced workbench/charts/tables | `web/src/components/workbench/*`, `web/src/components/*WorkspacePanel.vue`, `web/src/visualizations/*` | Rich surface with tests and virtualized models | Low-risk ready, gate gap | `05_frontend_advanced_workbench_and_visualizations.md` |
| Artifacts/history/replay | `src-tauri/src/run_artifacts.rs`, `src-tauri/src/commands_artifacts.rs`, `web/src/services/historyService.ts`, `web/src/services/workflowArtifactsSync.ts` | Strong artifact identity work; needs health/recovery dashboard | Low-risk ready | `06_artifacts_history_reports_and_exports.md` |
| Reports/exports | `src-tauri/src/commands_reports*.rs`, `web/src/services/validationService.ts` | XLSX/CSV/JSON exports exist; parity authority should be tighter | Low-risk ready, gate gap | `06_artifacts_history_reports_and_exports.md` |
| Meta selection | `crates/app_core/src/meta_select.rs`, `src-tauri/src/commands_meta.rs`, `web/src/services/metaRequestBuilder.ts` | Mature Stage 3 style workflow with holdout and cohort metadata | Low-risk ready | `07_meta_stage5_ml_confidence.md` |
| Stage 5 ML confidence | `crates/app_core/src/ml_confidence.rs`, `src-tauri/src/commands_meta.rs`, `web/src/services/metaStage4RequestBuilder.ts`, `web/src/services/metaService.ts` | Functional with lineage metadata; release gate gap remains | Low-risk ready, needs design | `07_meta_stage5_ml_confidence.md` |
| ML Signal | `crates/app_core/src/ml_signal.rs`, `src-tauri/src/commands_ml_signal.rs`, `web/src/services/mlSignalService.ts`, `web/src/components/MlSignalWorkspacePanel.vue` | Strong workflow and recent hardening; policy sweep reporting can improve | Low-risk ready, needs design | `08_ml_signal_workflow.md` |
| Options expression | `src-tauri/src/commands_options_expression.rs`, `src-tauri/src/commands_options_data.rs`, `crates/app_core/src/options_expression.rs`, `web/src/components/OptionsExpressionPanel.vue` | Shipped research lane with provenance and validation replay | Low-risk ready, needs design | `09_options_expression_workflow.md` |
| Portfolio feasibility | `crates/app_core/src/portfolio_feasibility.rs`, `src-tauri/src/commands_portfolio.rs`, `web/src/services/portfolioService.ts` | Useful but narrow | Needs design | `10_portfolio_and_live_execution.md` |
| Local paper/live execution | `src-tauri/src/commands_execution.rs`, `src-tauri/src/broker_paper.rs`, `src-tauri/src/execution_policy.rs`, `web/src/services/executionService.ts` | Safety-focused local paper path exists; public scope should remain Advanced | Low-risk ready, needs design | `10_portfolio_and_live_execution.md` |
| CUDA/PTX/runtime | `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/strategy_gpu_runtime.rs`, `crates/app_core/src/kernels/*`, `scripts/run_gpu_*` | Strong Double MA GPU path; advanced GPU research lanes exist | Low-risk ready, research needed | `11_cuda_gpu_runtime_and_performance.md` |
| Settings/local state/privacy | `src-tauri/src/settings.rs`, `src-tauri/src/redaction.rs`, `web/src/services/settingsService.ts`, `web/src/stores/settingsStore.ts` | Solid basics; needs user-facing data inventory | Low-risk ready | `12_settings_privacy_security_and_local_state.md` |
| Tauri command boundary/generated bindings | `src-tauri/src/command_bindings.rs`, `src-tauri/src/commands_*.rs`, `web/src/tauri/generatedBindings.ts`, `src-tauri/tests/specta_bindings_export.rs` | Specta export exists but covers only a pilot subset | Low-risk ready, gate gap | `12_settings_privacy_security_and_local_state.md`, `14_verification_ci_benchmarking_and_qa.md` |
| Secrets/credential migration | `src-tauri/src/polygon.rs`, `src-tauri/src/commands_execution.rs`, `src-tauri/src/redaction.rs`, `web/src/utils/storage.ts`, `web/src/utils/core/storageKeys.ts` | Keyring-first provider storage exists; legacy browser aliases need purge checks | Low-risk ready | `12_settings_privacy_security_and_local_state.md` |
| Native/model bundle supply chain | `crates/app_core/src/ml_signal.rs`, `crates/app_core/src/kernels/*`, `scripts/generate_release_manifest.ps1` | Native/PTX assets are present in specialized lanes; release manifest authority should be stricter | Low-risk ready, needs design | `11_cuda_gpu_runtime_and_performance.md`, `13_release_packaging_and_public_docs.md` |
| Release/public docs | `README.md`, `docs/PUBLIC_RELEASE.md`, `docs/RELEASE.md`, `scripts/run_public_release_gate.ps1`, `src-tauri/tauri.conf.json` | Release gate exists; public support/docs need alignment with desktop release goal | Low-risk ready, external dependency | `13_release_packaging_and_public_docs.md` |
| Verification/QA/benchmarks | `scripts/run_repo_quality_gate.ps1`, `scripts/run_frontend_e2e_gate.ps1`, `scripts/run_*_gate.ps1`, `web/e2e`, `crates/app_core/benches` | Many gates exist; coverage dashboard and CI activation remain gaps | Low-risk ready, gate gap | `14_verification_ci_benchmarking_and_qa.md` |

## Cross-Cutting Themes

1. **Evidence clarity over algorithm changes.** Many useful improvements are metadata/report/UX changes that do not alter math.
2. **Avoid route-policy drift.** Frontend capability text, request builders, and Rust route selection should share or verify one capability source.
3. **Make public Simple Mode demonstrable.** The path must work from a clean install with bundled sample data.
4. **Treat artifacts as product contracts.** Schema version, provenance, replay class, and repair affordances should be visible.
5. **Separate research lanes from release lanes.** Options, ML, tensor-core/RT-core, and advanced GPU work should carry explicit evidence labels.
6. **Prefer targeted gates over broad hope.** Every implementation plan names a direct test lane plus the broad repo gate.
