import type { Directive } from 'vue'

interface MotionListState {
  observer: MutationObserver
  snapshot: Map<Element, DOMRectReadOnly>
  frameId: number | null
  enabled: boolean | undefined
}

const states = new WeakMap<HTMLElement, MotionListState>()
const DURATION_MS = 160
const DISTANCE_PX = 4

function animationAllowed(enabled: boolean | undefined): boolean {
  if (enabled === false) return false
  if (typeof window === 'undefined' || typeof MutationObserver === 'undefined') return false
  if (typeof Element === 'undefined' || typeof Element.prototype.animate !== 'function') return false
  return !window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches
}

function childrenOf(el: HTMLElement): Element[] {
  return Array.from(el.children)
}

function snapshotChildren(el: HTMLElement): Map<Element, DOMRectReadOnly> {
  const snapshot = new Map<Element, DOMRectReadOnly>()
  for (const child of childrenOf(el)) {
    snapshot.set(child, child.getBoundingClientRect())
  }
  return snapshot
}

function requestFrame(callback: FrameRequestCallback): number {
  if (typeof requestAnimationFrame === 'function') return requestAnimationFrame(callback)
  return window.setTimeout(() => callback(performance.now()), 16)
}

function cancelFrame(frameId: number): void {
  if (typeof cancelAnimationFrame === 'function') {
    cancelAnimationFrame(frameId)
    return
  }
  window.clearTimeout(frameId)
}

function animateChild(child: Element, first: DOMRectReadOnly | null, last: DOMRectReadOnly): void {
  if (!first) {
    child.animate(
      [
        { opacity: 0, transform: `translateY(${DISTANCE_PX * -1}px)` },
        { opacity: 1, transform: 'translateY(0)' },
      ],
      { duration: DURATION_MS, easing: 'cubic-bezier(0.2, 0, 0, 1)' },
    )
    return
  }

  const dx = first.left - last.left
  const dy = first.top - last.top
  if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) return

  child.animate(
    [
      { transform: `translate(${dx}px, ${dy}px)` },
      { transform: 'translate(0, 0)' },
    ],
    { duration: DURATION_MS, easing: 'cubic-bezier(0.2, 0, 0, 1)' },
  )
}

function runAnimation(el: HTMLElement, state: MotionListState): void {
  state.frameId = null
  if (!animationAllowed(state.enabled)) {
    state.snapshot = snapshotChildren(el)
    return
  }

  const nextSnapshot = snapshotChildren(el)
  for (const child of childrenOf(el)) {
    const last = nextSnapshot.get(child)
    if (!last) continue
    animateChild(child, state.snapshot.get(child) ?? null, last)
  }
  state.snapshot = nextSnapshot
}

function scheduleAnimation(el: HTMLElement, state: MotionListState): void {
  if (state.frameId !== null) return
  state.frameId = requestFrame(() => runAnimation(el, state))
}

export const vMotionList: Directive<HTMLElement, boolean | undefined> = {
  mounted(el, binding) {
    if (typeof MutationObserver === 'undefined') return
    const state: MotionListState = {
      observer: new MutationObserver(() => scheduleAnimation(el, state)),
      snapshot: snapshotChildren(el),
      frameId: null,
      enabled: binding.value,
    }
    states.set(el, state)
    state.observer.observe(el, { childList: true })
  },
  updated(el, binding) {
    const state = states.get(el)
    if (state) {
      state.enabled = binding.value
    }
  },
  unmounted(el) {
    const state = states.get(el)
    if (!state) return
    state.observer.disconnect()
    if (state.frameId !== null) cancelFrame(state.frameId)
    states.delete(el)
  },
}
