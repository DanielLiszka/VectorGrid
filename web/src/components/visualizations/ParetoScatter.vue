<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { ParetoRenderModel, ParetoRenderPoint } from '../../visualizations/paretoModel'

const props = withDefaults(
  defineProps<{
    model: ParetoRenderModel | null
    height?: number
    ariaLabel?: string
    describedBy?: string
  }>(),
  {
    height: 240,
    ariaLabel:
      'Pareto frontier chart comparing the selected X and Y metrics. Use the Top results tab for a non-pointer way to move between candidates.',
    describedBy: 'paretoKeyboardHint',
  },
)

const emit = defineEmits<{
  'select-point': [point: ParetoRenderPoint]
}>()

const canvasRef = ref<HTMLCanvasElement | null>(null)
const selectedIndex = ref(0)
let plottedPoints: Array<{ x: number; y: number; point: ParetoRenderPoint }> = []
let resizeObserver: ResizeObserver | null = null

const hasPoints = computed(() => (props.model?.points.length ?? 0) > 0)
const selectedPoint = computed(() => {
  const points = props.model?.points ?? []
  if (!points.length) return null
  const clamped = Math.max(0, Math.min(points.length - 1, selectedIndex.value))
  return points[clamped] ?? null
})
const selectedSummary = computed(() => selectedPoint.value?.summary ?? 'No Pareto point selected.')

function formatAxisValue(value: number): string {
  if (!Number.isFinite(value)) return '-'
  const abs = Math.abs(value)
  if (abs === 0) return '0'
  if (abs >= 1000) return value.toLocaleString(undefined, { maximumFractionDigits: 1 })
  return value.toFixed(3).replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
}

function drawPareto(): void {
  const canvas = canvasRef.value
  const model = props.model
  if (!canvas || !model || !model.points.length) {
    plottedPoints = []
    return
  }

  const rect = canvas.getBoundingClientRect()
  const cssWidth = Math.max(320, Math.floor(rect.width || canvas.clientWidth || 1024))
  const cssHeight = Math.max(160, Math.floor(props.height || 240))
  const dpr = Math.max(1, window.devicePixelRatio || 1)
  canvas.width = Math.floor(cssWidth * dpr)
  canvas.height = Math.floor(cssHeight * dpr)
  canvas.style.height = `${cssHeight}px`

  const ctx = canvas.getContext('2d')
  if (!ctx) {
    plottedPoints = []
    return
  }

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, cssWidth, cssHeight)

  const padding = { left: 56, right: 18, top: 18, bottom: 34 }
  const plotWidth = Math.max(1, cssWidth - padding.left - padding.right)
  const plotHeight = Math.max(1, cssHeight - padding.top - padding.bottom)
  const xMin = model.xRange.min
  const xMax = model.xRange.max
  const yMin = model.yRange.min
  const yMax = model.yRange.max
  const xSpan = Math.max(1e-12, xMax - xMin)
  const ySpan = Math.max(1e-12, yMax - yMin)
  const toX = (value: number): number => padding.left + ((value - xMin) / xSpan) * plotWidth
  const toY = (value: number): number =>
    padding.top + (plotHeight - ((value - yMin) / ySpan) * plotHeight)

  ctx.fillStyle = 'rgba(248, 250, 252, 0.02)'
  ctx.fillRect(padding.left, padding.top, plotWidth, plotHeight)
  ctx.strokeStyle = 'rgba(118, 140, 166, 0.32)'
  ctx.lineWidth = 1
  ctx.strokeRect(padding.left, padding.top, plotWidth, plotHeight)

  ctx.font = '11px ui-monospace, monospace'
  ctx.fillStyle = 'rgba(118, 140, 166, 0.92)'
  ctx.fillText(model.xKey, cssWidth - padding.right - 52, cssHeight - 8)
  ctx.save()
  ctx.translate(13, padding.top + plotHeight / 2)
  ctx.rotate(-Math.PI / 2)
  ctx.fillText(model.yKey, 0, 0)
  ctx.restore()
  ctx.fillText(formatAxisValue(xMin), padding.left, cssHeight - 8)
  ctx.fillText(formatAxisValue(xMax), cssWidth - padding.right - 48, cssHeight - 8)
  ctx.fillText(formatAxisValue(yMax), 8, padding.top + 4)
  ctx.fillText(formatAxisValue(yMin), 8, padding.top + plotHeight)

  ctx.strokeStyle = 'rgba(82, 164, 255, 0.55)'
  ctx.lineWidth = 1.25
  ctx.beginPath()
  model.frontier.forEach((point, index) => {
    const x = toX(point.x)
    const y = toY(point.y)
    if (index === 0) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  })
  ctx.stroke()

  const plotted = model.points.map((point) => ({
    x: toX(point.x),
    y: toY(point.y),
    point,
  }))
  plottedPoints = plotted

  const selected = selectedPoint.value
  for (const plottedPoint of plotted) {
    const isSelected = selected?.rowId === plottedPoint.point.rowId
    ctx.fillStyle = isSelected ? 'rgba(16, 185, 129, 0.95)' : 'rgba(56, 189, 248, 0.88)'
    ctx.beginPath()
    ctx.arc(plottedPoint.x, plottedPoint.y, isSelected ? 5 : 3.5, 0, Math.PI * 2)
    ctx.fill()
    if (isSelected) {
      ctx.strokeStyle = 'rgba(16, 185, 129, 0.55)'
      ctx.lineWidth = 2
      ctx.stroke()
    }
  }
}

function syncSelectedIndex(): void {
  const points = props.model?.points ?? []
  selectedIndex.value = points.length ? Math.max(0, Math.min(points.length - 1, selectedIndex.value)) : 0
  void nextTick(drawPareto)
}

function activatePoint(point: ParetoRenderPoint | null): void {
  if (!point) return
  emit('select-point', point)
}

function onCanvasClick(event: MouseEvent): void {
  const canvas = canvasRef.value
  if (!canvas || !plottedPoints.length) return
  const rect = canvas.getBoundingClientRect()
  if (!rect.width || !rect.height) return
  const px = event.clientX - rect.left
  const py = event.clientY - rect.top
  let bestPoint: ParetoRenderPoint | null = null
  let bestIndex = -1
  let bestDist = Number.POSITIVE_INFINITY
  plottedPoints.forEach((plot, index) => {
    const dx = plot.x - px
    const dy = plot.y - py
    const dist = Math.sqrt(dx * dx + dy * dy)
    if (dist < bestDist) {
      bestPoint = plot.point
      bestIndex = index
      bestDist = dist
    }
  })
  if (!bestPoint || bestIndex < 0 || bestDist > 18) return
  selectedIndex.value = bestIndex
  drawPareto()
  activatePoint(bestPoint)
}

function onChartKeydown(event: KeyboardEvent): void {
  const points = props.model?.points ?? []
  if (!points.length) return
  const last = points.length - 1
  if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
    event.preventDefault()
    selectedIndex.value = selectedIndex.value >= last ? 0 : selectedIndex.value + 1
    drawPareto()
    return
  }
  if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
    event.preventDefault()
    selectedIndex.value = selectedIndex.value <= 0 ? last : selectedIndex.value - 1
    drawPareto()
    return
  }
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    activatePoint(selectedPoint.value)
  }
}

watch(
  () => props.model,
  () => syncSelectedIndex(),
  { immediate: true },
)

onMounted(() => {
  const canvas = canvasRef.value
  if (canvas && 'ResizeObserver' in window) {
    resizeObserver = new ResizeObserver(() => drawPareto())
    resizeObserver.observe(canvas)
  }
  void nextTick(drawPareto)
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
})
</script>

<template>
  <div
    id="paretoWrapVue"
    class="paretoScatter"
    :style="{ display: hasPoints ? 'block' : 'none' }"
  >
    <div id="paretoMetaVue" class="panelStatus paretoMeta" role="status" aria-live="polite">
      {{ model?.meta || '-' }}
    </div>
    <div
      class="chartSurface paretoChartSurface"
      tabindex="0"
      :aria-label="ariaLabel"
      :aria-describedby="describedBy"
      @keydown="onChartKeydown"
    >
      <canvas
        id="paretoCanvasVue"
        ref="canvasRef"
        class="chart paretoCanvas"
        role="img"
        :aria-label="`${model?.meta || 'Pareto frontier chart'}. ${selectedSummary}`"
        @click="onCanvasClick"
      ></canvas>
    </div>
    <div class="panelMuted paretoSelectionSummary" aria-live="polite">{{ selectedSummary }}</div>
    <div class="panelMuted">Select a Top result or click a point to drill down that parameter pair.</div>
    <div class="panelMuted">Frontier membership visualizes tradeoffs; validation still decides whether a candidate is acceptable.</div>
  </div>
</template>

<style scoped>
.paretoScatter {
  margin-top: 0.75rem;
}

.paretoMeta {
  margin: 0.25rem 0 0.6rem;
  font-size: 0.85rem;
}

.paretoChartSurface {
  min-width: 0;
}

.paretoCanvas {
  display: block;
  width: 100%;
  min-height: 160px;
}

.paretoChartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.paretoSelectionSummary {
  margin-top: 0.5rem;
  font-family: var(--mono);
}
</style>
