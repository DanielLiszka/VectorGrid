<script setup lang="ts">
import { ChevronLeft, ChevronRight, Maximize2 } from '@lucide/vue'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

interface ChartLegendItem {
  label: string
  color: string
  value?: string
}

const SUMMARY_ITEM_LIMIT = 5
const LEGEND_ITEM_LIMIT = 7

const props = withDefaults(
  defineProps<{
    chartId: string
    metaId: string
    title: string
    metaText: string
    summaryItems?: readonly string[]
    legendItems?: readonly ChartLegendItem[]
    stateLabel?: string
    renderReady?: boolean
    viewLabel?: string
    navigation?: boolean
    qaPrefix?: string
    advisory?: string
    frameAriaLabel?: string
    hostAriaLabel?: string
    describedBy?: string
  }>(),
  {
    summaryItems: () => [],
    legendItems: () => [],
    stateLabel: '',
    renderReady: false,
    viewLabel: '',
    navigation: false,
    qaPrefix: '',
    advisory: '',
    frameAriaLabel: '',
    hostAriaLabel: 'Price chart',
    describedBy: '',
  },
)

const emit = defineEmits<{
  'host-ready': [element: HTMLElement | null]
  'fit-content': []
  'pan-left': []
  'pan-right': []
}>()

const chartHost = ref<HTMLElement | null>(null)

function selectCompactItems<T>(
  items: readonly T[],
  limit: number,
  priorityForItem: (item: T) => number,
): T[] {
  if (items.length <= limit) return [...items]
  return items
    .map((item, index) => ({ item, index, priority: priorityForItem(item) }))
    .sort((a, b) => a.priority - b.priority || a.index - b.index)
    .slice(0, limit)
    .sort((a, b) => a.index - b.index)
    .map(({ item }) => item)
}

function summaryItemPriority(item: string): number {
  const normalized = item.toLowerCase()
  if (normalized.includes('price bars')) return 0
  if (normalized.includes('trades')) return 1
  if (normalized.includes('ma overlays')) return 2
  if (normalized.includes('ott')) return 3
  if (normalized.includes('mac-z')) return 4
  return 10
}

function legendItemPriority(item: ChartLegendItem): number {
  const normalized = item.label.toLowerCase()
  if (normalized === 'entries' || normalized === 'exits') return 0
  if (['fast ma', 'slow ma', 'halftrend', 'ott', 'mac-z'].includes(normalized)) return 1
  if (normalized === 'candles') return 20
  return 10
}

const visibleSummaryItems = computed(() => selectCompactItems(props.summaryItems, SUMMARY_ITEM_LIMIT, summaryItemPriority))
const hiddenSummaryCount = computed(() => Math.max(0, props.summaryItems.length - visibleSummaryItems.value.length))
const visibleLegendItems = computed(() => selectCompactItems(props.legendItems, LEGEND_ITEM_LIMIT, legendItemPriority))
const hiddenLegendCount = computed(() => Math.max(0, props.legendItems.length - visibleLegendItems.value.length))

onMounted(() => {
  emit('host-ready', chartHost.value)
})

onBeforeUnmount(() => {
  emit('host-ready', null)
})
</script>

<template>
  <div class="priceChartPanel">
    <div class="chartPanelHeader">
      <div class="panelMuted chartTitle">{{ title }}</div>
      <span
        v-if="stateLabel"
        class="chartStateBadge mono"
        :class="{ ready: renderReady }"
        :data-chart-state="renderReady ? 'ready' : 'pending'"
      >
        {{ stateLabel }}
      </span>
      <div v-if="navigation" class="chartNavControls" aria-label="Chart navigation">
        <button
          type="button"
          class="chartIconButton"
          :data-qa-action="qaPrefix ? `${qaPrefix}.panLeft` : undefined"
          aria-label="Pan chart left"
          title="Pan chart left"
          :disabled="!renderReady"
          @click="emit('pan-left')"
        >
          <ChevronLeft :size="14" aria-hidden="true" />
        </button>
        <button
          type="button"
          class="chartIconButton"
          :data-qa-action="qaPrefix ? `${qaPrefix}.fitContent` : undefined"
          aria-label="Fit chart content"
          title="Fit chart content"
          :disabled="!renderReady"
          @click="emit('fit-content')"
        >
          <Maximize2 :size="13" aria-hidden="true" />
        </button>
        <button
          type="button"
          class="chartIconButton"
          :data-qa-action="qaPrefix ? `${qaPrefix}.panRight` : undefined"
          aria-label="Pan chart right"
          title="Pan chart right"
          :disabled="!renderReady"
          @click="emit('pan-right')"
        >
          <ChevronRight :size="14" aria-hidden="true" />
        </button>
      </div>
    </div>
    <div v-if="summaryItems.length" class="chartEvidenceChips" aria-label="Chart evidence">
      <span
        v-for="item in visibleSummaryItems"
        :key="`chart_evidence_${item}`"
        class="chartEvidenceChip mono"
      >
        {{ item }}
      </span>
      <span v-if="hiddenSummaryCount" class="chartEvidenceChip chartEvidenceMore mono">
        +{{ hiddenSummaryCount }}
      </span>
    </div>
    <div v-if="advisory" class="panelAdvisory">{{ advisory }}</div>
    <div
      class="tvChartFrame chartSurface"
      role="group"
      :aria-label="frameAriaLabel || hostAriaLabel"
      :aria-describedby="describedBy || undefined"
      :data-chart-ready="renderReady ? 'true' : 'false'"
      :data-chart-meta="metaText"
      :data-chart-view="viewLabel"
    >
      <div v-if="legendItems.length" class="chartLegendOverlay" aria-label="Chart legend">
        <span
        v-for="item in visibleLegendItems"
        :key="`chart_legend_${item.label}_${item.value || ''}`"
        class="chartLegendItem"
      >
        <span class="chartLegendSwatch" :style="{ background: item.color }" aria-hidden="true"></span>
        <span>{{ item.label }}</span>
        <strong v-if="item.value" class="mono">{{ item.value }}</strong>
      </span>
      <span v-if="hiddenLegendCount" class="chartLegendItem chartLegendMore">
        +{{ hiddenLegendCount }}
      </span>
      </div>
      <div
        :id="chartId"
        ref="chartHost"
        class="tvChartHost"
        role="img"
        :aria-label="hostAriaLabel"
      ></div>
    </div>
    <div :id="metaId" class="panelStatus mono panelNumeric chartMeta" role="status" aria-live="polite">
      {{ viewLabel ? `${metaText} view=${viewLabel}` : metaText }}
    </div>
  </div>
</template>

<style scoped>
.priceChartPanel {
  display: grid;
  gap: 0.42rem;
}

.chartPanelHeader {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  min-width: 0;
}

.chartTitle {
  margin-bottom: 0;
  min-width: 0;
  overflow-wrap: anywhere;
}

.chartStateBadge {
  flex: 0 0 auto;
  border: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-inline) 78%, transparent);
  color: var(--muted-strong);
  padding: 0.18rem 0.4rem;
  font-size: 0.68rem;
  line-height: 1.2;
}

.chartStateBadge.ready {
  color: var(--ok-ink);
  background: color-mix(in oklab, var(--surface-success) 58%, var(--surface-working));
  border-color: color-mix(in oklab, var(--ok) 48%, var(--border));
}

.chartNavControls {
  display: inline-flex;
  align-items: center;
  gap: 0.18rem;
  flex: 0 0 auto;
}

.chartIconButton {
  width: 1.74rem;
  height: 1.74rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-inline) 82%, transparent);
  color: var(--muted-strong);
  padding: 0;
}

.chartIconButton:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.chartEvidenceChips {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.28rem;
  min-width: 0;
}

.chartEvidenceChip {
  max-width: 22ch;
  border: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 90%, var(--panel-footer));
  color: var(--muted-strong);
  padding: 0.16rem 0.38rem;
  font-size: 0.68rem;
  line-height: 1.18;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chartEvidenceMore {
  flex: 0 0 auto;
  background: color-mix(in oklab, var(--surface-inline) 86%, var(--panel-footer));
  color: var(--text);
  font-weight: 760;
}

.chartLegendOverlay {
  position: absolute;
  z-index: 12;
  top: 0.72rem;
  right: 0.72rem;
  max-width: min(32rem, calc(100% - 1.44rem));
  max-height: 4.65rem;
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.3rem;
  overflow: hidden;
  pointer-events: none;
}

.chartLegendItem {
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.24rem;
  border: 1px solid rgba(148, 163, 184, 0.34);
  border-radius: 8px;
  background: rgba(2, 6, 23, 0.74);
  color: rgba(226, 232, 240, 0.95);
  box-shadow: 0 4px 12px rgba(2, 6, 23, 0.18);
  padding: 0.16rem 0.34rem;
  font-size: 0.66rem;
  line-height: 1.15;
  backdrop-filter: blur(8px);
}

.chartLegendMore {
  font-weight: 760;
  letter-spacing: 0;
}

.chartLegendItem strong {
  color: rgba(248, 250, 252, 0.98);
  font-weight: 760;
}

.chartLegendSwatch {
  width: 0.56rem;
  height: 0.56rem;
  flex: 0 0 auto;
  border-radius: 999px;
  border: 1px solid rgba(248, 250, 252, 0.48);
}

.chartMeta {
  margin-top: 0;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.mono {
  font-family: var(--mono);
}
</style>
