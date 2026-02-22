import { useCommandActionStore } from '../../stores/commandActionStore'

const QUICK_ACTIONS_SNAPSHOT_EVENT = 'vta:quick-actions-snapshot'

interface QuickActionsSnapshotDetail {
  reportPath?: unknown
  metaReportPath?: unknown
}

function normalizePath(value: unknown): string {
  return String(value ?? '').trim()
}

function runtimePathFromRoot(key: 'latestReportPath' | 'latestMetaReportPath'): string {
  if (typeof document === 'undefined') return ''
  return normalizePath(document.documentElement.dataset[key])
}

export function attachQuickActionsBridge(): () => void {
  if (typeof window === 'undefined') return () => {}
  const store = useCommandActionStore()

  const syncFromRoot = () => {
    store.applyLegacyQuickActionsSnapshot({
      latestReportPath: runtimePathFromRoot('latestReportPath'),
      latestMetaReportPath: runtimePathFromRoot('latestMetaReportPath'),
    })
  }

  const onSnapshot = (event: Event): void => {
    const detail = ((event as CustomEvent<QuickActionsSnapshotDetail>).detail || {}) as QuickActionsSnapshotDetail
    store.applyLegacyQuickActionsSnapshot({
      latestReportPath: normalizePath(detail.reportPath),
      latestMetaReportPath: normalizePath(detail.metaReportPath),
    })
  }

  window.addEventListener(QUICK_ACTIONS_SNAPSHOT_EVENT, onSnapshot as EventListener)
  syncFromRoot()
  return () => {
    window.removeEventListener(QUICK_ACTIONS_SNAPSHOT_EVENT, onSnapshot as EventListener)
  }
}
