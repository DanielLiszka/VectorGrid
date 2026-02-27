export function fileBasename(pathValue: unknown): string {
  const path = String(pathValue || '')
  if (!path) return ''
  const parts = path.split(/[\\/]/g).filter((part) => part.length > 0)
  return parts.length > 0 ? String(parts[parts.length - 1] || '') : path
}
