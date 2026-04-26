const DEFAULT_COMMAND_RESULTS = {
  load_latest_run_artifact_hydration: null,
  load_latest_run_artifact_summary: null,
  list_live_session_artifacts: [],
  load_live_session_artifact_detail: null,
}

export async function seedAdvancedUiMode(context) {
  await context.addInitScript(() => {
    try {
      localStorage.setItem('vta_ui_mode_v1', 'advanced')
    } catch {
    }
  })
}

export async function installMockedTauri(context, options = {}) {
  const holdStartupOverlay = options.holdStartupOverlay ?? false
  const emitTestEvents = options.emitTestEvents ?? false
  const strictUnknownInvoke = options.strictUnknownInvoke ?? true
  const commandResults = {
    ...DEFAULT_COMMAND_RESULTS,
    ...(options.commandResults ?? {}),
  }
  await context.addInitScript(({ hold, emitEvents, holdKey, strictInvoke, results }) => {
    const handlers = {}
    const callbacks = {}
    const invokeLog = []
    let nextCallbackId = 1

    const addListener = (event, callbackId) => {
      if (!handlers[event]) handlers[event] = []
      handlers[event].push(callbackId)
      return callbackId
    }
    const removeListener = (event, callbackId) => {
      handlers[event] = (handlers[event] || []).filter((id) => id !== callbackId)
      unregisterCallback(callbackId)
    }
    const emitEvent = (event, payload) => {
      const listeners = handlers[event] || []
      for (const callbackId of listeners) {
        const cb = callbacks[callbackId]
        if (typeof cb !== 'function') continue
        try {
          cb({ event, id: callbackId, payload })
        } catch {
        }
      }
    }
    const listen = async (event, cb) => {
      const callbackId = transformCallback(cb)
      addListener(event, callbackId)
      return () => {
        removeListener(event, callbackId)
      }
    }

    const invoke = async (command, args = {}) => {
      const key = String(command ?? '')
      invokeLog.push(key)
      if (key === 'plugin:event|listen') {
        return addListener(String(args.event || ''), Number(args.handler))
      }
      if (key === 'plugin:event|unlisten') {
        removeListener(String(args.event || ''), Number(args.eventId))
        return null
      }
      if (key === 'plugin:event|emit') {
        emitEvent(String(args.event || ''), args.payload)
        return null
      }
      if (Object.prototype.hasOwnProperty.call(results, key)) {
        return results[key]
      }
      if (strictInvoke) {
        throw new Error(`Unexpected mocked Tauri invoke: ${key}`)
      }
      return null
    }
    const transformCallback = (cb) => {
      const id = nextCallbackId++
      callbacks[id] = cb
      return id
    }
    const unregisterCallback = (id) => {
      delete callbacks[id]
    }

    try {
      if (hold) {
        sessionStorage.setItem(holdKey, '1')
      } else {
        sessionStorage.removeItem(holdKey)
        localStorage.removeItem(holdKey)
      }
    } catch {

    }

    window.isTauri = true
    window.__TAURI_INTERNALS__ = {
      invoke,
      transformCallback,
      unregisterCallback,
      convertFileSrc: (value) => value,
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main' },
      },
    }
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: removeListener,
    }
    window.__TAURI__ = window.__TAURI__ || {}
    window.__TAURI__.event = { listen }
    window.__TAURI__.core = { invoke }
    window.__TAURI__.invoke = invoke

    if (emitEvents) {
      window.__VTA_TEST_EMIT = emitEvent
    }
    window.__VTA_TEST_INVOKES = invokeLog
  }, {
    hold: holdStartupOverlay,
    emitEvents: emitTestEvents,
    holdKey: 'VTA_STARTUP_OVERLAY_HOLD',
    strictInvoke: strictUnknownInvoke,
    results: commandResults,
  })
}

export async function dispatchClick(page, selector) {
  const clicked = await page.evaluate((sel) => {
    const el = document.querySelector(sel)
    if (!el) return false
    ;(el instanceof HTMLElement ? el : null)?.click?.()
    return true
  }, selector)
  if (!clicked) {
    throw new Error(`Selector not found for click: ${selector}`)
  }
}

export async function dispatchClickIfPresent(page, selector) {
  return page.evaluate((sel) => {
    const el = document.querySelector(sel)
    if (!el) return false
    ;(el instanceof HTMLElement ? el : null)?.click?.()
    return true
  }, selector)
}

export async function waitForStartupOverlayToSettle(page, timeoutMs = 5000) {
  await page.waitForFunction(() => !document.querySelector('#startupAnimationOverlay'), {
    timeout: timeoutMs,
  })
}

export async function dismissStartupOverlayIfPresent(page, options = {}) {
  const overlayVisibleTimeoutMs = options.overlayVisibleTimeoutMs ?? 2000
  const dismissDelayMs = options.dismissDelayMs ?? 460
  const settleTimeoutMs = options.settleTimeoutMs ?? 5000
  const overlay = page.locator('#startupAnimationOverlay')
  if (await overlay.count() === 0) return false
  try {
    await overlay.waitFor({ state: 'visible', timeout: overlayVisibleTimeoutMs })
  } catch {
    return false
  }
  await page.waitForTimeout(dismissDelayMs)
  await dispatchClickIfPresent(page, '.startupSkipButton')
  await waitForStartupOverlayToSettle(page, settleTimeoutMs)
  return true
}
