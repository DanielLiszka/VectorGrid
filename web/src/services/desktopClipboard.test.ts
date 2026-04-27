// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { tauriRuntimeAvailable } from '../tauri/invoke'
import { copyTextToDesktopClipboard } from './desktopClipboard'

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText: vi.fn(async () => undefined),
}))

vi.mock('../tauri/invoke', () => ({
  tauriRuntimeAvailable: vi.fn(() => false),
}))

const writeTextMock = vi.mocked(writeText)
const tauriRuntimeAvailableMock = vi.mocked(tauriRuntimeAvailable)

describe('desktopClipboard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    tauriRuntimeAvailableMock.mockReturnValue(false)
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: {},
    })
    document.body.innerHTML = ''
    document.execCommand = vi.fn(() => false)
  })

  it('uses native clipboard first in Tauri runtime', async () => {
    tauriRuntimeAvailableMock.mockReturnValue(true)

    const result = await copyTextToDesktopClipboard('diagnostics')

    expect(result).toEqual({ ok: true, method: 'tauri-clipboard' })
    expect(writeTextMock).toHaveBeenCalledWith('diagnostics')
  })

  it('falls back to navigator clipboard when native clipboard fails', async () => {
    tauriRuntimeAvailableMock.mockReturnValue(true)
    writeTextMock.mockRejectedValueOnce(new Error('permission denied'))
    const writeNavigatorText = vi.fn(async () => undefined)
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: { clipboard: { writeText: writeNavigatorText } },
    })

    const result = await copyTextToDesktopClipboard('summary')

    expect(result).toEqual({ ok: true, method: 'navigator-clipboard' })
    expect(writeNavigatorText).toHaveBeenCalledWith('summary')
  })

  it('uses execCommand as the final browser fallback', async () => {
    document.execCommand = vi.fn(() => true)

    const result = await copyTextToDesktopClipboard('legacy')

    expect(result).toEqual({ ok: true, method: 'exec-command' })
    expect(document.execCommand).toHaveBeenCalledWith('copy')
    expect(document.body.querySelector('textarea')).toBeNull()
  })

  it('reports failure when no clipboard path succeeds', async () => {
    document.execCommand = vi.fn(() => false)

    const result = await copyTextToDesktopClipboard('blocked')

    expect(result).toEqual({ ok: false, method: 'none' })
  })
})
