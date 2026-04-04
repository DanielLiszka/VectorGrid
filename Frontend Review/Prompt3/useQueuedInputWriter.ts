import { useDebounceFn } from '@vueuse/core'

interface UseQueuedInputWriterOptions {
  waitMs?: number
  maxWaitMs?: number
}

export interface QueuedInputWriter {
  queueInputValue: (id: string, nextValue: string) => void
  commitInputValue: (id: string, nextValue: string) => void
  flushQueuedInputValuesNow: () => void
  dropQueuedInputValue: (id: string) => void
}

export function useQueuedInputWriter(
  writeInputValue: (id: string, nextValue: string) => void,
  options: UseQueuedInputWriterOptions = {},
): QueuedInputWriter {
  const waitMs = Number.isFinite(options.waitMs) ? Math.max(1, Math.floor(options.waitMs as number)) : 120
  const maxWaitMs = Number.isFinite(options.maxWaitMs) ? Math.max(waitMs, Math.floor(options.maxWaitMs as number)) : 380
  const queuedInputValues = new Map<string, string>()

  function flushQueuedInputValuesNow(): void {
    if (queuedInputValues.size <= 0) return
    const queuedEntries = Array.from(queuedInputValues.entries())
    queuedInputValues.clear()
    for (const [id, value] of queuedEntries) {
      writeInputValue(id, value)
    }
  }

  const flushQueuedInputValues = useDebounceFn(
    () => {
      flushQueuedInputValuesNow()
    },
    waitMs,
    { maxWait: maxWaitMs },
  )

  function queueInputValue(id: string, nextValue: string): void {
    const key = String(id || '').trim()
    if (!key) return
    queuedInputValues.set(key, String(nextValue || ''))
    flushQueuedInputValues()
  }

  function commitInputValue(id: string, nextValue: string): void {
    const key = String(id || '').trim()
    if (!key) return
    queuedInputValues.delete(key)
    writeInputValue(key, String(nextValue || ''))
  }

  function dropQueuedInputValue(id: string): void {
    const key = String(id || '').trim()
    if (!key) return
    queuedInputValues.delete(key)
  }

  return {
    queueInputValue,
    commitInputValue,
    flushQueuedInputValuesNow,
    dropQueuedInputValue,
  }
}
