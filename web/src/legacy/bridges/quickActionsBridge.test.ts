import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useCommandActionStore } from '../../stores/commandActionStore'
import { attachQuickActionsBridge } from './quickActionsBridge'

function ensureDocumentDataset(): Record<string, string> {
  const existing = (globalThis as { document?: { documentElement?: { dataset?: Record<string, string> } } }).document
  if (existing?.documentElement?.dataset) return existing.documentElement.dataset
  const dataset: Record<string, string> = {}
  vi.stubGlobal('document', { documentElement: { dataset } })
  return dataset
}

function ensureWindowEventTarget(): {
  addEventListener: (type: string, listener: EventListenerOrEventListenerObject | null) => void
  removeEventListener: (type: string, listener: EventListenerOrEventListenerObject | null) => void
  dispatchEvent: (event: Event) => boolean
} {
  const existing = (globalThis as {
    window?: {
      addEventListener?: (type: string, listener: EventListenerOrEventListenerObject | null) => void
      removeEventListener?: (type: string, listener: EventListenerOrEventListenerObject | null) => void
      dispatchEvent?: (event: Event) => boolean
    }
  }).window
  if (existing?.addEventListener && existing.removeEventListener && existing.dispatchEvent) {
    return existing as {
      addEventListener: (type: string, listener: EventListenerOrEventListenerObject | null) => void
      removeEventListener: (type: string, listener: EventListenerOrEventListenerObject | null) => void
      dispatchEvent: (event: Event) => boolean
    }
  }

  const target = new EventTarget()
  const eventWindow = {
    addEventListener: target.addEventListener.bind(target),
    removeEventListener: target.removeEventListener.bind(target),
    dispatchEvent: target.dispatchEvent.bind(target),
  }
  vi.stubGlobal('window', eventWindow)
  return eventWindow
}

describe('quick actions bridge', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const dataset = ensureDocumentDataset()
    for (const key of Object.keys(dataset)) delete dataset[key]
    ensureWindowEventTarget()
  })

  it('hydrates latest report paths from runtime dataset on attach', () => {
    const dataset = ensureDocumentDataset()
    dataset.latestReportPath = 'C:/reports/latest.xlsx'
    dataset.latestMetaReportPath = 'C:/reports/meta_latest.xlsx'
    const store = useCommandActionStore()

    const detach = attachQuickActionsBridge()

    expect(store.latestReportPath).toBe('C:/reports/latest.xlsx')
    expect(store.latestMetaReportPath).toBe('C:/reports/meta_latest.xlsx')
    expect(store.canOpenLatestReport).toBe(true)
    expect(store.canOpenLatestMetaReport).toBe(true)
    detach()
  })

  it('syncs snapshot events and stops syncing after detach', () => {
    const eventWindow = ensureWindowEventTarget()
    const store = useCommandActionStore()
    const detach = attachQuickActionsBridge()

    eventWindow.dispatchEvent(new CustomEvent('vta:quick-actions-snapshot', {
      detail: {
        reportPath: 'C:/reports/new.xlsx',
        metaReportPath: 'C:/reports/new_meta.xlsx',
      },
    }))

    expect(store.latestReportPath).toBe('C:/reports/new.xlsx')
    expect(store.latestMetaReportPath).toBe('C:/reports/new_meta.xlsx')

    detach()

    eventWindow.dispatchEvent(new CustomEvent('vta:quick-actions-snapshot', {
      detail: {
        reportPath: 'C:/reports/ignored.xlsx',
        metaReportPath: 'C:/reports/ignored_meta.xlsx',
      },
    }))

    expect(store.latestReportPath).toBe('C:/reports/new.xlsx')
    expect(store.latestMetaReportPath).toBe('C:/reports/new_meta.xlsx')
  })
})
