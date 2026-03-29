export type WorkspaceKey = 'workflow' | 'library' | 'aggregate' | 'settings'

export const shellSelectors = {
  titlebar: '[aria-label="Desktop titlebar"]',
  shellHeader: '[aria-label="Workspace shell header"]',
  workspaceBar: '#workspaceBar',
  runFooter: '#runFooterShell',
  settingsGrid: '#settingsShellGrid',
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
} as const

export function workspaceTabSelector(key: WorkspaceKey): string {
  return `#workspaceBar [data-workspace="${key}"]`
}
