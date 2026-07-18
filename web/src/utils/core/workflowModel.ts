import { match } from 'ts-pattern'

export const SHELL_MODES = ['conventional', 'options', 'ml-signal'] as const
export type ShellMode = (typeof SHELL_MODES)[number]
export const SHELL_MODE_SET: ReadonlySet<string> = new Set(SHELL_MODES)

export const WORKFLOW_TABS = ['data', 'optimize', 'validate', 'analyze', 'finalize', 'history', 'settings'] as const
export type WorkflowTab = (typeof WORKFLOW_TABS)[number]
export const WORKFLOW_TAB_SET: ReadonlySet<string> = new Set(WORKFLOW_TABS)

export const WORKFLOW_DEFAULTS = {
  data: { controls: 'data', results: 'overview' },
  optimize: { controls: 'opt', results: 'overview' },
  validate: { controls: 'opt', results: 'validate' },
  analyze: { controls: 'opt', results: 'drill' },
  finalize: { controls: 'advanced', results: 'final' },
  history: { controls: 'data', results: 'history' },
  settings: { controls: 'advanced', results: 'meta' },
} as const

export type WorkflowDefaultEntry = {
  controls: ControlsTab
  results: ResultsTab
}

export type WorkflowDefaultMap = Record<WorkflowTab, WorkflowDefaultEntry>

export const WORKFLOW_ALLOWED = {
  data: { controls: ['data'], results: ['overview'] },
  optimize: { controls: ['opt', 'strategy', 'advanced'], results: ['overview', 'top', 'pareto', 'heatmap'] },
  validate: { controls: [], results: ['validate'] },
  analyze: { controls: [], results: ['drill', 'top', 'pareto', 'heatmap', 'overview'] },
  finalize: { controls: [], results: ['final', 'meta'] },
  history: { controls: ['data'], results: ['history'] },
  settings: { controls: ['advanced'], results: ['meta', 'debug'] },
} as const

export type WorkflowAllowedEntry = {
  controls: readonly ControlsTab[]
  results: readonly ResultsTab[]
}

export type WorkflowAllowedMap = Record<WorkflowTab, WorkflowAllowedEntry>

export const ML_SIGNAL_WORKFLOW_DEFAULTS = {
  data: { controls: 'data', results: 'overview' },
  optimize: { controls: 'opt', results: 'overview' },
  validate: { controls: 'opt', results: 'overview' },
  analyze: { controls: 'opt', results: 'overview' },
  finalize: { controls: 'opt', results: 'overview' },
  history: { controls: 'data', results: 'history' },
  settings: { controls: 'advanced', results: 'meta' },
} as const

export const ML_SIGNAL_WORKFLOW_ALLOWED = {
  data: { controls: ['data'], results: ['overview'] },
  optimize: { controls: ['opt'], results: ['overview'] },
  validate: { controls: ['opt'], results: ['overview'] },
  analyze: { controls: ['opt'], results: ['overview'] },
  finalize: { controls: ['opt'], results: ['overview'] },
  history: { controls: ['data'], results: ['history'] },
  settings: { controls: ['advanced'], results: ['meta', 'debug'] },
} as const

export const OPTIONS_WORKFLOW_DEFAULTS = {
  data: { controls: 'data', results: 'options' },
  optimize: { controls: 'data', results: 'options' },
  validate: { controls: 'data', results: 'options' },
  analyze: { controls: 'data', results: 'options' },
  finalize: { controls: 'data', results: 'options' },
  history: { controls: 'data', results: 'history' },
  settings: { controls: 'advanced', results: 'meta' },
} as const

export const OPTIONS_WORKFLOW_ALLOWED = {
  data: { controls: ['data'], results: ['options'] },
  optimize: { controls: [], results: ['options'] },
  validate: { controls: [], results: ['options'] },
  analyze: { controls: [], results: ['options'] },
  finalize: { controls: [], results: ['options'] },
  history: { controls: ['data'], results: ['history'] },
  settings: { controls: ['advanced'], results: ['meta', 'debug'] },
} as const

export const WORKFLOW_SUBTITLES = {
  data: 'Load a dataset (CSV or Polygon) and select the optimization timeframe.',
  optimize: 'Configure strategy + search space, then run an optimization.',
  validate: 'Run robustness checks on the current run’s candidates.',
  analyze: 'Inspect results via candidate detail curves and heatmaps.',
  finalize: 'Review dataset, optimization, validation, meta-analysis, and handoff evidence before promotion.',
  history: 'Review, compare, and rerun saved optimizations.',
  settings: 'Advanced tools, meta selection, and debug output.',
} as const

export const WORKSPACE_TABS = ['workflow', 'library', 'aggregate', 'portfolio', 'pro', 'live', 'settings'] as const
export type WorkspaceTab = (typeof WORKSPACE_TABS)[number]
export const WORKSPACE_TAB_SET: ReadonlySet<string> = new Set(WORKSPACE_TABS)

export const WORKFLOW_STEPS = ['data', 'optimize', 'validate', 'analyze', 'finalize'] as const
export type WorkflowStep = (typeof WORKFLOW_STEPS)[number]
export const WORKFLOW_STEP_SET: ReadonlySet<string> = new Set(WORKFLOW_STEPS)

export const AGGREGATE_TABS = ['meta', 'batch', 'saved', 'compare'] as const
export type AggregateTab = (typeof AGGREGATE_TABS)[number]
export const AGGREGATE_TAB_SET: ReadonlySet<string> = new Set(AGGREGATE_TABS)

export const WORKFLOW_STEP_SUBTITLES = {
  data: 'Data source: load CSV or Polygon candles, then confirm timeframe.',
  optimize: 'Strategy setup: configure ranges, backend, and run optimization.',
  validate: 'Validation: run robustness checks on the selected optimization.',
  analyze: 'Summary: inspect top results, Pareto, candidate detail, and heatmap.',
  finalize: 'Final review: confirm evidence completeness, open meta-analysis, and hand off accepted artifacts.',
} as const

export const WORKFLOW_STEP_LABELS = {
  data: 'Data Source',
  optimize: 'Strategy Setup',
  validate: 'Validation',
  analyze: 'Summary',
  finalize: 'Final Review',
} as const

export const ML_SIGNAL_WORKFLOW_STEP_LABELS = {
  data: 'Data Setup',
  optimize: 'Score Cache',
  validate: 'Policy Validation',
  analyze: 'Policy Sweep',
  finalize: 'Artifacts',
} as const

export const OPTIONS_WORKFLOW_STEP_LABELS = {
  data: 'Underlying Data',
  optimize: 'Options Data',
  validate: 'Expression Run',
  analyze: 'Validation',
  finalize: 'Final Review',
} as const

export const ML_SIGNAL_WORKFLOW_STEP_SUBTITLES = {
  data: 'Load the parent dataset and confirm timeframe before exporting ML datasets or score-cache inputs.',
  optimize: 'Stage 2: build the ML score cache from exported data and model-bundle inputs.',
  validate: 'Stage 3: validate one policy against the current score cache and execution settings.',
  analyze: 'Stage 4: sweep policy thresholds and hold settings across the score cache.',
  finalize: 'Stage 5: inspect the current ML artifact, bundle, sweep, and verdict outputs.',
} as const

export const OPTIONS_WORKFLOW_STEP_SUBTITLES = {
  data: 'Load or confirm the underlying candle dataset before options expression research.',
  optimize: 'Import or download options data and choose the options realism lane.',
  validate: 'Run the constrained options expression search against the selected parent artifact and dataset.',
  analyze: 'Validate options expression artifacts and inspect scenario robustness.',
  finalize: 'Review options finalist evidence, exports, and replay provenance.',
} as const

export interface MlSignalPrimaryActionDescriptor {
  kind: 'dataset_export' | 'score_cache' | 'validation' | 'policy_sweep' | 'artifacts'
  label: string
  shortLabel: string
  actionable: boolean
}

export const ML_SIGNAL_PRIMARY_ACTIONS: Record<WorkflowStep, MlSignalPrimaryActionDescriptor> = {
  data: {
    kind: 'dataset_export',
    label: 'Export dataset',
    shortLabel: 'Export',
    actionable: true,
  },
  optimize: {
    kind: 'score_cache',
    label: 'Build score cache',
    shortLabel: 'Build',
    actionable: true,
  },
  validate: {
    kind: 'validation',
    label: 'Run ML validation',
    shortLabel: 'Validate',
    actionable: true,
  },
  analyze: {
    kind: 'policy_sweep',
    label: 'Run ML policy sweep',
    shortLabel: 'Sweep',
    actionable: true,
  },
  finalize: {
    kind: 'artifacts',
    label: 'Artifacts ready',
    shortLabel: 'Artifacts',
    actionable: false,
  },
}

export const WORKSPACE_SUBTITLES = {
  workflow: 'Research pipeline for data, optimization, validation, analysis, and finalization.',
  library: 'Browse, compare, and rerun saved optimization artifacts.',
  aggregate: 'Combine many runs into robust shortlist recommendations.',
  portfolio: 'Research portfolio admission evidence, replay gaps, and provenance for candidate shortlists.',
  pro: 'Diagnostics and advanced debug output.',
  live: 'Narrow live broker surface: single symbol, Alpaca broker, Polygon stocks feed, and one active runner/stream.',
  settings: 'Application preferences and storage configuration.',
} as const

export const CONTROLS_TAB_VALUES = ['data', 'opt', 'strategy', 'advanced'] as const
export type ControlsTab = (typeof CONTROLS_TAB_VALUES)[number]
export const CONTROLS_TAB_SET: ReadonlySet<string> = new Set(CONTROLS_TAB_VALUES)

export const RESULTS_TAB_VALUES = ['overview', 'top', 'pareto', 'validate', 'drill', 'heatmap', 'history', 'meta', 'debug', 'options', 'final'] as const
export type ResultsTab = (typeof RESULTS_TAB_VALUES)[number]
export const RESULTS_TAB_SET: ReadonlySet<string> = new Set(RESULTS_TAB_VALUES)

export const TAB_LABELS = {
  controls: {
    data: 'Data',
    opt: 'Optimize',
    strategy: 'Strategy',
    advanced: 'Advanced',
  },
  results: {
    overview: 'Overview',
    top: 'Top',
    pareto: 'Pareto',
    validate: 'Validate',
    drill: 'Candidate Detail',
    heatmap: 'Heatmap',
    history: 'History',
    meta: 'Meta',
    debug: 'Debug',
    options: 'Options',
    final: 'Final Review',
  },
} as const

export function normalizeWorkflowTab(value: unknown): WorkflowTab | '' {
  const normalized = String(value || '').trim().toLowerCase()
  return WORKFLOW_TAB_SET.has(normalized) ? (normalized as WorkflowTab) : ''
}

export function normalizeShellMode(value: unknown): ShellMode | '' {
  const normalized = String(value || '').trim().toLowerCase()
  return SHELL_MODE_SET.has(normalized) ? (normalized as ShellMode) : ''
}

export function normalizeWorkspaceTab(value: unknown): WorkspaceTab | '' {
  const normalized = String(value || '').trim().toLowerCase()
  return WORKSPACE_TAB_SET.has(normalized) ? (normalized as WorkspaceTab) : ''
}

export function normalizeWorkflowStep(value: unknown): WorkflowStep | '' {
  const normalized = String(value || '').trim().toLowerCase()
  return WORKFLOW_STEP_SET.has(normalized) ? (normalized as WorkflowStep) : ''
}

export function normalizeAggregateTab(value: unknown): AggregateTab | '' {
  const normalized = String(value || '').trim().toLowerCase()
  return AGGREGATE_TAB_SET.has(normalized) ? (normalized as AggregateTab) : ''
}

export function normalizeControlsTab(value: unknown): ControlsTab | '' {
  const normalized = String(value || '').trim().toLowerCase()
  return CONTROLS_TAB_SET.has(normalized) ? (normalized as ControlsTab) : ''
}

export function normalizeResultsTab(value: unknown): ResultsTab | '' {
  const normalized = String(value || '').trim().toLowerCase()
  return RESULTS_TAB_SET.has(normalized) ? (normalized as ResultsTab) : ''
}

export function workflowStepFromWorkflow(workflow: unknown): string {
  const normalized = normalizeWorkflowTab(workflow)
  return normalizeWorkflowStep(normalized) || 'data'
}

export function shellModeLabel(mode: unknown): string {
  const normalized = normalizeShellMode(mode)
  if (normalized === 'ml-signal') return 'ML Signal'
  if (normalized === 'options') return 'Options'
  return 'Conventional'
}

export function workflowDefaultsForMode(mode: unknown): WorkflowDefaultMap {
  const normalized = normalizeShellMode(mode)
  if (normalized === 'ml-signal') return ML_SIGNAL_WORKFLOW_DEFAULTS
  if (normalized === 'options') return OPTIONS_WORKFLOW_DEFAULTS
  return WORKFLOW_DEFAULTS
}

export function workflowAllowedForMode(mode: unknown): WorkflowAllowedMap {
  const normalized = normalizeShellMode(mode)
  if (normalized === 'ml-signal') return ML_SIGNAL_WORKFLOW_ALLOWED
  if (normalized === 'options') return OPTIONS_WORKFLOW_ALLOWED
  return WORKFLOW_ALLOWED
}

export function workflowStepLabelForMode(step: unknown, mode: unknown): string {
  const normalizedStep = normalizeWorkflowStep(step) || 'data'
  const normalizedMode = normalizeShellMode(mode)
  if (normalizedMode === 'ml-signal') {
    return ML_SIGNAL_WORKFLOW_STEP_LABELS[normalizedStep]
  }
  if (normalizedMode === 'options') {
    return OPTIONS_WORKFLOW_STEP_LABELS[normalizedStep]
  }
  return WORKFLOW_STEP_LABELS[normalizedStep]
}

export function workflowStepSubtitleForMode(step: unknown, mode: unknown): string {
  const normalizedStep = normalizeWorkflowStep(step) || 'data'
  const normalizedMode = normalizeShellMode(mode)
  if (normalizedMode === 'ml-signal') {
    return ML_SIGNAL_WORKFLOW_STEP_SUBTITLES[normalizedStep]
  }
  if (normalizedMode === 'options') {
    return OPTIONS_WORKFLOW_STEP_SUBTITLES[normalizedStep]
  }
  return WORKFLOW_STEP_SUBTITLES[normalizedStep]
}

export function mlSignalPrimaryActionForStep(step: unknown): MlSignalPrimaryActionDescriptor {
  const normalizedStep = normalizeWorkflowStep(step) || 'data'
  return ML_SIGNAL_PRIMARY_ACTIONS[normalizedStep]
}

export function workflowStepForMlSignalStage(stage: unknown): WorkflowStep {
  const normalizedStage = String(stage || '').trim().toLowerCase()
  return match(normalizedStage)
    .with('dataset_export', () => 'data' as const)
    .with('validation', () => 'validate' as const)
    .with('policy_sweep', () => 'analyze' as const)
    .with('score_cache', () => 'optimize' as const)
    .otherwise(() => 'finalize' as const)
}
