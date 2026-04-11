# Execution Ledger

Last updated: 2026-04-10

## Purpose

Track implementation progress at the whole-document level for the completed non-options `REPO_Improvement` plan set.

This ledger is intentionally coarse:

- one checklist item here represents one full implementation-plan document
- an unchecked item means that document is not yet fully implemented and verified
- a checked item means the whole document has been executed far enough to count as implemented and verified for its current scope
- the detailed, longer checklist remains inside the linked implementation-plan document itself

## How To Use

1. Choose one implementation-plan document.
2. Re-run the pre-implementation research gate for that specific item.
3. Execute the work in bounded slices.
4. Track detailed progress inside the document that owns the work.
5. Mark the corresponding checklist item here only when the full document-level scope is implemented and verified.

## Status Notes

- use this file for program-level completion tracking
- use each linked document for detailed step tracking, verification notes, and rollout notes
- if work is in progress but not complete, leave the checkbox unchecked and record current activity in the commit/PR notes or a nearby maintenance note instead of half-checking the item

## Current Totals

- planned implementation-plan documents tracked here: `356`
- current completed items: `341`
- current in-progress items: `0`

## 00 Meta

- section total: `13`

- [x] [00_meta/00_program_index_2026-04-09.md](../00_meta/00_program_index_2026-04-09.md)
- [x] [00_meta/01_doc_creation_tracker_2026-04-09.md](../00_meta/01_doc_creation_tracker_2026-04-09.md)
- [x] [00_meta/02_pre_impl_research_gate_2026-04-08.md](../00_meta/02_pre_impl_research_gate_2026-04-08.md)
- [x] [00_meta/03_subagent_usage_policy_2026-04-08.md](../00_meta/03_subagent_usage_policy_2026-04-08.md)
- [x] [00_meta/04_doc_template_2026-04-08.md](../00_meta/04_doc_template_2026-04-08.md)
- [x] [00_meta/05_status_legend_2026-04-08.md](../00_meta/05_status_legend_2026-04-08.md)
- [x] [00_meta/06_folder_map_2026-04-08.md](../00_meta/06_folder_map_2026-04-08.md)
- [x] [00_meta/07_dependency_map_2026-04-08.md](../00_meta/07_dependency_map_2026-04-08.md)
- [x] [00_meta/08_priority_rules_2026-04-08.md](../00_meta/08_priority_rules_2026-04-08.md)
- [x] [00_meta/09_regression_safety_rules_2026-04-08.md](../00_meta/09_regression_safety_rules_2026-04-08.md)
- [x] [00_meta/10_verification_expectations_2026-04-08.md](../00_meta/10_verification_expectations_2026-04-08.md)
- [x] [00_meta/11_archive_policy_2026-04-08.md](../00_meta/11_archive_policy_2026-04-08.md)
- [x] [00_meta/12_program_change_log_2026-04-08.md](../00_meta/12_program_change_log_2026-04-08.md)

## 01 Shell

- section total: `35`

- [x] [01_shell/01_shell_app_root_2026-04-08.md](../01_shell/01_shell_app_root_2026-04-08.md)
- [x] [01_shell/02_shell_workspace_grid_2026-04-08.md](../01_shell/02_shell_workspace_grid_2026-04-08.md)
- [x] [01_shell/03_shell_titlebar_2026-04-08.md](../01_shell/03_shell_titlebar_2026-04-08.md)
- [x] [01_shell/04_shell_command_palette_2026-04-09.md](../01_shell/04_shell_command_palette_2026-04-09.md)
- [x] [01_shell/05_shell_task_center_2026-04-08.md](../01_shell/05_shell_task_center_2026-04-08.md)
- [x] [01_shell/06_shell_run_footer_2026-04-08.md](../01_shell/06_shell_run_footer_2026-04-08.md)
- [x] [01_shell/07_shell_runtime_boot_2026-04-08.md](../01_shell/07_shell_runtime_boot_2026-04-08.md)
- [x] [01_shell/08_shell_docks_shortcuts_2026-04-08.md](../01_shell/08_shell_docks_shortcuts_2026-04-08.md)
- [x] [01_shell/09_settings_appearance_layout_2026-04-09.md](../01_shell/09_settings_appearance_layout_2026-04-09.md)
- [x] [01_shell/10_settings_perspectives_2026-04-09.md](../01_shell/10_settings_perspectives_2026-04-09.md)
- [x] [01_shell/11_settings_storage_shortcuts_2026-04-08.md](../01_shell/11_settings_storage_shortcuts_2026-04-08.md)
- [x] [01_shell/12_settings_runtime_diagnostics_2026-04-08.md](../01_shell/12_settings_runtime_diagnostics_2026-04-08.md)
- [x] [01_shell/13_settings_desktop_notifications_2026-04-08.md](../01_shell/13_settings_desktop_notifications_2026-04-08.md)
- [x] [01_shell/14_state_workflow_tab_model_2026-04-09.md](../01_shell/14_state_workflow_tab_model_2026-04-09.md)
- [x] [01_shell/15_state_artifacts_commands_2026-04-09.md](../01_shell/15_state_artifacts_commands_2026-04-09.md)
- [x] [01_shell/16_ux_shell_mount_readiness_2026-04-09.md](../01_shell/16_ux_shell_mount_readiness_2026-04-09.md)
- [x] [01_shell/17_ux_table_virtualization_2026-04-09.md](../01_shell/17_ux_table_virtualization_2026-04-09.md)
- [x] [01_shell/18_ux_a11y_copy_regression_2026-04-09.md](../01_shell/18_ux_a11y_copy_regression_2026-04-09.md)
- [x] [01_shell/19_startup_overlay_disposition_2026-04-09.md](../01_shell/19_startup_overlay_disposition_2026-04-09.md)
- [x] [01_shell/20_shell_shared_dialogs_2026-04-09.md](../01_shell/20_shell_shared_dialogs_2026-04-09.md)
- [x] [01_shell/21_shell_context_menus_help_2026-04-09.md](../01_shell/21_shell_context_menus_help_2026-04-09.md)
- [x] [01_shell/22_shell_debug_workspace_2026-04-09.md](../01_shell/22_shell_debug_workspace_2026-04-09.md)
- [x] [01_shell/23_shell_visual_iconography_2026-04-09.md](../01_shell/23_shell_visual_iconography_2026-04-09.md)
- [x] [01_shell/24_shell_desktop_productization_2026-04-09.md](../01_shell/24_shell_desktop_productization_2026-04-09.md)
- [x] [01_shell/25_command_palette_store_2026-04-08.md](../01_shell/25_command_palette_store_2026-04-08.md)
- [x] [01_shell/26_runtime_task_store_2026-04-08.md](../01_shell/26_runtime_task_store_2026-04-08.md)
- [x] [01_shell/27_settings_service_bootstrap_2026-04-08.md](../01_shell/27_settings_service_bootstrap_2026-04-08.md)
- [x] [01_shell/28_runtime_store_status_2026-04-08.md](../01_shell/28_runtime_store_status_2026-04-08.md)
- [x] [01_shell/29_storage_schema_local_state_2026-04-08.md](../01_shell/29_storage_schema_local_state_2026-04-08.md)
- [x] [01_shell/30_status_presentation_format_2026-04-08.md](../01_shell/30_status_presentation_format_2026-04-08.md)
- [x] [01_shell/31_desktop_notification_policy_2026-04-09.md](../01_shell/31_desktop_notification_policy_2026-04-09.md)
- [x] [01_shell/32_shell_overlay_composables_2026-04-09.md](../01_shell/32_shell_overlay_composables_2026-04-09.md)
- [x] [01_shell/33_shell_task_lifecycle_summary_2026-04-09.md](../01_shell/33_shell_task_lifecycle_summary_2026-04-09.md)
- [x] [01_shell/34_frontend_tauri_bridge_2026-04-08.md](../01_shell/34_frontend_tauri_bridge_2026-04-08.md)
- [x] [01_shell/35_runtime_banner_surface_2026-04-08.md](../01_shell/35_runtime_banner_surface_2026-04-08.md)

## 02 Data

- section total: `34`

- [x] [02_data/01_repo_csv_inventory_2026-04-09.md](../02_data/01_repo_csv_inventory_2026-04-09.md)
- [x] [02_data/02_repo_csv_discovery_2026-04-09.md](../02_data/02_repo_csv_discovery_2026-04-09.md)
- [x] [02_data/03_csv_load_contract_2026-04-08.md](../02_data/03_csv_load_contract_2026-04-08.md)
- [x] [02_data/04_csv_header_compat_2026-04-08.md](../02_data/04_csv_header_compat_2026-04-08.md)
- [x] [02_data/05_csv_field_rules_2026-04-09.md](../02_data/05_csv_field_rules_2026-04-09.md)
- [x] [02_data/06_csv_timestamp_rules_2026-04-09.md](../02_data/06_csv_timestamp_rules_2026-04-09.md)
- [x] [02_data/07_data_cache_sources_2026-04-08.md](../02_data/07_data_cache_sources_2026-04-08.md)
- [x] [02_data/08_repo_data_browser_ux_2026-04-09.md](../02_data/08_repo_data_browser_ux_2026-04-09.md)
- [x] [02_data/09_startup_dataset_restore_2026-04-09.md](../02_data/09_startup_dataset_restore_2026-04-09.md)
- [x] [02_data/10_dataset_context_surface_2026-04-08.md](../02_data/10_dataset_context_surface_2026-04-08.md)
- [x] [02_data/11_polygon_root_settings_2026-04-09.md](../02_data/11_polygon_root_settings_2026-04-09.md)
- [x] [02_data/12_polygon_api_key_2026-04-09.md](../02_data/12_polygon_api_key_2026-04-09.md)
- [x] [02_data/13_polygon_request_contract_2026-04-09.md](../02_data/13_polygon_request_contract_2026-04-09.md)
- [x] [02_data/14_polygon_chunking_limits_2026-04-09.md](../02_data/14_polygon_chunking_limits_2026-04-09.md)
- [x] [02_data/15_polygon_manifest_schema_2026-04-09.md](../02_data/15_polygon_manifest_schema_2026-04-09.md)
- [x] [02_data/16_polygon_dataset_listing_2026-04-09.md](../02_data/16_polygon_dataset_listing_2026-04-09.md)
- [x] [02_data/17_polygon_download_load_2026-04-08.md](../02_data/17_polygon_download_load_2026-04-08.md)
- [x] [02_data/18_polygon_dataset_validate_2026-04-09.md](../02_data/18_polygon_dataset_validate_2026-04-09.md)
- [x] [02_data/19_polygon_dataset_repair_2026-04-09.md](../02_data/19_polygon_dataset_repair_2026-04-09.md)
- [x] [02_data/20_polygon_dataset_topoff_2026-04-09.md](../02_data/20_polygon_dataset_topoff_2026-04-09.md)
- [x] [02_data/21_polygon_selection_ux_2026-04-09.md](../02_data/21_polygon_selection_ux_2026-04-09.md)
- [x] [02_data/22_dataset_provenance_2026-04-08.md](../02_data/22_dataset_provenance_2026-04-08.md)
- [x] [02_data/23_resample_contract_2026-04-08.md](../02_data/23_resample_contract_2026-04-08.md)
- [x] [02_data/24_resample_rules_2026-04-09.md](../02_data/24_resample_rules_2026-04-09.md)
- [x] [02_data/25_resample_no_redownload_2026-04-09.md](../02_data/25_resample_no_redownload_2026-04-09.md)
- [x] [02_data/26_derived_dataset_identity_2026-04-08.md](../02_data/26_derived_dataset_identity_2026-04-08.md)
- [x] [02_data/27_dataset_meta_schema_2026-04-08.md](../02_data/27_dataset_meta_schema_2026-04-08.md)
- [x] [02_data/28_dataset_fingerprint_2026-04-08.md](../02_data/28_dataset_fingerprint_2026-04-08.md)
- [x] [02_data/29_dataset_invalidation_2026-04-08.md](../02_data/29_dataset_invalidation_2026-04-08.md)
- [x] [02_data/30_data_repair_replay_risk_2026-04-09.md](../02_data/30_data_repair_replay_risk_2026-04-09.md)
- [x] [02_data/31_data_service_store_2026-04-08.md](../02_data/31_data_service_store_2026-04-08.md)
- [x] [02_data/32_workflow_bootstrap_data_2026-04-09.md](../02_data/32_workflow_bootstrap_data_2026-04-09.md)
- [x] [02_data/33_data_cache_state_backend_2026-04-09.md](../02_data/33_data_cache_state_backend_2026-04-09.md)
- [x] [02_data/34_dataset_memory_retention_2026-04-09.md](../02_data/34_dataset_memory_retention_2026-04-09.md)

## 03 Opt

- section total: `33`

- [x] [03_opt/01_opt_panel_core_2026-04-09.md](../03_opt/01_opt_panel_core_2026-04-09.md)
- [x] [03_opt/02_opt_strategy_backend_2026-04-09.md](../03_opt/02_opt_strategy_backend_2026-04-09.md)
- [x] [03_opt/03_opt_ma_grid_editor_2026-04-09.md](../03_opt/03_opt_ma_grid_editor_2026-04-09.md)
- [x] [03_opt/04_opt_dynamic_builder_2026-04-09.md](../03_opt/04_opt_dynamic_builder_2026-04-09.md)
- [x] [03_opt/05_opt_route_gpu_capability_2026-04-09.md](../03_opt/05_opt_route_gpu_capability_2026-04-09.md)
- [x] [03_opt/06_opt_run_plan_readiness_2026-04-09.md](../03_opt/06_opt_run_plan_readiness_2026-04-09.md)
- [x] [03_opt/07_strategy_costs_risk_2026-04-09.md](../03_opt/07_strategy_costs_risk_2026-04-09.md)
- [x] [03_opt/08_strategy_family_params_2026-04-09.md](../03_opt/08_strategy_family_params_2026-04-09.md)
- [x] [03_opt/09_advanced_search_objective_2026-04-09.md](../03_opt/09_advanced_search_objective_2026-04-09.md)
- [x] [03_opt/10_advanced_shortlist_export_2026-04-09.md](../03_opt/10_advanced_shortlist_export_2026-04-09.md)
- [x] [03_opt/11_opt_request_builder_2026-04-08.md](../03_opt/11_opt_request_builder_2026-04-08.md)
- [x] [03_opt/12_opt_service_progress_2026-04-08.md](../03_opt/12_opt_service_progress_2026-04-08.md)
- [x] [03_opt/13_strategy_caps_surface_2026-04-08.md](../03_opt/13_strategy_caps_surface_2026-04-08.md)
- [x] [03_opt/14_opt_double_ma_command_2026-04-09.md](../03_opt/14_opt_double_ma_command_2026-04-09.md)
- [x] [03_opt/15_opt_composite_cpu_command_2026-04-09.md](../03_opt/15_opt_composite_cpu_command_2026-04-09.md)
- [x] [03_opt/16_opt_composite_mo_command_2026-04-09.md](../03_opt/16_opt_composite_mo_command_2026-04-09.md)
- [x] [03_opt/17_opt_cancel_progress_2026-04-09.md](../03_opt/17_opt_cancel_progress_2026-04-09.md)
- [x] [03_opt/18_eng_dm_request_resolution_2026-04-09.md](../03_opt/18_eng_dm_request_resolution_2026-04-09.md)
- [x] [03_opt/19_eng_dm_backend_routing_2026-04-08.md](../03_opt/19_eng_dm_backend_routing_2026-04-08.md)
- [x] [03_opt/20_eng_dm_gpu_kernel_vram_2026-04-09.md](../03_opt/20_eng_dm_gpu_kernel_vram_2026-04-09.md)
- [x] [03_opt/21_eng_dm_vdps_fidelity_2026-04-09.md](../03_opt/21_eng_dm_vdps_fidelity_2026-04-09.md)
- [x] [03_opt/22_eng_dm_output_contract_2026-04-09.md](../03_opt/22_eng_dm_output_contract_2026-04-09.md)
- [x] [03_opt/23_eng_dynamic_indicators_2026-04-09.md](../03_opt/23_eng_dynamic_indicators_2026-04-09.md)
- [x] [03_opt/24_eng_strategy_ir_plan_2026-04-09.md](../03_opt/24_eng_strategy_ir_plan_2026-04-09.md)
- [x] [03_opt/25_optimize_store_2026-04-08.md](../03_opt/25_optimize_store_2026-04-08.md)
- [x] [03_opt/26_strategy_store_2026-04-09.md](../03_opt/26_strategy_store_2026-04-09.md)
- [x] [03_opt/27_advanced_store_2026-04-09.md](../03_opt/27_advanced_store_2026-04-09.md)
- [x] [03_opt/28_recommended_defaults_2026-04-09.md](../03_opt/28_recommended_defaults_2026-04-09.md)
- [x] [03_opt/29_ma_catalog_registry_2026-04-09.md](../03_opt/29_ma_catalog_registry_2026-04-09.md)
- [x] [03_opt/30_dynamic_indicator_catalog_2026-04-09.md](../03_opt/30_dynamic_indicator_catalog_2026-04-09.md)
- [x] [03_opt/31_gpu_speedups_policy_2026-04-09.md](../03_opt/31_gpu_speedups_policy_2026-04-09.md)
- [x] [03_opt/32_run_control_cancel_bridge_2026-04-09.md](../03_opt/32_run_control_cancel_bridge_2026-04-09.md)
- [x] [03_opt/33_advanced_shortlist_diagnostics_2026-04-08.md](../03_opt/33_advanced_shortlist_diagnostics_2026-04-08.md)

## 04 Results

- section total: `20`

- [x] [04_results/01_results_overview_2026-04-08.md](../04_results/01_results_overview_2026-04-08.md)
- [x] [04_results/02_results_top_table_2026-04-08.md](../04_results/02_results_top_table_2026-04-08.md)
- [x] [04_results/03_results_top_compare_2026-04-09.md](../04_results/03_results_top_compare_2026-04-09.md)
- [x] [04_results/04_results_pareto_2026-04-09.md](../04_results/04_results_pareto_2026-04-09.md)
- [x] [04_results/05_results_drilldown_2026-04-09.md](../04_results/05_results_drilldown_2026-04-09.md)
- [x] [04_results/06_results_heatmap_2026-04-09.md](../04_results/06_results_heatmap_2026-04-09.md)
- [x] [04_results/07_results_empty_states_2026-04-09.md](../04_results/07_results_empty_states_2026-04-09.md)
- [x] [04_results/08_results_display_adequacy_2026-04-08.md](../04_results/08_results_display_adequacy_2026-04-08.md)
- [x] [04_results/09_results_analysis_contract_2026-04-09.md](../04_results/09_results_analysis_contract_2026-04-09.md)
- [x] [04_results/10_out_overview_decision_surface_2026-04-09.md](../04_results/10_out_overview_decision_surface_2026-04-09.md)
- [x] [04_results/11_out_top_results_table_2026-04-09.md](../04_results/11_out_top_results_table_2026-04-09.md)
- [x] [04_results/12_out_pareto_heatmap_readability_2026-04-09.md](../04_results/12_out_pareto_heatmap_readability_2026-04-09.md)
- [x] [04_results/13_out_drilldown_stability_2026-04-09.md](../04_results/13_out_drilldown_stability_2026-04-09.md)
- [x] [04_results/14_evidence_gaps_missing_data_2026-04-09.md](../04_results/14_evidence_gaps_missing_data_2026-04-09.md)
- [x] [04_results/15_dataset_stale_provenance_cues_2026-04-09.md](../04_results/15_dataset_stale_provenance_cues_2026-04-09.md)
- [x] [04_results/16_report_path_reopen_flow_2026-04-09.md](../04_results/16_report_path_reopen_flow_2026-04-09.md)
- [x] [04_results/17_analysis_store_service_2026-04-08.md](../04_results/17_analysis_store_service_2026-04-08.md)
- [x] [04_results/18_context_docks_2026-04-08.md](../04_results/18_context_docks_2026-04-08.md)
- [x] [04_results/19_chart_rendering_composables_2026-04-09.md](../04_results/19_chart_rendering_composables_2026-04-09.md)
- [x] [04_results/20_matrix_table_state_2026-04-08.md](../04_results/20_matrix_table_state_2026-04-08.md)

## 05 Validation

- section total: `27`

- [x] [05_validation/01_validate_panel_core_2026-04-09.md](../05_validation/01_validate_panel_core_2026-04-09.md)
- [x] [05_validation/02_validate_admission_cross_2026-04-09.md](../05_validation/02_validate_admission_cross_2026-04-09.md)
- [x] [05_validation/03_validate_results_table_2026-04-09.md](../05_validation/03_validate_results_table_2026-04-09.md)
- [x] [05_validation/04_validate_report_export_2026-04-09.md](../05_validation/04_validate_report_export_2026-04-09.md)
- [x] [05_validation/05_validate_artifact_persistence_2026-04-09.md](../05_validation/05_validate_artifact_persistence_2026-04-09.md)
- [x] [05_validation/06_val_dm_command_2026-04-09.md](../05_validation/06_val_dm_command_2026-04-09.md)
- [x] [05_validation/07_val_composite_command_2026-04-09.md](../05_validation/07_val_composite_command_2026-04-09.md)
- [x] [05_validation/08_val_cross_ticker_pricebars_2026-04-09.md](../05_validation/08_val_cross_ticker_pricebars_2026-04-09.md)
- [x] [05_validation/09_eng_validation_verdicts_2026-04-09.md](../05_validation/09_eng_validation_verdicts_2026-04-09.md)
- [x] [05_validation/10_eng_validation_stats_core_2026-04-09.md](../05_validation/10_eng_validation_stats_core_2026-04-09.md)
- [x] [05_validation/11_xlsx_workbook_contract_2026-04-08.md](../05_validation/11_xlsx_workbook_contract_2026-04-08.md)
- [x] [05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md](../05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md)
- [x] [05_validation/13_xlsx_dashboard_quality_2026-04-08.md](../05_validation/13_xlsx_dashboard_quality_2026-04-08.md)
- [x] [05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md](../05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md)
- [x] [05_validation/15_xlsx_profile_family_coverage_2026-04-09.md](../05_validation/15_xlsx_profile_family_coverage_2026-04-09.md)
- [x] [05_validation/16_xlsx_route_note_status_2026-04-09.md](../05_validation/16_xlsx_route_note_status_2026-04-09.md)
- [x] [05_validation/17_xlsx_open_reveal_flows_2026-04-09.md](../05_validation/17_xlsx_open_reveal_flows_2026-04-09.md)
- [x] [05_validation/18_csv_shortlist_export_2026-04-09.md](../05_validation/18_csv_shortlist_export_2026-04-09.md)
- [x] [05_validation/19_validation_request_builder_2026-04-08.md](../05_validation/19_validation_request_builder_2026-04-08.md)
- [x] [05_validation/20_validation_service_2026-04-08.md](../05_validation/20_validation_service_2026-04-08.md)
- [x] [05_validation/21_validation_store_2026-04-08.md](../05_validation/21_validation_store_2026-04-08.md)
- [x] [05_validation/22_validation_context_docks_2026-04-08.md](../05_validation/22_validation_context_docks_2026-04-08.md)
- [x] [05_validation/23_metric_surface_contract_2026-04-08.md](../05_validation/23_metric_surface_contract_2026-04-08.md)
- [x] [05_validation/24_validation_common_backend_2026-04-09.md](../05_validation/24_validation_common_backend_2026-04-09.md)
- [x] [05_validation/25_validation_context_compat_2026-04-08.md](../05_validation/25_validation_context_compat_2026-04-08.md)
- [x] [05_validation/26_cross_ticker_finalist_logic_2026-04-09.md](../05_validation/26_cross_ticker_finalist_logic_2026-04-09.md)
- [x] [05_validation/27_validation_robustness_methods_2026-04-09.md](../05_validation/27_validation_robustness_methods_2026-04-09.md)

## 06 Artifacts

- section total: `32`

- [x] [06_artifacts/01_run_artifact_schema_2026-04-08.md](../06_artifacts/01_run_artifact_schema_2026-04-08.md)
- [x] [06_artifacts/02_artifact_dataset_meta_norm_2026-04-09.md](../06_artifacts/02_artifact_dataset_meta_norm_2026-04-09.md)
- [x] [06_artifacts/03_artifact_library_index_2026-04-09.md](../06_artifacts/03_artifact_library_index_2026-04-09.md)
- [x] [06_artifacts/04_artifact_hydration_contract_2026-04-08.md](../06_artifacts/04_artifact_hydration_contract_2026-04-08.md)
- [x] [06_artifacts/05_meta_artifact_replay_2026-04-09.md](../06_artifacts/05_meta_artifact_replay_2026-04-09.md)
- [x] [06_artifacts/06_stage5_artifact_replay_2026-04-09.md](../06_artifacts/06_stage5_artifact_replay_2026-04-09.md)
- [x] [06_artifacts/07_policy_eval_artifact_replay_2026-04-09.md](../06_artifacts/07_policy_eval_artifact_replay_2026-04-09.md)
- [x] [06_artifacts/08_opt_metric_surface_artifact_2026-04-09.md](../06_artifacts/08_opt_metric_surface_artifact_2026-04-09.md)
- [x] [06_artifacts/09_workflow_artifact_store_2026-04-08.md](../06_artifacts/09_workflow_artifact_store_2026-04-08.md)
- [x] [06_artifacts/10_history_search_filter_ux_2026-04-08.md](../06_artifacts/10_history_search_filter_ux_2026-04-08.md)
- [x] [06_artifacts/11_art_fingerprint_provenance_2026-04-09.md](../06_artifacts/11_art_fingerprint_provenance_2026-04-09.md)
- [x] [06_artifacts/12_art_hydration_detail_replay_2026-04-09.md](../06_artifacts/12_art_hydration_detail_replay_2026-04-09.md)
- [x] [06_artifacts/13_art_retention_tags_compat_2026-04-09.md](../06_artifacts/13_art_retention_tags_compat_2026-04-09.md)
- [x] [06_artifacts/14_report_validation_dashboard_2026-04-09.md](../06_artifacts/14_report_validation_dashboard_2026-04-09.md)
- [x] [06_artifacts/15_report_validation_candidate_sheets_2026-04-09.md](../06_artifacts/15_report_validation_candidate_sheets_2026-04-09.md)
- [x] [06_artifacts/16_report_route_fallback_2026-04-09.md](../06_artifacts/16_report_route_fallback_2026-04-09.md)
- [x] [06_artifacts/17_report_meta_workbook_2026-04-09.md](../06_artifacts/17_report_meta_workbook_2026-04-09.md)
- [x] [06_artifacts/18_report_shortlist_csv_2026-04-09.md](../06_artifacts/18_report_shortlist_csv_2026-04-09.md)
- [x] [06_artifacts/19_report_save_open_reveal_2026-04-09.md](../06_artifacts/19_report_save_open_reveal_2026-04-09.md)
- [x] [06_artifacts/20_report_json_contract_2026-04-09.md](../06_artifacts/20_report_json_contract_2026-04-09.md)
- [x] [06_artifacts/21_report_artifact_path_persistence_2026-04-08.md](../06_artifacts/21_report_artifact_path_persistence_2026-04-08.md)
- [x] [06_artifacts/22_meta_stage3_report_flow_2026-04-09.md](../06_artifacts/22_meta_stage3_report_flow_2026-04-09.md)
- [x] [06_artifacts/23_mlsignal_report_path_semantics_2026-04-09.md](../06_artifacts/23_mlsignal_report_path_semantics_2026-04-09.md)
- [x] [06_artifacts/24_history_library_core_2026-04-08.md](../06_artifacts/24_history_library_core_2026-04-08.md)
- [x] [06_artifacts/25_history_detail_actions_2026-04-09.md](../06_artifacts/25_history_detail_actions_2026-04-09.md)
- [x] [06_artifacts/26_history_service_store_2026-04-08.md](../06_artifacts/26_history_service_store_2026-04-08.md)
- [x] [06_artifacts/27_command_action_store_2026-04-09.md](../06_artifacts/27_command_action_store_2026-04-09.md)
- [x] [06_artifacts/28_workflow_artifacts_sync_2026-04-08.md](../06_artifacts/28_workflow_artifacts_sync_2026-04-08.md)
- [x] [06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md](../06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md)
- [x] [06_artifacts/30_core_dataset_meta_paths_2026-04-09.md](../06_artifacts/30_core_dataset_meta_paths_2026-04-09.md)
- [x] [06_artifacts/31_batch_summary_artifacts_2026-04-09.md](../06_artifacts/31_batch_summary_artifacts_2026-04-09.md)
- [x] [06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md](../06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md)

## 07 Strats

- section total: `29`

- [x] [07_strats/01_strat_double_ma_family_2026-04-08.md](../07_strats/01_strat_double_ma_family_2026-04-08.md)
- [x] [07_strats/02_strat_dynamic_double_ma_2026-04-09.md](../07_strats/02_strat_dynamic_double_ma_2026-04-09.md)
- [x] [07_strats/03_strat_composite_trend_2026-04-08.md](../07_strats/03_strat_composite_trend_2026-04-08.md)
- [x] [07_strats/04_strat_breakout_momentum_2026-04-08.md](../07_strats/04_strat_breakout_momentum_2026-04-08.md)
- [x] [07_strats/05_strat_regime_hybrid_2026-04-08.md](../07_strats/05_strat_regime_hybrid_2026-04-08.md)
- [x] [07_strats/06_strat_composite_cpu_2026-04-09.md](../07_strats/06_strat_composite_cpu_2026-04-09.md)
- [x] [07_strats/07_strat_composite_mo_2026-04-09.md](../07_strats/07_strat_composite_mo_2026-04-09.md)
- [x] [07_strats/08_strat_ml_signal_2026-04-08.md](../07_strats/08_strat_ml_signal_2026-04-08.md)
- [x] [07_strats/09_strat_profile_cluster_a_2026-04-09.md](../07_strats/09_strat_profile_cluster_a_2026-04-09.md)
- [x] [07_strats/10_strat_profile_cluster_b_2026-04-09.md](../07_strats/10_strat_profile_cluster_b_2026-04-09.md)
- [x] [07_strats/11_eng_composite_route_policy_2026-04-08.md](../07_strats/11_eng_composite_route_policy_2026-04-08.md)
- [x] [07_strats/12_eng_composite_validation_2026-04-08.md](../07_strats/12_eng_composite_validation_2026-04-08.md)
- [x] [07_strats/13_eng_composite_mo_prescan_search_2026-04-09.md](../07_strats/13_eng_composite_mo_prescan_search_2026-04-09.md)
- [x] [07_strats/14_analysis_drilldown_stability_2026-04-08.md](../07_strats/14_analysis_drilldown_stability_2026-04-08.md)
- [x] [07_strats/15_dynamic_ir_campaign_surface_2026-04-09.md](../07_strats/15_dynamic_ir_campaign_surface_2026-04-09.md)
- [x] [07_strats/16_fidelity_audit_harness_2026-04-08.md](../07_strats/16_fidelity_audit_harness_2026-04-08.md)
- [x] [07_strats/17_research_diagnostic_bins_2026-04-09.md](../07_strats/17_research_diagnostic_bins_2026-04-09.md)
- [x] [07_strats/18_eng_csv_cuda_progress_2026-04-09.md](../07_strats/18_eng_csv_cuda_progress_2026-04-09.md)
- [x] [07_strats/19_data_discovery_catalogs_2026-04-09.md](../07_strats/19_data_discovery_catalogs_2026-04-09.md)
- [x] [07_strats/20_strat_surface_matrix_2026-04-09.md](../07_strats/20_strat_surface_matrix_2026-04-09.md)
- [x] [07_strats/21_optimizer_grid_stream_2026-04-09.md](../07_strats/21_optimizer_grid_stream_2026-04-09.md)
- [x] [07_strats/22_optimizer_mo_core_2026-04-09.md](../07_strats/22_optimizer_mo_core_2026-04-09.md)
- [x] [07_strats/23_strategy_plan_presets_2026-04-09.md](../07_strats/23_strategy_plan_presets_2026-04-09.md)
- [x] [07_strats/24_core_timeframe_matrix_stats_2026-04-09.md](../07_strats/24_core_timeframe_matrix_stats_2026-04-09.md)
- [x] [07_strats/25_strategy_runtime_gpu_stack_2026-04-09.md](../07_strats/25_strategy_runtime_gpu_stack_2026-04-09.md)
- [x] [07_strats/26_prescan_iterative_sim_2026-04-09.md](../07_strats/26_prescan_iterative_sim_2026-04-09.md)
- [x] [07_strats/27_alloc_policy_safety_2026-04-09.md](../07_strats/27_alloc_policy_safety_2026-04-09.md)
- [x] [07_strats/28_headless_cli_surface_2026-04-09.md](../07_strats/28_headless_cli_surface_2026-04-09.md)
- [x] [07_strats/29_trade_records_contract_2026-04-09.md](../07_strats/29_trade_records_contract_2026-04-09.md)

## 08 ML

- section total: `46`

- [x] [08_ml/01_ml_shell_handoff_2026-04-09.md](../08_ml/01_ml_shell_handoff_2026-04-09.md)
- [x] [08_ml/02_ml_config_paths_2026-04-09.md](../08_ml/02_ml_config_paths_2026-04-09.md)
- [x] [08_ml/03_ml_score_cache_bundles_2026-04-08.md](../08_ml/03_ml_score_cache_bundles_2026-04-08.md)
- [x] [08_ml/04_ml_validation_surface_2026-04-08.md](../08_ml/04_ml_validation_surface_2026-04-08.md)
- [x] [08_ml/05_ml_policy_sweep_2026-04-09.md](../08_ml/05_ml_policy_sweep_2026-04-09.md)
- [x] [08_ml/06_meta_panel_ia_2026-04-09.md](../08_ml/06_meta_panel_ia_2026-04-09.md)
- [x] [08_ml/07_meta_selection_compare_2026-04-09.md](../08_ml/07_meta_selection_compare_2026-04-09.md)
- [x] [08_ml/08_meta_batch_generation_2026-04-09.md](../08_ml/08_meta_batch_generation_2026-04-09.md)
- [x] [08_ml/09_meta_saved_artifact_families_2026-04-09.md](../08_ml/09_meta_saved_artifact_families_2026-04-09.md)
- [x] [08_ml/10_meta_ml_confidence_reports_2026-04-09.md](../08_ml/10_meta_ml_confidence_reports_2026-04-09.md)
- [x] [08_ml/11_mlsignal_dataset_export_2026-04-08.md](../08_ml/11_mlsignal_dataset_export_2026-04-08.md)
- [x] [08_ml/12_mlsignal_feature_schema_2026-04-09.md](../08_ml/12_mlsignal_feature_schema_2026-04-09.md)
- [x] [08_ml/13_mlsignal_split_plan_2026-04-09.md](../08_ml/13_mlsignal_split_plan_2026-04-09.md)
- [x] [08_ml/14_mlsignal_export_manifests_2026-04-09.md](../08_ml/14_mlsignal_export_manifests_2026-04-09.md)
- [x] [08_ml/15_mlsignal_score_csv_ingest_2026-04-09.md](../08_ml/15_mlsignal_score_csv_ingest_2026-04-09.md)
- [x] [08_ml/16_mlsignal_score_cache_build_2026-04-08.md](../08_ml/16_mlsignal_score_cache_build_2026-04-08.md)
- [x] [08_ml/17_mlsignal_score_cache_validation_2026-04-09.md](../08_ml/17_mlsignal_score_cache_validation_2026-04-09.md)
- [x] [08_ml/18_mlsignal_policy_params_2026-04-09.md](../08_ml/18_mlsignal_policy_params_2026-04-09.md)
- [x] [08_ml/19_mlsignal_validation_flow_2026-04-08.md](../08_ml/19_mlsignal_validation_flow_2026-04-08.md)
- [x] [08_ml/20_mlsignal_model_bundle_inventory_2026-04-09.md](../08_ml/20_mlsignal_model_bundle_inventory_2026-04-09.md)
- [x] [08_ml/21_mlsignal_artifact_attachment_2026-04-08.md](../08_ml/21_mlsignal_artifact_attachment_2026-04-08.md)
- [x] [08_ml/22_meta_selection_request_contract_2026-04-08.md](../08_ml/22_meta_selection_request_contract_2026-04-08.md)
- [x] [08_ml/23_meta_grouping_clustering_2026-04-09.md](../08_ml/23_meta_grouping_clustering_2026-04-09.md)
- [x] [08_ml/24_meta_scoring_weights_2026-04-09.md](../08_ml/24_meta_scoring_weights_2026-04-09.md)
- [x] [08_ml/25_meta_evidence_flags_2026-04-09.md](../08_ml/25_meta_evidence_flags_2026-04-09.md)
- [x] [08_ml/26_meta_holdout_modes_2026-04-09.md](../08_ml/26_meta_holdout_modes_2026-04-09.md)
- [x] [08_ml/27_meta_batch_registry_modes_2026-04-09.md](../08_ml/27_meta_batch_registry_modes_2026-04-09.md)
- [x] [08_ml/28_meta_batch_resume_retry_2026-04-09.md](../08_ml/28_meta_batch_resume_retry_2026-04-09.md)
- [x] [08_ml/29_ml_conf_scope_identity_2026-04-09.md](../08_ml/29_ml_conf_scope_identity_2026-04-09.md)
- [x] [08_ml/30_ml_conf_training_rows_2026-04-09.md](../08_ml/30_ml_conf_training_rows_2026-04-09.md)
- [x] [08_ml/31_ml_conf_label_config_2026-04-09.md](../08_ml/31_ml_conf_label_config_2026-04-09.md)
- [x] [08_ml/32_ml_conf_model_families_2026-04-09.md](../08_ml/32_ml_conf_model_families_2026-04-09.md)
- [x] [08_ml/33_ml_conf_health_staleness_2026-04-09.md](../08_ml/33_ml_conf_health_staleness_2026-04-09.md)
- [x] [08_ml/34_ml_conf_apply_contract_2026-04-09.md](../08_ml/34_ml_conf_apply_contract_2026-04-09.md)
- [x] [08_ml/35_stage4_request_contract_2026-04-08.md](../08_ml/35_stage4_request_contract_2026-04-08.md)
- [x] [08_ml/36_stage4_selection_logic_2026-04-09.md](../08_ml/36_stage4_selection_logic_2026-04-09.md)
- [x] [08_ml/37_stage5_execution_contract_2026-04-09.md](../08_ml/37_stage5_execution_contract_2026-04-09.md)
- [x] [08_ml/38_ml_policy_eval_contract_2026-04-09.md](../08_ml/38_ml_policy_eval_contract_2026-04-09.md)
- [x] [08_ml/39_stage4_stage5_saved_replay_2026-04-09.md](../08_ml/39_stage4_stage5_saved_replay_2026-04-09.md)
- [x] [08_ml/40_meta_ml_status_surface_2026-04-09.md](../08_ml/40_meta_ml_status_surface_2026-04-09.md)
- [x] [08_ml/41_meta_request_builders_2026-04-09.md](../08_ml/41_meta_request_builders_2026-04-09.md)
- [x] [08_ml/42_meta_scope_builders_2026-04-09.md](../08_ml/42_meta_scope_builders_2026-04-09.md)
- [x] [08_ml/43_meta_service_campaigns_2026-04-09.md](../08_ml/43_meta_service_campaigns_2026-04-09.md)
- [x] [08_ml/44_mlsignal_frontend_service_2026-04-09.md](../08_ml/44_mlsignal_frontend_service_2026-04-09.md)
- [x] [08_ml/45_meta_frontend_service_2026-04-09.md](../08_ml/45_meta_frontend_service_2026-04-09.md)
- [x] [08_ml/46_ml_meta_store_status_2026-04-09.md](../08_ml/46_ml_meta_store_status_2026-04-09.md)

## 09 Live

- section total: `14`

- [x] [09_live/01_execution_command_surface_2026-04-08.md](../09_live/01_execution_command_surface_2026-04-08.md)
- [x] [09_live/02_execution_workspace_ux_2026-04-09.md](../09_live/02_execution_workspace_ux_2026-04-09.md)
- [x] [09_live/03_execution_runner_lifecycle_2026-04-08.md](../09_live/03_execution_runner_lifecycle_2026-04-08.md)
- [x] [09_live/04_execution_order_position_review_2026-04-09.md](../09_live/04_execution_order_position_review_2026-04-09.md)
- [x] [09_live/05_execution_safety_modes_2026-04-09.md](../09_live/05_execution_safety_modes_2026-04-09.md)
- [x] [09_live/06_broker_connectivity_2026-04-09.md](../09_live/06_broker_connectivity_2026-04-09.md)
- [x] [09_live/07_live_candle_stream_data_2026-04-08.md](../09_live/07_live_candle_stream_data_2026-04-08.md)
- [x] [09_live/08_system_gpu_window_surface_2026-04-09.md](../09_live/08_system_gpu_window_surface_2026-04-09.md)
- [x] [09_live/09_file_picker_path_boundary_2026-04-09.md](../09_live/09_file_picker_path_boundary_2026-04-09.md)
- [x] [09_live/10_run_guards_cancel_tokens_2026-04-09.md](../09_live/10_run_guards_cancel_tokens_2026-04-09.md)
- [x] [09_live/11_settings_secret_storage_2026-04-09.md](../09_live/11_settings_secret_storage_2026-04-09.md)
- [x] [09_live/12_out_execution_monitoring_2026-04-09.md](../09_live/12_out_execution_monitoring_2026-04-09.md)
- [x] [09_live/13_execution_service_store_2026-04-08.md](../09_live/13_execution_service_store_2026-04-08.md)
- [x] [09_live/14_runtime_state_bridge_2026-04-08.md](../09_live/14_runtime_state_bridge_2026-04-08.md)

## 10 Tests Perf

- section total: `57`

- [x] [10_tests_perf/01_fe_test_matrix_2026-04-08.md](../10_tests_perf/01_fe_test_matrix_2026-04-08.md)
- [x] [10_tests_perf/02_fe_component_surface_tests_2026-04-09.md](../10_tests_perf/02_fe_component_surface_tests_2026-04-09.md)
- [x] [10_tests_perf/03_fe_store_service_tests_2026-04-09.md](../10_tests_perf/03_fe_store_service_tests_2026-04-09.md)
- [x] [10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md](../10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md)
- [x] [10_tests_perf/05_fe_results_surface_adequacy_2026-04-09.md](../10_tests_perf/05_fe_results_surface_adequacy_2026-04-09.md)
- [x] [10_tests_perf/06_fe_dense_table_virtualization_2026-04-09.md](../10_tests_perf/06_fe_dense_table_virtualization_2026-04-09.md)
- [x] [10_tests_perf/07_fe_a11y_smoke_2026-04-09.md](../10_tests_perf/07_fe_a11y_smoke_2026-04-09.md)
- [x] [10_tests_perf/08_fe_ui_baseline_capture_2026-04-09.md](../10_tests_perf/08_fe_ui_baseline_capture_2026-04-09.md)
- [x] [10_tests_perf/09_browser_harness_contract_2026-04-09.md](../10_tests_perf/09_browser_harness_contract_2026-04-09.md)
- [x] [10_tests_perf/10_browser_shell_ux_2026-04-09.md](../10_tests_perf/10_browser_shell_ux_2026-04-09.md)
- [x] [10_tests_perf/11_browser_palette_overlays_2026-04-09.md](../10_tests_perf/11_browser_palette_overlays_2026-04-09.md)
- [x] [10_tests_perf/12_browser_support_mocks_2026-04-09.md](../10_tests_perf/12_browser_support_mocks_2026-04-09.md)
- [x] [10_tests_perf/13_desktop_harness_contract_2026-04-09.md](../10_tests_perf/13_desktop_harness_contract_2026-04-09.md)
- [x] [10_tests_perf/14_desktop_shell_readiness_2026-04-09.md](../10_tests_perf/14_desktop_shell_readiness_2026-04-09.md)
- [x] [10_tests_perf/15_desktop_native_titlebar_2026-04-09.md](../10_tests_perf/15_desktop_native_titlebar_2026-04-09.md)
- [x] [10_tests_perf/16_desktop_task_center_nav_2026-04-09.md](../10_tests_perf/16_desktop_task_center_nav_2026-04-09.md)
- [x] [10_tests_perf/17_desktop_settings_diag_2026-04-09.md](../10_tests_perf/17_desktop_settings_diag_2026-04-09.md)
- [x] [10_tests_perf/18_desktop_geometry_behavior_2026-04-09.md](../10_tests_perf/18_desktop_geometry_behavior_2026-04-09.md)
- [x] [10_tests_perf/19_desktop_artifact_capture_2026-04-09.md](../10_tests_perf/19_desktop_artifact_capture_2026-04-09.md)
- [x] [10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md](../10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md)
- [x] [10_tests_perf/21_tauri_mock_runtime_smoke_2026-04-08.md](../10_tests_perf/21_tauri_mock_runtime_smoke_2026-04-08.md)
- [x] [10_tests_perf/22_tauri_settings_persistence_2026-04-08.md](../10_tests_perf/22_tauri_settings_persistence_2026-04-08.md)
- [x] [10_tests_perf/23_tauri_report_command_api_2026-04-08.md](../10_tests_perf/23_tauri_report_command_api_2026-04-08.md)
- [x] [10_tests_perf/24_tauri_run_artifact_paths_2026-04-08.md](../10_tests_perf/24_tauri_run_artifact_paths_2026-04-08.md)
- [x] [10_tests_perf/25_headless_report_builders_2026-04-08.md](../10_tests_perf/25_headless_report_builders_2026-04-08.md)
- [x] [10_tests_perf/26_cpu_perf_gate_2026-04-09.md](../10_tests_perf/26_cpu_perf_gate_2026-04-09.md)
- [x] [10_tests_perf/27_gpu_perf_gate_2026-04-09.md](../10_tests_perf/27_gpu_perf_gate_2026-04-09.md)
- [x] [10_tests_perf/28_workflow_gate_2026-04-09.md](../10_tests_perf/28_workflow_gate_2026-04-09.md)
- [x] [10_tests_perf/29_request_builder_bench_2026-04-09.md](../10_tests_perf/29_request_builder_bench_2026-04-09.md)
- [x] [10_tests_perf/30_workflow_benchmark_pack_2026-04-09.md](../10_tests_perf/30_workflow_benchmark_pack_2026-04-09.md)
- [ ] [10_tests_perf/31_vectorbt_faceoff_2026-04-09.md](../10_tests_perf/31_vectorbt_faceoff_2026-04-09.md)
- [x] [10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md](../10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md)
- [x] [10_tests_perf/33_ncu_profile_playbook_2026-04-09.md](../10_tests_perf/33_ncu_profile_playbook_2026-04-09.md)
- [x] [10_tests_perf/34_nsys_profile_playbook_2026-04-09.md](../10_tests_perf/34_nsys_profile_playbook_2026-04-09.md)
- [x] [10_tests_perf/35_cupti_sass_probe_2026-04-09.md](../10_tests_perf/35_cupti_sass_probe_2026-04-09.md)
- [x] [10_tests_perf/36_gpu_route_matrix_profile_2026-04-09.md](../10_tests_perf/36_gpu_route_matrix_profile_2026-04-09.md)
- [x] [10_tests_perf/37_optimization_memory_capture_2026-04-09.md](../10_tests_perf/37_optimization_memory_capture_2026-04-09.md)
- [x] [10_tests_perf/38_profile_runner_bins_2026-04-09.md](../10_tests_perf/38_profile_runner_bins_2026-04-09.md)
- [x] [10_tests_perf/39_post_release_telemetry_review_2026-04-09.md](../10_tests_perf/39_post_release_telemetry_review_2026-04-09.md)
- [x] [10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md](../10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md)
- [x] [10_tests_perf/41_gpu_alma_contract_gate_2026-04-09.md](../10_tests_perf/41_gpu_alma_contract_gate_2026-04-09.md)
- [x] [10_tests_perf/42_tauri_command_parity_2026-04-09.md](../10_tests_perf/42_tauri_command_parity_2026-04-09.md)
- [x] [10_tests_perf/43_output_parity_pack_2026-04-09.md](../10_tests_perf/43_output_parity_pack_2026-04-09.md)
- [x] [10_tests_perf/44_parity_pair_registry_2026-04-09.md](../10_tests_perf/44_parity_pair_registry_2026-04-09.md)
- [x] [10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md](../10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md)
- [x] [10_tests_perf/46_legacy_name_compat_2026-04-09.md](../10_tests_perf/46_legacy_name_compat_2026-04-09.md)
- [x] [10_tests_perf/47_tauri_config_sync_2026-04-09.md](../10_tests_perf/47_tauri_config_sync_2026-04-09.md)
- [x] [10_tests_perf/48_generated_command_inventory_2026-04-08.md](../10_tests_perf/48_generated_command_inventory_2026-04-08.md)
- [x] [10_tests_perf/49_python_faceoff_locking_2026-04-09.md](../10_tests_perf/49_python_faceoff_locking_2026-04-09.md)
- [x] [10_tests_perf/50_frontend_test_support_mocks_2026-04-09.md](../10_tests_perf/50_frontend_test_support_mocks_2026-04-09.md)
- [x] [10_tests_perf/51_vectorgrid_internal_benchmark_boundaries_2026-04-09.md](../10_tests_perf/51_vectorgrid_internal_benchmark_boundaries_2026-04-09.md)
- [x] [10_tests_perf/52_app_core_criterion_benches_2026-04-09.md](../10_tests_perf/52_app_core_criterion_benches_2026-04-09.md)
- [x] [10_tests_perf/53_refactor_snapshot_parity_2026-04-09.md](../10_tests_perf/53_refactor_snapshot_parity_2026-04-09.md)
- [x] [10_tests_perf/54_campaign_preload_vram_perf_2026-04-09.md](../10_tests_perf/54_campaign_preload_vram_perf_2026-04-09.md)
- [x] [10_tests_perf/55_headless_workflow_bins_2026-04-09.md](../10_tests_perf/55_headless_workflow_bins_2026-04-09.md)
- [x] [10_tests_perf/56_static_analysis_quality_gates_2026-04-09.md](../10_tests_perf/56_static_analysis_quality_gates_2026-04-09.md)
- [x] [10_tests_perf/57_manual_a11y_review_matrix_2026-04-09.md](../10_tests_perf/57_manual_a11y_review_matrix_2026-04-09.md)

## 11 Docs Ops

- section total: `16`

- [ ] [11_docs_ops/01_release_build_variants_2026-04-09.md](../11_docs_ops/01_release_build_variants_2026-04-09.md)
- [ ] [11_docs_ops/02_ptx_packaging_fallbacks_2026-04-09.md](../11_docs_ops/02_ptx_packaging_fallbacks_2026-04-09.md)
- [ ] [11_docs_ops/03_frontend_build_handoff_2026-04-09.md](../11_docs_ops/03_frontend_build_handoff_2026-04-09.md)
- [ ] [11_docs_ops/04_bundle_budget_analysis_2026-04-09.md](../11_docs_ops/04_bundle_budget_analysis_2026-04-09.md)
- [ ] [11_docs_ops/05_baseline_samples_manifest_2026-04-09.md](../11_docs_ops/05_baseline_samples_manifest_2026-04-09.md)
- [x] [11_docs_ops/06_repo_map_readme_hygiene_2026-04-09.md](../11_docs_ops/06_repo_map_readme_hygiene_2026-04-09.md)
- [x] [11_docs_ops/07_local_vs_ci_authority_2026-04-09.md](../11_docs_ops/07_local_vs_ci_authority_2026-04-09.md)
- [ ] [11_docs_ops/08_docs_archive_policy_2026-04-09.md](../11_docs_ops/08_docs_archive_policy_2026-04-09.md)
- [ ] [11_docs_ops/09_non_options_scope_boundary_2026-04-09.md](../11_docs_ops/09_non_options_scope_boundary_2026-04-09.md)
- [ ] [11_docs_ops/10_research_source_policy_2026-04-08.md](../11_docs_ops/10_research_source_policy_2026-04-08.md)
- [ ] [11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md](../11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md)
- [ ] [11_docs_ops/12_program_governance_2026-04-08.md](../11_docs_ops/12_program_governance_2026-04-08.md)
- [ ] [11_docs_ops/13_migration_freeze_scope_2026-04-09.md](../11_docs_ops/13_migration_freeze_scope_2026-04-09.md)
- [ ] [11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md](../11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md)
- [ ] [11_docs_ops/15_public_repo_community_health_2026-04-09.md](../11_docs_ops/15_public_repo_community_health_2026-04-09.md)
- [ ] [11_docs_ops/16_public_release_operations_2026-04-09.md](../11_docs_ops/16_public_release_operations_2026-04-09.md)

## Related Docs

- [REPO_IMPROVEMENT_META_PLAN_2026-04-08.md](../REPO_IMPROVEMENT_META_PLAN_2026-04-08.md)
- [program_index_2026-04-08.md](./program_index_2026-04-08.md)
- [doc_creation_tracker_2026-04-08.md](./doc_creation_tracker_2026-04-08.md)

## Post-Checklist Expansion Note

After the current implementation-plan checklist is substantially completed, start a second-tier improvement program for larger-scope work rather than mixing those items into the current ledger.

The follow-on program should prefer objective, additive improvements with stronger entry criteria:

- baseline measurements before implementation
- explicit user-facing objective and non-goals
- stricter regression gates and rollback notes
- numeric success criteria where possible
- separate epic-scale implementation documents instead of widening the current checklist

Recommended priority areas for that next phase:

- end-to-end performance program
  - startup time, optimization latency, validation latency, memory pressure, artifact load times, and UI responsiveness
- dense-results and history scalability
  - large shortlist tables, validation tables, and history/artifact libraries under heavier real-world loads
- validation throughput and report generation
  - faster robustness passes and report/export generation without weakening correctness
- GPU route maturity and route-policy refinement
  - clearer, more measurable route selection and route-fallback behavior across supported strategies
- artifact/history library scale improvements
  - indexing, filtering, hydration, reopen speed, and cross-run comparison usability
- release/productization hardening
  - public-repo readiness, packaging reliability, diagnostics, and desktop polish

Default posture for the post-checklist phase:

- performance and scale work should come before broad new-feature expansion
- larger functionality changes should only be admitted when they clearly strengthen the core workflow
- every new epic document should still preserve the same non-options, additive, no-regression policy used by the current program
