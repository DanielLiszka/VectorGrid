import { toCacheKey, useQueryCache, type EntryKey } from '@pinia/colada'
import { hasInjectionContext } from 'vue'
import { resolveRuntimePinia } from '../stores/runtimePinia'

export type CatalogReadKey = readonly unknown[]

interface CatalogReadOptions {
  forceRefresh?: boolean
}

const inFlightReads = new Map<string, Promise<unknown>>()

function asEntryKey(key: CatalogReadKey): EntryKey {
  return key as EntryKey
}

function stableFallbackKey(value: unknown): string {
  if (value == null) return 'null'
  const kind = typeof value
  if (kind === 'string' || kind === 'number' || kind === 'boolean') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map((entry) => stableFallbackKey(entry)).join(',')}]`
  if (kind !== 'object') return JSON.stringify(String(value))
  const row = value as Record<string, unknown>
  return `{${Object.keys(row)
    .sort()
    .map((field) => `${JSON.stringify(field)}:${stableFallbackKey(row[field])}`)
    .join(',')}}`
}

function cacheKeyFor(key: CatalogReadKey): string {
  return toCacheKey(asEntryKey(key)) ?? stableFallbackKey(key)
}

export async function readCachedCatalog<T>(
  key: CatalogReadKey,
  read: () => Promise<T>,
  options: CatalogReadOptions = {},
): Promise<T> {
  const pinia = hasInjectionContext() ? resolveRuntimePinia() : null
  const queryCache = pinia ? useQueryCache(pinia) : null
  const entryKey = asEntryKey(key)

  if (queryCache && !options.forceRefresh) {
    const cached = queryCache.getQueryData<T>(entryKey)
    if (cached !== undefined) return cached
  }

  const stableKey = cacheKeyFor(key)
  const existing = inFlightReads.get(stableKey) as Promise<T> | undefined
  if (existing) return existing

  const pending = read()
    .then((data) => {
      if (queryCache) {
        queryCache.setQueryData<T>(entryKey, data)
      }
      return data
    })
    .finally(() => {
      inFlightReads.delete(stableKey)
    })

  inFlightReads.set(stableKey, pending)
  return pending
}

export function writeCachedCatalog<T>(key: CatalogReadKey, data: T): void {
  const pinia = hasInjectionContext() ? resolveRuntimePinia() : null
  if (!pinia) return
  useQueryCache(pinia).setQueryData<T>(asEntryKey(key), data)
}

export function removeCachedCatalog(key: CatalogReadKey): void {
  inFlightReads.delete(cacheKeyFor(key))
  const pinia = hasInjectionContext() ? resolveRuntimePinia() : null
  if (!pinia) return
  const queryCache = useQueryCache(pinia)
  const entry = queryCache.get(asEntryKey(key))
  if (entry) queryCache.remove(entry)
}

export function resetCatalogReadCacheForTests(): void {
  inFlightReads.clear()
}
