import type { FunctionalComponent, SVGAttributes } from 'vue'
import {
  ArrowRightLeft,
  CircleAlert,
  CircleCheckBig,
  Clock3,
  FileSpreadsheet,
  FolderOpen,
  FolderOpenDot,
  Info,
  LoaderCircle,
  Pin,
  RefreshCcw,
  Square,
  Trash2,
} from 'lucide-vue-next'

export type AppIcon = FunctionalComponent<SVGAttributes>

const TASK_STATUS_ICONS: Record<string, AppIcon> = {
  queued: Clock3,
  running: LoaderCircle,
  cancelling: CircleAlert,
  success: CircleCheckBig,
  failed: CircleAlert,
  cancelled: Square,
  idle: Info,
}

const TASK_LEVEL_ICONS: Record<string, AppIcon> = {
  info: Info,
  warn: CircleAlert,
  error: CircleAlert,
  success: CircleCheckBig,
}

const ARTIFACT_ACTION_ICONS: Record<string, AppIcon> = {
  load: FolderOpenDot,
  refresh: RefreshCcw,
  report: FileSpreadsheet,
  reveal: FolderOpen,
  delete: Trash2,
}

const COMPARE_ACTION_ICONS: Record<string, AppIcon> = {
  pin: Pin,
  compare: ArrowRightLeft,
  replace: ArrowRightLeft,
  remove: Trash2,
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
  return (COMPARE_ACTION_ICONS[action] ?? ArrowRightLeft) as AppIcon
}
