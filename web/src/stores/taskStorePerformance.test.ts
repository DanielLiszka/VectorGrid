import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { applyDoubleMaProgress, beginTrackedCommand, useTaskStore } from './taskStore'

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

describe('task-store progress throughput', () => {
  let pinia: Pinia
  let warnSpy: ReturnType<typeof vi.spyOn>
  let errorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    ;(globalThis as Record<string, unknown>).window = { localStorage: new MemoryStorage() }
    const store = useTaskStore(pinia)
    store.$reset()
  })

  afterEach(() => {
    warnSpy.mockRestore()
    errorSpy.mockRestore()
  })

  it('handles high-volume progress events without becoming a bottleneck', () => {
    beginTrackedCommand('run_double_ma_optimization')
    let now = 1_700_000_000_000
    const nowSpy = vi.spyOn(Date, 'now').mockImplementation(() => {
      now += 125
      return now
    })

    const started = performance.now()
    for (let i = 1; i <= 5000; i += 1) {
      applyDoubleMaProgress({
        processed_pairs: i,
        total_pairs: 5000,
        phase: 'optimize',
      })
    }
    const elapsedMs = performance.now() - started
    nowSpy.mockRestore()

    const store = useTaskStore(pinia)
    expect(store.overallProgress?.pct).toBeCloseTo(100, 5)
    expect(elapsedMs).toBeLessThan(1000)
  })
})
