import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

interface UseShellMountOptions {
  paneSelector?: string
  paneDatasetKey?: string
}

export function useShellMount(targetSelector: string, options: UseShellMountOptions = {}) {
  const mountReady = ref(false)
  const paneRoot = ref<HTMLElement | null>(null)

  function resolveMount(): void {
    const mount = document.querySelector(targetSelector) as HTMLElement | null
    if (!mount) return
    if (options.paneSelector) {
      const pane = document.querySelector(options.paneSelector) as HTMLElement | null
      if (!pane) return
      paneRoot.value = pane
      if (options.paneDatasetKey) {
        ;(pane.dataset as Record<string, string | undefined>)[options.paneDatasetKey] = '1'
      }
    }
    mountReady.value = true
  }

  onMounted(async () => {
    resolveMount()
    if (!mountReady.value) {
      await nextTick()
      resolveMount()
    }
  })

  onBeforeUnmount(() => {
    if (paneRoot.value && options.paneDatasetKey) {
      delete (paneRoot.value.dataset as Record<string, string | undefined>)[options.paneDatasetKey]
    }
    paneRoot.value = null
  })

  return {
    mountReady,
    paneRoot,
    resolveMount,
  }
}
