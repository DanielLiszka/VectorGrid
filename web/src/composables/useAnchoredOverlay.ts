import { autoUpdate, flip, offset, shift, useFloating, type Placement, type Strategy } from '@floating-ui/vue'
import { useEventListener } from '@vueuse/core'
import { nextTick, ref, watch, type Ref } from 'vue'

interface UseAnchoredOverlayOptions {
  placement?: Placement
  offsetPx?: number
  paddingPx?: number
  strategy?: Strategy
}

export interface AnchoredOverlayHandle {
  referenceRef: Ref<HTMLElement | null>
  floatingRef: Ref<HTMLElement | null>
  floatingStyles: Ref<Record<string, string>>
  close: () => void
  update: () => void
}

export function useAnchoredOverlay(
  open: Ref<boolean>,
  options: UseAnchoredOverlayOptions = {},
): AnchoredOverlayHandle {
  const referenceRef = ref<HTMLElement | null>(null)
  const floatingRef = ref<HTMLElement | null>(null)
  const {
    placement = 'bottom-end',
    offsetPx = 8,
    paddingPx = 8,
    strategy = 'fixed',
  } = options

  const floatingResult = useFloating(referenceRef, floatingRef, {
    placement,
    strategy,
    whileElementsMounted: autoUpdate,
    middleware: [offset(offsetPx), flip({ padding: paddingPx }), shift({ padding: paddingPx })],
  })

  function close(): void {
    open.value = false
  }

  function update(): void {
    void floatingResult.update()
  }

  function targetInsideOverlay(target: EventTarget | null): boolean {
    const node = target as Node | null
    if (!node) return false
    return !!referenceRef.value?.contains(node) || !!floatingRef.value?.contains(node)
  }

  useEventListener(document, 'pointerdown', (event) => {
    if (!open.value) return
    if (targetInsideOverlay(event.target)) return
    close()
  })

  useEventListener(document, 'keydown', (event) => {
    if (!open.value) return
    if ((event as KeyboardEvent).key !== 'Escape') return
    close()
  })

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return
      void nextTick(() => {
        update()
      })
    },
    { flush: 'post' },
  )

  return {
    referenceRef,
    floatingRef,
    floatingStyles: floatingResult.floatingStyles as Ref<Record<string, string>>,
    close,
    update,
  }
}

