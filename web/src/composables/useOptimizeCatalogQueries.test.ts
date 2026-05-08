import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref, toValue } from 'vue'
import { useQuery } from '@pinia/colada'
import { useOptimizeCatalogQueries } from './useOptimizeCatalogQueries'

vi.mock('@pinia/colada', () => ({
  useQuery: vi.fn((options: { initialData?: () => unknown }) => ({
    data: { value: options.initialData ? options.initialData() : undefined },
  })),
}))

const useQueryMock = vi.mocked(useQuery)

describe('useOptimizeCatalogQueries', () => {
  beforeEach(() => {
    useQueryMock.mockClear()
  })

  it('applies the runtime enabled gate to every catalog query', () => {
    const enabled = ref(false)

    useOptimizeCatalogQueries({ enabled })

    const calls = useQueryMock.mock.calls.map(
      ([options]) => options as { enabled?: unknown },
    )
    expect(calls).toHaveLength(3)
    for (const options of calls) {
      expect(toValue(options.enabled)).toBe(false)
    }

    enabled.value = true
    for (const options of calls) {
      expect(toValue(options.enabled)).toBe(true)
    }
  })
})
