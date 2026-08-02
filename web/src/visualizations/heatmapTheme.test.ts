import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  heatmapContrastRatio,
  heatmapLegendRangeAnchors,
  heatmapLegendStops,
  heatmapLegendZeroOffset,
  heatmapPalette,
  heatmapPaletteSemanticLabels,
  heatmapVisualizationTokens,
  interpolateHeatmapColorOklab,
  observeHeatmapTheme,
  resolveHeatmapPaletteTheme,
  sampleHeatmapColor,
} from './heatmapTheme'

describe('heatmapTheme', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('keeps diverging endpoints exact and reserves the midpoint for an honest zero', () => {
    const palette = heatmapPalette('objective', 'light')

    expect(sampleHeatmapColor(-1, -1, 1, palette, 'light')).toBe(
      'rgb(182, 64, 91)',
    )
    expect(sampleHeatmapColor(0, -1, 1, palette, 'light')).toBe(
      'rgb(244, 242, 236)',
    )
    expect(sampleHeatmapColor(1, -1, 1, palette, 'light')).toBe(
      'rgb(0, 106, 115)',
    )
    expect(heatmapPaletteSemanticLabels(palette)).toEqual({
      low: 'Negative',
      middle: 'Neutral · 0',
      high: 'Positive',
    })

    const legend = heatmapLegendStops(palette, 'light', 24)
    expect(legend[0]?.color).toBe('rgb(182, 64, 91)')
    expect(legend[12]?.color).toBe('rgb(244, 242, 236)')
    expect(legend[24]?.color).toBe('rgb(0, 106, 115)')
  })

  it('samples diverging legends over the actual asymmetric value domain', () => {
    const palette = heatmapPalette('objective', 'light')
    const range = { min: -2, max: 6 }
    const legend = heatmapLegendStops(palette, 'light', 24, range)

    expect(heatmapLegendZeroOffset(range)).toBe(0.25)
    expect(heatmapLegendRangeAnchors(range, palette)).toEqual({
      min: -2,
      middle: 0,
      max: 6,
      middleOffset: 0.25,
      zeroOffset: 0.25,
    })
    expect(legend[0]?.color).toBe(
      sampleHeatmapColor(-2, -2, 6, palette, 'light'),
    )
    expect(legend[6]?.color).toBe(
      sampleHeatmapColor(0, -2, 6, palette, 'light'),
    )
    expect(legend[24]?.color).toBe(
      sampleHeatmapColor(6, -2, 6, palette, 'light'),
    )
  })

  it('uses real midpoint semantics and no zero anchor for one-sided domains', () => {
    const palette = heatmapPalette('objective', 'dark')
    const range = { min: 2, max: 6 }

    expect(heatmapLegendZeroOffset(range)).toBeNull()
    expect(heatmapLegendRangeAnchors(range, palette)).toEqual({
      min: 2,
      middle: 4,
      max: 6,
      middleOffset: 0.5,
      zeroOffset: null,
    })
    expect(heatmapPaletteSemanticLabels(palette, range)).toEqual({
      low: 'Lower positive',
      middle: 'Positive',
      high: 'Higher positive',
    })

    const legend = heatmapLegendStops(palette, 'dark', 8, range)
    expect(legend[0]?.color).toBe(
      sampleHeatmapColor(2, 2, 6, palette, 'dark'),
    )
    expect(legend[4]?.color).toBe(
      sampleHeatmapColor(4, 2, 6, palette, 'dark'),
    )
  })

  it('uses perceptual OKLab interpolation instead of straight RGB channel mixing', () => {
    const midpoint = interpolateHeatmapColorOklab('#B6405B', '#F4F2EC', 0.5)

    expect(midpoint).toMatch(/^rgb\(\d+, \d+, \d+\)$/u)
    expect(midpoint).not.toBe('rgb(213, 153, 164)')
  })

  it('provides color-vision-safer endpoint separation in both themes', () => {
    for (const theme of ['light', 'dark'] as const) {
      const objective = heatmapPalette('objective', theme)
      const plateau = heatmapPalette('plateau', theme)
      expect(objective.kind).toBe('diverging')
      expect(plateau.kind).toBe('sequential')
      if (objective.kind !== 'diverging' || plateau.kind !== 'sequential') continue

      expect(
        heatmapContrastRatio(objective.negative, objective.zero),
      ).toBeGreaterThanOrEqual(4.5)
      expect(
        heatmapContrastRatio(objective.positive, objective.zero),
      ).toBeGreaterThanOrEqual(4.5)
      expect(
        heatmapContrastRatio(plateau.low, plateau.high),
      ).toBeGreaterThanOrEqual(5)
    }
  })

  it('keeps Canvas and Three.js contrast tokens aligned by resolved theme', () => {
    const light = heatmapVisualizationTokens('light')
    const dark = heatmapVisualizationTokens('dark')

    expect(heatmapContrastRatio(light.markerFill, light.markerStroke)).toBeGreaterThan(12)
    expect(heatmapContrastRatio(dark.markerFill, dark.markerStroke)).toBeGreaterThan(12)
    expect(light.axisFast).not.toBe(dark.axisFast)
    expect(light.surfacePartial).not.toBe(dark.surfacePartial)

    const lightPalette = heatmapPalette('objective', 'light')
    expect(resolveHeatmapPaletteTheme(lightPalette, 'dark')).toEqual(
      heatmapPalette('objective', 'dark'),
    )
  })

  it('keeps sequential stability scales bounded at their declared endpoints', () => {
    const palette = heatmapPalette('plateau', 'dark')

    expect(sampleHeatmapColor(0, 0, 1, palette, 'dark')).toBe(
      'rgb(76, 75, 157)',
    )
    expect(sampleHeatmapColor(1, 0, 1, palette, 'dark')).toBe(
      'rgb(245, 215, 110)',
    )
    expect(sampleHeatmapColor(0.5, 0, 1, palette, 'dark')).not.toBe(
      sampleHeatmapColor(0, 0, 1, palette, 'dark'),
    )
  })

  it('treats MutationObserver construction and observe failures as nonfatal', () => {
    class ThrowingConstructor {
      constructor() {
        throw new Error('constructor unavailable')
      }
    }
    vi.stubGlobal('MutationObserver', ThrowingConstructor)
    expect(() => observeHeatmapTheme(() => undefined)).not.toThrow()
    expect(() => observeHeatmapTheme(() => undefined)()).not.toThrow()

    const disconnect = vi.fn()
    class ThrowingObserve {
      observe(): void {
        throw new Error('observe unavailable')
      }

      disconnect(): void {
        disconnect()
      }
    }
    vi.stubGlobal('MutationObserver', ThrowingObserve)
    expect(() => observeHeatmapTheme(() => undefined)).not.toThrow()
    expect(disconnect).toHaveBeenCalledTimes(1)
  })
})
