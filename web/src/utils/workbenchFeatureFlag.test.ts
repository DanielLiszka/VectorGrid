import { describe, expect, it } from 'vitest'
import { resolveWorkbenchShellFlag } from './workbenchFeatureFlag'

describe('workbenchFeatureFlag', () => {
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
})
