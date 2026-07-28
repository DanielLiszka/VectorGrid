import {
  type ShellMode,
  type WorkflowStep,
  mlSignalPrimaryActionForStep,
  normalizeShellMode,
  normalizeWorkflowStep,
  optionsPrimaryActionForStep,
} from './core/workflowModel'

export type WorkbenchPrimaryActionKind = 'cancel' | 'optimize' | 'validate' | 'ml_signal' | 'options' | 'inspect'
export type WorkbenchPrimaryActionTone = 'primary' | 'danger' | 'subtle'

export interface WorkbenchPrimaryActionInput {
  canCancel?: boolean
  mode?: unknown
  workflowStep?: unknown
  dataLoaded?: boolean
  hasOptimizationResults?: boolean
  runDisabled?: boolean
  validateDisabled?: boolean
  runReadinessHint?: string
  validationHint?: string
  validationStage?: string
  mlSignalDatasetExportPath?: string
  mlSignalScoreInputPath?: string
  mlSignalScoreCachePath?: string
  mlSignalScoreSource?: string
  mlSignalModelBundleId?: string
  optionsDatasetReady?: boolean
  optionsArtifactReady?: boolean
  optionsValidationReady?: boolean
  optionsRunBusy?: boolean
  optionsValidationBusy?: boolean
  workspace?: unknown
}

export interface WorkbenchPrimaryAction {
  kind: WorkbenchPrimaryActionKind
  label: string
  disabled: boolean
  disabledReason: string
  tone: WorkbenchPrimaryActionTone
}

function normalizeMode(value: unknown): ShellMode {
  return normalizeShellMode(value) || 'conventional'
}

function normalizeStep(value: unknown): WorkflowStep {
  return normalizeWorkflowStep(value) || 'data'
}

function hasText(value: unknown): boolean {
  return String(value ?? '').trim().length > 0
}

function isNativeScoreSource(value: unknown): boolean {
  return String(value ?? '').trim().toLowerCase() === 'native'
}

export function mlSignalScoreCacheDisabledReason(input: WorkbenchPrimaryActionInput): string {
  if (isNativeScoreSource(input.mlSignalScoreSource)) {
    if (!hasText(input.mlSignalModelBundleId)) return 'Choose an ML model bundle before native scoring.'
  } else if (!hasText(input.mlSignalScoreInputPath)) {
    return 'Choose a score input path before building the cache.'
  }
  return hasText(input.mlSignalScoreCachePath) ? '' : 'Choose a score-cache output path before building the cache.'
}

function validationDisabledReason(input: WorkbenchPrimaryActionInput): string {
  const hint = String(input.validationHint || '').trim()
  if (!input.dataLoaded) return 'Load a dataset before running validation.'
  if (!input.hasOptimizationResults) return 'Run an optimization before running validation.'
  if (hint && hint !== '—' && hint !== '-') return hint
  if (input.validationStage !== 'ready') return 'Select or review an optimization candidate before validation.'
  return 'Validation is not available for the current state.'
}

function optimizationDisabledReason(input: WorkbenchPrimaryActionInput): string {
  if (!input.dataLoaded) return 'Load a dataset before running optimization.'
  return String(input.runReadinessHint || '').trim() || 'Configure data and strategy inputs before running.'
}

function mlSignalDisabledReason(input: WorkbenchPrimaryActionInput, step: WorkflowStep): string {
  if (!input.dataLoaded) return 'Load a dataset before running this ML Signal stage.'
  const descriptor = mlSignalPrimaryActionForStep(step)
  switch (descriptor.kind) {
    case 'dataset_export':
      return hasText(input.mlSignalDatasetExportPath) ? '' : 'Choose a dataset export path before exporting.'
    case 'score_cache':
      return mlSignalScoreCacheDisabledReason(input)
    case 'validation':
    case 'policy_sweep':
      return hasText(input.mlSignalScoreCachePath) ? '' : 'Build or select a score cache before this stage.'
    case 'artifacts':
      return 'Current ML artifacts are inspect-only from this stage.'
  }
}

export function resolveWorkbenchPrimaryAction(input: WorkbenchPrimaryActionInput): WorkbenchPrimaryAction {
  if (input.canCancel) {
    return {
      kind: 'cancel',
      label: 'Stop run',
      disabled: false,
      disabledReason: '',
      tone: 'danger',
    }
  }

  if (hasText(input.workspace) && String(input.workspace).trim().toLowerCase() !== 'workflow') {
    return {
      kind: 'inspect',
      label: 'Review workspace',
      disabled: true,
      disabledReason: 'Primary run actions are available from the Research workspace.',
      tone: 'subtle',
    }
  }

  const mode = normalizeMode(input.mode)
  const step = normalizeStep(input.workflowStep)

  if (mode === 'ml-signal') {
    const descriptor = mlSignalPrimaryActionForStep(step)
    const specificReason = mlSignalDisabledReason(input, step)
    const backendBlocked = descriptor.kind === 'validation' || descriptor.kind === 'policy_sweep'
      ? input.validateDisabled === true
      : input.runDisabled === true
    const disabled = !descriptor.actionable || specificReason.length > 0 || backendBlocked
    return {
      kind: descriptor.actionable ? 'ml_signal' : 'inspect',
      label: descriptor.label,
      disabled,
      disabledReason: disabled
        ? specificReason || String(input.runReadinessHint || '').trim() || 'This ML Signal stage is not ready.'
        : '',
      tone: descriptor.actionable ? 'primary' : 'subtle',
    }
  }

  if (mode === 'options') {
    const descriptor = optionsPrimaryActionForStep(step)
    if (!input.dataLoaded) {
      return {
        kind: 'inspect',
        label: descriptor.label,
        disabled: true,
        disabledReason: 'Load the underlying dataset used by the parent artifact before continuing.',
        tone: 'subtle',
      }
    }
    if (!descriptor.actionable) {
      const reason = descriptor.kind === 'underlying'
        ? 'Confirm the underlying dataset and parent artifact in this stage.'
        : descriptor.kind === 'options_data'
          ? 'Import or select an options dataset with the controls in this stage.'
          : 'Review finalist evidence and exports in this stage.'
      return {
        kind: 'inspect',
        label: descriptor.label,
        disabled: true,
        disabledReason: reason,
        tone: 'subtle',
      }
    }
    const expressionStage = descriptor.kind === 'expression'
    const busy = expressionStage ? input.optionsRunBusy : input.optionsValidationBusy
    const ready = expressionStage
      ? input.optionsDatasetReady
      : input.optionsArtifactReady && input.optionsValidationReady !== false
    return {
      kind: 'options',
      label: descriptor.label,
      disabled: !!busy || !ready,
      disabledReason: busy
        ? `${descriptor.label} is already in progress.`
        : ready
          ? ''
          : expressionStage
            ? 'Select an options dataset before running the expression search.'
            : !input.optionsArtifactReady
              ? 'Run or load an options expression artifact before validation.'
              : 'Complete the options validation setup before running validation.',
      tone: 'primary',
    }
  }

  if (step === 'validate') {
    const disabled = input.validateDisabled === true
    return {
      kind: 'validate',
      label: 'Run validation',
      disabled,
      disabledReason: disabled ? validationDisabledReason(input) : '',
      tone: 'primary',
    }
  }

  const disabled = input.runDisabled === true || !input.dataLoaded
  return {
    kind: 'optimize',
    label: 'Run optimization',
    disabled,
    disabledReason: disabled ? optimizationDisabledReason(input) : '',
    tone: 'primary',
  }
}
