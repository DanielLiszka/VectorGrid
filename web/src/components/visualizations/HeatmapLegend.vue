<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  formatHeatmapDisplay,
  paintHeatmapLegend,
  type HeatmapLegendTargets,
} from '../../visualizations/heatmapCanvasModel'
import type { HeatmapPalette, HeatmapRange } from '../../visualizations/heatmapModel'
import type { LightweightChartTheme } from '../../visualizations/chartTheme'
import {
  heatmapLegendRangeAnchors,
  heatmapPaletteSemanticLabels,
  resolveHeatmapPaletteTheme,
} from '../../visualizations/heatmapTheme'

const props = withDefaults(
  defineProps<{
    label?: string
    note?: string
  }>(),
  {
    label: 'Legend',
    note: '',
  },
)

const canvasRef = ref<HTMLCanvasElement | null>(null)
const minLabelRef = ref<HTMLElement | null>(null)
const midLabelRef = ref<HTMLElement | null>(null)
const maxLabelRef = ref<HTMLElement | null>(null)
const semanticLabels = ref({
  low: 'Lower',
  middle: 'Neutral',
  high: 'Higher',
})
const paletteKind = ref<HeatmapPalette['kind']>('diverging')
const semanticMiddleOffset = ref('50%')
const accessibleScaleSummary = ref('Scale values are not available yet')
const accessibleLegendText = computed(() => {
  const segments = [props.label, accessibleScaleSummary.value, props.note]
    .map((segment) => String(segment || '').trim().replace(/[.]+$/u, ''))
    .filter(Boolean)
  return `${segments.join('. ')}.`
})

function accessibleAnchor(label: string, value: number): string {
  if (value === 0 && /(?:^|\s)0$/u.test(label)) return label
  return `${label} ${formatHeatmapDisplay(value, 4)}`
}

function legendTargets(): HeatmapLegendTargets {
  return {
    canvas: canvasRef.value,
    minLabel: minLabelRef.value,
    midLabel: midLabelRef.value,
    maxLabel: maxLabelRef.value,
  }
}

function paint(
  range: HeatmapRange,
  palette: HeatmapPalette,
  theme?: LightweightChartTheme,
): void {
  const resolved = resolveHeatmapPaletteTheme(palette, theme)
  const anchors = heatmapLegendRangeAnchors(range, resolved)
  paletteKind.value = resolved.kind
  semanticLabels.value = heatmapPaletteSemanticLabels(resolved, range)
  semanticMiddleOffset.value = `${(anchors.middleOffset * 100).toFixed(4).replace(/0+$/u, '').replace(/\.$/u, '')}%`
  accessibleScaleSummary.value = [
    accessibleAnchor(semanticLabels.value.low, anchors.min),
    accessibleAnchor(semanticLabels.value.middle, anchors.middle),
    accessibleAnchor(semanticLabels.value.high, anchors.max),
  ].join('; ')
  paintHeatmapLegend(legendTargets(), range, resolved, theme)
}

defineExpose({ paint })
</script>

<template>
  <div
    class="heatmapLegendBlock"
    role="group"
    :aria-label="accessibleLegendText"
    :data-palette-kind="paletteKind"
  >
    <div class="heatmapLegendLabel" aria-hidden="true">{{ label }}</div>
    <div class="heatmapLegendSemantics" aria-hidden="true">
      <span>{{ semanticLabels.low }}</span>
      <span :style="{ left: semanticMiddleOffset }">{{ semanticLabels.middle }}</span>
      <span>{{ semanticLabels.high }}</span>
    </div>
    <canvas
      id="heatmapLegendVue"
      ref="canvasRef"
      class="heatmapLegendCanvas"
      width="512"
      height="18"
      aria-hidden="true"
    ></canvas>
    <div class="heatmapLegendLabels" aria-hidden="true">
      <span id="heatmapLegendMinVue" ref="minLabelRef" class="mono">-</span>
      <span id="heatmapLegendMidVue" ref="midLabelRef" class="mono">-</span>
      <span id="heatmapLegendMaxVue" ref="maxLabelRef" class="mono">-</span>
    </div>
    <div v-if="note" class="heatmapLegendNote" aria-hidden="true">{{ note }}</div>
  </div>
</template>

<style scoped>
.heatmapLegendBlock {
  min-width: min(100%, 32rem);
}

.heatmapLegendLabel {
  color: var(--muted-strong);
  font-size: 0.74rem;
  font-weight: 700;
}

.heatmapLegendSemantics {
  position: relative;
  min-height: 0.84rem;
  margin: 0.28rem 0 0.2rem;
  color: var(--muted-strong);
  font-size: 0.68rem;
  font-weight: 650;
  letter-spacing: 0.015em;
}

.heatmapLegendSemantics span,
.heatmapLegendLabels span {
  position: absolute;
  top: 0;
  max-width: 42%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.heatmapLegendSemantics span:first-child,
.heatmapLegendLabels span:first-child {
  left: 0;
}

.heatmapLegendSemantics span:nth-child(2) {
  transform: translateX(-50%);
  text-align: center;
}

.heatmapLegendSemantics span:last-child,
.heatmapLegendLabels span:last-child {
  right: 0;
  text-align: right;
}

.heatmapLegendLabels {
  position: relative;
  display: block;
  min-height: 1rem;
}

.heatmapLegendLabels span:nth-child(2) {
  transform: translateX(-50%);
  text-align: center;
}

.heatmapLegendCanvas {
  overflow: hidden;
  border-radius: 999px;
  box-shadow:
    inset 0 0 0 1px color-mix(in oklab, var(--border) 72%, transparent),
    0 5px 14px color-mix(in oklab, var(--shadow-color, #020617) 12%, transparent);
}

.heatmapLegendNote {
  margin-top: 0.35rem;
  color: var(--muted);
  font-size: 0.78rem;
}

.mono {
  font-family: var(--mono);
}
</style>
