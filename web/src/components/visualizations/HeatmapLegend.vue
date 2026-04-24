<script setup lang="ts">
import { ref } from 'vue'
import { paintHeatmapLegend, type HeatmapLegendTargets } from '../../visualizations/heatmapCanvasModel'
import type { HeatmapPalette, HeatmapRange } from '../../visualizations/heatmapModel'

withDefaults(
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

function legendTargets(): HeatmapLegendTargets {
  return {
    canvas: canvasRef.value,
    minLabel: minLabelRef.value,
    midLabel: midLabelRef.value,
    maxLabel: maxLabelRef.value,
  }
}

function paint(range: HeatmapRange, palette: HeatmapPalette): void {
  paintHeatmapLegend(legendTargets(), range, palette)
}

defineExpose({ paint })
</script>

<template>
  <div class="heatmapLegendBlock">
    <label>{{ label }}</label>
    <canvas
      id="heatmapLegendVue"
      ref="canvasRef"
      class="heatmapLegendCanvas"
      width="512"
      height="18"
      aria-hidden="true"
    ></canvas>
    <div class="heatmapLegendLabels">
      <span id="heatmapLegendMinVue" ref="minLabelRef" class="mono">-</span>
      <span id="heatmapLegendMidVue" ref="midLabelRef" class="mono">-</span>
      <span id="heatmapLegendMaxVue" ref="maxLabelRef" class="mono">-</span>
    </div>
    <div v-if="note" class="heatmapLegendNote">{{ note }}</div>
  </div>
</template>

<style scoped>
.heatmapLegendBlock {
  min-width: min(100%, 32rem);
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
