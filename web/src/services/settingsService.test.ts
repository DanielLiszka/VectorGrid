import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import { openReportsDir, openRunsDir, setWindowEffect } from './settingsService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

describe('settings service', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('maps window effect payload for native command', async () => {
    await setWindowEffect('none', false)
    await setWindowEffect('mica', true)

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'set_window_effect', { effect: null, dark: false })
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'set_window_effect', {
      effect: 'mica',
      dark: true,
    })
  })

  it('opens runs and reports directories', async () => {
    await openRunsDir()
    await openReportsDir()

    expect(invokeCommandMock).toHaveBeenCalledWith('open_runs_dir')
    expect(invokeCommandMock).toHaveBeenCalledWith('open_reports_dir')
  })
})
