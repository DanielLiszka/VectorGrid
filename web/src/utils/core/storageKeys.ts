export const CONTROLS_TAB_KEY = 'vectorgrid_controls_tab_v1'
export const LEGACY_CONTROLS_TAB_KEY = 'vectorbt_controls_tab_v1'
export const RESULTS_TAB_KEY = 'vectorgrid_results_tab_v1'
export const LEGACY_RESULTS_TAB_KEY = 'vectorbt_results_tab_v1'
export const WORKFLOW_TAB_KEY = 'vta_workflow_tab_v1'
export const WORKFLOW_STATE_KEY = 'vta_workflow_state_v1'
export const WORKSPACE_TAB_KEY = 'vta_workspace_tab_v1'
export const WORKFLOW_STEP_KEY = 'vta_workflow_step_v1'
export const WORKFLOW_ADVANCED_KEY = 'vta_workflow_advanced_v1'
export const AGGREGATE_TAB_KEY = 'vta_aggregate_tab_v1'
export const CONTROLS_WIDTH_KEY = 'vta_controls_width_v1'
export const CONTROLS_COLLAPSED_KEY = 'vta_controls_collapsed_v1'
export const WINDOW_EFFECT_KEY = 'vta_window_effect_v1'
export const THEME_PREF_KEY = 'vta_theme_pref_v1'
export const HELP_MODE_KEY = 'vta_help_mode_v1'
export const SETTINGS_KEY = 'vectorgrid_settings_v1'
export const LEGACY_SETTINGS_KEY = 'vectorbt_settings_v1'
export const HISTORY_KEY = 'vectorgrid_history_v1'
export const LEGACY_HISTORY_KEY = 'vectorbt_history_v1'
export const RECENT_CSV_KEY = 'vectorgrid_recent_csv_v1'
export const LEGACY_RECENT_CSV_KEY = 'vectorbt_recent_csv_v1'
export const LAST_BATCH_SUMMARY_KEY = 'vectorgrid_last_batch_summary_path_v1'
export const LEGACY_LAST_BATCH_SUMMARY_KEY = 'vectorbt_last_batch_summary_path_v1'
export const EXEC_KEYS_STORAGE_KEY = 'vectorgrid_exec_keys_v1'
export const LEGACY_EXEC_KEYS_STORAGE_KEY = 'vectorbt_exec_keys_v1'
export const DIST_SETTINGS_KEY = 'vta_dist_settings_v1'

export const STORAGE_KEY_ALIASES: Record<string, readonly string[]> = {
  [CONTROLS_TAB_KEY]: [LEGACY_CONTROLS_TAB_KEY],
  [RESULTS_TAB_KEY]: [LEGACY_RESULTS_TAB_KEY],
  [SETTINGS_KEY]: [LEGACY_SETTINGS_KEY],
  [HISTORY_KEY]: [LEGACY_HISTORY_KEY],
  [RECENT_CSV_KEY]: [LEGACY_RECENT_CSV_KEY],
  [LAST_BATCH_SUMMARY_KEY]: [LEGACY_LAST_BATCH_SUMMARY_KEY],
  [EXEC_KEYS_STORAGE_KEY]: [LEGACY_EXEC_KEYS_STORAGE_KEY],
}
