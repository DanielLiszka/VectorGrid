import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import {
  applyDoubleMaProgress,
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

  it('treats double-ma phase changes as immediate updates while throttling duplicate progress chatter', () => {
    beginTrackedCommand('run_double_ma_optimization')

    let now = Date.now() + 5_000
    const nowSpy = vi.spyOn(Date, 'now').mockImplementation(() => now)

    applyDoubleMaProgress({
      processed_pairs: 1,
      total_pairs: 10,
      phase: 'prepare',
    })

    const store = useTaskStore(pinia)
    const task = store.activeById[store.activeOrder[0]!]
    expect(task?.phase).toBe('prepare')
    expect(task?.progressPct).toBeCloseTo(10, 5)
    expect(store.overallProgress?.source).toBe('double_ma_progress')
    expect(store.events.filter((evt) => evt.source === 'double_ma_progress')).toHaveLength(1)

    now += 40
    applyDoubleMaProgress({
      processed_pairs: 2,
      total_pairs: 10,
      phase: 'prepare',
    })

    expect(task?.progressPct).toBeCloseTo(10, 5)
    expect(task?.message).toContain('prepare 1/10')
    expect(store.events.filter((evt) => evt.source === 'double_ma_progress')).toHaveLength(1)

    now += 20
    applyDoubleMaProgress({
      processed_pairs: 2,
      total_pairs: 10,
      phase: 'optimize',
    })

    expect(task?.phase).toBe('optimize')
    expect(task?.progressPct).toBeCloseTo(20, 5)
    expect(task?.message).toContain('optimize 2/10')
    expect(store.events.filter((evt) => evt.source === 'double_ma_progress')).toHaveLength(2)
    expect(
      store.timeline.some(
        (entry) =>
          entry.source === 'double_ma_progress'
          && entry.message.includes('Phase changed: optimize'),
      ),
    ).toBe(true)

    nowSpy.mockRestore()
  })

  it('refreshes campaign progress immediately when the signature changes, even inside the throttle window', () => {
    beginTrackedCommand('run_research_campaign')

    let now = Date.now() + 10_000
    const nowSpy = vi.spyOn(Date, 'now').mockImplementation(() => now)

    applyResearchCampaignProgress({
      stage: 'prepare',
      datasetIndex: 1,
      datasetTotal: 3,
      label: 'AAPL',
      message: 'preloading host cache',
      optimized: 0,
      validated: 0,
      skipped: 0,
      failed: 0,
    })

    const store = useTaskStore(pinia)
    const task = store.activeById[store.activeOrder[0]!]
    expect(task?.phase).toBe('prepare')
    expect(task?.progressPct).toBeCloseTo(100 / 3, 5)
    expect(store.events.filter((evt) => evt.source === 'research_campaign_progress')).toHaveLength(1)

    now += 30
    applyResearchCampaignProgress({
      stage: 'optimize',
      datasetIndex: 2,
      datasetTotal: 3,
      label: 'AAPL',
      message: 'running optimization',
      optimized: 1,
      validated: 0,
      skipped: 0,
      failed: 0,
    })

    expect(task?.phase).toBe('optimize')
    expect(task?.progressPct).toBeCloseTo(200 / 3, 5)
    expect(task?.message).toContain('running optimization')
    expect(store.overallProgress?.label).toContain('2/3')
    expect(store.events.filter((evt) => evt.source === 'research_campaign_progress')).toHaveLength(2)
    expect(
      store.timeline.some(
        (entry) =>
          entry.source === 'research_campaign_progress'
          && entry.message.includes('AAPL'),
      ),
    ).toBe(true)

    nowSpy.mockRestore()
  })
})
