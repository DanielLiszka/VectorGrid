import { enableAutoUnmount, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Object3D } from 'three'
import type { HeatmapCanvasRenderModel } from '../../visualizations/heatmapCanvasModel'
import type { HeatmapSurface3dModel, HeatmapSurface3dSample } from '../../visualizations/heatmapSurface3dModel'
import HeatmapSurface3D from './HeatmapSurface3D.vue'

const webglRendererHarness = vi.hoisted(() => ({
  failCreation: true,
  instances: [] as unknown[],
}))

vi.mock('three', async (importOriginal) => {
  const actual = await importOriginal<typeof import('three')>()

  class TestWebGLRenderer {
    readonly domElement = document.createElement('canvas')
    renderCount = 0
    lastScene: unknown = null
    outputColorSpace = actual.SRGBColorSpace

    constructor() {
      if (webglRendererHarness.failCreation) {
        throw new Error('WebGL context unavailable in test')
      }
      webglRendererHarness.instances.push(this)
    }

    setPixelRatio(): void {}
    setClearColor(): void {}

    setSize(width: number, height: number): void {
      this.domElement.width = width
      this.domElement.height = height
    }

    render(scene?: unknown): void {
      this.renderCount += 1
      this.lastScene = scene ?? null
    }

    dispose(): void {}
    forceContextLoss(): void {}
  }

  return {
    ...actual,
    WebGLRenderer: TestWebGLRenderer,
  }
})

enableAutoUnmount(afterEach)

interface TestWebGLRendererInstance {
  domElement: HTMLCanvasElement
  lastScene: unknown
  renderCount: number
}

function latestRenderer(): TestWebGLRendererInstance {
  const instance = webglRendererHarness.instances[webglRendererHarness.instances.length - 1]
  if (!instance) throw new Error('Expected a test WebGL renderer instance')
  return instance as TestWebGLRendererInstance
}

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
      fastAxisLabel: 'Fast EMA length',
      slowAxisLabel: 'Slow EMA length',
      fastAxisDescription: 'Fast EMA length used by each grid column.',
      slowAxisDescription: 'Slow EMA length used by each grid row.',
      supports3dSlices: true,
      threeDimensionalUnavailableReason: '',
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
        { key: 'first', label: 'EMA / EMA', shortLabel: 'EMA/EMA', values: [0.1, 0.2, 0.3, 0.4], sampleCounts: [1, 1, 1, 1], cells, sourceRowCount: 4, finiteCells: 4, bestScore: 0.4 },
        { key: 'second', label: 'SMA / EMA', shortLabel: 'SMA/EMA', values: [0.2, 0.3, 0.4, 0.5], sampleCounts: [1, 1, 1, 1], cells, sourceRowCount: 4, finiteCells: 4, bestScore: 0.5 },
      ],
      range: { min: -0.5, max: 0.5, label: 'zero-centered' },
      palette: { kind: 'diverging', negative: '#b91c1c', zero: '#2563eb', positive: '#15803d' },
      sourceRowCount: 8,
      retainedSliceRowCount: 8,
      unavailableReason: '',
    },
  }
}

function sparseDownsampledRenderModel(): HeatmapCanvasRenderModel {
  const base = renderModel()
  const firstValues = Array<number | null>(100).fill(null)
  const secondValues = Array<number | null>(100).fill(null)
  const firstCells = Array<NonNullable<HeatmapCanvasRenderModel['slice3d']>['slices'][number]['cells'][number]>(100).fill(null)
  const secondCells = Array<NonNullable<HeatmapCanvasRenderModel['slice3d']>['slices'][number]['cells'][number]>(100).fill(null)
  firstValues[0] = 0.4
  secondValues[2] = 0.3
  firstCells[0] = { params: { fast_len: 5, slow_len: 20 }, metrics: { sharpe: 0.4 } }
  secondCells[2] = { params: { fast_len: 7, slow_len: 20 }, metrics: { sharpe: 0.3 } }
  return {
    ...base,
    signature: 'heatmap:3d:sparse-downsampled',
    model: {
      ...base.model,
      binsFast: 100,
      binsSlow: 1,
      fastMin: 5,
      fastMax: 104,
      slowMin: 20,
      slowMax: 20,
      values: firstValues,
    },
    displayValues: firstValues,
    slice3d: {
      ...base.slice3d!,
      slices: [
        {
          ...base.slice3d!.slices[0]!,
          values: firstValues,
          sampleCounts: firstValues.map((value) => value == null ? null : 1),
          cells: firstCells,
          sourceRowCount: 1,
          finiteCells: 1,
          bestScore: 0.4,
        },
        {
          ...base.slice3d!.slices[1]!,
          values: secondValues,
          sampleCounts: secondValues.map((value) => value == null ? null : 1),
          cells: secondCells,
          sourceRowCount: 1,
          finiteCells: 1,
          bestScore: 0.3,
        },
      ],
      sourceRowCount: 2,
      retainedSliceRowCount: 2,
    },
  }
}

interface HeatmapSurfaceSetupState {
  hoverSample: HeatmapSurface3dSample | null
  surfaceModel: HeatmapSurface3dModel | null
  surfaceReady: boolean
}

function setupState(wrapper: ReturnType<typeof mount>): HeatmapSurfaceSetupState {
  return (wrapper.vm as unknown as { $: { setupState: HeatmapSurfaceSetupState } }).$.setupState
}

function dispatchWheel(
  element: Element,
  init: { ctrlKey?: boolean; deltaY: number; metaKey?: boolean },
): WheelEvent {
  const event = new WheelEvent('wheel', {
    bubbles: true,
    cancelable: true,
    ctrlKey: init.ctrlKey ?? false,
    deltaY: init.deltaY,
    metaKey: init.metaKey ?? false,
  })
  element.dispatchEvent(event)
  return event
}

function dispatchPointer(
  element: Element,
  type: string,
  init: { clientX: number; clientY: number; pointerId?: number },
): MouseEvent {
  const event = new MouseEvent(type, {
    bubbles: true,
    button: 0,
    cancelable: true,
    clientX: init.clientX,
    clientY: init.clientY,
  })
  Object.defineProperty(event, 'pointerId', {
    configurable: true,
    value: init.pointerId ?? 1,
  })
  element.dispatchEvent(event)
  return event
}

describe('HeatmapSurface3D', () => {
  beforeEach(() => {
    webglRendererHarness.failCreation = true
    webglRendererHarness.instances.length = 0
  })

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
    webglRendererHarness.failCreation = false
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D optimization heatmap.',
        renderModel: renderModel(),
      },
    })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    const viewport = wrapper.get('#heatmapSurface3DViewportVue')
    const testRenderer = latestRenderer()
    const renderedScene = testRenderer.lastScene as Object3D
    expect(viewport.attributes('tabindex')).toBe('0')
    expect(viewport.attributes('data-keyboard-selected-index')).toBe('-1')
    expect(renderedScene.getObjectByName('heatmap-axis-guides')).toBeDefined()
    expect(renderedScene.getObjectByName('heatmap-keyboard-selection')).toBeUndefined()
    expect(wrapper.get('#heatmapSurface3DAxisGuideVue').text()).toContain('Fast EMA length: 5 → 6')
    expect(wrapper.get('#heatmapSurface3DAxisGuideVue').text()).toContain('Slow EMA length: 20 → 30')

    const rendersBeforeMove = testRenderer.renderCount
    await viewport.trigger('keydown', { key: 'ArrowRight' })
    const movedLocator = renderedScene.getObjectByName('heatmap-keyboard-selection-locator')
    expect(viewport.attributes('data-keyboard-selected-index')).toBe('1')
    expect(testRenderer.renderCount).toBeGreaterThan(rendersBeforeMove)
    expect(renderedScene.getObjectByName('heatmap-keyboard-selection')).toBeDefined()
    expect(movedLocator).toBeDefined()
    expect(movedLocator?.position.x).toBeGreaterThan(0)
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('Fast EMA length 6')
    await viewport.trigger('keydown', { key: 'Enter' })

    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 6,
      slow_len: 20,
    })
  })

  it('clears pointer hover and its tooltip when keyboard navigation takes ownership', async () => {
    webglRendererHarness.failCreation = false
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D optimization heatmap.',
        renderModel: renderModel(),
      },
    })
    const state = setupState(wrapper)
    const hoveredSample = state.surfaceModel?.samples[2] ?? null
    expect(hoveredSample).not.toBeNull()
    state.hoverSample = hoveredSample
    const tooltip = wrapper.get<HTMLElement>('#heatmapSurface3DTooltipVue').element
    tooltip.style.display = 'block'
    await wrapper.vm.$nextTick()
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('Fast EMA length 5')
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('Slow EMA length 30')

    const viewport = wrapper.get('#heatmapSurface3DViewportVue')
    await viewport.trigger('keydown', { key: 'ArrowRight' })

    expect(state.hoverSample).toBeNull()
    expect(tooltip.style.display).toBe('none')
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('Fast EMA length 6')
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('Slow EMA length 20')

    await viewport.trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 6,
      slow_len: 20,
    })
  })

  it('preserves ordinary wheel scrolling and only consumes requested camera zoom while it changes', () => {
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D optimization heatmap.',
        renderModel: renderModel(),
      },
    })
    const viewport = wrapper.get('#heatmapSurface3DViewportVue')
    const ordinaryWheel = dispatchWheel(viewport.element, { deltaY: 100 })
    expect(ordinaryWheel.defaultPrevented).toBe(false)

    const state = setupState(wrapper)
    state.surfaceReady = true
    const requestedZoom = dispatchWheel(viewport.element, { ctrlKey: true, deltaY: 100 })
    const zoomToLimit = dispatchWheel(viewport.element, { ctrlKey: true, deltaY: 10000 })
    const wheelAtLimit = dispatchWheel(viewport.element, { ctrlKey: true, deltaY: 10000 })

    expect(requestedZoom.defaultPrevented).toBe(true)
    expect(zoomToLimit.defaultPrevented).toBe(true)
    expect(wheelAtLimit.defaultPrevented).toBe(false)
  })

  it('only consumes pointer movement when an orbit drag changes the camera', () => {
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D optimization heatmap.',
        renderModel: renderModel(),
      },
    })
    const viewport = wrapper.get('#heatmapSurface3DViewportVue')

    const idleMove = dispatchPointer(viewport.element, 'pointermove', { clientX: 40, clientY: 40 })
    dispatchPointer(viewport.element, 'pointerdown', { clientX: 100, clientY: 100 })
    const orbitMove = dispatchPointer(viewport.element, 'pointermove', { clientX: 120, clientY: 100 })
    const unchangedMove = dispatchPointer(viewport.element, 'pointermove', { clientX: 120, clientY: 100 })
    dispatchPointer(viewport.element, 'pointerup', { clientX: 120, clientY: 100 })

    expect(idleMove.defaultPrevented).toBe(false)
    expect(orbitMove.defaultPrevented).toBe(true)
    expect(unchangedMove.defaultPrevented).toBe(false)
  })

  it('describes unavailable source bins truthfully when sparse cells share sampled blocks', () => {
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: 'Sparse 3D optimization heatmap.',
        renderModel: sparseDownsampledRenderModel(),
      },
    })

    expect(wrapper.get('#heatmapSurface3DDomainSummaryVue').text()).toContain(
      'Fully empty sampled blocks sit at the muted base',
    )
    expect(wrapper.get('#heatmapSurface3DDomainSummaryVue').text()).toContain(
      'gaps inside partial blocks contribute to coverage muting',
    )
    expect(wrapper.get('#heatmapSurface3DSamplingSummaryVue').text()).toContain(
      'partially populated blocks are muted',
    )
    expect(wrapper.get('#heatmapSurface3DDomainSummaryVue').text()).not.toContain(
      'shown as muted base cells',
    )
  })

  it('marks context loss unavailable, restores rendering, and removes lifecycle listeners', async () => {
    webglRendererHarness.failCreation = false
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D optimization heatmap.',
        renderModel: renderModel(),
        selectedSummary: 'Heatmap ready.',
      },
    })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    const viewport = wrapper.get('#heatmapSurface3DViewportVue')
    const testRenderer = latestRenderer()
    const canvas = testRenderer.domElement
    expect(viewport.attributes('data-render-ready')).toBe('true')
    expect(viewport.attributes('data-export-ready')).toBe('true')
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toBe('Heatmap ready.')
    await viewport.trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('Fast EMA length 6')

    const contextLost = new Event('webglcontextlost', { cancelable: true })
    canvas.dispatchEvent(contextLost)
    await wrapper.vm.$nextTick()

    expect(contextLost.defaultPrevented).toBe(true)
    expect(viewport.attributes('data-render-ready')).toBe('false')
    expect(viewport.attributes('data-export-ready')).toBe('false')
    expect(wrapper.get('#heatmapSurface3DErrorVue').text()).toContain('graphics context was lost')
    expect(wrapper.find('#heatmapSurface3DSelectedSummaryVue').exists()).toBe(false)

    const rendersBeforeRestore = testRenderer.renderCount
    canvas.dispatchEvent(new Event('webglcontextrestored'))
    await wrapper.vm.$nextTick()

    expect(viewport.attributes('data-render-ready')).toBe('true')
    expect(viewport.attributes('data-export-ready')).toBe('true')
    expect(wrapper.find('#heatmapSurface3DErrorVue').exists()).toBe(false)
    expect(wrapper.get('#heatmapSurface3DSelectedSummaryVue').text()).toContain('Fast EMA length 6')
    expect(testRenderer.renderCount).toBeGreaterThan(rendersBeforeRestore)

    wrapper.unmount()
    const postUnmountLoss = new Event('webglcontextlost', { cancelable: true })
    canvas.dispatchEvent(postUnmountLoss)
    expect(postUnmountLoss.defaultPrevented).toBe(false)
  })

  it('keeps PNG export unavailable when WebGL initialization fails', async () => {
    const wrapper = mount(HeatmapSurface3D, {
      props: {
        ariaLabel: '3D optimization heatmap.',
        renderModel: renderModel(),
        selectedSummary: 'Heatmap ready.',
      },
    })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.get('#heatmapSurface3DViewportVue').attributes('data-export-ready')).toBe('false')
    expect(wrapper.get('#heatmapSurface3DErrorVue').text()).toContain('WebGL context unavailable in test')
    expect(wrapper.find('#heatmapSurface3DSelectedSummaryVue').exists()).toBe(false)
    expect((wrapper.vm as unknown as { exportPng: (fileName?: string) => boolean }).exportPng()).toBe(false)
  })
})
