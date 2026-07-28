import { describe, expect, it } from 'vitest'
import { maximizedBoundsExceedMonitor } from './windowGeometrySafety'

describe('windowGeometrySafety', () => {
  it('accepts a monitor-sized maximized window with a small platform tolerance', () => {
    expect(maximizedBoundsExceedMonitor(
      { width: 3842, height: 2162 },
      { width: 3840, height: 2160 },
    )).toBe(false)
  })

  it('rejects virtual-desktop-sized maximize bounds', () => {
    expect(maximizedBoundsExceedMonitor(
      { width: 7680, height: 4272 },
      { width: 3840, height: 2160 },
    )).toBe(true)
  })

  it('fails closed for invalid geometry', () => {
    expect(maximizedBoundsExceedMonitor(
      { width: Number.NaN, height: 720 },
      { width: 1920, height: 1080 },
    )).toBe(true)
  })
})
