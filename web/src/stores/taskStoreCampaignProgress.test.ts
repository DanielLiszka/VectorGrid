import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import {
  applyResearchCampaignProgress,
  beginTrackedCommand,
  resetTaskStoreForTests,
  useTaskStore,
} from './taskStore'
import { bindRuntimePinia } from './runtimePinia'

class MemoryStorage implements Storage {
  private readonly values = new Map<string, string>()

  get length(): number {
    return this.values.size
  }

  clear(): void {
    this.values.clear()
  }

  getItem(key: string): string | null {
    return this.values.has(key) ? this.values.get(key) ?? null : null
  }

  key(index: number): string | null {
    const keys = Array.from(this.values.keys())
    return index >= 0 && index < keys.length ? (keys[index] ?? null) : null
  }

  removeItem(key: string): void {
    this.values.delete(key)
  }

  setItem(key: string, value: string): void {
    this.values.set(key, String(value))
  }
}

describe('task-store campaign progress', () => {
  let pinia: Pinia
  let warnSpy: ReturnType<typeof vi.spyOn>
  let errorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    ;(globalThis as Record<string, unknown>).window = { localStorage: new MemoryStorage() }
    resetTaskStoreForTests()
    const store = useTaskStore(pinia)
    store.$reset()
  })

  afterEach(() => {
    warnSpy.mockRestore()
    errorSpy.mockRestore()
  })

  it('updates tracked task and overall progress from research campaign events', () => {
    beginTrackedCommand('run_research_campaign')

    applyResearchCampaignProgress({
      stage: 'validate',
      datasetIndex: 2,
      datasetTotal: 5,
      label: 'SPY',
      message: 'running validation',
      optimized: 2,
      validated: 1,
      skipped: 0,
      failed: 0,
    })

    const store = useTaskStore(pinia)
    expect(store.overallProgress?.source).toBe('research_campaign_progress')
    expect(store.overallProgress?.pct).toBeCloseTo(40, 5)
    expect(store.overallProgress?.label).toContain('SPY')
    expect(store.activeOrder).toHaveLength(1)
    const task = store.activeById[store.activeOrder[0]!]
    expect(task?.phase).toBe('validate')
    expect(task?.message).toContain('running validation')
  })
})
