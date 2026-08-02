<script setup lang="ts">
import { computed, ref, watch } from 'vue'
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

const selectedMetric = ref('')

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
const histogramBins = computed(() => {
  const model = selectedModel.value
  if (!model) return []
  const maxCount = model.bins.reduce((max, bin) => Math.max(max, bin.count), 0)
  return model.bins.map((bin) => ({
    ...bin,
    heightPercent: maxCount > 0 && bin.count > 0
      ? Math.max(3, (bin.count / maxCount) * 100)
      : 0,
    isPeak: bin.count > 0 && bin.count === maxCount,
    title: `${bin.label}: ${bin.count} ${bin.count === 1 ? 'candidate' : 'candidates'}`,
  }))
})
const histogramMaxCount = computed(() =>
  histogramBins.value.reduce((max, bin) => Math.max(max, bin.count), 0),
)
const histogramMidCount = computed<number | null>(() => {
  const max = histogramMaxCount.value
  return max > 1 ? Math.ceil(max / 2) : null
})
const histogramAxisLabels = computed(() => {
  const bins = histogramBins.value
  if (bins.length <= 1) return bins.map((bin) => bin.label)
  if (bins.length === 2) return bins.map((bin) => bin.label)
  const middle = bins[Math.floor((bins.length - 1) / 2)]
  return [bins[0]?.label, middle?.label, bins[bins.length - 1]?.label].filter(
    (label): label is string => Boolean(label),
  )
})
const histogramAriaLabel = computed(() => {
  if (!selectedModel.value) return selectedSummary.value
  const bins = histogramBins.value
    .map((bin) => bin.title)
    .join('; ')
  return `${selectedSummary.value}. Histogram bins: ${bins}.`
})

function formatMetric(value: number): string {
  if (!Number.isFinite(value)) return '0'
  const abs = Math.abs(value)
  if (abs >= 100) return value.toFixed(0)
  if (abs >= 10) return value.toFixed(1)
  return value.toFixed(3).replace(/\.?0+$/, '')
}

watch(
  () => [metricOptions.value, props.preferredKeys, selectedMetric.value] as const,
  ([options, preferredKeys, current]) => {
    const next = chooseMetricDistributionKey(options, preferredKeys, current)
    if (selectedMetric.value !== next) selectedMetric.value = next
  },
  { immediate: true },
)

function onMetricChange(event: Event): void {
  selectedMetric.value = String((event.target as HTMLSelectElement).value || '')
}
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
          class="metricDistributionHistogram"
          role="img"
          :aria-label="histogramAriaLabel"
          :style="{ '--metric-bin-count': histogramBins.length }"
        >
          <div class="metricDistributionYAxis" aria-hidden="true">
            <span>{{ histogramMaxCount }}</span>
            <span>{{ histogramMidCount ?? '' }}</span>
            <span>0</span>
          </div>
          <div class="metricDistributionPlot" aria-hidden="true">
            <span
              v-for="bin in histogramBins"
              :key="bin.label"
              class="metricDistributionBarSlot"
              :title="bin.title"
            >
              <span
                class="metricDistributionBar"
                :class="{ isPeak: bin.isPeak, hasValue: bin.count > 0 }"
                :data-bin-label="bin.label"
                :data-bin-count="bin.count"
                :style="{ height: `${bin.heightPercent}%` }"
              ></span>
            </span>
          </div>
          <div
            class="metricDistributionXAxis"
            :class="{ isSingle: histogramAxisLabels.length === 1 }"
            aria-hidden="true"
          >
            <span v-for="label in histogramAxisLabels" :key="label" :title="label">
              {{ label }}
            </span>
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

.metricDistributionChartFrame {
  min-width: 0;
  height: 128px;
  position: relative;
}

.metricDistributionHistogram {
  --metric-grid-line: color-mix(in oklab, var(--muted-soft) 23%, transparent);

  min-width: 0;
  height: 100%;
  display: grid;
  grid-template-columns: 1.75rem minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr) 1.35rem;
  column-gap: 0.32rem;
  font-variant-numeric: tabular-nums;
}

.metricDistributionYAxis {
  grid-column: 1;
  grid-row: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: flex-end;
  color: var(--muted-soft);
  font-size: 0.58rem;
  line-height: 1;
  padding-block: 0.05rem 0.1rem;
}

.metricDistributionPlot {
  grid-column: 2;
  grid-row: 1;
  min-width: 0;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(var(--metric-bin-count), minmax(2px, 1fr));
  align-items: end;
  gap: clamp(2px, 0.55vw, 7px);
  border-bottom: 1px solid var(--metric-grid-line);
  background:
    linear-gradient(
      to bottom,
      var(--metric-grid-line) 0,
      var(--metric-grid-line) 1px,
      transparent 1px
    ),
    linear-gradient(
      to bottom,
      transparent calc(50% - 0.5px),
      var(--metric-grid-line) calc(50% - 0.5px),
      var(--metric-grid-line) calc(50% + 0.5px),
      transparent calc(50% + 0.5px)
    );
}

.metricDistributionBarSlot {
  min-width: 0;
  height: 100%;
  display: flex;
  align-items: flex-end;
  justify-content: center;
}

.metricDistributionBar {
  width: min(100%, 28px);
  height: 0;
  border-radius: 3px 3px 0 0;
  background: color-mix(in oklab, var(--accent-soft) 78%, var(--surface));
  transition: height var(--motion-ui, 160ms) var(--ease-standard, ease-out);
}

.metricDistributionBar.hasValue {
  min-height: 3px;
}

.metricDistributionBar.isPeak {
  background: var(--accent);
}

.metricDistributionXAxis {
  grid-column: 2;
  grid-row: 2;
  min-width: 0;
  display: flex;
  justify-content: space-between;
  align-items: end;
  gap: 0.2rem;
  color: var(--muted-soft);
  font-size: 0.58rem;
  line-height: 1.1;
}

.metricDistributionXAxis span {
  max-width: 34%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.metricDistributionXAxis span:last-child {
  text-align: right;
}

.metricDistributionXAxis.isSingle {
  justify-content: center;
  text-align: center;
}

.metricDistributionXAxis.isSingle span {
  max-width: 100%;
  text-align: center;
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

@media (prefers-reduced-motion: reduce) {
  .metricDistributionBar {
    transition: none;
  }
}
</style>
