import { describe, expect, it } from 'vitest'
import {
  artifactLibraryQueryKey,
  dynamicIndicatorCatalogQueryKey,
  gpuSpeedupsQueryKey,
  movingAverageCatalogQueryKey,
} from './catalogQueryKeys'

describe('catalogQueryKeys', () => {
  it('keeps stable keys for idempotent catalog reads', () => {
    expect(gpuSpeedupsQueryKey).toEqual(['gpu-speedups'])
    expect(movingAverageCatalogQueryKey).toEqual(['moving-averages'])
    expect(dynamicIndicatorCatalogQueryKey).toEqual(['dynamic-indicators'])
    expect(artifactLibraryQueryKey({ family: 'runs' })).toEqual([
      'artifact-library',
      { family: 'runs' },
    ])
  })
})
