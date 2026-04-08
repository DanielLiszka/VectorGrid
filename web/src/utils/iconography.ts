import type { FunctionalComponent, SVGAttributes } from 'vue'
import {
  CircleAlert,
  CircleCheckBig,
  FolderOpenDot,
  Info,
  RefreshCcw,
  Trash2,
} from 'lucide-vue-next'
import {
  artifactReopenReportIcon,
  artifactReportWorkbookIcon,
  artifactRevealPathIcon,
  compareCandidatesIcon,
  comparePinCandidateIcon,
  compareRemoveFromSetIcon,
  compareReplacePinnedIcon,
  taskCancelledIcon,
  taskCancellingIcon,
  taskCompletedIcon,
  taskFailedIcon,
  taskQueuedIcon,
  taskRunningIcon,
  type AssetIconSpec,
} from './appSvgAssets'

export type AppIcon = FunctionalComponent<SVGAttributes> | AssetIconSpec

export function isAssetIcon(icon: AppIcon | null | undefined): icon is AssetIconSpec {
  return !!icon && typeof icon === 'object' && 'kind' in icon && icon.kind === 'asset'
}

const TASK_STATUS_ICONS: Record<string, AppIcon> = {
  queued: taskQueuedIcon,
  running: taskRunningIcon,
  cancelling: taskCancellingIcon,
  success: taskCompletedIcon,
  failed: taskFailedIcon,
  cancelled: taskCancelledIcon,
  idle: Info,
}

const TASK_LEVEL_ICONS: Record<string, AppIcon> = {
  info: Info,
  warn: CircleAlert,
  error: CircleAlert,
  success: CircleCheckBig,
}

const ARTIFACT_ACTION_ICONS: Record<string, AppIcon> = {
  load: artifactReopenReportIcon,
  refresh: RefreshCcw,
  report: artifactReportWorkbookIcon,
  reveal: artifactRevealPathIcon,
  delete: Trash2,
}

const COMPARE_ACTION_ICONS: Record<string, AppIcon> = {
  pin: comparePinCandidateIcon,
  compare: compareCandidatesIcon,
  replace: compareReplacePinnedIcon,
  remove: compareRemoveFromSetIcon,
  clear: Trash2,
}

export function taskStatusIcon(status: string | null | undefined): AppIcon {
  const key = String(status || 'idle').trim().toLowerCase()
  return (TASK_STATUS_ICONS[key] ?? Info) as AppIcon
}

export function taskLevelIcon(level: string | null | undefined): AppIcon {
  const key = String(level || 'info').trim().toLowerCase()
  return (TASK_LEVEL_ICONS[key] ?? Info) as AppIcon
}

export function artifactActionIcon(action: 'load' | 'refresh' | 'report' | 'reveal' | 'delete'): AppIcon {
  return (ARTIFACT_ACTION_ICONS[action] ?? FolderOpenDot) as AppIcon
}

export function compareActionIcon(action: 'pin' | 'compare' | 'replace' | 'remove' | 'clear'): AppIcon {
  return (COMPARE_ACTION_ICONS[action] ?? compareCandidatesIcon) as AppIcon
}
