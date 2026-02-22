import { useWorkflowStore } from '../../stores/workflowStore'
import { createCoalescedSync } from './coalescedSync'

function attr(root: HTMLElement, name: string): string {
  return String(root.dataset[name as keyof DOMStringMap] || '').trim()
}

interface WorkflowSnapshotDetail {
  workspace?: unknown
  workflow?: unknown
  workflowStep?: unknown
  controlsTab?: unknown
  resultsTab?: unknown
  aggregateTab?: unknown
  workflowSubtitle?: unknown
}

const WORKFLOW_ROOT_ATTRS = [
  'data-workspace',
  'data-workflow',
  'data-workflow-step',
  'data-controls-tab',
  'data-results-tab',
  'data-aggregate-tab',
  'data-workflow-subtitle',
] as const

function isVueWorkflowOwner(): boolean {
  return !!window.__VTA_VUE_WORKFLOW_OWNER__
}

function domSnapshot() {
  const root = document.documentElement
  return {
    workspace: attr(root, 'workspace') || 'workflow',
    workflow: attr(root, 'workflow') || 'data',
    workflowStep: attr(root, 'workflowStep') || 'data',
    controlsTab: attr(root, 'controlsTab') || 'data',
    resultsTab: attr(root, 'resultsTab') || 'overview',
    aggregateTab: attr(root, 'aggregateTab') || 'meta',
    workflowSubtitle: attr(root, 'workflowSubtitle') || '—',
  }
}

function eventSnapshot(detail: WorkflowSnapshotDetail, fallback: ReturnType<typeof domSnapshot>) {
  return {
    workspace: String(detail.workspace ?? fallback.workspace ?? '').trim() || 'workflow',
    workflow: String(detail.workflow ?? fallback.workflow ?? '').trim() || 'data',
    workflowStep: String(detail.workflowStep ?? fallback.workflowStep ?? '').trim() || 'data',
    controlsTab: String(detail.controlsTab ?? fallback.controlsTab ?? '').trim() || 'data',
    resultsTab: String(detail.resultsTab ?? fallback.resultsTab ?? '').trim() || 'overview',
    aggregateTab: String(detail.aggregateTab ?? fallback.aggregateTab ?? '').trim() || 'meta',
    workflowSubtitle: String(detail.workflowSubtitle ?? fallback.workflowSubtitle ?? '').trim() || '—',
  }
}

function vueOwnedSnapshot(detail: WorkflowSnapshotDetail | null, fallback: ReturnType<typeof domSnapshot>) {
  const src = detail && typeof detail === 'object' ? detail : {}
  return {
    controlsTab: String(src.controlsTab ?? fallback.controlsTab ?? '').trim() || 'data',
    resultsTab: String(src.resultsTab ?? fallback.resultsTab ?? '').trim() || 'overview',
    aggregateTab: String(src.aggregateTab ?? fallback.aggregateTab ?? '').trim() || 'meta',
    workflowSubtitle: String(src.workflowSubtitle ?? fallback.workflowSubtitle ?? '').trim() || '—',
  }
}

export function attachWorkflowBridge(): () => void {
  const store = useWorkflowStore()
  let queuedDetail: WorkflowSnapshotDetail | null = null

  const syncNow = () => {
    const fallback = domSnapshot()
    const detail = queuedDetail
    queuedDetail = null
    if (isVueWorkflowOwner()) {
      store.applyLegacySnapshot(vueOwnedSnapshot(detail, fallback))
      return
    }
    if (!detail || typeof detail !== 'object') {
      store.applyLegacySnapshot(fallback)
      return
    }
    store.applyLegacySnapshot(eventSnapshot(detail, fallback))
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const rootObserver = new MutationObserver(() => scheduleSync())
  rootObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: [...WORKFLOW_ROOT_ATTRS],
  })

  const onWorkflowSnapshot = (event: Event): void => {
    const detail = (event as CustomEvent<WorkflowSnapshotDetail>).detail
    queuedDetail = detail && typeof detail === 'object' ? detail : null
    scheduleSync()
  }
  const attachEventListener = !isVueWorkflowOwner()
  if (attachEventListener) {
    window.addEventListener('vta:workflow-snapshot', onWorkflowSnapshot)
  }

  syncNow()

  return () => {
    rootObserver.disconnect()
    if (attachEventListener) {
      window.removeEventListener('vta:workflow-snapshot', onWorkflowSnapshot)
    }
  }
}
