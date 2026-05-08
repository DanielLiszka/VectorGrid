import { useQuery } from '@pinia/colada'
import { computed, type MaybeRefOrGetter, toValue } from 'vue'
import {
  listDynamicIndicatorCatalog,
  dynamicIndicatorCatalogQueryKey,
  type DynamicIndicatorCatalogEntry,
} from '../services/dynamicIndicatorCatalogService'
import {
  listGpuSpeedups,
  gpuSpeedupsQueryKey,
  type GpuSpeedupEntry,
} from '../services/gpuSpeedupsService'
import {
  listMovingAverageCatalog,
  movingAverageCatalogQueryKey,
  type MovingAverageCatalogEntry,
} from '../services/movingAverageCatalogService'

interface OptimizeCatalogQueryOptions {
  enabled?: MaybeRefOrGetter<boolean>
}

const CATALOG_STALE_TIME_MS = 5 * 60_000

function resolveEnabled(value: MaybeRefOrGetter<boolean> | undefined): boolean {
  return value == null ? true : !!toValue(value)
}

export function useOptimizeCatalogQueries(options: OptimizeCatalogQueryOptions = {}) {
  const enabled = computed(() => resolveEnabled(options.enabled))
  const movingAverageCatalogQuery = useQuery<
    MovingAverageCatalogEntry[],
    Error,
    MovingAverageCatalogEntry[]
  >({
    key: movingAverageCatalogQueryKey,
    query: () => listMovingAverageCatalog(),
    initialData: () => [],
    initialDataUpdatedAt: 0,
    enabled,
    staleTime: CATALOG_STALE_TIME_MS,
  })
  const gpuSpeedupsQuery = useQuery<GpuSpeedupEntry[], Error, GpuSpeedupEntry[]>({
    key: gpuSpeedupsQueryKey,
    query: () => listGpuSpeedups(),
    initialData: () => [],
    initialDataUpdatedAt: 0,
    enabled,
    staleTime: CATALOG_STALE_TIME_MS,
  })
  const dynamicIndicatorCatalogQuery = useQuery<
    DynamicIndicatorCatalogEntry[],
    Error,
    DynamicIndicatorCatalogEntry[]
  >({
    key: dynamicIndicatorCatalogQueryKey,
    query: () => listDynamicIndicatorCatalog(),
    initialData: () => [],
    initialDataUpdatedAt: 0,
    enabled,
    staleTime: CATALOG_STALE_TIME_MS,
  })
  const gpuSpeedupById = computed<Record<string, GpuSpeedupEntry>>(() => {
    const out: Record<string, GpuSpeedupEntry> = {}
    for (const entry of gpuSpeedupsQuery.data.value ?? []) {
      out[entry.id] = entry
    }
    return out
  })

  return {
    dynamicIndicatorCatalogQuery,
    gpuSpeedupById,
    gpuSpeedupsQuery,
    movingAverageCatalogQuery,
  }
}
