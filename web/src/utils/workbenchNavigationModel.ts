import {
  TAB_LABELS,
  WORKFLOW_STEPS,
  type ControlsTab,
  type ResultsTab,
  type ShellMode,
  type WorkflowStep,
  type WorkspaceTab,
  normalizeControlsTab,
  normalizeResultsTab,
  normalizeShellMode,
  normalizeWorkflowStep,
  normalizeWorkspaceTab,
  shellModeLabel,
  workflowAllowedForMode,
  workflowStepLabelForMode,
} from './core/workflowModel'

export type WorkbenchWorkspaceId = WorkspaceTab

export interface WorkbenchNavigationInput {
  workspace?: unknown
  mode?: unknown
  workflowStep?: unknown
  controlsTab?: unknown
  resultsTab?: unknown
}

export interface WorkbenchNavigationItem {
  id: string
  label: string
  shortLabel: string
  description: string
  icon: string
  actionId: string
  tabId: string
  ariaControls: string
  selected: boolean
  secondary: boolean
}

export interface WorkbenchLocalTabItem<T extends ControlsTab | ResultsTab = ControlsTab | ResultsTab> {
  id: T
  label: string
  actionId: string
  tabId: string
  panelId: string
  selected: boolean
}

export interface WorkbenchNavigationState {
  mode: ShellMode
  modeLabel: string
  activeWorkspaceId: WorkbenchWorkspaceId
  activeWorkflowStep: WorkflowStep
  activeControlsTab: ControlsTab | ''
  activeResultsTab: ResultsTab | ''
  workspaceItems: WorkbenchNavigationItem[]
  stageItems: WorkbenchNavigationItem[]
  controlsItems: WorkbenchLocalTabItem<ControlsTab>[]
  resultsItems: WorkbenchLocalTabItem<ResultsTab>[]
  selectedShellTabId: string
  selectionAnnouncement: string
}

const DEFAULT_MODE: ShellMode = 'conventional'
const DEFAULT_WORKSPACE: WorkbenchWorkspaceId = 'workflow'
const DEFAULT_STEP: WorkflowStep = 'data'

const WORKBENCH_WORKSPACES: Array<{
  id: WorkbenchWorkspaceId
  label: string
  shortLabel: string
  description: string
  icon: string
  secondary?: boolean
}> = [
  {
    id: 'workflow',
    label: 'Research',
    shortLabel: 'Research',
    description: 'Data, optimization, validation, analysis, and finalization.',
    icon: 'chart',
  },
  {
    id: 'library',
    label: 'Library',
    shortLabel: 'Library',
    description: 'Saved runs, comparisons, and follow-up artifacts.',
    icon: 'library',
  },
  {
    id: 'aggregate',
    label: 'Runs',
    shortLabel: 'Runs',
    description: 'Batch and meta-analysis surfaces.',
    icon: 'runs',
  },
  {
    id: 'live',
    label: 'Live',
    shortLabel: 'Live',
    description: 'Broker review, live feed, and runner controls.',
    icon: 'live',
  },
  {
    id: 'settings',
    label: 'Settings',
    shortLabel: 'Settings',
    description: 'Preferences, storage, diagnostics, and desktop settings.',
    icon: 'settings',
  },
  {
    id: 'pro',
    label: 'Diagnostics',
    shortLabel: 'Diag',
    description: 'Advanced debug and verification output.',
    icon: 'diagnostics',
    secondary: true,
  },
]

function workspaceTabId(key: string): string {
  return `workspace_tab_${key}`
}

function workflowStepTabId(key: string): string {
  return `workflow_step_tab_${key}`
}

function controlTabId(key: string): string {
  return `controls_tab_${key}`
}

function resultTabId(key: string): string {
  return `results_tab_${key}`
}

function workspaceActionId(key: WorkbenchWorkspaceId): string {
  return key === 'pro' ? 'shell.workspace.diagnostics' : `shell.workspace.${key}`
}

function normalizeMode(value: unknown): ShellMode {
  return normalizeShellMode(value) || DEFAULT_MODE
}

function normalizeWorkspace(value: unknown): WorkbenchWorkspaceId {
  return normalizeWorkspaceTab(value) || DEFAULT_WORKSPACE
}

function normalizeStep(value: unknown): WorkflowStep {
  return normalizeWorkflowStep(value) || DEFAULT_STEP
}

function controlsForState(mode: ShellMode, workspace: WorkbenchWorkspaceId, step: WorkflowStep): ControlsTab[] {
  if (workspace === 'pro') return ['advanced']
  if (workspace !== 'workflow') return []
  return [...workflowAllowedForMode(mode)[step].controls]
}

function resultsForState(mode: ShellMode, workspace: WorkbenchWorkspaceId, step: WorkflowStep): ResultsTab[] {
  if (workspace === 'library') return ['history']
  if (workspace === 'aggregate') return ['meta']
  if (workspace === 'pro') return ['debug']
  if (workspace !== 'workflow') return []
  if (mode === 'ml-signal') return []
  return [...workflowAllowedForMode(mode)[step].results]
}

function activeControls(value: unknown, items: ControlsTab[]): ControlsTab | '' {
  const normalized = normalizeControlsTab(value)
  if (normalized && items.includes(normalized)) return normalized
  return items[0] ?? ''
}

function activeResults(value: unknown, items: ResultsTab[]): ResultsTab | '' {
  const normalized = normalizeResultsTab(value)
  if (normalized && items.includes(normalized)) return normalized
  return items[0] ?? ''
}

export function workbenchWorkspaceLabel(workspace: unknown): string {
  const normalized = normalizeWorkspace(workspace)
  return WORKBENCH_WORKSPACES.find((item) => item.id === normalized)?.label ?? 'Research'
}

export function buildWorkbenchNavigationState(input: WorkbenchNavigationInput): WorkbenchNavigationState {
  const mode = normalizeMode(input.mode)
  const activeWorkspaceId = normalizeWorkspace(input.workspace)
  const activeWorkflowStep = normalizeStep(input.workflowStep)
  const controls = controlsForState(mode, activeWorkspaceId, activeWorkflowStep)
  const results = resultsForState(mode, activeWorkspaceId, activeWorkflowStep)
  const activeControlsTab = activeControls(input.controlsTab, controls)
  const activeResultsTab = activeResults(input.resultsTab, results)

  const workspaceItems = WORKBENCH_WORKSPACES.map((item) => ({
    id: item.id,
    label: item.label,
    shortLabel: item.shortLabel,
    description: item.description,
    icon: item.icon,
    actionId: workspaceActionId(item.id),
    tabId: workspaceTabId(item.id),
    ariaControls: 'shellMainPanel',
    selected: activeWorkspaceId === item.id,
    secondary: item.secondary === true,
  }))

  const stageItems = WORKFLOW_STEPS.map((step, index) => ({
    id: step,
    label: `${index + 1} ${workflowStepLabelForMode(step, mode)}`,
    shortLabel: workflowStepLabelForMode(step, mode),
    description: `${workflowStepLabelForMode(step, mode)} workflow step.`,
    icon: step,
    actionId: `shell.workflowStep.${step}`,
    tabId: workflowStepTabId(step),
    ariaControls: 'shellMainPanel',
    selected: activeWorkspaceId === 'workflow' && activeWorkflowStep === step,
    secondary: false,
  }))

  const controlsItems = controls.map((tab) => ({
    id: tab,
    label: TAB_LABELS.controls[tab],
    actionId: tab === 'opt' ? 'shell.controlsTab.optimize' : `shell.controlsTab.${tab}`,
    tabId: controlTabId(tab),
    panelId: `controls_panel_${tab}`,
    selected: activeControlsTab === tab,
  }))

  const resultsItems = results.map((tab) => ({
    id: tab,
    label: TAB_LABELS.results[tab],
    actionId: `shell.resultsTab.${tab}`,
    tabId: resultTabId(tab),
    panelId: `results_panel_${tab}`,
    selected: activeResultsTab === tab,
  }))

  const modeLabel = shellModeLabel(mode)
  const workspaceLabel = workbenchWorkspaceLabel(activeWorkspaceId)
  const selectedShellTabId =
    activeWorkspaceId === 'workflow' ? workflowStepTabId(activeWorkflowStep) : workspaceTabId(activeWorkspaceId)
  const parts = [`Workspace ${workspaceLabel}.`]
  if (activeWorkspaceId === 'workflow') {
    parts.push(`Mode ${modeLabel}.`)
    parts.push(`Workflow step ${stageItems.find((item) => item.id === activeWorkflowStep)?.label ?? '1 Data Source'}.`)
  }
  const activeControl = controlsItems.find((item) => item.selected)
  const activeResult = resultsItems.find((item) => item.selected)
  if (activeControl) parts.push(`Setup panel ${activeControl.label}.`)
  if (activeResult) parts.push(`Results panel ${activeResult.label}.`)

  return {
    mode,
    modeLabel,
    activeWorkspaceId,
    activeWorkflowStep,
    activeControlsTab,
    activeResultsTab,
    workspaceItems,
    stageItems,
    controlsItems,
    resultsItems,
    selectedShellTabId,
    selectionAnnouncement: parts.join(' '),
  }
}
