export type SharedShellShortcutGroup = 'actions' | 'data' | 'exports' | 'layout'
export type SharedShellShortcutId =
  | 'run'
  | 'validate'
  | 'browse_csv'
  | 'open_polygon'
  | 'export_csv'
  | 'export_json'
  | 'open_latest_report'
  | 'open_latest_meta_report'
  | 'toggle_controls'

export interface SharedShellShortcutContext {
  canRun: boolean
  canValidate: boolean
  canBrowseCsv: boolean
  canPolygonOpen: boolean
  canExportCsv: boolean
  canExportJson: boolean
  canOpenLatestReport: boolean
  canOpenLatestMetaReport: boolean
  browseCsv: () => void
  openPolygon: () => void
  toggleControls: () => void
  runOptimization: () => void
  validateOptimization: () => void
  exportTopCsv: () => void
  exportTopJson: () => void
  openLatestReport: () => void
  openLatestMetaReport: () => void
}

export interface SharedShellShortcutCommand {
  id: SharedShellShortcutId
  group: SharedShellShortcutGroup
  name: string
  desc: string
  keys: string[]
  keywords: string[]
  enabled: boolean
  run: () => void
}

export interface ShellShortcutSummaryRow {
  keysLabel: string
  label: string
}

interface SharedShellShortcutDef {
  id: SharedShellShortcutId
  group: SharedShellShortcutGroup
  name: string
  desc: string
  keys: string[]
  keywords: string[]
  enabled: (ctx: SharedShellShortcutContext) => boolean
  run: (ctx: SharedShellShortcutContext) => void
  matches: (event: KeyboardEvent) => boolean
}

function hasMod(event: KeyboardEvent): boolean {
  return !!(event.ctrlKey || event.metaKey)
}

function lowerKey(event: KeyboardEvent): string {
  return String(event.key || '').toLowerCase()
}

function matchesModifiedKey(event: KeyboardEvent, key: string, shift = false): boolean {
  return hasMod(event) && lowerKey(event) === key && (!!event.shiftKey === shift)
}

const SHARED_SHELL_SHORTCUT_DEFS: readonly SharedShellShortcutDef[] = [
  {
    id: 'run',
    group: 'actions',
    name: 'Run optimization',
    desc: 'Start optimization for current setup.',
    keys: ['Ctrl', 'Enter'],
    keywords: ['run', 'optimize', 'backtest'],
    enabled: (ctx) => ctx.canRun,
    run: (ctx) => ctx.runOptimization(),
    matches: (event) => hasMod(event) && String(event.key || '') === 'Enter',
  },
  {
    id: 'validate',
    group: 'actions',
    name: 'Run validation',
    desc: 'Execute validation checks for shortlisted candidates.',
    keys: ['Ctrl', 'Shift', 'V'],
    keywords: ['validate', 'robustness', 'shortlist'],
    enabled: (ctx) => ctx.canValidate,
    run: (ctx) => ctx.validateOptimization(),
    matches: (event) => matchesModifiedKey(event, 'v', true),
  },
  {
    id: 'browse_csv',
    group: 'data',
    name: 'Browse CSV',
    desc: 'Pick dataset CSV from disk.',
    keys: ['Ctrl', 'O'],
    keywords: ['csv', 'data', 'browse'],
    enabled: (ctx) => ctx.canBrowseCsv,
    run: (ctx) => ctx.browseCsv(),
    matches: (event) => matchesModifiedKey(event, 'o', false),
  },
  {
    id: 'open_polygon',
    group: 'data',
    name: 'Open Polygon Downloader',
    desc: 'Open Polygon dataset downloader modal.',
    keys: ['Ctrl', 'Shift', 'O'],
    keywords: ['polygon', 'download', 'market'],
    enabled: (ctx) => ctx.canPolygonOpen,
    run: (ctx) => ctx.openPolygon(),
    matches: (event) => matchesModifiedKey(event, 'o', true),
  },
  {
    id: 'export_csv',
    group: 'exports',
    name: 'Export Top CSV',
    desc: 'Export current top table as CSV.',
    keys: ['Ctrl', 'Shift', 'C'],
    keywords: ['export', 'csv', 'top'],
    enabled: (ctx) => ctx.canExportCsv,
    run: (ctx) => ctx.exportTopCsv(),
    matches: (event) => matchesModifiedKey(event, 'c', true),
  },
  {
    id: 'export_json',
    group: 'exports',
    name: 'Export Top JSON',
    desc: 'Export current top result as JSON.',
    keys: ['Ctrl', 'Shift', 'J'],
    keywords: ['export', 'json', 'top'],
    enabled: (ctx) => ctx.canExportJson,
    run: (ctx) => ctx.exportTopJson(),
    matches: (event) => matchesModifiedKey(event, 'j', true),
  },
  {
    id: 'open_latest_report',
    group: 'exports',
    name: 'Open latest report',
    desc: 'Open most recent generated report.',
    keys: ['Ctrl', 'Shift', 'R'],
    keywords: ['report', 'xlsx'],
    enabled: (ctx) => ctx.canOpenLatestReport,
    run: (ctx) => ctx.openLatestReport(),
    matches: (event) => matchesModifiedKey(event, 'r', true),
  },
  {
    id: 'open_latest_meta_report',
    group: 'exports',
    name: 'Open latest meta report',
    desc: 'Open most recent meta-analysis report.',
    keys: ['Ctrl', 'Shift', 'M'],
    keywords: ['meta', 'report', 'xlsx'],
    enabled: (ctx) => ctx.canOpenLatestMetaReport,
    run: (ctx) => ctx.openLatestMetaReport(),
    matches: (event) => matchesModifiedKey(event, 'm', true),
  },
  {
    id: 'toggle_controls',
    group: 'layout',
    name: 'Toggle controls panel',
    desc: 'Collapse or expand controls column.',
    keys: ['Ctrl', '\\'],
    keywords: ['controls', 'sidebar', 'layout'],
    enabled: () => true,
    run: (ctx) => ctx.toggleControls(),
    matches: (event) => hasMod(event) && String(event.key || '') === '\\',
  },
]

function keysLabel(keys: string[]): string {
  return keys.join('+')
}

export const SETTINGS_SHORTCUT_SUMMARY_ROWS: readonly ShellShortcutSummaryRow[] = [
  {
    keysLabel: 'Ctrl+K',
    label: 'Command palette',
  },
  ...(['run', 'validate', 'export_csv', 'browse_csv'] as const).map((id) => {
    const match = SHARED_SHELL_SHORTCUT_DEFS.find((definition) => definition.id === id)
    if (!match) {
      throw new Error(`Missing shell shortcut definition for summary row: ${id}`)
    }
    return {
      keysLabel: keysLabel(match.keys),
      label: match.name,
    }
  }),
]

export function createSharedShellShortcutCommands(
  ctx: SharedShellShortcutContext,
): SharedShellShortcutCommand[] {
  return SHARED_SHELL_SHORTCUT_DEFS.map((definition) => ({
    id: definition.id,
    group: definition.group,
    name: definition.name,
    desc: definition.desc,
    keys: [...definition.keys],
    keywords: [...definition.keywords],
    enabled: definition.enabled(ctx),
    run: () => definition.run(ctx),
  }))
}

export function dispatchSharedShellShortcut(
  event: KeyboardEvent,
  ctx: SharedShellShortcutContext,
): boolean {
  for (const definition of SHARED_SHELL_SHORTCUT_DEFS) {
    if (!definition.matches(event)) continue
    event.preventDefault()
    if (definition.enabled(ctx)) definition.run(ctx)
    return true
  }
  return false
}
