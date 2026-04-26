// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import {
  WORKBENCH_SHELL_STORAGE_KEY,
  resolveWorkbenchShellFlag,
  shouldUseWorkbenchShell,
} from './workbenchFeatureFlag'

describe('workbenchFeatureFlag', () => {
  beforeEach(() => {
    localStorage.clear()
    window.history.pushState({}, '', '/')
  })

  it('recognizes explicit enabled and disabled values', () => {
    expect(resolveWorkbenchShellFlag('1')).toBe(true)
    expect(resolveWorkbenchShellFlag('true')).toBe(true)
    expect(resolveWorkbenchShellFlag('enabled')).toBe(true)
    expect(resolveWorkbenchShellFlag('0')).toBe(false)
    expect(resolveWorkbenchShellFlag('false')).toBe(false)
    expect(resolveWorkbenchShellFlag('off')).toBe(false)
  })

  it('treats empty or unknown values as unset', () => {
    expect(resolveWorkbenchShellFlag('')).toBeNull()
    expect(resolveWorkbenchShellFlag(null)).toBeNull()
    expect(resolveWorkbenchShellFlag('maybe')).toBeNull()
  })

  it('uses the workbench shell by default with explicit query and storage opt-outs', () => {
    window.history.pushState({}, '', '/')
    localStorage.clear()
    expect(shouldUseWorkbenchShell()).toBe(true)

    localStorage.setItem(WORKBENCH_SHELL_STORAGE_KEY, '0')
    expect(shouldUseWorkbenchShell()).toBe(false)

    window.history.pushState({}, '', '/?workbench_shell=1')
    expect(shouldUseWorkbenchShell()).toBe(true)

    window.history.pushState({}, '', '/?workbench_shell=0')
    expect(shouldUseWorkbenchShell()).toBe(false)
  })
})
