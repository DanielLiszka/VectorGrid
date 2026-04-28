import { PiniaColada, useQueryCache } from '@pinia/colada'
import { createPinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp } from 'vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import {
  readCachedCatalog,
  removeCachedCatalog,
  resetCatalogReadCacheForTests,
  writeCachedCatalog,
} from './catalogReadCache'

describe('catalogReadCache', () => {
  beforeEach(() => {
    bindRuntimePinia(null)
    resetCatalogReadCacheForTests()
  })

  afterEach(() => {
    bindRuntimePinia(null)
    resetCatalogReadCacheForTests()
  })

  it('deduplicates concurrent catalog reads and stores successful data in Pinia Colada', async () => {
    const pinia = createPinia()
    const app = createApp({})
    app.use(pinia)
    app.use(PiniaColada)
    bindRuntimePinia(pinia)
    const read = vi.fn(async () => [{ id: 'sma' }])
    const key = ['catalog-read-cache', 'dedupe'] as const

    const [first, second] = await app.runWithContext(() =>
      Promise.all([
        readCachedCatalog(key, read),
        readCachedCatalog(key, read),
      ]),
    )
    const third = await app.runWithContext(() => readCachedCatalog(key, read))

    expect(read).toHaveBeenCalledTimes(1)
    expect(first).toEqual([{ id: 'sma' }])
    expect(second).toEqual([{ id: 'sma' }])
    expect(third).toEqual([{ id: 'sma' }])
    expect(app.runWithContext(() => useQueryCache(pinia).getQueryData(key))).toEqual([{ id: 'sma' }])
  })

  it('supports force refresh and explicit cache replacement', async () => {
    const pinia = createPinia()
    const app = createApp({})
    app.use(pinia)
    app.use(PiniaColada)
    bindRuntimePinia(pinia)
    const key = ['catalog-read-cache', 'refresh'] as const
    const read = vi
      .fn()
      .mockResolvedValueOnce([{ id: 'first' }])
      .mockResolvedValueOnce([{ id: 'second' }])

    await expect(app.runWithContext(() => readCachedCatalog(key, read))).resolves.toEqual([{ id: 'first' }])
    await expect(
      app.runWithContext(() => readCachedCatalog(key, read, { forceRefresh: true })),
    ).resolves.toEqual([{ id: 'second' }])
    app.runWithContext(() => writeCachedCatalog(key, [{ id: 'manual' }]))

    expect(read).toHaveBeenCalledTimes(2)
    await expect(app.runWithContext(() => readCachedCatalog(key, read))).resolves.toEqual([{ id: 'manual' }])
  })

  it('removes cached entries when service-level reset hooks run', async () => {
    const pinia = createPinia()
    const app = createApp({})
    app.use(pinia)
    app.use(PiniaColada)
    bindRuntimePinia(pinia)
    const key = ['catalog-read-cache', 'remove'] as const

    app.runWithContext(() => writeCachedCatalog(key, [{ id: 'cached' }]))
    expect(app.runWithContext(() => useQueryCache(pinia).getQueryData(key))).toEqual([{ id: 'cached' }])

    app.runWithContext(() => removeCachedCatalog(key))

    expect(app.runWithContext(() => useQueryCache(pinia).getQueryData(key))).toBeUndefined()
  })
})
