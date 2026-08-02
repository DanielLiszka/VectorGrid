<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Minus, Plus, RotateCcw } from '@lucide/vue'
import {
  buildHeatmapCellParams,
  buildHeatmapCanvasSummaryItems,
  clampHeatmapViewport,
  clearHeatmapCanvas,
  DEFAULT_HEATMAP_VIEWPORT,
  downloadHeatmapPng,
  fitHeatmapViewportToFiniteDomain,
  HEATMAP_MAX_CANVAS_PIXEL_RATIO,
  HEATMAP_MAX_VIEWPORT_SCALE,
  HEATMAP_MIN_VIEWPORT_SCALE,
  heatmapSelectionLegendText,
  heatmapTooltipText,
  heatmapViewportIsZoomed,
  paintHeatmapBase,
  paintHeatmapOverlay,
  resolveHeatmapCanvasBackingStore,
  resolveHeatmapCellFromPoint,
  type HeatmapCanvasRenderModel,
  type HeatmapCanvasViewport,
  type HeatmapCellParams,
  type HeatmapFitMode,
  type HeatmapHoverCell,
  type HeatmapResolvedCell,
} from '../../visualizations/heatmapCanvasModel'
import {
  observeDevicePixelRatioChanges,
  resolveDevicePixelRatio,
} from '../../visualizations/devicePixelRatioObserver'
import { resolveLightweightChartTheme } from '../../visualizations/chartTheme'
import { observeHeatmapTheme } from '../../visualizations/heatmapTheme'
import AppIconButton from '../ui/AppIconButton.vue'
import AppMetricStrip from '../ui/AppMetricStrip.vue'
import HeatmapLegend from './HeatmapLegend.vue'

const props = withDefaults(
  defineProps<{
    ariaLabel: string
    renderModel?: HeatmapCanvasRenderModel | null
    selectedSummary?: string
    fitMode?: HeatmapFitMode
  }>(),
  {
    renderModel: null,
    selectedSummary: '',
    fitMode: 'full',
  },
)

const emit = defineEmits<{
  'review-top-results': []
  'request-candidate-detail': [params: HeatmapCellParams]
}>()

const stackRef = ref<HTMLElement | null>(null)
const canvasRef = ref<HTMLCanvasElement | null>(null)
const overlayRef = ref<HTMLCanvasElement | null>(null)
const tooltipRef = ref<HTMLElement | null>(null)
const legendRef = ref<InstanceType<typeof HeatmapLegend> | null>(null)
const hoverCell = ref<HeatmapHoverCell | null>(null)
const selectedCell = ref<HeatmapResolvedCell | null>(null)
const viewport = ref<HeatmapCanvasViewport>({ ...DEFAULT_HEATMAP_VIEWPORT })
const isPanning = ref(false)
const lastBaseSignature = ref('')
const activeTheme = ref(resolveLightweightChartTheme())
let resizeObserver: ResizeObserver | null = null
let stopDevicePixelRatioObserver: (() => void) | null = null
let stopThemeObserver: (() => void) | null = null
let panPointerId: number | null = null
let panStartPoint: { x: number; y: number } | null = null
let panLastPoint: { x: number; y: number } | null = null
let panMoved = false
let suppressNextClick = false
const viewportKey = computed(() =>
  `${viewport.value.scale.toFixed(4)}:${viewport.value.offsetX.toFixed(2)}:${viewport.value.offsetY.toFixed(2)}`,
)
const viewportScalePercent = computed(() => `${Math.round(viewport.value.scale * 100)}%`)
const viewportScaleData = computed(() => viewport.value.scale.toFixed(3))
const viewportOffsetXData = computed(() => viewport.value.offsetX.toFixed(1))
const viewportOffsetYData = computed(() => viewport.value.offsetY.toFixed(1))
const viewportZoomed = computed(() => heatmapViewportIsZoomed(viewport.value))
const canZoomIn = computed(() => viewport.value.scale < HEATMAP_MAX_VIEWPORT_SCALE - 1.0e-6)
const canZoomOut = computed(() => viewport.value.scale > HEATMAP_MIN_VIEWPORT_SCALE + 1.0e-6)
const heatmapSummaryItems = computed(() => (
  props.renderModel ? buildHeatmapCanvasSummaryItems(props.renderModel) : []
))
const fitModeLabel = computed(() => (props.fitMode === 'valid' ? 'valid domain' : 'full grid'))
const totalCellCount = computed(() => {
  const model = props.renderModel?.model
  return model ? Math.max(0, model.binsFast * model.binsSlow) : 0
})
const unavailableCellCount = computed(() => {
  const values = props.renderModel?.displayValues ?? []
  const total = totalCellCount.value
  let unavailable = 0
  for (let index = 0; index < total; index += 1) {
    const value = values[index]
    if (typeof value !== 'number' || !Number.isFinite(value)) unavailable += 1
  }
  return unavailable
})
const finiteCellCount = computed(() => Math.max(0, totalCellCount.value - unavailableCellCount.value))
const finiteCoverageData = computed(() => {
  const total = totalCellCount.value
  if (!total) return '0.000'
  return (finiteCellCount.value / total).toFixed(3)
})
const axisDataSummary = computed(() => {
  const renderModel = props.renderModel
  const model = renderModel?.model
  if (!renderModel || !model) return ''
  const fastLabel = String(renderModel.request.fastAxisLabel || 'Fast length').trim()
  const slowLabel = String(renderModel.request.slowAxisLabel || 'Slow length').trim()
  return `${fastLabel} ${model.fastMin}-${model.fastMax}; ${slowLabel} ${model.slowMin}-${model.slowMax}`
})
const axisAccessibilitySummary = computed(() => {
  const renderModel = props.renderModel
  if (!renderModel) return ''
  const request = renderModel.request
  return [
    `Horizontal axis: ${request.fastAxisLabel}, ${request.fastAxisDescription}`,
    `Vertical axis: ${request.slowAxisLabel}, ${request.slowAxisDescription}`,
    `Displayed ranges: ${axisDataSummary.value}.`,
  ].join(' ')
})
const unavailableDomainSummary = computed(() => {
  const unavailable = unavailableCellCount.value
  const total = totalCellCount.value
  if (!unavailable || !total) return ''
  const verb = unavailable === 1 ? 'is' : 'are'
  return `${unavailable} of ${total} bins ${verb} unavailable and shown as a muted mask. These are invalid or unscored parameter pairs.`
})
const selectedCellSummary = computed(() => {
  const renderModel = props.renderModel
  const cell = selectedCell.value
  return renderModel && cell ? heatmapTooltipText(renderModel, cell) : ''
})
const visibleSelectedSummary = computed(() => selectedCellSummary.value || props.selectedSummary)
const selectionLegendNote = computed(() => (
  props.renderModel
    ? heatmapSelectionLegendText(props.renderModel)
    : 'Populated cells can open Candidate Detail when a faithful parameter set is available.'
))
const keyboardSelectableCells = computed<HeatmapResolvedCell[]>(() => {
  const renderModel = props.renderModel
  if (!renderModel) return []
  const { binsFast, binsSlow } = renderModel.model
  const cells: HeatmapResolvedCell[] = []
  for (let drawRow = 0; drawRow < binsSlow; drawRow += 1) {
    const logicalRow = binsSlow - 1 - drawRow
    for (let col = 0; col < binsFast; col += 1) {
      const idx = logicalRow * binsFast + col
      const value = renderModel.displayValues[idx]
      if (typeof value !== 'number' || !Number.isFinite(value)) continue
      cells.push({ col, drawRow, logicalRow, idx })
    }
  }
  return cells
})

function overlayCell(): HeatmapHoverCell | null {
  if (hoverCell.value) return hoverCell.value
  const cell = selectedCell.value
  return cell ? { col: cell.col, row: cell.drawRow } : null
}

function hideTooltip(): void {
  const tooltip = tooltipRef.value
  if (tooltip) tooltip.style.display = 'none'
}

function clearCanvases(): void {
  clearHeatmapCanvas(canvasRef.value)
  clearHeatmapCanvas(overlayRef.value)
  if (canvasRef.value) delete canvasRef.value.dataset.renderSignature
  if (overlayRef.value) delete overlayRef.value.dataset.renderSignature
  lastBaseSignature.value = ''
}

function syncCanvasBackingStore(): boolean {
  const stack = stackRef.value
  const canvases = [canvasRef.value, overlayRef.value].filter((canvas): canvas is HTMLCanvasElement => Boolean(canvas))
  if (!stack || canvases.length === 0) return false
  const rect = stack.getBoundingClientRect()
  const cssWidth = Math.max(240, Math.round(rect.width || stack.clientWidth || 512))
  const cssHeight = Math.max(240, Math.round(rect.height || stack.clientHeight || cssWidth))
  const backingStore = resolveHeatmapCanvasBackingStore({
    cssWidth,
    cssHeight,
    devicePixelRatio: resolveDevicePixelRatio(HEATMAP_MAX_CANVAS_PIXEL_RATIO),
  })
  const ratio = backingStore.pixelRatio
  const targetWidth = backingStore.width
  const targetHeight = backingStore.height
  let changed = false
  for (const canvas of canvases) {
    if (canvas.width !== targetWidth || canvas.height !== targetHeight) {
      canvas.width = targetWidth
      canvas.height = targetHeight
      changed = true
    }
    if (canvas.dataset.pixelRatio !== String(ratio)) {
      canvas.dataset.pixelRatio = String(ratio)
      changed = true
    }
  }
  return changed
}

function canvasLogicalSize(): { width: number; height: number } | null {
  const canvas = overlayRef.value ?? canvasRef.value
  if (!canvas) return null
  const ratioRaw = Number(canvas.dataset.pixelRatio || 1)
  const ratio = Number.isFinite(ratioRaw) && ratioRaw > 0 ? ratioRaw : 1
  return {
    width: Math.max(1, canvas.width / ratio),
    height: Math.max(1, canvas.height / ratio),
  }
}

function viewportStateChanged(
  current: HeatmapCanvasViewport,
  next: HeatmapCanvasViewport,
): boolean {
  return (
    Math.abs(current.scale - next.scale) > 1.0e-6 ||
    Math.abs(current.offsetX - next.offsetX) > 1.0e-3 ||
    Math.abs(current.offsetY - next.offsetY) > 1.0e-3
  )
}

function clampViewportState(next: HeatmapCanvasViewport = viewport.value): boolean {
  const size = canvasLogicalSize()
  const clamped = size
    ? clampHeatmapViewport(next, size.width, size.height)
    : clampHeatmapViewport(next, 512, 512)
  if (!viewportStateChanged(viewport.value, clamped)) return false
  viewport.value = clamped
  return true
}

function resetViewport(): void {
  if (props.fitMode === 'valid' && fitViewportToValidDomain()) return
  viewport.value = { ...DEFAULT_HEATMAP_VIEWPORT }
  endPan()
  repaint(true)
}

function fitViewportToValidDomain(): boolean {
  const renderModel = props.renderModel
  if (!renderModel) return false
  syncCanvasBackingStore()
  const size = canvasLogicalSize()
  if (!size) return false
  viewport.value = fitHeatmapViewportToFiniteDomain(renderModel, size.width, size.height)
  endPan()
  repaint(true)
  return true
}

function clientPointToLogicalCanvas(clientX: number, clientY: number): { x: number; y: number } | null {
  const overlay = overlayRef.value
  const size = canvasLogicalSize()
  if (!overlay || !size) return null
  const rect = overlay.getBoundingClientRect()
  if (!rect.width || !rect.height) return null
  return {
    x: ((clientX - rect.left) / rect.width) * size.width,
    y: ((clientY - rect.top) / rect.height) * size.height,
  }
}

function zoomAtLogicalPoint(point: { x: number; y: number }, factor: number): boolean {
  const size = canvasLogicalSize()
  if (!size || !Number.isFinite(factor) || factor <= 0) return false
  const current = viewport.value
  const nextScale = Math.max(
    HEATMAP_MIN_VIEWPORT_SCALE,
    Math.min(HEATMAP_MAX_VIEWPORT_SCALE, current.scale * factor),
  )
  if (Math.abs(nextScale - current.scale) <= 1.0e-6) return false
  const modelX = (point.x - current.offsetX) / current.scale
  const modelY = (point.y - current.offsetY) / current.scale
  const changed = clampViewportState({
    scale: nextScale,
    offsetX: point.x - modelX * nextScale,
    offsetY: point.y - modelY * nextScale,
  })
  if (!changed) return false
  if (!viewportZoomed.value) endPan()
  repaint(true)
  return true
}

function zoomAtClientPoint(clientX: number, clientY: number, factor: number): boolean {
  const point = clientPointToLogicalCanvas(clientX, clientY)
  return point ? zoomAtLogicalPoint(point, factor) : false
}

function zoomFromCenter(factor: number): void {
  const size = canvasLogicalSize()
  if (!size) return
  zoomAtLogicalPoint({ x: size.width / 2, y: size.height / 2 }, factor)
}

function panByLogical(deltaX: number, deltaY: number): boolean {
  if (!viewportZoomed.value) return false
  const changed = clampViewportState({
    ...viewport.value,
    offsetX: viewport.value.offsetX + deltaX,
    offsetY: viewport.value.offsetY + deltaY,
  })
  if (!changed) return false
  hideTooltip()
  repaint(true)
  return true
}

function endPan(pointerId: number | null = null): void {
  if (pointerId != null && panPointerId != null && pointerId !== panPointerId) return
  const overlay = overlayRef.value
  if (pointerId != null) {
    try {
      overlay?.releasePointerCapture?.(pointerId)
    } catch {
      // Pointer capture may already be released by the browser.
    }
  }
  isPanning.value = false
  panPointerId = null
  panStartPoint = null
  panLastPoint = null
  panMoved = false
}

function repaint(forceBase = false): void {
  const renderModel = props.renderModel
  const canvas = canvasRef.value
  const overlay = overlayRef.value
  if (!renderModel || !canvas || !overlay) {
    if (lastBaseSignature.value) clearCanvases()
    hideTooltip()
    return
  }
  const backingStoreChanged = syncCanvasBackingStore()
  if (backingStoreChanged) clampViewportState()
  const renderSignature = `${renderModel.signature}:${viewportKey.value}:${activeTheme.value}`
  if (forceBase || backingStoreChanged || lastBaseSignature.value !== renderSignature) {
    paintHeatmapBase(renderModel, canvas, viewport.value, activeTheme.value)
    canvas.dataset.renderSignature = renderModel.signature
    lastBaseSignature.value = renderSignature
  }
  paintHeatmapOverlay(
    renderModel,
    overlay,
    overlayCell(),
    viewport.value,
    activeTheme.value,
  )
  overlay.dataset.renderSignature = renderModel.signature
  legendRef.value?.paint(renderModel.range, renderModel.palette, activeTheme.value)
}

function setTooltipFromEvent(event: MouseEvent, content: string): void {
  const tooltip = tooltipRef.value
  const overlay = overlayRef.value
  if (!tooltip || !overlay) return
  const rect = overlay.getBoundingClientRect()
  const inset = 8
  const gap = 12
  const anchorX = event.clientX - rect.left
  const anchorY = event.clientY - rect.top
  tooltip.textContent = content
  tooltip.style.display = 'block'
  const tooltipRect = tooltip.getBoundingClientRect()
  const tooltipWidth = Math.max(0, tooltip.offsetWidth || tooltipRect.width)
  const tooltipHeight = Math.max(0, tooltip.offsetHeight || tooltipRect.height)
  const maxLeft = Math.max(inset, rect.width - tooltipWidth - inset)
  const maxTop = Math.max(inset, rect.height - tooltipHeight - inset)
  const preferredLeft = anchorX + gap + tooltipWidth > rect.width - inset
    ? anchorX - tooltipWidth - gap
    : anchorX + gap
  const preferredTop = anchorY + gap + tooltipHeight > rect.height - inset
    ? anchorY - tooltipHeight - gap
    : anchorY + gap
  tooltip.style.left = `${Math.max(inset, Math.min(maxLeft, preferredLeft))}px`
  tooltip.style.top = `${Math.max(inset, Math.min(maxTop, preferredTop))}px`
}

function handleMouseLeave(): void {
  hoverCell.value = null
  hideTooltip()
  repaint()
}

function handleMouseMove(event: MouseEvent): void {
  if (isPanning.value) return
  const renderModel = props.renderModel
  const overlay = overlayRef.value
  if (!renderModel || !overlay) return
  const cell = resolveHeatmapCellFromPoint(
    overlay,
    renderModel.model,
    event.clientX,
    event.clientY,
    viewport.value,
  )
  const tooltipText = cell ? heatmapTooltipText(renderModel, cell) : null
  if (!cell || !tooltipText) {
    hoverCell.value = null
    hideTooltip()
    repaint()
    return
  }
  hoverCell.value = { col: cell.col, row: cell.drawRow }
  setTooltipFromEvent(event, tooltipText)
  repaint()
}

function handleWheel(event: WheelEvent): void {
  if (!props.renderModel || (!event.ctrlKey && !event.metaKey)) return
  const delta = Number(event.deltaY)
  if (!Number.isFinite(delta) || delta === 0) return
  const factor = Math.exp(-delta * 0.0015)
  if (zoomAtClientPoint(event.clientX, event.clientY, factor)) {
    event.preventDefault()
  }
}

function handlePointerDown(event: PointerEvent): void {
  if (!props.renderModel || !viewportZoomed.value || event.button !== 0) return
  const overlay = overlayRef.value
  hideTooltip()
  hoverCell.value = null
  isPanning.value = true
  panPointerId = event.pointerId
  panStartPoint = { x: event.clientX, y: event.clientY }
  panLastPoint = { ...panStartPoint }
  panMoved = false
  try {
    overlay?.setPointerCapture?.(event.pointerId)
  } catch {
    // Pointer capture is best-effort in test and embedded-webview environments.
  }
  repaint()
}

function handlePointerMove(event: PointerEvent): void {
  if (
    !isPanning.value ||
    panPointerId !== event.pointerId ||
    !panStartPoint ||
    !panLastPoint
  ) return
  const overlay = overlayRef.value
  const size = canvasLogicalSize()
  if (!overlay || !size) return
  const rect = overlay.getBoundingClientRect()
  if (!rect.width || !rect.height) return
  const deltaX = ((event.clientX - panLastPoint.x) / rect.width) * size.width
  const deltaY = ((event.clientY - panLastPoint.y) / rect.height) * size.height
  const movedEnough = Math.hypot(
    event.clientX - panStartPoint.x,
    event.clientY - panStartPoint.y,
  ) > 2
  panLastPoint = { x: event.clientX, y: event.clientY }
  if (movedEnough) panMoved = true
  if (panByLogical(deltaX, deltaY)) {
    event.preventDefault()
  }
}

function handlePointerUp(event: PointerEvent): void {
  if (panPointerId === event.pointerId && panMoved) suppressNextClick = true
  endPan(event.pointerId)
  repaint()
}

function handleClick(event: MouseEvent): void {
  if (suppressNextClick) {
    suppressNextClick = false
    return
  }
  if (isPanning.value) return
  const renderModel = props.renderModel
  const overlay = overlayRef.value
  if (!renderModel || !overlay) return
  const cell = resolveHeatmapCellFromPoint(
    overlay,
    renderModel.model,
    event.clientX,
    event.clientY,
    viewport.value,
  )
  if (!cell) return
  const params = buildHeatmapCellParams(renderModel, cell)
  selectedCell.value = cell
  repaint()
  if (params) emit('request-candidate-detail', params)
}

function firstSelectableCell(reverse = false): HeatmapResolvedCell | null {
  const cells = keyboardSelectableCells.value
  return (reverse ? cells[cells.length - 1] : cells[0]) ?? null
}

function directionalSelectableCell(
  dCol: number,
  dRow: number,
): HeatmapResolvedCell | null {
  const current = selectedCell.value
  if (!current) return firstSelectableCell(dCol < 0 || dRow < 0)
  let best: HeatmapResolvedCell | null = null
  let bestPrimaryDistance = Number.POSITIVE_INFINITY
  let bestSecondaryDistance = Number.POSITIVE_INFINITY
  for (const cell of keyboardSelectableCells.value) {
    const primaryDistance = dCol !== 0
      ? (cell.col - current.col) * dCol
      : (cell.drawRow - current.drawRow) * dRow
    if (primaryDistance <= 0) continue
    const secondaryDistance = dCol !== 0
      ? Math.abs(cell.drawRow - current.drawRow)
      : Math.abs(cell.col - current.col)
    if (
      secondaryDistance < bestSecondaryDistance ||
      (
        secondaryDistance === bestSecondaryDistance &&
        primaryDistance < bestPrimaryDistance
      )
    ) {
      best = cell
      bestPrimaryDistance = primaryDistance
      bestSecondaryDistance = secondaryDistance
    }
  }
  return best
}

function selectKeyboardCell(cell: HeatmapResolvedCell | null): void {
  if (!cell) return
  selectedCell.value = cell
  hoverCell.value = null
  hideTooltip()
  repaint()
}

function requestSelectedCandidateDetail(): void {
  const renderModel = props.renderModel
  const cell = selectedCell.value
  if (!renderModel || !cell) return
  const params = buildHeatmapCellParams(renderModel, cell)
  if (params) emit('request-candidate-detail', params)
}

function takeKeyboardOwnership(): void {
  hoverCell.value = null
  hideTooltip()
  repaint()
}

function handleKeydown(event: KeyboardEvent): void {
  const renderModel = props.renderModel
  if (!renderModel) return
  if (event.key === '+' || event.key === '=') {
    event.preventDefault()
    takeKeyboardOwnership()
    zoomFromCenter(1.25)
    return
  }
  if (event.key === '-' || event.key === '_') {
    event.preventDefault()
    takeKeyboardOwnership()
    zoomFromCenter(1 / 1.25)
    return
  }
  if (event.key === '0') {
    event.preventDefault()
    takeKeyboardOwnership()
    resetViewport()
    return
  }
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    takeKeyboardOwnership()
    requestSelectedCandidateDetail()
    return
  }
  const movement: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  }
  const vector = movement[event.key]
  if (vector) {
    event.preventDefault()
    takeKeyboardOwnership()
    if (event.shiftKey && viewportZoomed.value) {
      panByLogical(-vector[0] * 48, -vector[1] * 48)
    } else {
      selectKeyboardCell(directionalSelectableCell(vector[0], vector[1]))
    }
    return
  }
  if (event.key === 'Home' || event.key === 'End') {
    event.preventDefault()
    takeKeyboardOwnership()
    selectKeyboardCell(firstSelectableCell(event.key === 'End'))
  }
}

function exportPng(fileName = 'heatmap.png'): boolean {
  const renderModel = props.renderModel
  const base = canvasRef.value
  const overlay = overlayRef.value
  if (
    !renderModel ||
    !base ||
    !overlay ||
    base.dataset.renderSignature !== renderModel.signature ||
    overlay.dataset.renderSignature !== renderModel.signature
  ) {
    return false
  }
  return downloadHeatmapPng(base, overlay, fileName, () => {
    paintHeatmapBase(renderModel, base, viewport.value, activeTheme.value)
  })
}

watch(
  () => `${props.renderModel?.signature ?? ''}:${props.fitMode}`,
  async () => {
    hoverCell.value = null
    selectedCell.value = null
    hideTooltip()
    await nextTick()
    if (!props.renderModel) {
      viewport.value = { ...DEFAULT_HEATMAP_VIEWPORT }
      endPan()
      clearCanvases()
      return
    }
    if (props.fitMode === 'valid') {
      fitViewportToValidDomain()
    } else {
      viewport.value = { ...DEFAULT_HEATMAP_VIEWPORT }
      repaint(true)
    }
  },
  { immediate: true },
)

onMounted(() => {
  stopThemeObserver = observeHeatmapTheme((theme) => {
    activeTheme.value = theme
    lastBaseSignature.value = ''
    repaint(true)
  })
  stopDevicePixelRatioObserver = observeDevicePixelRatioChanges(() => {
    repaint(true)
  }, {
    maxPixelRatio: HEATMAP_MAX_CANVAS_PIXEL_RATIO,
  })
  if (typeof ResizeObserver !== 'undefined' && stackRef.value) {
    resizeObserver = new ResizeObserver(() => {
      if (props.fitMode === 'valid') {
        fitViewportToValidDomain()
      } else {
        repaint(true)
      }
    })
    resizeObserver.observe(stackRef.value)
  }
  void nextTick(() => {
    if (props.fitMode === 'valid') {
      fitViewportToValidDomain()
    } else {
      repaint(true)
    }
  })
})

onBeforeUnmount(() => {
  stopThemeObserver?.()
  stopThemeObserver = null
  stopDevicePixelRatioObserver?.()
  stopDevicePixelRatioObserver = null
  resizeObserver?.disconnect()
  resizeObserver = null
  hoverCell.value = null
  selectedCell.value = null
  hideTooltip()
  if (lastBaseSignature.value) clearCanvases()
})

defineExpose({ exportPng, repaint })
</script>

<template>
  <div class="heatmapCanvasHost">
    <div class="heatmapCanvasHeader">
      <h3 class="panelPaneTitle">Heatmap Chart</h3>
      <div v-if="renderModel" class="heatmapViewportControls" role="group" aria-label="Heatmap viewport controls">
        <AppIconButton
          :icon="Minus"
          label="Zoom out"
          title="Zoom out"
          size="sm"
          variant="subtle"
          :disabled="!canZoomOut"
          disabled-reason="Already fit to view"
          @click="zoomFromCenter(1 / 1.25)"
        />
        <span class="heatmapZoomLevel mono panelNumeric">{{ viewportScalePercent }}</span>
        <AppIconButton
          :icon="Plus"
          label="Zoom in"
          title="Zoom in"
          size="sm"
          variant="subtle"
          :disabled="!canZoomIn"
          disabled-reason="Maximum zoom reached"
          @click="zoomFromCenter(1.25)"
        />
        <AppIconButton
          :icon="RotateCcw"
          label="Reset heatmap viewport"
          title="Reset viewport"
          size="sm"
          variant="subtle"
          :disabled="!viewportZoomed"
          disabled-reason="Viewport already reset"
          @click="resetViewport"
        />
        <span id="heatmapFitModeBadgeVue" class="heatmapFitModeBadge mono panelNumeric">Fit {{ fitModeLabel }}</span>
      </div>
    </div>
    <div id="heatmapEmptyHintVue" v-show="!renderModel" class="panelStatus" role="status" aria-live="polite">
      Run an optimization with Heatmap bins &gt; 0 to generate a heatmap.
    </div>
    <div class="chartAssist">
      <div id="heatmapKeyboardHint" class="panelMuted">
        Click a populated cell, use arrow keys and Enter, or hold Ctrl/Command while scrolling to zoom. Top results keeps exact ranked candidates.
      </div>
      <div class="actions">
        <button type="button" data-qa-action="heatmap.reviewTop" @click="$emit('review-top-results')">Review Top results</button>
      </div>
    </div>
    <details v-if="!renderModel" class="heatmapDetails">
      <summary>Heatmap requirements</summary>
      <p>
        Heatmaps require a scored optimization run with Heatmap bins above 0. If the renderer stays empty,
        verify those settings before rerunning.
      </p>
    </details>
    <div id="heatmapWrapVue" v-show="!!renderModel" class="heatmapWrap">
      <div id="heatmapMetaVue" class="panelStatus heatmapRenderMeta">
        {{ renderModel?.meta || '-' }}
      </div>
      <div
        v-if="unavailableDomainSummary"
        id="heatmapDomainSummaryVue"
        class="panelMuted heatmapDomainSummary"
      >
        {{ unavailableDomainSummary }}
      </div>
      <AppMetricStrip
        v-if="heatmapSummaryItems.length"
        id="heatmapMarginalSummaryVue"
        class="heatmapMarginalSummary"
        :items="heatmapSummaryItems"
        aria-label="Heatmap marginal summary"
        min-item-width="6.8rem"
        size="compact"
        surface="sunken"
      />
      <div
        id="heatmapCanvasStackVue"
        ref="stackRef"
        class="heatmapCanvasStack chartSurface"
        role="group"
        :aria-label="ariaLabel"
        :data-heatmap-theme="activeTheme"
        aria-describedby="heatmapKeyboardHint heatmapAxisDescriptionVue"
        :class="{ zoomed: viewportZoomed, panning: isPanning }"
        :data-viewport-scale="viewportScaleData"
        :data-viewport-offset-x="viewportOffsetXData"
        :data-viewport-offset-y="viewportOffsetYData"
        :data-render-ready="renderModel ? 'true' : 'false'"
        :data-finite-cells="String(finiteCellCount)"
        :data-total-cells="String(totalCellCount)"
        :data-finite-coverage="finiteCoverageData"
        :data-axis-summary="axisDataSummary"
        tabindex="0"
        @keydown="handleKeydown"
        @wheel="handleWheel"
      >
        <canvas id="heatmapCanvasVue" ref="canvasRef" width="512" height="512" aria-hidden="true"></canvas>
        <canvas
          id="heatmapOverlayVue"
          ref="overlayRef"
          width="512"
          height="512"
          aria-hidden="true"
          @mouseleave="handleMouseLeave"
          @mousemove="handleMouseMove"
          @pointerdown="handlePointerDown"
          @pointermove="handlePointerMove"
          @pointerup="handlePointerUp"
          @pointercancel="handlePointerUp"
          @click="handleClick"
        ></canvas>
        <div id="heatmapTooltipVue" ref="tooltipRef" class="heatmapTooltip" style="display: none;"></div>
      </div>
      <div
        id="heatmapAxisDescriptionVue"
        class="srOnly"
      >
        {{ axisAccessibilitySummary }}
      </div>
      <div class="heatmapLegendRow">
        <HeatmapLegend ref="legendRef" :note="renderModel?.displayLegendNote || ''" />
        <div>
          <div class="heatmapLegendSpacer" aria-hidden="true"></div>
          <div class="panelMuted heatmapLegendNote">
            {{ selectionLegendNote }}
          </div>
        </div>
      </div>
    </div>
    <div
      v-if="visibleSelectedSummary"
      id="heatmapSelectedCellSummaryVue"
      class="panelStatus mono panelNumeric"
      role="status"
      aria-live="polite"
    >
      {{ visibleSelectedSummary }}
    </div>
  </div>
</template>

<style scoped>
.heatmapCanvasHost {
  display: grid;
  gap: 0.65rem;
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.heatmapCanvasHeader {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.heatmapCanvasHeader .panelPaneTitle {
  margin: 0;
}

.heatmapViewportControls {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.heatmapZoomLevel {
  min-width: 3.25rem;
  text-align: center;
  color: var(--muted-strong);
  font-size: 0.76rem;
}

.heatmapFitModeBadge {
  color: var(--muted);
  font-size: 0.72rem;
  white-space: nowrap;
}

.chartAssist {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.chartAssist .panelMuted {
  flex: 1 1 22rem;
  margin: 0;
}

.heatmapDetails {
  border: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-inline) 76%, transparent);
  padding: 0.5rem 0.65rem;
  color: var(--muted-strong);
  font-size: 0.78rem;
}

.heatmapDetails summary {
  cursor: pointer;
  font-weight: 700;
  color: var(--text);
}

.heatmapDetails p {
  margin: 0.45rem 0 0;
  line-height: 1.45;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.heatmapWrap {
  margin-top: 0.1rem;
  display: grid;
  gap: 0.55rem;
}

.heatmapRenderMeta {
  margin: 0;
}

.heatmapDomainSummary {
  max-width: 64rem;
}

.heatmapMarginalSummary {
  margin-top: 0.05rem;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.heatmapCanvasStack {
  position: relative;
  inline-size: min(100%, 72rem);
  width: min(100%, 72rem);
  aspect-ratio: 1 / 1;
  margin-inline: auto;
  overflow: hidden;
  border: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  border-radius: 10px;
  background:
    linear-gradient(135deg, color-mix(in oklab, var(--surface-inline) 96%, transparent), transparent 44%),
    color-mix(in oklab, var(--surface-working) 96%, var(--panel-footer));
  cursor: crosshair;
  touch-action: pan-y pinch-zoom;
  user-select: none;
}

.heatmapCanvasStack canvas {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
}

.heatmapCanvasStack.zoomed {
  cursor: grab;
}

.heatmapCanvasStack.panning {
  cursor: grabbing;
}

.heatmapLegendRow {
  margin-top: 0.75rem;
  display: grid;
  grid-template-columns: minmax(0, auto) minmax(0, 1fr);
  gap: 0.75rem;
  align-items: start;
}

.heatmapLegendNote {
  margin-top: 0.5rem;
}

.heatmapLegendSpacer {
  min-height: 1.1rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 760px) {
  .heatmapCanvasHeader {
    align-items: flex-start;
    flex-direction: column;
  }

  .heatmapLegendRow {
    grid-template-columns: 1fr;
  }

  .chartAssist {
    align-items: flex-start;
  }

  .heatmapCanvasStack {
    width: 100%;
  }
}
</style>
