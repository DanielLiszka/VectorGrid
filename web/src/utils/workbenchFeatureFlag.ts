export const WORKBENCH_SHELL_STORAGE_KEY = 'vta_workbench_shell_v1'

const ENABLED_VALUES = new Set(['1', 'true', 'yes', 'on', 'enabled'])
const DISABLED_VALUES = new Set(['0', 'false', 'no', 'off', 'disabled'])

function normalizeFlagValue(value: unknown): string {
  return String(value ?? '').trim().toLowerCase()
}

export function resolveWorkbenchShellFlag(value: unknown): boolean | null {
  const normalized = normalizeFlagValue(value)
  if (!normalized) return null
  if (ENABLED_VALUES.has(normalized)) return true
  if (DISABLED_VALUES.has(normalized)) return false
  return null
}

export function shouldUseWorkbenchShell(): boolean {
  if (typeof window === 'undefined') return true
  try {
    const params = new URLSearchParams(window.location.search)
    const queryValue =
      params.get('workbench_shell')
      ?? params.get('workbench')
      ?? params.get('ui_workbench')
    const queryResolved = resolveWorkbenchShellFlag(queryValue)
    if (queryResolved !== null) return queryResolved

    const storageResolved = resolveWorkbenchShellFlag(window.localStorage.getItem(WORKBENCH_SHELL_STORAGE_KEY))
    if (storageResolved !== null) return storageResolved
    return true
  } catch {
    return true
  }
}
