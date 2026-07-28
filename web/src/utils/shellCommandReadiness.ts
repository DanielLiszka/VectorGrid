import { activeRunCompatibility } from './activeContextCompatibility'

export type ResultCommandId = 'run' | 'validate' | 'export_csv' | 'export_json'

export interface ShellResultCommandReadinessInput {
  workspace: unknown
  mode: unknown
  datasetReady: boolean
  activeDataId: unknown
  activeDatasetPath?: unknown
  activeTimeframe?: unknown
  currentRun: unknown
  runDisabled: boolean
  validateDisabled: boolean
  canExportCsv: boolean
  canExportJson: boolean
}

export interface ShellResultCommandReadiness {
  canRun: boolean
  canValidate: boolean
  canExportCsv: boolean
  canExportJson: boolean
  reasons: Partial<Record<ResultCommandId, string>>
}

function normalized(value: unknown): string {
  return String(value ?? '').trim().toLowerCase()
}

export function resolveShellResultCommandReadiness(
  input: ShellResultCommandReadinessInput,
): ShellResultCommandReadiness {
  const inResearch = normalized(input.workspace) === 'workflow'
  const conventionalMode = normalized(input.mode) === 'conventional'
  const compatibility = activeRunCompatibility({
    activeDataId: input.activeDataId,
    activeDatasetPath: input.activeDatasetPath,
    activeMode: input.mode,
    activeTimeframe: input.activeTimeframe,
    run: input.currentRun,
  })
  const currentConventionalResult =
    inResearch && conventionalMode && compatibility.compatible

  const reasons: Partial<Record<ResultCommandId, string>> = {}
  const canRun = inResearch && input.datasetReady && !input.runDisabled
  if (!canRun) {
    reasons.run = !inResearch
      ? 'Open Research before starting an optimization.'
      : !input.datasetReady
        ? 'Load a dataset before starting an optimization.'
        : 'Optimization is not ready for the current setup.'
  }

  const contextReason = !inResearch
    ? 'Open Research to use actions for the active result.'
    : !conventionalMode
      ? 'This command applies to Conventional results. Use the selected mode stage action instead.'
      : compatibility.reason

  const canValidate = currentConventionalResult && !input.validateDisabled
  if (!canValidate) {
    reasons.validate =
      contextReason || 'Validation is not ready for the current result.'
  }

  const canExportCsv = currentConventionalResult && input.canExportCsv
  if (!canExportCsv) {
    reasons.export_csv =
      contextReason || 'The active result does not contain an exportable Top table.'
  }

  const canExportJson = currentConventionalResult && input.canExportJson
  if (!canExportJson) {
    reasons.export_json =
      contextReason || 'The active result does not contain an exportable Top result.'
  }

  return {
    canRun,
    canValidate,
    canExportCsv,
    canExportJson,
    reasons,
  }
}
