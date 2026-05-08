import { afterEach, describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'
import {
  type QueuedInputWriter,
  useQueuedInputWriter,
} from './useQueuedInputWriter'

describe('useQueuedInputWriter', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('flushes pending values when the owning scope is disposed', () => {
    vi.useFakeTimers()
    const writes: Array<[string, string]> = []
    let writer!: QueuedInputWriter
    const scope = effectScope()

    scope.run(() => {
      writer = useQueuedInputWriter(
        (id, value) => writes.push([id, value]),
        { waitMs: 50, maxWaitMs: 100 },
      )
    })

    writer.queueInputValue('field', 'draft')
    expect(writes).toEqual([])

    scope.stop()
    expect(writes).toEqual([['field', 'draft']])

    vi.advanceTimersByTime(150)
    expect(writes).toEqual([['field', 'draft']])
  })
})
