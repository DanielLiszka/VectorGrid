const DEFAULT_COMMAND_RESULTS = {
  load_latest_run_artifact_summary: null,
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

    const listen = async (event, cb) => {
      if (!handlers[event]) handlers[event] = []
      handlers[event].push(cb)
      return () => {
        handlers[event] = (handlers[event] || []).filter((fn) => fn !== cb)
      }
    }

    const invoke = async (command) => {
      const key = String(command ?? '')
      invokeLog.push(key)
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
        localStorage.setItem(holdKey, '1')
      } else {
        localStorage.removeItem(holdKey)
      }
    } catch {
      // ignore storage failures in mocked browser harnesses
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
      unregisterListener: () => {},
    }
    window.__TAURI__ = window.__TAURI__ || {}
    window.__TAURI__.event = { listen }
    window.__TAURI__.core = { invoke }
    window.__TAURI__.invoke = invoke

    if (emitEvents) {
      window.__VTA_TEST_EMIT = (event, payload) => {
        const listeners = handlers[event] || []
        for (const cb of listeners) {
          try {
            cb({ event, payload })
          } catch {
            // ignore callback failures in mocked browser harnesses
          }
        }
      }
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
