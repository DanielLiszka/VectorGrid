import { ref, watch } from 'vue'
import { readStorageString, removeStorageKey, writeStorageString } from '../utils/storage'

export type TableDensity = 'compact' | 'comfortable' | 'spacious'

export const TABLE_DENSITY_OPTIONS: readonly TableDensity[] = [
  'compact',
  'comfortable',
  'spacious',
]

export function normalizeTableDensity(
  value: string | null | undefined,
  fallback: TableDensity = 'comfortable',
): TableDensity {
  const normalized = String(value || '').trim().toLowerCase()
  if (normalized === 'compact' || normalized === 'comfortable' || normalized === 'spacious') {
    return normalized
  }
  return fallback
}

export function usePersistedTableDensity(storageKey: string, fallback: TableDensity = 'comfortable') {
  const tableDensity = ref<TableDensity>(
    normalizeTableDensity(readStorageString(storageKey), fallback),
  )

  function setTableDensity(nextDensity: string): void {
    tableDensity.value = normalizeTableDensity(nextDensity, fallback)
  }

  function restoreDefaultTableDensity(): void {
    tableDensity.value = fallback
  }

  watch(
    tableDensity,
    (nextDensity) => {
      if (nextDensity === fallback) {
        removeStorageKey(storageKey)
        return
      }
      writeStorageString(storageKey, nextDensity)
    },
    { immediate: true },
  )

  return {
    tableDensity,
    tableDensityOptions: TABLE_DENSITY_OPTIONS,
    setTableDensity,
    restoreDefaultTableDensity,
  }
}
