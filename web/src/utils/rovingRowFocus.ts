export function resolveRovingRowIndex(
  currentIndex: number,
  rowCount: number,
  key: string,
): number | null {
  if (rowCount <= 0) return null
  const current = Math.min(rowCount - 1, Math.max(0, currentIndex))
  if (key === 'Home') return 0
  if (key === 'End') return rowCount - 1
  if (key === 'ArrowDown') return Math.min(rowCount - 1, current + 1)
  if (key === 'ArrowUp') return Math.max(0, current - 1)
  return null
}
