// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { vMotionList } from './motionList'

type MotionListHook = (
  el: HTMLElement,
  binding: { value: boolean | undefined },
  vnode: unknown,
  prevVnode: unknown,
) => void

function callHook(name: 'mounted' | 'updated' | 'unmounted', el: HTMLElement, value?: boolean): void {
  const hook = (vMotionList as unknown as Record<string, MotionListHook | undefined>)[name]
  if (typeof hook === 'function') {
    hook(el, { value }, null, null)
  }
}

async function flushMotionObserver(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
  await new Promise((resolve) => window.setTimeout(resolve, 0))
}

describe('vMotionList', () => {
  let animateSpy: ReturnType<typeof vi.fn>
  let mutationCallback: MutationCallback | null

  beforeEach(() => {
    animateSpy = vi.fn(() => ({}) as Animation)
    mutationCallback = null
    class FakeMutationObserver {
      constructor(callback: MutationCallback) {
        mutationCallback = callback
      }

      observe(): void {}
      disconnect(): void {}
    }
    vi.stubGlobal('MutationObserver', FakeMutationObserver)
    Object.defineProperty(Element.prototype, 'animate', {
      configurable: true,
      value: animateSpy as unknown as Element['animate'],
    })
    const requestFrame = vi.fn((callback: FrameRequestCallback) => {
      window.setTimeout(() => callback(0), 0)
      return 1
    })
    vi.stubGlobal('requestAnimationFrame', requestFrame)
    window.requestAnimationFrame = requestFrame
    const cancelFrame = vi.fn()
    vi.stubGlobal('cancelAnimationFrame', cancelFrame)
    window.cancelAnimationFrame = cancelFrame
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: vi.fn().mockReturnValue({ matches: false }),
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('uses the latest binding value when scheduling list animations', async () => {
    const list = document.createElement('div')
    const first = document.createElement('div')
    list.appendChild(first)
    document.body.appendChild(list)

    callHook('mounted', list, true)
    expect(mutationCallback).toBeTruthy()
    list.appendChild(document.createElement('div'))
    mutationCallback?.([], {} as MutationObserver)
    await flushMotionObserver()
    expect(animateSpy).toHaveBeenCalled()
    animateSpy.mockClear()

    callHook('updated', list, false)
    list.appendChild(document.createElement('div'))
    mutationCallback?.([], {} as MutationObserver)
    await flushMotionObserver()

    expect(animateSpy).not.toHaveBeenCalled()

    callHook('updated', list, true)
    list.appendChild(document.createElement('div'))
    mutationCallback?.([], {} as MutationObserver)
    await flushMotionObserver()

    expect(animateSpy).toHaveBeenCalled()
    callHook('unmounted', list)
  })
})
