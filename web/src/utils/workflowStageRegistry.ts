import {
  WORKFLOW_STEPS,
  type ShellMode,
  type WorkflowStep,
  normalizeShellMode,
  normalizeWorkflowStep,
  workflowStepLabelForMode,
  workflowStepSubtitleForMode,
} from './core/workflowModel'

export interface WorkflowStageDefinition {
  id: WorkflowStep
  label: string
  description: string
  icon: string
}

function normalizedMode(mode: unknown): ShellMode {
  return normalizeShellMode(mode) || 'conventional'
}

export function workflowStageDefinitionsForMode(mode: unknown): WorkflowStageDefinition[] {
  const nextMode = normalizedMode(mode)
  return WORKFLOW_STEPS.map((step) => ({
    id: step,
    label: workflowStepLabelForMode(step, nextMode),
    description: workflowStepSubtitleForMode(step, nextMode),
    icon: step,
  }))
}

export function workflowStageDefinitionForMode(
  step: unknown,
  mode: unknown,
): WorkflowStageDefinition {
  const normalizedStep = normalizeWorkflowStep(step) || 'data'
  const stages = workflowStageDefinitionsForMode(mode)
  return stages.find((entry) => entry.id === normalizedStep) ?? stages[0]!
}
