import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { HeatmapCanvasRenderModel } from '../../visualizations/heatmapCanvasModel'
import HeatmapSurface3D from './HeatmapSurface3D.vue'

function renderModel(): HeatmapCanvasRenderModel {
  const cells = [
    { params: { fast_len: 5, slow_len: 20 }, metrics: { sharpe: 0.1 } },
    { params: { fast_len: 6, slow_len: 20 }, metrics: { sharpe: 0.2 } },
    { params: { fast_len: 5, slow_len: 30 }, metrics: { sharpe: 0.3 } },
    { params: { fast_len: 6, slow_len: 30 }, metrics: { sharpe: 0.4 } },
  ]
  return {
    signature: 'heatmap:3d:keyboard',
    meta: 'Sharpe heatmap',
    statusLine: 'Heatmap ready.',
    model: {
      binsFast: 2,
      binsSlow: 2,
      fastMin: 5,
      fastMax: 6,
      slowMin: 20,
      slowMax: 30,
      values: [0.1, 0.2, 0.3, 0.4],
      meanValues: null,
      sampleCounts: null,
    },
    displayMode: 'objective',
    displayValues: [0.1, 0.2, 0.3, 0.4],
    displayLabel: 'Sharpe',
    displayLegendNote: 'Zero-centered',
    request: {
      fastMaType: 'ema',
      slowMaType: 'ema',
      fastMaParams: null,
      slowMaParams: null,
      scoreLabel: 'Sharpe',
    },
    markers: [],
    range: { min: -0.4, max: 0.4, label: 'zero-centered' },
    palette: { kind: 'diverging', negative: '#b91c1c', zero: '#2563eb', positive: '#15803d' },
    plateauDiagnostics: null,
    slice3d: {
      axisLabel: 'MA variants',
      scoreLabel: 'Sharpe',
      slices: [
        { key: 'first', label: 'EMA / EMA', shortLabel: 'EMA/EMA', values: [0.1, 0.2, 0.3, 0.4], sampleCounts: [1, 1, 1, 1], cells, finiteCells: 4, bestScore: 0.4 },
        { key: 'second', label: 'SMA / EMA', shortLabel: 'SMA/EMA', values: [0.2, 0.3, 0.4, 0.5], sampleCounts: [1, 1, 1, 1], cells, finiteCells: 4, bestScore: 0.5 },
      ],
      range: { min: -0.5, max: 0.5, label: 'zero-centered' },
      palette: { kind: 'diverging', negative: '#b91c1c', zero: '#2563eb', positive: '#15803d' },
      sourceRowCount: 8,
      retainedSliceRowCount: 8,
      unavailableReason: '',
    },
  }
}

describe('HeatmapSurface3D', () => {
  it('keeps an explicit Top results handoff and describes the direct keyboard chart path', async () => {
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D sliced optimization heatmap for retained parameter variants.',
        selectedSummary: 'Selected 3D slice summary.',
      },
    })

    expect(wrapper.get('#heatmap3dEmptyHintVue').text()).toContain('3D slices require include_all=true')
    expect(wrapper.get('#heatmapSurface3DInstructionsVue').text()).toContain('use Arrow keys')
    expect(wrapper.get('#heatmapSurface3DViewportVue').attributes('role')).toBe('region')
    expect(wrapper.get('#heatmapSurface3DViewportVue').attributes('tabindex')).toBe('-1')
    expect(wrapper.get('#heatmapSurface3DViewportVue').attributes('aria-describedby')).toContain(
      'heatmapSurface3DInstructionsVue',
    )

    await wrapper.get('button').trigger('click')

    expect(wrapper.emitted('review-top-results')).toHaveLength(1)
  })

  it('navigates retained bins with the keyboard and opens candidate detail', async () => {
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D optimization heatmap.',
        renderModel: renderModel(),
      },
    })
    const viewport = wrapper.get('#heatmapSurface3DViewportVue')
    expect(viewport.attributes('tabindex')).toBe('0')

    await viewport.trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('EMA 6')
    await viewport.trigger('keydown', { key: 'Enter' })

    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 6,
      slow_len: 20,
    })
  })
})
