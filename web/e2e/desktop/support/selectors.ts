export type WorkspaceKey = 'workflow' | 'library' | 'aggregate' | 'portfolio' | 'pro' | 'live' | 'settings'

export const shellSelectors = {
  titlebar: '[aria-label="Desktop titlebar"]',
  shellHeader: '[aria-label="Workspace shell header"]',
  workspaceBar: '#workspaceBar',
  workflowStepBar: '#workflowStepBar',
  runFooter: '#runFooterShell',
  liveGrid: '#liveShellGrid',
  settingsGrid: '#settingsShellGrid',
  settingsDiagnosticsSection: '[aria-label="Desktop settings diagnostics"]',
  settingsDiagnosticsRuntimeCard: '[aria-label="Runtime diagnostics card"]',
  settingsDiagnosticsNativeDesktopCard: '[aria-label="Native desktop diagnostics card"]',
  settingsDiagnosticsDesktopPresenceCard: '[aria-label="Desktop presence diagnostics card"]',
  settingsDiagnosticsTaskPostureCard: '[aria-label="Task posture diagnostics card"]',
  settingsDiagnosticsDesktopLogsCard: '[aria-label="Desktop logs diagnostics card"]',
  settingsDesktopNotificationPermission: '[aria-label="Desktop notification permission"]',
  settingsDesktopUpdaterStatus: '[aria-label="Desktop updater status"]',
  settingsDesktopWindowStateSummary: '[aria-label="Desktop window state summary"]',
  settingsDesktopPresenceSummary: '[aria-label="Desktop presence summary"]',
  settingsOpenTaskCenterButton: 'button[aria-label="Open task center"]',
  settingsCopyDiagnosticsButton: 'button[aria-label="Copy diagnostics snapshot"]',
  libraryPanelTitle: '.historyPanelVueRoot .panelTitle',
  aggregatePanelTitle: '.metaPanelVueRoot .panelTitle',
  librarySessionRuns: '[aria-label="Session history runs"]',
  aggregateSections: '[aria-label="Meta analysis sections"]',
  commandPaletteButton: '[aria-label="Open command palette"]',
  settingsWorkspaceButton: '[aria-label="Open settings workspace"]',
  minimizeWindowButton: '[aria-label="Minimize window"]',
  maximizeWindowButton: '[aria-label="Maximize window"]',
  restoreWindowButton: '[aria-label="Restore window"]',
  closeWindowButton: '[aria-label="Close window"]',
} as const

export const commandPaletteSelectors = {
  dialog: '[aria-label="Command palette"]',
  searchInput: '[aria-label="Command search"]',
  listbox: '[aria-label="Available commands"]',
} as const

export const taskCenterSelectors = {
  launcher: 'button[aria-controls="vta-task-center-panel"][aria-haspopup="dialog"]',
  dialog: '[aria-label="Task center"]',
  activePostureSection: '[aria-label="Task center active posture"]',
  recentOutcomesSection: '[aria-label="Task center recent outcomes"]',
  eventsSection: '[aria-label="Task center events"]',
  timelineSection: '[aria-label="Task center timeline"]',
} as const

export const workbenchSelectors = {
  root: '[data-qa-shell="workbench"]',
  rail: '[data-qa-panel="workbench.rail"]',
  researchCockpit: '[data-qa-panel="workbench.researchCockpit"]',
  inspector: '[data-qa-panel="workbench.inspector"]',
  actionBar: '[data-qa-panel="workbench.actionBar"]',
  inspectorActiveMode: '[data-qa-state="workbench.inspector.activeMode"]',
  inspectorToggle: '[data-qa-action="workbench.inspector.toggle"]',
  inspectorSection: (id: string) => `[data-qa-state="workbench.inspector.section.${id}"]`,
  cockpitCard: (id: string) => `[data-qa-state="workbench.cockpit.card.${id}"]`,
} as const

export const nativeWorkflowSelectors = {
  conventionalMode: '[data-qa-action="shell.mode.conventional"]',
  dataStep: '[data-qa-action="shell.workflowStep.data"]',
  validationStep: '[data-qa-action="shell.workflowStep.validate"]',
  optimizeStep: '[data-qa-action="shell.workflowStep.optimize"]',
  sampleDatasetButton: '[data-qa-action="data.sample.load"]',
  clearDatasetButton: '[data-qa-action="data.dataset.clear"]',
  csvPath: '#vueCsvPath',
  strategyKind: '#vueStrategyKind',
  backend: '#vueBackend',
  mode: '#vueMode',
  fastStart: '#vueFastStart',
  fastEnd: '#vueFastEnd',
  fastStep: '#vueFastStep',
  slowStart: '#vueSlowStart',
  slowEnd: '#vueSlowEnd',
  slowStep: '#vueSlowStep',
  runOptimization: '[data-qa-action="optimize.run"]',
  cancelOptimization: '[data-qa-action="footer.primaryAction.run"]',
  advancedControlsTab: '#controlsTabBar [data-tab="advanced"]',
  exportAllCsvPath: '#vueExportAllCsvPath',
  exportAllCsvBrowse: '#vueExportAllCsvPath + button',
  overviewResultsTab: '#resultsTabBar [data-tab="overview"]',
  overviewRouteCard: '#results_panel_overview .bestCard',
  topResultsTab: '[data-qa-action="shell.resultsTab.top"]',
  topResultsPanel: '#results_panel_top',
  firstTopResult: '.topTableVue [data-top-row="1"]',
  validationAutoRun: '//label[contains(., "Auto-run after optimization")]//input[@type="checkbox"]',
  validationCandidates: '#vueValCandidates',
  validationMinPsr: '#vueValMinPsr',
  validationMinDsr: '#vueValMinDsr',
  validationMinCscvRank: '#vueValMinCscvRank',
  validationMaxPbo: '#vueValMaxPbo',
  validationRun: '[data-qa-action="validation.run"]',
  firstValidationResult: '.validationTableVue [data-validation-row-index="0"]',
  validationAutoExportXlsx:
    '//label[contains(., "Auto-export report XLSX after validation")]//input[@type="checkbox"]',
  validationAutoRevealReport:
    '//label[contains(., "Reveal report in folder after save")]//input[@type="checkbox"]',
  validationWalkForward:
    '//label[contains(., "Walk-forward (recommended)")]//input[@type="checkbox"]',
  validationPerturbations:
    '//label[contains(., "Parameter perturbations")]//input[@type="checkbox"]',
  validationMonteCarlo:
    '//label[contains(., "Trade bootstrap (Monte Carlo)")]//input[@type="checkbox"]',
  validationCscv:
    '//label[contains(., "CSCV/PBO (recommended)")]//input[@type="checkbox"]',
  validationSpa:
    '//label[contains(., "Hansen SPA")]//input[@type="checkbox"]',
  validationReoptWalkForward:
    '//label[contains(., "Finalist re-opt walk-forward")]//input[@type="checkbox"]',
  savedRunsRefresh: '[data-qa-action="library.savedRun.refresh"]',
  savedRunsSearch: '#vueSavedRunsSearch',
  savedRunList: '[aria-label="Saved run artifacts"]',
  firstSavedRun: '[aria-label="Saved run artifacts"] [data-saved-run="1"]',
  savedRunWithValidation:
    '//ul[@aria-label="Saved run artifacts"]//button[@data-saved-run="1" and .//span[normalize-space()="validation"]]',
  savedRunDetail: '.historyPanelVueRoot .detailCard',
  savedRunLoad: '[data-qa-action="library.savedRun.load"]',
} as const

export function workspaceTabSelector(key: WorkspaceKey): string {
  return `#workspaceBar [data-workspace="${key}"]`
}
