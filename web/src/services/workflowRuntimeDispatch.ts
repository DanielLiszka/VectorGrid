import { useWorkflowStore } from '../stores/workflowStore'

export type WorkflowRuntimeDispatchAction =
  | 'workspace.select'
  | 'workflow.select'
  | 'workflowStep.select'
  | 'controls.select'
  | 'results.select'
  | 'aggregate.select'

export type WorkflowRuntimeDispatchFn = (
  action: string,
  payload?: Record<string, unknown>,
) => boolean

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizePayload(payload?: Record<string, unknown>): Record<string, unknown> {
  if (!payload || typeof payload !== 'object') return {}
  return payload
}

function shouldPersist(payload: Record<string, unknown>): boolean {
  return payload.persist !== false
}

export function installWorkflowRuntimeDispatch(): () => void {
  if (typeof window === 'undefined') return () => {}

  const workflowStore = useWorkflowStore()
  const handler: WorkflowRuntimeDispatchFn = (action, payload) => {
    const key = normalizeText(action)
    const data = normalizePayload(payload)
    const persist = shouldPersist(data)

    if (key === 'workspace.select') {
      const workspace = normalizeText(data.workspace)
      if (!workspace) return false
      const step = normalizeText(data.step)
      workflowStore.selectWorkspace(workspace, step || undefined, { persist })
      return true
    }

    if (key === 'workflow.select') {
      const workflow = normalizeText(data.workflow)
      if (!workflow) return false
      workflowStore.selectWorkflow(workflow, { persist })
      return true
    }

    if (key === 'workflowStep.select') {
      const workflowStep = normalizeText(data.workflowStep)
      if (!workflowStep) return false
      workflowStore.selectWorkflowStep(workflowStep, { persist })
      return true
    }

    if (key === 'controls.select') {
      const tab = normalizeText(data.tab)
      if (!tab) return false
      workflowStore.selectControlsTab(tab, { persist })
      return true
    }

    if (key === 'results.select') {
      const tab = normalizeText(data.tab)
      if (!tab) return false
      workflowStore.selectResultsTab(tab, { persist })
      return true
    }

    if (key === 'aggregate.select') {
      const tab = normalizeText(data.tab)
      if (!tab) return false
      const activeWorkspace = normalizeText(data.activeWorkspace) || normalizeText(workflowStore.workspace)
      if (activeWorkspace === 'aggregate') {
        workflowStore.selectAggregateTab(tab, { persist })
      } else {
        workflowStore.primeAggregateTab(tab, { persist })
      }
      return true
    }

    return false
  }

  window.__VTA_VUE_WORKFLOW_DISPATCH__ = handler
  return () => {
    if (window.__VTA_VUE_WORKFLOW_DISPATCH__ === handler) {
      delete window.__VTA_VUE_WORKFLOW_DISPATCH__
    }
  }
}
