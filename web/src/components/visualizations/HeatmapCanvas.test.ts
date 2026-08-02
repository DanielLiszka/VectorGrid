
import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import HeatmapCanvas from './HeatmapCanvas.vue'
import {
  HEATMAP_MAX_CANVAS_PIXELS,
  type HeatmapCanvasRenderModel,
} from '../../visualizations/heatmapCanvasModel'

function renderModel(): HeatmapCanvasRenderModel {
  return {
    signature: 'heatmap:test',
    meta: 'Sharpe heatmap',
    statusLine: 'Heatmap ready.',
    model: {
      binsFast: 2,
      binsSlow: 2,
      fastMin: 5,
      fastMax: 9,
      slowMin: 20,
      slowMax: 30,
      values: [0.1, 0.2, -0.1, 0.05],
      meanValues: null,
      sampleCounts: null,
    },
    displayMode: 'objective',
    displayValues: [0.1, 0.2, -0.1, 0.05],
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
    markers: [{ col: 0, drawRow: 1, label: '1' }],
    range: { min: -0.2, max: 0.2, label: 'zero-centered' },
    palette: {
      kind: 'diverging',
      negative: '#b91c1c',
      zero: '#2563eb',
      positive: '#15803d',
    },
    plateauDiagnostics: null,
    slice3d: null,
  }
}

function mockCanvasContext() {
  const clearRect = vi.fn()
  const drawImage = vi.fn()
  const getContext = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(
    () => ({
      clearRect,
      setTransform: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      scale: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      beginPath: vi.fn(),
      arc: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      fillText: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      drawImage,
      createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
    }) as never,
  )
  return { clearRect, drawImage, getContext }
}

function setElementRect(element: Element, width = 512, height = 512): void {
  Object.defineProperty(element, 'getBoundingClientRect', {
    configurable: true,
    value: () => ({
      left: 0,
      top: 0,
      width,
      height,
      right: width,
      bottom: height,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }),
  })
}

function dispatchPointerLikeEvent(
  element: Element,
  type: string,
  init: MouseEventInit & { deltaY?: number; pointerId?: number } = {},
): MouseEvent | WheelEvent {
  const event = type === 'wheel'
    ? new WheelEvent(type, {
        bubbles: true,
        cancelable: true,
        clientX: init.clientX ?? 0,
        clientY: init.clientY ?? 0,
        ctrlKey: init.ctrlKey ?? false,
        deltaY: init.deltaY ?? 0,
        metaKey: init.metaKey ?? false,
      })
    : new MouseEvent(type, {
        bubbles: true,
        cancelable: true,
        button: init.button ?? 0,
        clientX: init.clientX ?? 0,
        clientY: init.clientY ?? 0,
        ctrlKey: init.ctrlKey ?? false,
        metaKey: init.metaKey ?? false,
      })
  if (init.pointerId != null) {
    Object.defineProperty(event, 'pointerId', {
      configurable: true,
      value: init.pointerId,
    })
  }
  element.dispatchEvent(event)
  return event
}

describe('HeatmapCanvas', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('owns the heatmap DOM contract and empty state', () => {
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        selectedSummary: 'Selected cell: fast 12, slow 48.',
      },
    })

    expect(wrapper.get('#heatmapEmptyHintVue').text()).toContain('Heatmap bins')
    expect(wrapper.get('#heatmapWrapVue').attributes('style')).toContain('display: none')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('aria-label')).toContain('Optimization heatmap')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('role')).toBe('group')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('tabindex')).toBe('0')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-render-ready')).toBe('false')
    expect(wrapper.find('#heatmapCanvasVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapOverlayVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapTooltipVue').exists()).toBe(true)
    expect(wrapper.get('.heatmapDetails').text()).toContain('Heatmap requirements')
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('fast 12')
  })

  it('emits an explicit Top results handoff from the visualization host', async () => {
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
      },
    })

    await wrapper.get('[data-qa-action="heatmap.reviewTop"]').trigger('click')

    expect(wrapper.emitted('review-top-results')).toHaveLength(1)
  })

  it('renders model-owned metadata and emits representative candidate-detail params from clicks', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    const overlay = wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element
    setElementRect(overlay)

    expect(wrapper.get('#heatmapMetaVue').text()).toContain('Sharpe heatmap')
    expect(wrapper.find('.heatmapDetails').exists()).toBe(false)
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-render-ready')).toBe('true')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-finite-cells')).toBe('4')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-total-cells')).toBe('4')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-finite-coverage')).toBe('1.000')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-axis-summary')).toBe(
      'Fast EMA length 5-9; Slow EMA length 20-30',
    )
    expect(wrapper.get('#heatmapAxisDescriptionVue').text()).toContain(
      'Horizontal axis: Fast EMA length',
    )
    expect(wrapper.get('#heatmapAxisDescriptionVue').text()).toContain(
      'Vertical axis: Slow EMA length',
    )

    await wrapper.get('#heatmapOverlayVue').trigger('mousemove', { clientX: 128, clientY: 128 })
    expect(wrapper.get('#heatmapTooltipVue').attributes('style') || '').toContain('display: block')
    expect(wrapper.get('#heatmapTooltipVue').text()).toContain('Sharpe')

    await wrapper.get('#heatmapOverlayVue').trigger('click', { clientX: 128, clientY: 384 })
    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 5,
      fast_ma_type: 'ema',
      slow_len: 20,
      slow_ma_type: 'ema',
    })
  })

  it('emits the exact retained aggregate winner instead of reconstructing an ambiguous cell', async () => {
    mockCanvasContext()
    const exact = renderModel()
    exact.aggregateSelectionRequiresExactParams = true
    exact.exactAggregateWinnerCells = [
      {
        score: 0.1,
        params: {
          fast_len: 6,
          fast_ma_type: 'alma',
          fast_ma_params: { offset: 0.85, sigma: 6 },
          slow_len: 21,
          slow_ma_type: 'ema',
        },
        metrics: { sharpe: 0.1 },
      },
    ]
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: exact,
      },
    })
    const overlay = wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element
    setElementRect(overlay)

    await wrapper.get('#heatmapOverlayVue').trigger('click', { clientX: 128, clientY: 384 })

    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 6,
      fast_ma_type: 'alma',
      fast_ma_params: { offset: 0.85, sigma: 6 },
      slow_len: 21,
    })
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain(
      'exact retained best-in-bin winner',
    )
  })

  it('keeps ambiguous cells inspectable while withholding an unfaithful candidate request', async () => {
    mockCanvasContext()
    const ambiguous = renderModel()
    ambiguous.aggregateSelectionRequiresExactParams = true
    ambiguous.aggregateSelectionUnavailableReason =
      'Exact winning MA variant was not retained. Use Top Results, or rerun with Return all results.'
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: ambiguous,
      },
    })
    const overlay = wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element
    setElementRect(overlay)

    await wrapper.get('#heatmapOverlayVue').trigger('mousemove', { clientX: 128, clientY: 384 })
    expect(wrapper.get('#heatmapTooltipVue').text()).toContain(
      'Exact winning MA variant was not retained',
    )

    await wrapper.get('#heatmapOverlayVue').trigger('click', { clientX: 128, clientY: 384 })
    expect(wrapper.emitted('request-candidate-detail')).toBeUndefined()
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain(
      'Return all results',
    )
    expect(wrapper.text()).toContain(
      'Exact winning MA variant was not retained',
    )

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Home' })
    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('request-candidate-detail')).toBeUndefined()
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain(
      'Return all results',
    )
  })

  it('flips and clamps the tooltip inside the heatmap at the right and bottom edges', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    const overlay = wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element
    const tooltip = wrapper.get<HTMLElement>('#heatmapTooltipVue').element
    setElementRect(overlay)
    Object.defineProperty(tooltip, 'offsetWidth', { configurable: true, value: 180 })
    Object.defineProperty(tooltip, 'offsetHeight', { configurable: true, value: 80 })

    await wrapper.get('#heatmapOverlayVue').trigger('mousemove', { clientX: 500, clientY: 500 })

    const left = Number.parseFloat(tooltip.style.left)
    const top = Number.parseFloat(tooltip.style.top)
    expect(left).toBeLessThan(500)
    expect(top).toBeLessThan(500)
    expect(left + tooltip.offsetWidth).toBeLessThanOrEqual(504)
    expect(top + tooltip.offsetHeight).toBeLessThanOrEqual(504)
  })

  it('labels unavailable heatmap bins as a masked domain instead of silent whitespace', () => {
    mockCanvasContext()
    const model = renderModel()
    model.displayValues = [0.1, null, Number.NaN, 0.05]
    model.model.values = [0.1, Number.NaN, Number.NaN, 0.05]

    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: model,
      },
    })

    expect(wrapper.get('#heatmapDomainSummaryVue').text()).toContain('2 of 4 bins are unavailable')
    expect(wrapper.get('#heatmapDomainSummaryVue').text()).toContain('invalid or unscored parameter pairs')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-finite-cells')).toBe('2')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-finite-coverage')).toBe('0.500')
  })

  it('counts sparse array holes as unavailable bins', () => {
    mockCanvasContext()
    const model = renderModel()
    const sparseValues = Array<number>(4)
    sparseValues[0] = 0.1
    sparseValues[3] = 0.05
    model.displayValues = sparseValues

    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: model,
      },
    })

    expect(wrapper.get('#heatmapDomainSummaryVue').text()).toContain('2 of 4 bins are unavailable')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-finite-cells')).toBe('2')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-finite-coverage')).toBe('0.500')
  })

  it('supports keyboard cell selection with a persistent selected-cell summary', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Home' })
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Fast EMA length bin 5')
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Slow EMA length bin 30')

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Fast EMA length bin 9')

    const overlay = wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element
    setElementRect(overlay)
    await wrapper.get('#heatmapOverlayVue').trigger('mousemove', { clientX: 128, clientY: 384 })
    expect(wrapper.get('#heatmapTooltipVue').attributes('style') || '').toContain('display: block')

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Enter' })
    expect(wrapper.get('#heatmapTooltipVue').attributes('style') || '').toContain('display: none')
    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 9,
      fast_ma_type: 'ema',
      slow_len: 30,
      slow_ma_type: 'ema',
    })
  })

  it('skips objective cells that are unavailable on the rendered display surface', async () => {
    mockCanvasContext()
    const masked = renderModel()
    masked.displayValues = [null, 0.2, null, null]

    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Masked optimization heatmap.',
        renderModel: masked,
      },
    })

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Home' })

    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Fast EMA length bin 9')
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Slow EMA length bin 20')
  })

  it('reaches populated diagonal islands with directional keyboard navigation', async () => {
    mockCanvasContext()
    const sparse = renderModel()
    sparse.model = {
      ...sparse.model,
      binsFast: 3,
      binsSlow: 3,
      fastMin: 5,
      fastMax: 15,
      slowMin: 20,
      slowMax: 40,
      values: [
        null, null, 0.1,
        null, 0.2, null,
        0.3, null, null,
      ],
    }
    sparse.displayValues = sparse.model.values

    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Sparse optimization heatmap.',
        renderModel: sparse,
      },
    })

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Home' })
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Fast EMA length bin 5–9')
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Slow EMA length bin 40')

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Fast EMA length bin 10–14')
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('Slow EMA length bin 30–39')

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 10,
      slow_len: 30,
    })
  })

  it('clears stale backing pixels and refuses export after the render model is removed', async () => {
    const context = mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap.',
        renderModel: renderModel(),
        fitMode: 'valid',
      },
    })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    const clearsBeforeRemoval = context.clearRect.mock.calls.length
    await wrapper.setProps({ renderModel: null })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(context.clearRect.mock.calls.length).toBeGreaterThan(clearsBeforeRemoval)
    expect(wrapper.get<HTMLCanvasElement>('#heatmapCanvasVue').element.dataset.renderSignature).toBeUndefined()
    expect(wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element.dataset.renderSignature).toBeUndefined()
    expect((wrapper.vm as unknown as { exportPng: (name?: string) => boolean }).exportPng()).toBe(false)
  })

  it('repaints DPR-only changes within the canvas pixel budget', async () => {
    mockCanvasContext()
    let pixelRatio = 1
    const listeners: Array<(event: MediaQueryListEvent) => void> = []
    const removeEventListener = vi.fn()
    const matchMedia = vi.fn((media: string) => ({
      media,
      matches: true,
      onchange: null,
      addEventListener: vi.fn(
        (_type: string, listener: (event: MediaQueryListEvent) => void) => {
          listeners.push(listener)
        },
      ),
      removeEventListener,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(() => true),
    }) as unknown as MediaQueryList)
    vi.stubGlobal('matchMedia', matchMedia)
    vi.spyOn(window, 'devicePixelRatio', 'get').mockImplementation(() => pixelRatio)

    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap.',
        renderModel: renderModel(),
      },
    })
    const stack = wrapper.get('#heatmapCanvasStackVue')
    const overlay = wrapper.get('#heatmapOverlayVue')
    setElementRect(stack.element, 1152, 1152)
    setElementRect(overlay.element, 1152, 1152)
    ;(wrapper.vm as unknown as { repaint: (force?: boolean) => void }).repaint(true)

    const canvas = wrapper.get<HTMLCanvasElement>('#heatmapCanvasVue').element
    expect(canvas.width).toBe(1152)
    expect(canvas.height).toBe(1152)

    pixelRatio = 3
    listeners[0]?.({ matches: false, media: '(resolution: 1dppx)' } as MediaQueryListEvent)
    await wrapper.vm.$nextTick()

    expect(canvas.width * canvas.height).toBeLessThanOrEqual(HEATMAP_MAX_CANVAS_PIXELS)
    expect(canvas.width).toBeLessThan(1152 * 3)
    expect(Number(canvas.dataset.pixelRatio)).toBeGreaterThan(1)
    expect(Number(canvas.dataset.pixelRatio)).toBeLessThan(3)

    wrapper.unmount()
    expect(removeEventListener).toHaveBeenCalled()
  })

  it('exports the composited image in place without allocating a third full-size canvas', async () => {
    const context = mockCanvasContext()
    const toDataUrl = vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL')
      .mockReturnValue('data:image/png;base64,heatmap')
    const anchorClick = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => undefined)
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap.',
        renderModel: renderModel(),
      },
    })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    const createElement = vi.spyOn(document, 'createElement')

    const exported = (wrapper.vm as unknown as {
      exportPng: (name?: string) => boolean
    }).exportPng('bounded-heatmap.png')

    expect(exported).toBe(true)
    expect(context.drawImage).toHaveBeenCalledTimes(1)
    expect(toDataUrl).toHaveBeenCalledWith('image/png')
    expect(anchorClick).toHaveBeenCalledTimes(1)
    expect(
      createElement.mock.calls.filter(([tagName]) => String(tagName).toLowerCase() === 'canvas'),
    ).toHaveLength(0)
  })

  it('preserves page scrolling and only consumes a requested wheel zoom that changes scale', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    setElementRect(wrapper.get('#heatmapCanvasStackVue').element)
    setElementRect(wrapper.get('#heatmapOverlayVue').element)
    await wrapper.vm.$nextTick()

    const ordinaryWheel = dispatchPointerLikeEvent(wrapper.get('#heatmapCanvasStackVue').element, 'wheel', {
      clientX: 256,
      clientY: 256,
      deltaY: -300,
    })
    await wrapper.vm.$nextTick()

    expect(ordinaryWheel.defaultPrevented).toBe(false)
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-viewport-scale')).toBe('1.000')

    const requestedZoom = dispatchPointerLikeEvent(wrapper.get('#heatmapCanvasStackVue').element, 'wheel', {
      clientX: 256,
      clientY: 256,
      ctrlKey: true,
      deltaY: -300,
    })
    await wrapper.vm.$nextTick()

    expect(requestedZoom.defaultPrevented).toBe(true)
    expect(Number(wrapper.get('#heatmapCanvasStackVue').attributes('data-viewport-scale'))).toBeGreaterThan(1)

    const zoomToLimit = dispatchPointerLikeEvent(wrapper.get('#heatmapCanvasStackVue').element, 'wheel', {
      clientX: 256,
      clientY: 256,
      ctrlKey: true,
      deltaY: -10000,
    })
    const wheelAtLimit = dispatchPointerLikeEvent(wrapper.get('#heatmapCanvasStackVue').element, 'wheel', {
      clientX: 256,
      clientY: 256,
      ctrlKey: true,
      deltaY: -10000,
    })
    expect(zoomToLimit.defaultPrevented).toBe(true)
    expect(wheelAtLimit.defaultPrevented).toBe(false)

    await wrapper.get('button[aria-label="Reset heatmap viewport"]').trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-viewport-scale')).toBe('1.000')
    expect(wrapper.get('button[aria-label="Reset heatmap viewport"]').attributes('disabled')).toBeDefined()
  })

  it('pans a zoomed heatmap and suppresses the drag-release click handoff', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    const stack = wrapper.get('#heatmapCanvasStackVue')
    const overlay = wrapper.get('#heatmapOverlayVue')
    setElementRect(stack.element)
    setElementRect(overlay.element)
    await wrapper.vm.$nextTick()

    dispatchPointerLikeEvent(stack.element, 'wheel', {
      clientX: 256,
      clientY: 256,
      ctrlKey: true,
      deltaY: -300,
    })
    await wrapper.vm.$nextTick()
    const offsetBeforePan = Number(stack.attributes('data-viewport-offset-x'))

    const pointerDown = dispatchPointerLikeEvent(overlay.element, 'pointerdown', {
      pointerId: 7,
      button: 0,
      clientX: 256,
      clientY: 256,
    })
    const pointerMove = dispatchPointerLikeEvent(overlay.element, 'pointermove', {
      pointerId: 7,
      clientX: 296,
      clientY: 256,
    })
    dispatchPointerLikeEvent(overlay.element, 'pointerup', {
      pointerId: 7,
      clientX: 296,
      clientY: 256,
    })
    await wrapper.vm.$nextTick()

    expect(pointerDown.defaultPrevented).toBe(false)
    expect(pointerMove.defaultPrevented).toBe(true)
    expect(Number(stack.attributes('data-viewport-offset-x'))).toBeGreaterThan(offsetBeforePan)

    dispatchPointerLikeEvent(overlay.element, 'click', {
      clientX: 296,
      clientY: 256,
    })

    expect(wrapper.emitted('request-candidate-detail')).toBeUndefined()
  })

  it('suppresses a click after cumulative micro-movements become a drag', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    const stack = wrapper.get('#heatmapCanvasStackVue')
    const overlay = wrapper.get('#heatmapOverlayVue')
    setElementRect(stack.element)
    setElementRect(overlay.element)
    await wrapper.vm.$nextTick()

    dispatchPointerLikeEvent(stack.element, 'wheel', {
      clientX: 256,
      clientY: 256,
      ctrlKey: true,
      deltaY: -300,
    })
    await wrapper.vm.$nextTick()
    const offsetBeforePan = Number(stack.attributes('data-viewport-offset-x'))

    dispatchPointerLikeEvent(overlay.element, 'pointerdown', {
      pointerId: 8,
      button: 0,
      clientX: 256,
      clientY: 256,
    })
    for (const clientX of [257, 258, 259, 260]) {
      dispatchPointerLikeEvent(overlay.element, 'pointermove', {
        pointerId: 8,
        clientX,
        clientY: 256,
      })
    }
    dispatchPointerLikeEvent(overlay.element, 'pointerup', {
      pointerId: 8,
      clientX: 260,
      clientY: 256,
    })
    await wrapper.vm.$nextTick()

    expect(Number(stack.attributes('data-viewport-offset-x'))).toBeGreaterThan(offsetBeforePan)

    dispatchPointerLikeEvent(overlay.element, 'click', {
      clientX: 260,
      clientY: 256,
    })

    expect(wrapper.emitted('request-candidate-detail')).toBeUndefined()
  })
})
