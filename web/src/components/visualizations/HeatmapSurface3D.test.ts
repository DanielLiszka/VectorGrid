import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import HeatmapSurface3D from './HeatmapSurface3D.vue'

describe('HeatmapSurface3D', () => {
  it('keeps an explicit Top results handoff and describes the non-keyboard chart path', async () => {
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D sliced optimization heatmap for retained parameter variants.',
        selectedSummary: 'Selected 3D slice summary.',
      },
    })

    expect(wrapper.get('#heatmap3dEmptyHintVue').text()).toContain('3D slices require include_all=true')
    expect(wrapper.get('#heatmapSurface3DInstructionsVue').text()).toContain('keyboard-friendly exact candidate path')
    expect(wrapper.get('#heatmapSurface3DViewportVue').attributes('role')).toBe('img')
    expect(wrapper.get('#heatmapSurface3DViewportVue').attributes('tabindex')).toBeUndefined()
    expect(wrapper.get('#heatmapSurface3DViewportVue').attributes('aria-describedby')).toContain(
      'heatmapSurface3DInstructionsVue',
    )

    await wrapper.get('button').trigger('click')

    expect(wrapper.emitted('review-top-results')).toHaveLength(1)
  })
})
