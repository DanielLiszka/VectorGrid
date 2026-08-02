import {
  Teleport,
  defineComponent,
  h,
  markRaw,
  onBeforeUnmount,
  ref,
  shallowRef,
  type Component,
} from 'vue'
import AsyncPanelLoadState from './AsyncPanelLoadState.vue'

type AsyncPanelModule = Component | { default: Component }

export interface RecoverableAsyncPanelOptions {
  label: string
  loader: () => Promise<AsyncPanelModule>
  recoveryTarget?: string
  loadingDelayMs?: number
}

function unwrapAsyncPanel(module: AsyncPanelModule): Component {
  if (module && typeof module === 'object' && 'default' in module) {
    if (module.default) return module.default
  } else if (module) {
    return module
  }

  throw new Error('The asynchronous panel module did not export a Vue component.')
}

function componentName(label: string): string {
  const suffix = label.replace(/[^a-zA-Z0-9]+/g, '')
  return `RecoverableAsync${suffix || 'Panel'}`
}

export function createRecoverableAsyncPanel(options: RecoverableAsyncPanelOptions): Component {
  const loadingDelayMs = Math.max(0, options.loadingDelayMs ?? 180)
  let cachedPanel: Component | null = null
  let pendingLoad: Promise<Component> | null = null

  function resolvePanel(): Promise<Component> {
    if (cachedPanel) return Promise.resolve(cachedPanel)
    if (pendingLoad) return pendingLoad

    pendingLoad = options
      .loader()
      .then((loadedModule) => {
        cachedPanel = markRaw(unwrapAsyncPanel(loadedModule))
        return cachedPanel
      })
      .finally(() => {
        pendingLoad = null
      })
    return pendingLoad
  }

  return defineComponent({
    name: componentName(options.label),
    inheritAttrs: false,
    setup(_props, { attrs, slots }) {
      const resolvedPanel = shallowRef<Component | null>(cachedPanel)
      const loading = ref(false)
      const showLoadingState = ref(false)
      const failed = ref(false)
      let disposed = false
      let requestId = 0
      let loadingTimer: ReturnType<typeof setTimeout> | undefined

      function clearLoadingTimer(): void {
        if (loadingTimer === undefined) return
        clearTimeout(loadingTimer)
        loadingTimer = undefined
      }

      async function loadPanel(): Promise<void> {
        const activeRequestId = ++requestId
        clearLoadingTimer()
        failed.value = false
        loading.value = true
        showLoadingState.value = loadingDelayMs === 0

        if (loadingDelayMs > 0) {
          loadingTimer = setTimeout(() => {
            if (!disposed && requestId === activeRequestId && loading.value) {
              showLoadingState.value = true
            }
          }, loadingDelayMs)
        }

        try {
          const loadedPanel = await resolvePanel()
          if (disposed || requestId !== activeRequestId) return
          resolvedPanel.value = loadedPanel
        } catch {
          if (disposed || requestId !== activeRequestId) return
          failed.value = true
        } finally {
          if (!disposed && requestId === activeRequestId) {
            clearLoadingTimer()
            loading.value = false
            showLoadingState.value = false
          }
        }
      }

      function renderRecoveryState() {
        const recoveryState = h(AsyncPanelLoadState, {
          label: options.label,
          loading: loading.value,
          onRetry: loadPanel,
        })
        const target = options.recoveryTarget

        if (
          target &&
          typeof document !== 'undefined' &&
          document.querySelector(target)
        ) {
          return h(Teleport, { to: target }, recoveryState)
        }

        return recoveryState
      }

      onBeforeUnmount(() => {
        disposed = true
        requestId += 1
        clearLoadingTimer()
      })

      if (!resolvedPanel.value) {
        void loadPanel()
      }

      return () => {
        if (resolvedPanel.value) {
          return h(resolvedPanel.value, attrs, slots)
        }
        if (failed.value || (loading.value && showLoadingState.value)) {
          return renderRecoveryState()
        }
        return null
      }
    },
  })
}
