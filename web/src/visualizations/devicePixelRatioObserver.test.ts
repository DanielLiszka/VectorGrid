import { describe, expect, it, vi } from 'vitest'
import {
  observeDevicePixelRatioChanges,
  resolveDevicePixelRatio,
} from './devicePixelRatioObserver'

interface MockMediaQuery {
  listeners: Set<(event: MediaQueryListEvent) => void>
  media: string
  query: MediaQueryList
  removeEventListener: ReturnType<typeof vi.fn>
}

function mockMediaQuery(media: string): MockMediaQuery {
  const listeners = new Set<(event: MediaQueryListEvent) => void>()
  const removeEventListener = vi.fn(
    (_type: string, listener: (event: MediaQueryListEvent) => void) => {
      listeners.delete(listener)
    },
  )
  const query = {
    media,
    matches: true,
    onchange: null,
    addEventListener: vi.fn(
      (_type: string, listener: (event: MediaQueryListEvent) => void) => {
        listeners.add(listener)
      },
    ),
    removeEventListener,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(() => true),
  } as unknown as MediaQueryList
  return {
    listeners,
    media,
    query,
    removeEventListener,
  }
}

describe('devicePixelRatioObserver', () => {
  it('normalizes invalid ratios and applies a bounded maximum', () => {
    expect(resolveDevicePixelRatio(2, 3)).toBe(2)
    expect(resolveDevicePixelRatio(2, 1.25)).toBe(1.25)
    expect(resolveDevicePixelRatio(2, 0)).toBe(1)
    expect(resolveDevicePixelRatio(2, Number.NaN)).toBe(1)
  })

  it('notifies, rearms for the new ratio, and disconnects the active query', () => {
    let pixelRatio = 1
    const queries: MockMediaQuery[] = []
    const matchMedia = vi.fn((media: string) => {
      const query = mockMediaQuery(media)
      queries.push(query)
      return query.query
    })
    const onChange = vi.fn()

    const disconnect = observeDevicePixelRatioChanges(onChange, {
      maxPixelRatio: 2,
      matchMedia,
      readPixelRatio: () => pixelRatio,
    })

    expect(matchMedia).toHaveBeenNthCalledWith(1, '(resolution: 1dppx)')
    pixelRatio = 2.5
    for (const listener of [...queries[0]!.listeners]) {
      listener({ matches: false, media: queries[0]!.media } as MediaQueryListEvent)
    }

    expect(onChange).toHaveBeenCalledOnce()
    expect(onChange).toHaveBeenCalledWith(2)
    expect(queries[0]!.removeEventListener).toHaveBeenCalledOnce()
    expect(matchMedia).toHaveBeenNthCalledWith(2, '(resolution: 2.5dppx)')

    disconnect()

    expect(queries[1]!.removeEventListener).toHaveBeenCalledOnce()
    expect(queries[1]!.listeners.size).toBe(0)
  })

  it('observes the raw ratio while capping the ratio sent to the renderer', () => {
    let pixelRatio = 3
    const queries: MockMediaQuery[] = []
    const matchMedia = vi.fn((media: string) => {
      const query = mockMediaQuery(media)
      queries.push(query)
      return query.query
    })
    const onChange = vi.fn()
    const disconnect = observeDevicePixelRatioChanges(onChange, {
      maxPixelRatio: 2,
      matchMedia,
      readPixelRatio: () => pixelRatio,
    })

    expect(matchMedia).toHaveBeenNthCalledWith(1, '(resolution: 3dppx)')
    pixelRatio = 1
    for (const listener of [...queries[0]!.listeners]) {
      listener({ matches: false, media: queries[0]!.media } as MediaQueryListEvent)
    }

    expect(onChange).toHaveBeenCalledWith(1)
    expect(matchMedia).toHaveBeenNthCalledWith(2, '(resolution: 1dppx)')

    disconnect()
  })

  it('reads the uncapped browser ratio by default when arming the query', () => {
    const pixelRatio = vi.spyOn(window, 'devicePixelRatio', 'get').mockReturnValue(3)
    const matchMedia = vi.fn((media: string) => mockMediaQuery(media).query)
    const disconnect = observeDevicePixelRatioChanges(vi.fn(), {
      maxPixelRatio: 2,
      matchMedia,
    })

    expect(matchMedia).toHaveBeenCalledWith('(resolution: 3dppx)')

    disconnect()
    pixelRatio.mockRestore()
  })

  it('removes a legacy listener with the matching legacy API', () => {
    const listeners = new Set<(event: MediaQueryListEvent) => void>()
    const removeListener = vi.fn((listener: (event: MediaQueryListEvent) => void) => {
      listeners.delete(listener)
    })
    const query = {
      media: '(resolution: 1dppx)',
      matches: true,
      onchange: null,
      addEventListener: vi.fn(() => {
        throw new Error('Modern listeners unavailable')
      }),
      removeEventListener: vi.fn(),
      addListener: vi.fn((listener: (event: MediaQueryListEvent) => void) => {
        listeners.add(listener)
      }),
      removeListener,
      dispatchEvent: vi.fn(() => true),
    } as unknown as MediaQueryList
    const disconnect = observeDevicePixelRatioChanges(vi.fn(), {
      matchMedia: () => query,
      readPixelRatio: () => 1,
    })

    expect(listeners.size).toBe(1)

    disconnect()

    expect(removeListener).toHaveBeenCalledOnce()
    expect(listeners.size).toBe(0)
  })

  it('rearms even if the redraw callback throws', () => {
    let pixelRatio = 1
    const queries: MockMediaQuery[] = []
    const matchMedia = vi.fn((media: string) => {
      const query = mockMediaQuery(media)
      queries.push(query)
      return query.query
    })
    const disconnect = observeDevicePixelRatioChanges(() => {
      throw new Error('redraw failed')
    }, {
      matchMedia,
      readPixelRatio: () => pixelRatio,
    })
    pixelRatio = 2

    expect(() => {
      for (const listener of [...queries[0]!.listeners]) {
        listener({ matches: false, media: queries[0]!.media } as MediaQueryListEvent)
      }
    }).toThrow('redraw failed')
    expect(matchMedia).toHaveBeenNthCalledWith(2, '(resolution: 2dppx)')

    disconnect()
  })

  it('stays inert when matchMedia is unavailable', () => {
    const onChange = vi.fn()
    const disconnect = observeDevicePixelRatioChanges(onChange, {
      matchMedia: null,
    })

    expect(() => disconnect()).not.toThrow()
    expect(onChange).not.toHaveBeenCalled()
  })
})
