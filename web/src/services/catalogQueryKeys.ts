export const gpuSpeedupsQueryKey = ['gpu-speedups'] as const
export const movingAverageCatalogQueryKey = ['moving-averages'] as const
export const dynamicIndicatorCatalogQueryKey = ['dynamic-indicators'] as const
export const artifactCatalogSummaryQueryKey = ['artifact-catalog', 'summary'] as const

export function artifactLibraryQueryKey(filters: unknown = null): readonly ['artifact-library', unknown] {
  return ['artifact-library', filters] as const
}
