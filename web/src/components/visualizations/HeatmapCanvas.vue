<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Minus, Plus, RotateCcw } from 'lucide-vue-next'
import {
  buildHeatmapCellParams,
  clampHeatmapViewport,
  clearHeatmapCanvas,
  DEFAULT_HEATMAP_VIEWPORT,
  downloadHeatmapPng,
  HEATMAP_MAX_VIEWPORT_SCALE,
  HEATMAP_MIN_VIEWPORT_SCALE,
  heatmapTooltipText,
  heatmapViewportIsZoomed,
  paintHeatmapBase,
  paintHeatmapOverlay,
  resolveHeatmapCellAt,
  resolveHeatmapCellFromPoint,
  type HeatmapCanvasRenderModel,
  type HeatmapCanvasViewport,
  type HeatmapCellParams,
  type HeatmapHoverCell,
  type HeatmapResolvedCell,
} from '../../visualizations/heatmapCanvasModel'
import AppIconButton from '../ui/AppIconButton.vue'
import HeatmapLegend from './HeatmapLegend.vue'

const props = withDefaults(
  defineProps<{
    ariaLabel: string
    renderModel?: HeatmapCanvasRenderModel | null
    selectedSummary?: string
  }>(),
  {
    renderModel: null,
    selectedSummary: '',
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
let resizeObserver: ResizeObserver | null = null
let panPointerId: number | null = null
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
const selectedCellSummary = computed(() => {
  const renderModel = props.renderModel
  const cell = selectedCell.value
  return renderModel && cell ? heatmapTooltipText(renderModel, cell) : ''
})
const visibleSelectedSummary = computed(() => selectedCellSummary.value || props.selectedSummary)

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
  lastBaseSignature.value = ''
}

function syncCanvasBackingStore(): boolean {
  const stack = stackRef.value
  const canvases = [canvasRef.value, overlayRef.value].filter((canvas): canvas is HTMLCanvasElement => Boolean(canvas))
  if (!stack || canvases.length === 0) return false
  const rect = stack.getBoundingClientRect()
  const cssWidth = Math.max(240, Math.round(rect.width || stack.clientWidth || 512))
  const cssHeight = Math.max(240, Math.round(rect.height || stack.clientHeight || cssWidth))
  const ratio = typeof window === 'undefined'
    ? 1
    : Math.min(3, Math.max(1, Math.round((window.devicePixelRatio || 1) * 100) / 100))
  const targetWidth = Math.max(1, Math.round(cssWidth * ratio))
  const targetHeight = Math.max(1, Math.round(cssHeight * ratio))
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

function clampViewportState(next: HeatmapCanvasViewport = viewport.value): void {
  const size = canvasLogicalSize()
  viewport.value = size
    ? clampHeatmapViewport(next, size.width, size.height)
    : clampHeatmapViewport(next, 512, 512)
}

function resetViewport(): void {
  viewport.value = { ...DEFAULT_HEATMAP_VIEWPORT }
  endPan()
  repaint(true)
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

function zoomAtLogicalPoint(point: { x: number; y: number }, factor: number): void {
  const size = canvasLogicalSize()
  if (!size || !Number.isFinite(factor) || factor <= 0) return
  const current = viewport.value
  const nextScale = Math.max(
    HEATMAP_MIN_VIEWPORT_SCALE,
    Math.min(HEATMAP_MAX_VIEWPORT_SCALE, current.scale * factor),
  )
  const modelX = (point.x - current.offsetX) / current.scale
  const modelY = (point.y - current.offsetY) / current.scale
  clampViewportState({
    scale: nextScale,
    offsetX: point.x - modelX * nextScale,
    offsetY: point.y - modelY * nextScale,
  })
  if (!viewportZoomed.value) endPan()
  repaint(true)
}

function zoomAtClientPoint(clientX: number, clientY: number, factor: number): void {
  const point = clientPointToLogicalCanvas(clientX, clientY)
  if (point) zoomAtLogicalPoint(point, factor)
}

function zoomFromCenter(factor: number): void {
  const size = canvasLogicalSize()
  if (!size) return
  zoomAtLogicalPoint({ x: size.width / 2, y: size.height / 2 }, factor)
}

function panByLogical(deltaX: number, deltaY: number): void {
  if (!viewportZoomed.value) return
  clampViewportState({
    ...viewport.value,
    offsetX: viewport.value.offsetX + deltaX,
    offsetY: viewport.value.offsetY + deltaY,
  })
  hideTooltip()
  repaint(true)
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
  const renderSignature = `${renderModel.signature}:${viewportKey.value}`
  if (forceBase || backingStoreChanged || lastBaseSignature.value !== renderSignature) {
    paintHeatmapBase(renderModel, canvas, viewport.value)
    canvas.dataset.renderSignature = renderModel.signature
    lastBaseSignature.value = renderSignature
  }
  paintHeatmapOverlay(renderModel, overlay, overlayCell(), viewport.value)
  overlay.dataset.renderSignature = renderModel.signature
  legendRef.value?.paint(renderModel.range, renderModel.palette)
}

function setTooltipFromEvent(event: MouseEvent, content: string): void {
  const tooltip = tooltipRef.value
  const overlay = overlayRef.value
  if (!tooltip || !overlay) return
  const rect = overlay.getBoundingClientRect()
  tooltip.style.display = 'block'
  tooltip.style.left = `${Math.max(8, event.clientX - rect.left + 12)}px`
  tooltip.style.top = `${Math.max(8, event.clientY - rect.top + 12)}px`
  tooltip.textContent = content
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
  if (!props.renderModel) return
  event.preventDefault()
  const delta = Number(event.deltaY)
  if (!Number.isFinite(delta) || delta === 0) return
  const factor = Math.exp(-delta * 0.0015)
  zoomAtClientPoint(event.clientX, event.clientY, factor)
}

function handlePointerDown(event: PointerEvent): void {
  if (!props.renderModel || !viewportZoomed.value || event.button !== 0) return
  const overlay = overlayRef.value
  event.preventDefault()
  hideTooltip()
  hoverCell.value = null
  isPanning.value = true
  panPointerId = event.pointerId
  panLastPoint = { x: event.clientX, y: event.clientY }
  panMoved = false
  try {
    overlay?.setPointerCapture?.(event.pointerId)
  } catch {
    // Pointer capture is best-effort in test and embedded-webview environments.
  }
  repaint()
}

function handlePointerMove(event: PointerEvent): void {
  if (!isPanning.value || panPointerId !== event.pointerId || !panLastPoint) return
  const overlay = overlayRef.value
  const size = canvasLogicalSize()
  if (!overlay || !size) return
  const rect = overlay.getBoundingClientRect()
  if (!rect.width || !rect.height) return
  event.preventDefault()
  const deltaX = ((event.clientX - panLastPoint.x) / rect.width) * size.width
  const deltaY = ((event.clientY - panLastPoint.y) / rect.height) * size.height
  if (Math.abs(event.clientX - panLastPoint.x) + Math.abs(event.clientY - panLastPoint.y) > 2) {
    panMoved = true
  }
  panLastPoint = { x: event.clientX, y: event.clientY }
  panByLogical(deltaX, deltaY)
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
  if (params) {
    selectedCell.value = cell
    repaint()
    emit('request-candidate-detail', params)
  }
}

function cellIsSelectable(renderModel: HeatmapCanvasRenderModel, cell: HeatmapResolvedCell | null): cell is HeatmapResolvedCell {
  return Boolean(cell && heatmapTooltipText(renderModel, cell) && buildHeatmapCellParams(renderModel, cell))
}

function firstSelectableCell(renderModel: HeatmapCanvasRenderModel, reverse = false): HeatmapResolvedCell | null {
  const { binsFast, binsSlow } = renderModel.model
  const total = binsFast * binsSlow
  for (let offset = 0; offset < total; offset += 1) {
    const next = reverse ? total - 1 - offset : offset
    const drawRow = Math.floor(next / binsFast)
    const col = next % binsFast
    const cell = resolveHeatmapCellAt(renderModel.model, col, drawRow)
    if (cellIsSelectable(renderModel, cell)) return cell
  }
  return null
}

function directionalSelectableCell(
  renderModel: HeatmapCanvasRenderModel,
  dCol: number,
  dRow: number,
): HeatmapResolvedCell | null {
  const current = selectedCell.value
  if (!current) return firstSelectableCell(renderModel, dCol < 0 || dRow < 0)
  const steps = dCol !== 0 ? renderModel.model.binsFast : renderModel.model.binsSlow
  for (let step = 1; step < steps; step += 1) {
    const cell = resolveHeatmapCellAt(
      renderModel.model,
      current.col + dCol * step,
      current.drawRow + dRow * step,
    )
    if (cellIsSelectable(renderModel, cell)) return cell
  }
  return null
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

function handleKeydown(event: KeyboardEvent): void {
  const renderModel = props.renderModel
  if (!renderModel) return
  if (event.key === '+' || event.key === '=') {
    event.preventDefault()
    zoomFromCenter(1.25)
    return
  }
  if (event.key === '-' || event.key === '_') {
    event.preventDefault()
    zoomFromCenter(1 / 1.25)
    return
  }
  if (event.key === '0') {
    event.preventDefault()
    resetViewport()
    return
  }
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
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
    if (event.shiftKey && viewportZoomed.value) {
      panByLogical(-vector[0] * 48, -vector[1] * 48)
    } else {
      selectKeyboardCell(directionalSelectableCell(renderModel, vector[0], vector[1]))
    }
    return
  }
  if (event.key === 'Home' || event.key === 'End') {
    event.preventDefault()
    selectKeyboardCell(firstSelectableCell(renderModel, event.key === 'End'))
  }
}

function exportPng(fileName = 'heatmap.png'): boolean {
  return downloadHeatmapPng(canvasRef.value, overlayRef.value, fileName)
}

watch(
  () => props.renderModel?.signature ?? '',
  async () => {
    hoverCell.value = null
    selectedCell.value = null
    hideTooltip()
    await nextTick()
    repaint(true)
  },
  { immediate: true },
)

onMounted(() => {
  if (typeof ResizeObserver !== 'undefined' && stackRef.value) {
    resizeObserver = new ResizeObserver(() => repaint(true))
    resizeObserver.observe(stackRef.value)
  }
  void nextTick(() => repaint(true))
})

onBeforeUnmount(() => {
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
        <span class="heatmapZoomLevel mono panelNumeric" aria-live="polite">{{ viewportScalePercent }}</span>
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
      </div>
    </div>
    <div id="heatmapEmptyHintVue" v-show="!renderModel" class="panelStatus" role="status" aria-live="polite">
      Run an optimization with Heatmap bins &gt; 0 to generate a heatmap.
    </div>
    <div class="chartAssist">
      <div id="heatmapKeyboardHint" class="panelMuted">
        Click a populated cell or use arrow keys and Enter. Top results remains the exact candidate path.
      </div>
      <div class="actions">
        <button type="button" @click="$emit('review-top-results')">Review Top results</button>
      </div>
    </div>
    <details class="heatmapDetails">
      <summary>Heatmap requirements</summary>
      <p>
        Heatmaps require a scored optimization run with Heatmap bins above 0. If the renderer stays empty,
        verify those settings before rerunning.
      </p>
    </details>
    <div id="heatmapWrapVue" v-show="!!renderModel" class="heatmapWrap">
      <div id="heatmapMetaVue" class="panelStatus heatmapRenderMeta" role="status" aria-live="polite">
        {{ renderModel?.meta || '-' }}
      </div>
      <div
        id="heatmapCanvasStackVue"
        ref="stackRef"
        class="heatmapCanvasStack chartSurface"
        :aria-label="ariaLabel"
        aria-describedby="heatmapKeyboardHint"
        :class="{ zoomed: viewportZoomed, panning: isPanning }"
        :data-viewport-scale="viewportScaleData"
        :data-viewport-offset-x="viewportOffsetXData"
        :data-viewport-offset-y="viewportOffsetYData"
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
      <div class="heatmapLegendRow">
        <HeatmapLegend ref="legendRef" :note="renderModel?.displayLegendNote || ''" />
        <div>
          <label>&nbsp;</label>
          <div class="panelMuted heatmapLegendNote">
            Select a Top result for exact retained candidates, or click a populated cell for representative heatmap-region candidate detail. Markers and crosshair stay on for both paths.
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

.chartAssist {
  display: grid;
  gap: 0.35rem;
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
}

.heatmapRenderMeta {
  margin-bottom: 0.5rem;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.heatmapCanvasStack {
  cursor: crosshair;
  touch-action: none;
  user-select: none;
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
}
</style>
