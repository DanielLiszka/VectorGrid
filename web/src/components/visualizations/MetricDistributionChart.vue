<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  buildMetricDistributionModel,
  chooseMetricDistributionKey,
  metricDistributionOptions,
  type MetricDistributionModel,
} from '../../visualizations/metricDistributionModel'
import type { MetricVisualColumn, MetricVisualRow } from '../../visualizations/metricCellVisualModel'

const props = withDefaults(
  defineProps<{
    title: string
    columns: MetricVisualColumn[]
    rows: MetricVisualRow[]
    preferredKeys?: string[]
  }>(),
  {
    preferredKeys: () => [],
  },
)

const chartHost = ref<HTMLElement | null>(null)
const selectedMetric = ref('')
const chartReady = ref(false)
const chartLoadFailed = ref(false)
let chart: { setOption: (options: Record<string, unknown>, notMerge?: boolean) => void; resize: () => void; dispose: () => void } | null = null
let resizeObserver: ResizeObserver | null = null

const metricOptions = computed(() => metricDistributionOptions(props.columns, props.rows))
const selectedModel = computed<MetricDistributionModel | null>(() =>
  buildMetricDistributionModel({
    columns: props.columns,
    rows: props.rows,
    metricKey: selectedMetric.value,
  }),
)
const selectedSummary = computed(() => {
  const model = selectedModel.value
  if (!model) return 'No numeric metric distribution available.'
  return `${model.label}: n=${model.values.length} min=${formatMetric(model.min)} median=${formatMetric(model.median)} max=${formatMetric(model.max)}`
})
const fallbackBins = computed(() => {
  const model = selectedModel.value
  if (!model) return []
  const maxCount = model.bins.reduce((max, bin) => Math.max(max, bin.count), 0)
  return model.bins.map((bin) => ({
    ...bin,
    width: `${maxCount > 0 ? Math.max(8, Math.round((bin.count / maxCount) * 100)) : 0}%`,
  }))
})

function formatMetric(value: number): string {
  if (!Number.isFinite(value)) return '0'
  const abs = Math.abs(value)
  if (abs >= 100) return value.toFixed(0)
  if (abs >= 10) return value.toFixed(1)
  return value.toFixed(3).replace(/\.?0+$/, '')
}

function hostColor(name: string, fallback: string): string {
  const host = chartHost.value
  if (!host || typeof getComputedStyle === 'undefined') return fallback
  return getComputedStyle(host).getPropertyValue(name).trim() || fallback
}

function chartColors(): { accent: string; muted: string; grid: string; secondaryBar: string } {
  const accent = hostColor('--accent', '#3b82f6')
  const muted = hostColor('--muted-soft', '#94a3b8')
  return {
    accent,
    muted,
    grid: 'rgba(148, 163, 184, 0.22)',
    secondaryBar: hostColor('--accent-soft', '#60a5fa'),
  }
}

watch(
  () => [metricOptions.value, props.preferredKeys, selectedMetric.value] as const,
  ([options, preferredKeys, current]) => {
    const next = chooseMetricDistributionKey(options, preferredKeys, current)
    if (selectedMetric.value !== next) selectedMetric.value = next
  },
  { immediate: true },
)

async function ensureChart(): Promise<boolean> {
  if (!chartHost.value) return false
  if (chart) return true
  try {
    const [{ init, use }, { CanvasRenderer }] =
      await Promise.all([
        import('echarts/core'),
        import('echarts/renderers'),
        import('echarts/lib/chart/bar'),
        import('echarts/lib/component/grid'),
      ])
    use([CanvasRenderer])
    chart = init(chartHost.value, undefined, { renderer: 'canvas' })
    chartReady.value = true
    chartLoadFailed.value = false
    if (typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(() => chart?.resize())
      resizeObserver.observe(chartHost.value)
    }
    return true
  } catch {
    chartReady.value = false
    chartLoadFailed.value = true
    return false
  }
}

async function renderChart(): Promise<void> {
  const model = selectedModel.value
  if (!model || !chartHost.value) return
  const chartReady = await ensureChart()
  if (!chartReady) return
  const maxCount = model.bins.reduce((max, bin) => Math.max(max, bin.count), 0)
  const colors = chartColors()
  chart?.setOption(
    {
      grid: {
        top: 12,
        right: 10,
        bottom: 30,
        left: 34,
      },
      xAxis: {
        type: 'category',
        data: model.bins.map((bin) => bin.label),
        axisLabel: {
          color: colors.muted,
          fontSize: 10,
          hideOverlap: true,
        },
        axisTick: { show: false },
      },
      yAxis: {
        type: 'value',
        minInterval: 1,
        axisLabel: {
          color: colors.muted,
          fontSize: 10,
        },
        splitLine: {
          lineStyle: { color: colors.grid },
        },
      },
      series: [
        {
          type: 'bar',
          name: model.label,
          data: model.bins.map((bin) => ({
            value: bin.count,
            itemStyle: {
              borderRadius: [3, 3, 0, 0],
              color: bin.count === maxCount ? colors.accent : colors.secondaryBar,
            },
          })),
          barMaxWidth: 28,
        },
      ],
    },
    true,
  )
}

watch(
  selectedModel,
  async () => {
    await nextTick()
    await renderChart()
  },
  { immediate: true },
)

onMounted(async () => {
  await nextTick()
  await renderChart()
})

function onMetricChange(event: Event): void {
  selectedMetric.value = String((event.target as HTMLSelectElement).value || '')
}

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
  chart?.dispose()
  chart = null
})
</script>

<template>
  <section v-if="metricOptions.length > 0" class="metricDistribution surfaceSunken">
    <header class="metricDistributionHeader">
      <div>
        <div class="panelEyebrow">Visual summary</div>
        <h4 class="panelPaneTitle">{{ title }}</h4>
      </div>
      <label class="metricPicker">
        <span>Metric</span>
        <select :value="selectedMetric" @change="onMetricChange">
          <option v-for="option in metricOptions" :key="option.key" :value="option.key">
            {{ option.label }}
          </option>
        </select>
      </label>
    </header>
    <div class="metricDistributionBody">
      <div class="metricDistributionChartFrame">
        <div
          ref="chartHost"
          class="metricDistributionChart"
          :class="{ isReady: chartReady && !chartLoadFailed }"
          role="img"
          :aria-label="selectedSummary"
          :aria-hidden="chartReady && !chartLoadFailed ? undefined : 'true'"
        ></div>
        <div
          v-show="!chartReady || chartLoadFailed"
          class="metricDistributionFallback"
          role="img"
          :aria-label="`${selectedSummary}. Histogram fallback.`"
        >
          <div v-for="bin in fallbackBins" :key="bin.label" class="metricDistributionFallbackRow">
            <span class="metricDistributionFallbackLabel">{{ bin.label }}</span>
            <span class="metricDistributionFallbackTrack" aria-hidden="true">
              <span class="metricDistributionFallbackFill" :style="{ width: bin.width }"></span>
            </span>
            <span class="metricDistributionFallbackCount">{{ bin.count }}</span>
          </div>
        </div>
      </div>
      <div class="metricDistributionStats mono" role="status" aria-live="polite">{{ selectedSummary }}</div>
    </div>
  </section>
</template>

<style scoped>
.metricDistribution {
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-sunken) 92%, var(--panel));
  padding: 0.58rem 0.62rem;
  display: grid;
  gap: 0.5rem;
}

.metricDistributionHeader {
  min-width: 0;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.metricDistributionHeader .panelPaneTitle {
  margin: 0.08rem 0 0;
  font-size: 0.86rem;
}

.metricPicker {
  min-width: min(12rem, 42%);
  display: grid;
  gap: 0.18rem;
  color: var(--muted-soft);
  font-size: 0.68rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.metricPicker select {
  min-width: 0;
  letter-spacing: 0;
  text-transform: none;
  font-size: 0.76rem;
}

.metricDistributionBody {
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(10rem, 0.38fr);
  align-items: center;
  gap: 0.6rem;
}

.metricDistributionChart {
  min-width: 0;
  height: 128px;
}

.metricDistributionChartFrame {
  min-width: 0;
  height: 128px;
  position: relative;
}

.metricDistributionFallback {
  min-width: 0;
  height: 128px;
  position: absolute;
  inset: 0;
  display: grid;
  gap: 0.18rem;
  align-content: center;
}

.metricDistributionChart {
  position: absolute;
  inset: 0;
  opacity: 0;
  pointer-events: none;
}

.metricDistributionChart.isReady {
  opacity: 1;
  pointer-events: auto;
}

.metricDistributionFallbackRow {
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(3.5rem, 0.35fr) minmax(0, 1fr) 1.75rem;
  align-items: center;
  gap: 0.32rem;
  font-size: 0.64rem;
  color: var(--muted-strong);
}

.metricDistributionFallbackLabel,
.metricDistributionFallbackCount {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.metricDistributionFallbackTrack {
  min-width: 0;
  height: 0.42rem;
  border-radius: 999px;
  background: color-mix(in oklab, var(--muted-soft) 16%, transparent);
  overflow: hidden;
}

.metricDistributionFallbackFill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: color-mix(in oklab, var(--accent) 72%, var(--surface));
}

.metricDistributionStats {
  min-width: 0;
  color: var(--muted-strong);
  font-size: 0.76rem;
  line-height: 1.45;
}

@media (max-width: 760px) {
  .metricDistributionHeader,
  .metricDistributionBody {
    grid-template-columns: 1fr;
    display: grid;
  }

  .metricPicker {
    min-width: 0;
  }
}
</style>
