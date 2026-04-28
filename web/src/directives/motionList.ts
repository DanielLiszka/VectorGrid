import type { Directive, DirectiveBinding } from 'vue'

interface MotionListState {
  observer: MutationObserver
  snapshot: Map<Element, DOMRectReadOnly>
  frameId: number | null
}

const states = new WeakMap<HTMLElement, MotionListState>()
const DURATION_MS = 160
const DISTANCE_PX = 4

function animationAllowed(binding: DirectiveBinding<boolean | undefined>): boolean {
  if (binding.value === false) return false
  if (typeof window === 'undefined' || typeof MutationObserver === 'undefined') return false
  if (typeof Element === 'undefined' || typeof Element.prototype.animate !== 'function') return false
  return !window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches
}

function childrenOf(el: HTMLElement): HTMLElement[] {
  return Array.from(el.children).filter((child): child is HTMLElement => child instanceof HTMLElement)
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

function animateChild(child: HTMLElement, first: DOMRectReadOnly | null, last: DOMRectReadOnly): void {
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

function runAnimation(el: HTMLElement, state: MotionListState, binding: DirectiveBinding<boolean | undefined>): void {
  state.frameId = null
  if (!animationAllowed(binding)) {
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

function scheduleAnimation(el: HTMLElement, state: MotionListState, binding: DirectiveBinding<boolean | undefined>): void {
  if (state.frameId !== null) return
  state.frameId = requestFrame(() => runAnimation(el, state, binding))
}

export const vMotionList: Directive<HTMLElement, boolean | undefined> = {
  mounted(el, binding) {
    if (typeof MutationObserver === 'undefined') return
    const state: MotionListState = {
      observer: new MutationObserver(() => scheduleAnimation(el, state, binding)),
      snapshot: snapshotChildren(el),
      frameId: null,
    }
    states.set(el, state)
    state.observer.observe(el, { childList: true })
  },
  unmounted(el) {
    const state = states.get(el)
    if (!state) return
    state.observer.disconnect()
    if (state.frameId !== null) cancelFrame(state.frameId)
    states.delete(el)
  },
}
