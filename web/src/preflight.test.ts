// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  CONTROLS_COLLAPSED_KEY,
  CONTROLS_WIDTH_KEY,
  DENSITY_MODE_KEY,
  GUIDED_STEP_KEY,
  HELP_MODE_KEY,
  THEME_PREF_KEY,
  UI_MODE_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_TAB_KEY,
} from './utils/core/storageKeys'

function stubMatchMedia(prefersDark: boolean): void {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: query === '(prefers-color-scheme: dark)' ? prefersDark : false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  })
}

async function importPreflightFor(userAgent: string): Promise<void> {
  Object.defineProperty(window.navigator, 'userAgent', {
    configurable: true,
    value: userAgent,
  })
  await import('./preflight')
}

describe('preflight', () => {
  beforeEach(() => {
    vi.resetModules()
    localStorage.clear()
    document.documentElement.style.removeProperty('--controls-w')
    for (const key of [
      'themePref',
      'theme',
      'controlsCollapsed',
      'workflowAdvanced',
      'windowEffect',
      'workflow',
      'workspace',
      'density',
      'helpMode',
      'uiMode',
      'guidedStep',
    ]) {
      delete document.documentElement.dataset[key as keyof DOMStringMap]
    }
    stubMatchMedia(false)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('defaults first-run users to guided mode before Vue mounts', async () => {
    await importPreflightFor('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')

    expect(document.documentElement.dataset.uiMode).toBe('guided')
    expect(document.documentElement.dataset.guidedStep).toBe('data')
  })

  it('applies root dataset attributes from persisted shell preferences', async () => {
    localStorage.setItem(THEME_PREF_KEY, 'system')
    localStorage.setItem(CONTROLS_COLLAPSED_KEY, '1')
    localStorage.setItem(WORKFLOW_ADVANCED_KEY, 'true')
    localStorage.setItem(WORKFLOW_TAB_KEY, 'optimize')
    localStorage.setItem(WORKSPACE_TAB_KEY, 'workflow')
    localStorage.setItem(DENSITY_MODE_KEY, 'dense')
    localStorage.setItem(HELP_MODE_KEY, 'concise')
    localStorage.setItem(UI_MODE_KEY, 'guided')
    localStorage.setItem(GUIDED_STEP_KEY, 'trade')
    localStorage.setItem(CONTROLS_WIDTH_KEY, '417')
    stubMatchMedia(true)

    await importPreflightFor('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')

    expect(document.documentElement.dataset.themePref).toBe('system')
    expect(document.documentElement.dataset.theme).toBe('dark')
    expect(document.documentElement.dataset.controlsCollapsed).toBe('1')
    expect(document.documentElement.dataset.workflowAdvanced).toBe('1')
    expect(document.documentElement.dataset.workflow).toBe('optimize')
    expect(document.documentElement.dataset.workspace).toBe('workflow')
    expect(document.documentElement.dataset.density).toBe('dense')
    expect(document.documentElement.dataset.helpMode).toBe('concise')
    expect(document.documentElement.dataset.uiMode).toBe('guided')
    expect(document.documentElement.dataset.guidedStep).toBe('trade')
    expect(document.documentElement.style.getPropertyValue('--controls-w')).toBe('417px')
  })

  it('filters unsupported window effects by platform during preflight', async () => {
    localStorage.setItem(WINDOW_EFFECT_KEY, 'vibrancy')

    await importPreflightFor('Mozilla/5.0 (X11; Linux x86_64)')

    expect(document.documentElement.dataset.windowEffect).toBeUndefined()

    vi.resetModules()
    delete document.documentElement.dataset.windowEffect
    localStorage.setItem(WINDOW_EFFECT_KEY, 'blur')

    await importPreflightFor('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')

    expect(document.documentElement.dataset.windowEffect).toBe('blur')

    vi.resetModules()
    delete document.documentElement.dataset.windowEffect
    localStorage.setItem(WINDOW_EFFECT_KEY, 'vibrancy')

    await importPreflightFor('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5)')

    expect(document.documentElement.dataset.windowEffect).toBe('vibrancy')
  })
})
