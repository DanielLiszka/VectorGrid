# Repo Improvement Meta Plan

Last updated: 2026-04-09

## Purpose

This is the master meta-planning document for the non-options repo-wide improvement-document program.

Its job is to plan the creation of a large Markdown document system that covers every current, non-options feature, workflow, subsystem, quality surface, and operational aspect already present in this repository.

This is not a single implementation plan.

It is the program plan for generating the implementation-planning documents.

## Active Scope

This program covers the current repo/project except anything options-related.

Included scope:

- core shell and desktop UX
- data importation, storage, repair, restore, and provenance
- optimization workflows and backend route clarity
- result display adequacy and analyst usability
- validation, reports, XLSX, CSV, and evidence quality
- history, artifacts, replay, and report reopening
- strategy families and backend engine surfaces
- ML Signal, meta selection, Stage 4/Stage 5, and AI/ML meta-analysis surfaces
- live execution, runtime state, broker/runtime safety, and system surfaces
- tests, perf gates, profiling, parity, benchmark posture, and release/build/docs operations
- public-repo community health, static-analysis quality gates, and release-operations expectations

Excluded scope:

- `options_research/`
- options-specific frontend panels, stores, services, and tests
- options-specific Tauri commands
- options-specific backend evaluation, GPU paths, data stores, and reports
- options-specific artifact/replay/report flows

Shared-code rule:

- shared modules remain in scope only for their non-options responsibilities
- no implementation doc in this program should plan options-first work or drift into options cleanup

## End Goal

The intended end state is:

- one discrete Markdown implementation or inventory document for each meaningful existing non-options feature/aspect
- short, scoped documents instead of oversized catch-all ledgers
- a maintainable document tree inside `REPO_Improvement/`
- clear routing from high-level docs to narrower subsystem docs
- a standing improvement program that does not depend on rediscovering the repo every session

## Improvement Charter

This document program is improvement-focused, not feature-expansion-focused.

Every detailed document created under this program must default to:

- additive improvements only
- objectively useful improvements only
- no regressions
- no functionality removal
- no silent narrowing of current capability
- explicit preservation of CPU fallback and existing safe routes where applicable
- explicit preservation of current user workflows unless a change is clearly superior and backward-safe

If a document uncovers a real new-feature concept, that concept must be labeled separately as:

- `out_of_improvement_scope`
- or `future_feature_concept`

It must not be silently absorbed into the main improvement backlog.

## Mandatory Pre-Implementation Research Gate

Before any single-item implementation Markdown document is created from this meta plan, there must be extensive research first.

Required sequence:

1. repo grounding using `REPO_MAP.md`, the relevant routing files, and the current code
2. bounded sub-agent exploration when parallel context gathering is helpful
3. extensive web search and external research on the relevant topic before drafting the implementation plan
4. source triage that prioritizes official documentation, primary sources, standards, authoritative framework guidance, and high-signal references
5. comparative review where relevant:
   - UI/UX references
   - accessibility guidance
   - report/XLSX examples
   - benchmark methodology references
   - profiling/tooling guidance
   - ML/validation literature or methodology notes
6. synthesis of repo facts plus external research into planning assumptions
7. only then creation of the detailed implementation document

No implementation doc should be created directly from a meta item without that research gate.

## Required Quality Bar For Every Detailed Implementation Doc

Every detailed implementation document should be fully detailed and comprehensive.

Default required sections:

1. purpose
2. scope boundaries
3. current user-facing behavior
4. current code surface
5. current verification surface
6. current strengths already present
7. current weaknesses and failure modes
8. regressions to avoid
9. external research summary
10. proposed improvement approach
11. step-by-step checklist
12. verification plan
13. rollout / fallback / compatibility notes
14. related docs

If a topic is complex enough to need more sections, expand it rather than compressing it.

## Sub-Agent Usage

Sub-agents should be used whenever they materially improve planning or implementation quality.

Preferred uses:

- parallel repo-surface mapping
- frontend/backend split analysis
- report/XLSX quality review
- verification/perf surface review
- external-research assistance
- dependency/risk cross-checking before a detailed plan is finalized

Sub-agents are helpful, not decorative. Use them where they reduce blind spots or preserve context.

## Canonical Inputs

The main inputs for this document program are:

- [REPO_MAP.md](../REPO_MAP.md)
- [IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md](./IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md)
- [IMPROVEMENT_MASTER_PLAN_2026-04-08.md](./IMPROVEMENT_MASTER_PLAN_2026-04-08.md)

Interpretation:

- `REPO_MAP.md` is the routing anchor
- `IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md` is the coarse inventory
- `IMPROVEMENT_MASTER_PLAN_2026-04-08.md` is the coarse improvement backlog
- if those seed docs mention options surfaces, those mentions are out of scope for this non-options program

## Why This Needs Many Documents

The repo is already large enough that:

- broad ledgers become too dense
- frontend, backend, runtime, verification, reporting, and ML/meta concerns are easy to mix incorrectly
- shell/UI concerns and engine/report concerns move at different speeds
- small but important quality surfaces such as XLSX clarity, result visibility, stale-dataset cues, and report reopening are easy to miss in larger plans

A many-document structure is justified because it provides:

- tighter scope
- easier maintenance
- easier continuation across sessions
- better verification ownership
- easier archival when a document is superseded
- clearer separation between repo facts, external research, and implementation intent

## Windows Path-Length Constraint

This repo already encountered Windows Git long-path issues.

So the document program must keep paths short.

Required rules:

- use short top-level folder names
- use short file slugs
- avoid deeply nested descriptive directories
- avoid repeating long product prefixes in every filename

## Naming Convention

Planned file pattern:

- `NN_slug_DATE.md`

Examples:

- `01_shell_app_root_2026-04-08.md`
- `03_csv_load_contract_2026-04-08.md`
- `13_xlsx_dashboard_quality_2026-04-08.md`

For future planned docs not yet created, this meta doc uses:

- `NN_slug_DATE.md`

where `DATE` means the actual creation/update date used when that file is created.

## Status Model

Every planned document should eventually have one of these statuses:

- `planned`
- `drafted`
- `active`
- `partially_executed`
- `completed`
- `archived`

## Folder Structure

Use short folder names under `REPO_Improvement/`:

- `00_meta/`
- `01_shell/`
- `02_data/`
- `03_opt/`
- `04_results/`
- `05_validation/`
- `06_artifacts/`
- `07_strats/`
- `08_ml/`
- `09_live/`
- `10_tests_perf/`
- `11_docs_ops/`

## Catalog Summary

The active `v7` target is `356` planned docs.

| Folder | Planned |
| --- | ---: |
| `00_meta` | `13` |
| `01_shell` | `35` |
| `02_data` | `34` |
| `03_opt` | `33` |
| `04_results` | `20` |
| `05_validation` | `27` |
| `06_artifacts` | `32` |
| `07_strats` | `29` |
| `08_ml` | `46` |
| `09_live` | `14` |
| `10_tests_perf` | `57` |
| `11_docs_ops` | `16` |
| **Total** | **`356`** |

## Planned Document Catalog

### 00_meta

Purpose:

- hold rules, templates, research gates, trackers, and governance for the document program itself

Planned docs:

- `00_meta/00_program_index_DATE.md`
- `00_meta/01_doc_creation_tracker_DATE.md`
- `00_meta/02_pre_impl_research_gate_DATE.md`
- `00_meta/03_subagent_usage_policy_DATE.md`
- `00_meta/04_doc_template_DATE.md`
- `00_meta/05_status_legend_DATE.md`
- `00_meta/06_folder_map_DATE.md`
- `00_meta/07_dependency_map_DATE.md`
- `00_meta/08_priority_rules_DATE.md`
- `00_meta/09_regression_safety_rules_DATE.md`
- `00_meta/10_verification_expectations_DATE.md`
- `00_meta/11_archive_policy_DATE.md`
- `00_meta/12_program_change_log_DATE.md`

Folder count:

- `13`

### 01_shell

Purpose:

- cover app-shell composition, desktop chrome, runtime UX, settings, state truthfulness, and shell-level accessibility/copy quality

Planned docs:

- `01_shell/01_shell_app_root_DATE.md`
- `01_shell/02_shell_workspace_grid_DATE.md`
- `01_shell/03_shell_titlebar_DATE.md`
- `01_shell/04_shell_command_palette_DATE.md`
- `01_shell/05_shell_task_center_DATE.md`
- `01_shell/06_shell_run_footer_DATE.md`
- `01_shell/07_shell_runtime_boot_DATE.md`
- `01_shell/08_shell_docks_shortcuts_DATE.md`
- `01_shell/09_settings_appearance_layout_DATE.md`
- `01_shell/10_settings_perspectives_DATE.md`
- `01_shell/11_settings_storage_shortcuts_DATE.md`
- `01_shell/12_settings_runtime_diagnostics_DATE.md`
- `01_shell/13_settings_desktop_notifications_DATE.md`
- `01_shell/14_state_workflow_tab_model_DATE.md`
- `01_shell/15_state_artifacts_commands_DATE.md`
- `01_shell/16_ux_shell_mount_readiness_DATE.md`
- `01_shell/17_ux_table_virtualization_DATE.md`
- `01_shell/18_ux_a11y_copy_regression_DATE.md`
- `01_shell/19_startup_overlay_disposition_DATE.md`
- `01_shell/20_shell_shared_dialogs_DATE.md`
- `01_shell/21_shell_context_menus_help_DATE.md`
- `01_shell/22_shell_debug_workspace_DATE.md`
- `01_shell/23_shell_visual_iconography_DATE.md`
- `01_shell/24_shell_desktop_productization_DATE.md`
- `01_shell/25_command_palette_store_DATE.md`
- `01_shell/26_runtime_task_store_DATE.md`
- `01_shell/27_settings_service_bootstrap_DATE.md`
- `01_shell/28_runtime_store_status_DATE.md`
- `01_shell/29_storage_schema_local_state_DATE.md`
- `01_shell/30_status_presentation_format_DATE.md`
- `01_shell/31_desktop_notification_policy_DATE.md`
- `01_shell/32_shell_overlay_composables_DATE.md`
- `01_shell/33_shell_task_lifecycle_summary_DATE.md`
- `01_shell/34_frontend_tauri_bridge_DATE.md`
- `01_shell/35_runtime_banner_surface_DATE.md`

Folder count:

- `35`

### 02_data

Purpose:

- cover repo CSV import, data discovery, Polygon workflows, resampling, dataset identity, provenance, repair, invalidation, and data UX truthfulness

Planned docs:

- `02_data/01_repo_csv_inventory_DATE.md`
- `02_data/02_repo_csv_discovery_DATE.md`
- `02_data/03_csv_load_contract_DATE.md`
- `02_data/04_csv_header_compat_DATE.md`
- `02_data/05_csv_field_rules_DATE.md`
- `02_data/06_csv_timestamp_rules_DATE.md`
- `02_data/07_data_cache_sources_DATE.md`
- `02_data/08_repo_data_browser_ux_DATE.md`
- `02_data/09_startup_dataset_restore_DATE.md`
- `02_data/10_dataset_context_surface_DATE.md`
- `02_data/11_polygon_root_settings_DATE.md`
- `02_data/12_polygon_api_key_DATE.md`
- `02_data/13_polygon_request_contract_DATE.md`
- `02_data/14_polygon_chunking_limits_DATE.md`
- `02_data/15_polygon_manifest_schema_DATE.md`
- `02_data/16_polygon_dataset_listing_DATE.md`
- `02_data/17_polygon_download_load_DATE.md`
- `02_data/18_polygon_dataset_validate_DATE.md`
- `02_data/19_polygon_dataset_repair_DATE.md`
- `02_data/20_polygon_dataset_topoff_DATE.md`
- `02_data/21_polygon_selection_ux_DATE.md`
- `02_data/22_dataset_provenance_DATE.md`
- `02_data/23_resample_contract_DATE.md`
- `02_data/24_resample_rules_DATE.md`
- `02_data/25_resample_no_redownload_DATE.md`
- `02_data/26_derived_dataset_identity_DATE.md`
- `02_data/27_dataset_meta_schema_DATE.md`
- `02_data/28_dataset_fingerprint_DATE.md`
- `02_data/29_dataset_invalidation_DATE.md`
- `02_data/30_data_repair_replay_risk_DATE.md`
- `02_data/31_data_service_store_DATE.md`
- `02_data/32_workflow_bootstrap_data_DATE.md`
- `02_data/33_data_cache_state_backend_DATE.md`
- `02_data/34_dataset_memory_retention_DATE.md`

Folder count:

- `34`

### 03_opt

Purpose:

- cover non-options optimization UX, request shaping, backend command contracts, backend route truthfulness, and DoubleMa/composite optimization semantics

Planned docs:

- `03_opt/01_opt_panel_core_DATE.md`
- `03_opt/02_opt_strategy_backend_DATE.md`
- `03_opt/03_opt_ma_grid_editor_DATE.md`
- `03_opt/04_opt_dynamic_builder_DATE.md`
- `03_opt/05_opt_route_gpu_capability_DATE.md`
- `03_opt/06_opt_run_plan_readiness_DATE.md`
- `03_opt/07_strategy_costs_risk_DATE.md`
- `03_opt/08_strategy_family_params_DATE.md`
- `03_opt/09_advanced_search_objective_DATE.md`
- `03_opt/10_advanced_shortlist_export_DATE.md`
- `03_opt/11_opt_request_builder_DATE.md`
- `03_opt/12_opt_service_progress_DATE.md`
- `03_opt/13_strategy_caps_surface_DATE.md`
- `03_opt/14_opt_double_ma_command_DATE.md`
- `03_opt/15_opt_composite_cpu_command_DATE.md`
- `03_opt/16_opt_composite_mo_command_DATE.md`
- `03_opt/17_opt_cancel_progress_DATE.md`
- `03_opt/18_eng_dm_request_resolution_DATE.md`
- `03_opt/19_eng_dm_backend_routing_DATE.md`
- `03_opt/20_eng_dm_gpu_kernel_vram_DATE.md`
- `03_opt/21_eng_dm_vdps_fidelity_DATE.md`
- `03_opt/22_eng_dm_output_contract_DATE.md`
- `03_opt/23_eng_dynamic_indicators_DATE.md`
- `03_opt/24_eng_strategy_ir_plan_DATE.md`
- `03_opt/25_optimize_store_DATE.md`
- `03_opt/26_strategy_store_DATE.md`
- `03_opt/27_advanced_store_DATE.md`
- `03_opt/28_recommended_defaults_DATE.md`
- `03_opt/29_ma_catalog_registry_DATE.md`
- `03_opt/30_dynamic_indicator_catalog_DATE.md`
- `03_opt/31_gpu_speedups_policy_DATE.md`
- `03_opt/32_run_control_cancel_bridge_DATE.md`
- `03_opt/33_advanced_shortlist_diagnostics_DATE.md`

Folder count:

- `33`

### 04_results

Purpose:

- cover result-pane usefulness, ranking-table clarity, chart/readability quality, stale-data cues, and whether the app actually shows enough evidence to support decisions

Planned docs:

- `04_results/01_results_overview_DATE.md`
- `04_results/02_results_top_table_DATE.md`
- `04_results/03_results_top_compare_DATE.md`
- `04_results/04_results_pareto_DATE.md`
- `04_results/05_results_drilldown_DATE.md`
- `04_results/06_results_heatmap_DATE.md`
- `04_results/07_results_empty_states_DATE.md`
- `04_results/08_results_display_adequacy_DATE.md`
- `04_results/09_results_analysis_contract_DATE.md`
- `04_results/10_out_overview_decision_surface_DATE.md`
- `04_results/11_out_top_results_table_DATE.md`
- `04_results/12_out_pareto_heatmap_readability_DATE.md`
- `04_results/13_out_drilldown_stability_DATE.md`
- `04_results/14_evidence_gaps_missing_data_DATE.md`
- `04_results/15_dataset_stale_provenance_cues_DATE.md`
- `04_results/16_report_path_reopen_flow_DATE.md`
- `04_results/17_analysis_store_service_DATE.md`
- `04_results/18_context_docks_DATE.md`
- `04_results/19_chart_rendering_composables_DATE.md`
- `04_results/20_matrix_table_state_DATE.md`

Folder count:

- `20`

### 05_validation

Purpose:

- cover validation controls, validation output truthfulness, backend validation contracts, shortlist export, workbook quality, route/fallback notes, and analyst-grade evidence quality

Planned docs:

- `05_validation/01_validate_panel_core_DATE.md`
- `05_validation/02_validate_admission_cross_DATE.md`
- `05_validation/03_validate_results_table_DATE.md`
- `05_validation/04_validate_report_export_DATE.md`
- `05_validation/05_validate_artifact_persistence_DATE.md`
- `05_validation/06_val_dm_command_DATE.md`
- `05_validation/07_val_composite_command_DATE.md`
- `05_validation/08_val_cross_ticker_pricebars_DATE.md`
- `05_validation/09_eng_validation_verdicts_DATE.md`
- `05_validation/10_eng_validation_stats_core_DATE.md`
- `05_validation/11_xlsx_workbook_contract_DATE.md`
- `05_validation/12_xlsx_sheet_chart_contract_DATE.md`
- `05_validation/13_xlsx_dashboard_quality_DATE.md`
- `05_validation/14_xlsx_shortlist_cap_policy_DATE.md`
- `05_validation/15_xlsx_profile_family_coverage_DATE.md`
- `05_validation/16_xlsx_route_note_status_DATE.md`
- `05_validation/17_xlsx_open_reveal_flows_DATE.md`
- `05_validation/18_csv_shortlist_export_DATE.md`
- `05_validation/19_validation_request_builder_DATE.md`
- `05_validation/20_validation_service_DATE.md`
- `05_validation/21_validation_store_DATE.md`
- `05_validation/22_validation_context_docks_DATE.md`
- `05_validation/23_metric_surface_contract_DATE.md`
- `05_validation/24_validation_common_backend_DATE.md`
- `05_validation/25_validation_context_compat_DATE.md`
- `05_validation/26_cross_ticker_finalist_logic_DATE.md`
- `05_validation/27_validation_robustness_methods_DATE.md`

Folder count:

- `27`

### 06_artifacts

Purpose:

- cover run/meta artifacts, replay, saved-run usability, path persistence, report builders, report JSON/XLSX/CSV linkage, and artifact-library truthfulness

Planned docs:

- `06_artifacts/01_run_artifact_schema_DATE.md`
- `06_artifacts/02_artifact_dataset_meta_norm_DATE.md`
- `06_artifacts/03_artifact_library_index_DATE.md`
- `06_artifacts/04_artifact_hydration_contract_DATE.md`
- `06_artifacts/05_meta_artifact_replay_DATE.md`
- `06_artifacts/06_stage5_artifact_replay_DATE.md`
- `06_artifacts/07_policy_eval_artifact_replay_DATE.md`
- `06_artifacts/08_opt_metric_surface_artifact_DATE.md`
- `06_artifacts/09_workflow_artifact_store_DATE.md`
- `06_artifacts/10_history_search_filter_ux_DATE.md`
- `06_artifacts/11_art_fingerprint_provenance_DATE.md`
- `06_artifacts/12_art_hydration_detail_replay_DATE.md`
- `06_artifacts/13_art_retention_tags_compat_DATE.md`
- `06_artifacts/14_report_validation_dashboard_DATE.md`
- `06_artifacts/15_report_validation_candidate_sheets_DATE.md`
- `06_artifacts/16_report_route_fallback_DATE.md`
- `06_artifacts/17_report_meta_workbook_DATE.md`
- `06_artifacts/18_report_shortlist_csv_DATE.md`
- `06_artifacts/19_report_save_open_reveal_DATE.md`
- `06_artifacts/20_report_json_contract_DATE.md`
- `06_artifacts/21_report_artifact_path_persistence_DATE.md`
- `06_artifacts/22_meta_stage3_report_flow_DATE.md`
- `06_artifacts/23_mlsignal_report_path_semantics_DATE.md`
- `06_artifacts/24_history_library_core_DATE.md`
- `06_artifacts/25_history_detail_actions_DATE.md`
- `06_artifacts/26_history_service_store_DATE.md`
- `06_artifacts/27_command_action_store_DATE.md`
- `06_artifacts/28_workflow_artifacts_sync_DATE.md`
- `06_artifacts/29_artifact_identity_workflow_model_DATE.md`
- `06_artifacts/30_core_dataset_meta_paths_DATE.md`
- `06_artifacts/31_batch_summary_artifacts_DATE.md`
- `06_artifacts/32_campaign_summary_checkpoint_DATE.md`

Folder count:

- `32`

### 07_strats

Purpose:

- cover strategy-family ownership, composite routing policy, research/fidelity binaries, dynamic IR/campaign surfaces, and strategy-specific quirks that should not be hidden inside giant generic plans

Planned docs:

- `07_strats/01_strat_double_ma_family_DATE.md`
- `07_strats/02_strat_dynamic_double_ma_DATE.md`
- `07_strats/03_strat_composite_trend_DATE.md`
- `07_strats/04_strat_breakout_momentum_DATE.md`
- `07_strats/05_strat_regime_hybrid_DATE.md`
- `07_strats/06_strat_composite_cpu_DATE.md`
- `07_strats/07_strat_composite_mo_DATE.md`
- `07_strats/08_strat_ml_signal_DATE.md`
- `07_strats/09_strat_profile_cluster_a_DATE.md`
- `07_strats/10_strat_profile_cluster_b_DATE.md`
- `07_strats/11_eng_composite_route_policy_DATE.md`
- `07_strats/12_eng_composite_validation_DATE.md`
- `07_strats/13_eng_composite_mo_prescan_search_DATE.md`
- `07_strats/14_analysis_drilldown_stability_DATE.md`
- `07_strats/15_dynamic_ir_campaign_surface_DATE.md`
- `07_strats/16_fidelity_audit_harness_DATE.md`
- `07_strats/17_research_diagnostic_bins_DATE.md`
- `07_strats/18_eng_csv_cuda_progress_DATE.md`
- `07_strats/19_data_discovery_catalogs_DATE.md`
- `07_strats/20_strat_surface_matrix_DATE.md`
- `07_strats/21_optimizer_grid_stream_DATE.md`
- `07_strats/22_optimizer_mo_core_DATE.md`
- `07_strats/23_strategy_plan_presets_DATE.md`
- `07_strats/24_core_timeframe_matrix_stats_DATE.md`
- `07_strats/25_strategy_runtime_gpu_stack_DATE.md`
- `07_strats/26_prescan_iterative_sim_DATE.md`
- `07_strats/27_alloc_policy_safety_DATE.md`
- `07_strats/28_headless_cli_surface_DATE.md`
- `07_strats/29_trade_records_contract_DATE.md`

Folder count:

- `29`

### 08_ml

Purpose:

- cover ML Signal, meta selection, meta artifacts, AI/ML meta-analysis surfaces, Stage 4 / Stage 5 confidence, and the current ML evidence/review flows

Planned docs:

- `08_ml/01_ml_shell_handoff_DATE.md`
- `08_ml/02_ml_config_paths_DATE.md`
- `08_ml/03_ml_score_cache_bundles_DATE.md`
- `08_ml/04_ml_validation_surface_DATE.md`
- `08_ml/05_ml_policy_sweep_DATE.md`
- `08_ml/06_meta_panel_ia_DATE.md`
- `08_ml/07_meta_selection_compare_DATE.md`
- `08_ml/08_meta_batch_generation_DATE.md`
- `08_ml/09_meta_saved_artifact_families_DATE.md`
- `08_ml/10_meta_ml_confidence_reports_DATE.md`
- `08_ml/11_mlsignal_dataset_export_DATE.md`
- `08_ml/12_mlsignal_feature_schema_DATE.md`
- `08_ml/13_mlsignal_split_plan_DATE.md`
- `08_ml/14_mlsignal_export_manifests_DATE.md`
- `08_ml/15_mlsignal_score_csv_ingest_DATE.md`
- `08_ml/16_mlsignal_score_cache_build_DATE.md`
- `08_ml/17_mlsignal_score_cache_validation_DATE.md`
- `08_ml/18_mlsignal_policy_params_DATE.md`
- `08_ml/19_mlsignal_validation_flow_DATE.md`
- `08_ml/20_mlsignal_model_bundle_inventory_DATE.md`
- `08_ml/21_mlsignal_artifact_attachment_DATE.md`
- `08_ml/22_meta_selection_request_contract_DATE.md`
- `08_ml/23_meta_grouping_clustering_DATE.md`
- `08_ml/24_meta_scoring_weights_DATE.md`
- `08_ml/25_meta_evidence_flags_DATE.md`
- `08_ml/26_meta_holdout_modes_DATE.md`
- `08_ml/27_meta_batch_registry_modes_DATE.md`
- `08_ml/28_meta_batch_resume_retry_DATE.md`
- `08_ml/29_ml_conf_scope_identity_DATE.md`
- `08_ml/30_ml_conf_training_rows_DATE.md`
- `08_ml/31_ml_conf_label_config_DATE.md`
- `08_ml/32_ml_conf_model_families_DATE.md`
- `08_ml/33_ml_conf_health_staleness_DATE.md`
- `08_ml/34_ml_conf_apply_contract_DATE.md`
- `08_ml/35_stage4_request_contract_DATE.md`
- `08_ml/36_stage4_selection_logic_DATE.md`
- `08_ml/37_stage5_execution_contract_DATE.md`
- `08_ml/38_ml_policy_eval_contract_DATE.md`
- `08_ml/39_stage4_stage5_saved_replay_DATE.md`
- `08_ml/40_meta_ml_status_surface_DATE.md`
- `08_ml/41_meta_request_builders_DATE.md`
- `08_ml/42_meta_scope_builders_DATE.md`
- `08_ml/43_meta_service_campaigns_DATE.md`
- `08_ml/44_mlsignal_frontend_service_DATE.md`
- `08_ml/45_meta_frontend_service_DATE.md`
- `08_ml/46_ml_meta_store_status_DATE.md`

Folder count:

- `46`

### 09_live

Purpose:

- cover live execution, runner/runtime control, broker-facing safety, live-bar ingestion, system/window surfaces, and file/path boundaries that impact live workflows

Planned docs:

- `09_live/01_execution_command_surface_DATE.md`
- `09_live/02_execution_workspace_ux_DATE.md`
- `09_live/03_execution_runner_lifecycle_DATE.md`
- `09_live/04_execution_order_position_review_DATE.md`
- `09_live/05_execution_safety_modes_DATE.md`
- `09_live/06_broker_connectivity_DATE.md`
- `09_live/07_live_candle_stream_data_DATE.md`
- `09_live/08_system_gpu_window_surface_DATE.md`
- `09_live/09_file_picker_path_boundary_DATE.md`
- `09_live/10_run_guards_cancel_tokens_DATE.md`
- `09_live/11_settings_secret_storage_DATE.md`
- `09_live/12_out_execution_monitoring_DATE.md`
- `09_live/13_execution_service_store_DATE.md`
- `09_live/14_runtime_state_bridge_DATE.md`

Folder count:

- `14`

### 10_tests_perf

Purpose:

- cover frontend tests, browser/desktop harnesses, backend/Tauri verification, report regressions, perf gates, benchmarks, profiling, and evidence hygiene

Planned docs:

- `10_tests_perf/01_fe_test_matrix_DATE.md`
- `10_tests_perf/02_fe_component_surface_tests_DATE.md`
- `10_tests_perf/03_fe_store_service_tests_DATE.md`
- `10_tests_perf/04_fe_tauri_bridge_tests_DATE.md`
- `10_tests_perf/05_fe_results_surface_adequacy_DATE.md`
- `10_tests_perf/06_fe_dense_table_virtualization_DATE.md`
- `10_tests_perf/07_fe_a11y_smoke_DATE.md`
- `10_tests_perf/08_fe_ui_baseline_capture_DATE.md`
- `10_tests_perf/09_browser_harness_contract_DATE.md`
- `10_tests_perf/10_browser_shell_ux_DATE.md`
- `10_tests_perf/11_browser_palette_overlays_DATE.md`
- `10_tests_perf/12_browser_support_mocks_DATE.md`
- `10_tests_perf/13_desktop_harness_contract_DATE.md`
- `10_tests_perf/14_desktop_shell_readiness_DATE.md`
- `10_tests_perf/15_desktop_native_titlebar_DATE.md`
- `10_tests_perf/16_desktop_task_center_nav_DATE.md`
- `10_tests_perf/17_desktop_settings_diag_DATE.md`
- `10_tests_perf/18_desktop_geometry_behavior_DATE.md`
- `10_tests_perf/19_desktop_artifact_capture_DATE.md`
- `10_tests_perf/20_tauri_runtime_wiring_DATE.md`
- `10_tests_perf/21_tauri_mock_runtime_smoke_DATE.md`
- `10_tests_perf/22_tauri_settings_persistence_DATE.md`
- `10_tests_perf/23_tauri_report_command_api_DATE.md`
- `10_tests_perf/24_tauri_run_artifact_paths_DATE.md`
- `10_tests_perf/25_headless_report_builders_DATE.md`
- `10_tests_perf/26_cpu_perf_gate_DATE.md`
- `10_tests_perf/27_gpu_perf_gate_DATE.md`
- `10_tests_perf/28_workflow_gate_DATE.md`
- `10_tests_perf/29_request_builder_bench_DATE.md`
- `10_tests_perf/30_workflow_benchmark_pack_DATE.md`
- `10_tests_perf/31_vectorbt_faceoff_DATE.md`
- `10_tests_perf/32_benchmark_claims_hygiene_DATE.md`
- `10_tests_perf/33_ncu_profile_playbook_DATE.md`
- `10_tests_perf/34_nsys_profile_playbook_DATE.md`
- `10_tests_perf/35_cupti_sass_probe_DATE.md`
- `10_tests_perf/36_gpu_route_matrix_profile_DATE.md`
- `10_tests_perf/37_optimization_memory_capture_DATE.md`
- `10_tests_perf/38_profile_runner_bins_DATE.md`
- `10_tests_perf/39_post_release_telemetry_review_DATE.md`
- `10_tests_perf/40_xlsx_regression_tooling_DATE.md`
- `10_tests_perf/41_gpu_alma_contract_gate_DATE.md`
- `10_tests_perf/42_tauri_command_parity_DATE.md`
- `10_tests_perf/43_output_parity_pack_DATE.md`
- `10_tests_perf/44_parity_pair_registry_DATE.md`
- `10_tests_perf/45_artifact_report_shape_parity_DATE.md`
- `10_tests_perf/46_legacy_name_compat_DATE.md`
- `10_tests_perf/47_tauri_config_sync_DATE.md`
- `10_tests_perf/48_generated_command_inventory_DATE.md`
- `10_tests_perf/49_python_faceoff_locking_DATE.md`
- `10_tests_perf/50_frontend_test_support_mocks_DATE.md`
- `10_tests_perf/51_vectorgrid_internal_benchmark_boundaries_DATE.md`
- `10_tests_perf/52_app_core_criterion_benches_DATE.md`
- `10_tests_perf/53_refactor_snapshot_parity_DATE.md`
- `10_tests_perf/54_campaign_preload_vram_perf_DATE.md`
- `10_tests_perf/55_headless_workflow_bins_DATE.md`
- `10_tests_perf/56_static_analysis_quality_gates_DATE.md`
- `10_tests_perf/57_manual_a11y_review_matrix_DATE.md`

Folder count:

- `57`

### 11_docs_ops

Purpose:

- cover build/release/docs operations, scope boundary hygiene, research-source policy, bundle/baseline handling, and program governance

Planned docs:

- `11_docs_ops/01_release_build_variants_DATE.md`
- `11_docs_ops/02_ptx_packaging_fallbacks_DATE.md`
- `11_docs_ops/03_frontend_build_handoff_DATE.md`
- `11_docs_ops/04_bundle_budget_analysis_DATE.md`
- `11_docs_ops/05_baseline_samples_manifest_DATE.md`
- `11_docs_ops/06_repo_map_readme_hygiene_DATE.md`
- `11_docs_ops/07_local_vs_ci_authority_DATE.md`
- `11_docs_ops/08_docs_archive_policy_DATE.md`
- `11_docs_ops/09_non_options_scope_boundary_DATE.md`
- `11_docs_ops/10_research_source_policy_DATE.md`
- `11_docs_ops/11_impl_doc_quality_bar_DATE.md`
- `11_docs_ops/12_program_governance_DATE.md`
- `11_docs_ops/13_migration_freeze_scope_DATE.md`
- `11_docs_ops/14_repo_surface_coverage_audit_DATE.md`
- `11_docs_ops/15_public_repo_community_health_DATE.md`
- `11_docs_ops/16_public_release_operations_DATE.md`

Folder count:

- `16`

## Creation Waves

### Wave 0

Purpose:

- keep the meta layer honest and usable

Current status:

- already started

Focus:

- meta rules
- indexes
- tracker upkeep
- scope-boundary cleanup

### Wave 1

Purpose:

- establish the foundation docs that everything else depends on

Queue:

1. `00_meta/02_pre_impl_research_gate_DATE.md`
2. `00_meta/09_regression_safety_rules_DATE.md`
3. `01_shell/07_shell_runtime_boot_DATE.md`
4. `02_data/03_csv_load_contract_DATE.md`
5. `02_data/17_polygon_download_load_DATE.md`
6. `03_opt/11_opt_request_builder_DATE.md`
7. `03_opt/19_eng_dm_backend_routing_DATE.md`
8. `04_results/08_results_display_adequacy_DATE.md`
9. `05_validation/11_xlsx_workbook_contract_DATE.md`
10. `05_validation/13_xlsx_dashboard_quality_DATE.md`
11. `06_artifacts/01_run_artifact_schema_DATE.md`
12. `06_artifacts/21_report_artifact_path_persistence_DATE.md`
13. `08_ml/19_mlsignal_validation_flow_DATE.md`
14. `08_ml/22_meta_selection_request_contract_DATE.md`
15. `09_live/01_execution_command_surface_DATE.md`
16. `10_tests_perf/01_fe_test_matrix_DATE.md`
17. `10_tests_perf/40_xlsx_regression_tooling_DATE.md`
18. `11_docs_ops/10_research_source_policy_DATE.md`

### Wave 2

Purpose:

- create the first broad surface-ownership docs after the foundation is stable

Focus:

- shell
- data UX
- optimize UX
- validation UX
- artifacts/history
- result surfaces

### Wave 3

Purpose:

- deepen backend, ML/meta, and live/runtime coverage

Focus:

- engine contracts
- strategy families
- ML Signal
- meta selection
- Stage 4 / Stage 5
- live execution

### Wave 4

Purpose:

- harden verification, perf posture, profiling, docs hygiene, and release evidence

Focus:

- tests
- harnesses
- perf gates
- benchmark posture
- profiling playbooks
- release/build/docs operations

## Program Rules

- keep counts honest when docs are merged, split, or retired
- do not create all `356` docs at once
- create foundation docs before deep leaf docs where dependencies are obvious
- if a topic is still too broad, split it rather than hiding complexity in one oversized file
- if a topic is too narrow to justify a whole file, merge it intentionally and update the tracker
- every new detailed doc must explicitly state regressions to avoid
- every new detailed doc must explicitly list verification expectations
- every new detailed doc must cite its external research inputs

## Current Recommendation

The next useful work is not more broad meta discussion.

The next useful work is:

1. keep this meta catalog as the authoritative non-options scope boundary
2. create the Wave 1 foundation docs
3. use the mandatory research gate before drafting each one
4. keep the tracker and counts synchronized as the program grows
