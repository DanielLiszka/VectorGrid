// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { resolveRuntimePinia, bindRuntimePinia } from './stores/runtimePinia'
import { useRuntimeStore } from './stores/runtimeStore'

function createDeferred<T = void>() {
  let resolve!: (value: T | PromiseLike<T>) => void
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve
    void nextReject
  })
  return { promise, resolve }
}

vi.mock('./App.vue', () => ({
  default: {
    name: 'AppStub',
    render: () => null,
  },
}))

vi.mock('./services/recommendedOptimizationDefaults', () => ({
  applyRecommendedOptimizationDefaults: vi.fn(),
}))

vi.mock('./services/desktopProductization', () => ({
  initializeDesktopProductization: vi.fn(async () => undefined),
}))

vi.mock('./services/settingsBootstrap', () => ({
  applySettingsBootstrapFromStorage: vi.fn(),
}))

vi.mock('./services/workflowBootstrap', () => ({
  applyWorkflowBootstrapFromStorage: vi.fn(async () => undefined),
}))

vi.mock('./tauri/instrumentation', () => ({
  installTauriInstrumentation: vi.fn(),
}))

vi.mock('./utils/storage', async () => {
  const actual = await vi.importActual<typeof import('./utils/storage')>('./utils/storage')
  return {
    ...actual,
    runStorageMigrations: vi.fn(() => 2),
  }
})

import { startVectorGridApp } from './main'
import { initializeDesktopProductization } from './services/desktopProductization'
import { applyWorkflowBootstrapFromStorage } from './services/workflowBootstrap'

describe('startVectorGridApp', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    bindRuntimePinia(null)
    vi.clearAllMocks()
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      cb(0)
      return 1
    })
  })

  it('waits for critical workflow bootstrap hydration before marking the runtime ready', async () => {
    const deferred = createDeferred<void>()
    vi.mocked(initializeDesktopProductization).mockResolvedValueOnce(undefined)
    vi.mocked(applyWorkflowBootstrapFromStorage).mockReturnValueOnce(deferred.promise)

    const root = document.createElement('div')
    document.body.appendChild(root)

    let settled = false
    const appPromise = startVectorGridApp(root).then(() => {
      settled = true
    })

    await Promise.resolve()
    await Promise.resolve()

    expect(vi.mocked(applyWorkflowBootstrapFromStorage)).toHaveBeenCalledWith({ includeDeferredData: false })
    expect(vi.mocked(initializeDesktopProductization)).toHaveBeenCalled()
    expect(settled).toBe(false)

    const pinia = resolveRuntimePinia()
    expect(pinia).not.toBeNull()
    const runtimeStore = useRuntimeStore(pinia!)
    expect(runtimeStore.bootPhase).toBe('booting')
    expect(runtimeStore.bootStage).toBe('Finalizing dataset readiness')

    deferred.resolve()
    await appPromise

    expect(runtimeStore.bootPhase).toBe('ready')
    expect(runtimeStore.bootStage).toBe('Frontend runtime ready')
    expect(settled).toBe(true)
  })
})
