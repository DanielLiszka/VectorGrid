// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { resolveRuntimePinia, bindRuntimePinia } from './stores/runtimePinia'
import { BOOT_STAGES, useRuntimeStore } from './stores/runtimeStore'

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
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      cb(0)
      return 1
    })
  })

  it('waits for the first paint boundary before starting bootstrap work', async () => {
    let paintCallback: FrameRequestCallback | null = null
    let paintCalls = 0
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      paintCalls += 1
      if (paintCalls === 1) {
        paintCallback = cb
      } else {
        cb(0)
      }
      return 1
    })

    const root = document.createElement('div')
    document.body.appendChild(root)

    const appPromise = startVectorGridApp(root)

    await Promise.resolve()
    expect(vi.mocked(applyWorkflowBootstrapFromStorage)).not.toHaveBeenCalled()
    expect(vi.mocked(initializeDesktopProductization)).not.toHaveBeenCalled()

    expect(paintCallback).toBeTruthy()
    const releaseFirstPaint = paintCallback!
    releaseFirstPaint(0)
    await appPromise

    expect(vi.mocked(applyWorkflowBootstrapFromStorage)).toHaveBeenCalledWith({ includeDeferredData: false })
    expect(vi.mocked(initializeDesktopProductization)).toHaveBeenCalled()
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
    expect(runtimeStore.bootStage).toBe(BOOT_STAGES.RESTORING_WORKSPACE_STATE)

    deferred.resolve()
    await appPromise

    expect(runtimeStore.bootPhase).toBe('ready')
    expect(runtimeStore.bootStage).toBe(BOOT_STAGES.FRONTEND_RUNTIME_READY)
    expect(settled).toBe(true)
  })

  it('records a recoverable desktop startup warning but still reaches ready', async () => {
    vi.mocked(initializeDesktopProductization).mockRejectedValueOnce(new Error('Desktop bridge failed'))
    vi.mocked(applyWorkflowBootstrapFromStorage).mockResolvedValueOnce(undefined)

    const root = document.createElement('div')
    document.body.appendChild(root)

    await startVectorGridApp(root)

    const pinia = resolveRuntimePinia()
    expect(pinia).not.toBeNull()
    const runtimeStore = useRuntimeStore(pinia!)
    expect(runtimeStore.bootPhase).toBe('ready')
    expect(runtimeStore.bootStage).toBe(BOOT_STAGES.FRONTEND_RUNTIME_READY)
    expect(runtimeStore.desktopLogs[0]?.level).toBe('warn')
    expect(runtimeStore.desktopLogs[0]?.message).toContain('Desktop startup service failed: Desktop bridge failed')
  })

  it('marks the runtime failed when critical bootstrap work throws', async () => {
    vi.mocked(initializeDesktopProductization).mockResolvedValueOnce(undefined)
    vi.mocked(applyWorkflowBootstrapFromStorage).mockRejectedValueOnce(new Error('Dataset bootstrap failed'))

    const root = document.createElement('div')
    document.body.appendChild(root)

    await startVectorGridApp(root)

    const pinia = resolveRuntimePinia()
    expect(pinia).not.toBeNull()
    const runtimeStore = useRuntimeStore(pinia!)
    expect(runtimeStore.bootPhase).toBe('error')
    expect(runtimeStore.bootStage).toBe(BOOT_STAGES.FRONTEND_RUNTIME_FAILED)
    expect(runtimeStore.bootError).toContain('Dataset bootstrap failed')
  })
})
