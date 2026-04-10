export type WorkspaceKey = 'workflow' | 'library' | 'aggregate' | 'pro' | 'settings'

export const shellSelectors = {
  titlebar: '[aria-label="Desktop titlebar"]',
  shellHeader: '[aria-label="Workspace shell header"]',
  workspaceBar: '#workspaceBar',
  runFooter: '#runFooterShell',
  settingsGrid: '#settingsShellGrid',
  settingsDiagnosticsSection: '[aria-label="Desktop settings diagnostics"]',
  settingsDiagnosticsRuntimeCard: '[aria-label="Runtime diagnostics card"]',
  settingsDiagnosticsNativeDesktopCard: '[aria-label="Native desktop diagnostics card"]',
  settingsDiagnosticsDesktopPresenceCard: '[aria-label="Desktop presence diagnostics card"]',
  settingsDiagnosticsTaskPostureCard: '[aria-label="Task posture diagnostics card"]',
  settingsDiagnosticsDesktopLogsCard: '[aria-label="Desktop logs diagnostics card"]',
  settingsDesktopNotificationPermission: '[aria-label="Desktop notification permission"]',
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

export function workspaceTabSelector(key: WorkspaceKey): string {
  return `#workspaceBar [data-workspace="${key}"]`
}
