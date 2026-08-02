<script setup lang="ts">
import { ChevronLeft, ChevronRight, Crosshair, Maximize2 } from '@lucide/vue'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  lightweightChartLegendColor,
  resolveLightweightChartTheme,
  type LightweightChartLegendItem,
  type LightweightChartTheme,
} from '../../visualizations/chartTheme'
import { namedChartSeriesColor } from '../../visualizations/lightweightChartRuntime'

type ChartStateKind = 'idle' | 'loading' | 'ready' | 'warning' | 'stale' | 'provenance' | 'error'

const SUMMARY_ITEM_LIMIT = 5
const LEGEND_ITEM_LIMIT = 7
const NAMED_SERIES_COLOR_TOKEN = /^named-series:(\d+)$/u
const NAMED_HISTOGRAM_COLOR_TOKEN = /^named-histogram:(\d+)$/u
const INSPECTOR_META_PATTERN = / · inspect\[(pointer|keyboard)\]: (.+)$/u

const props = withDefaults(
  defineProps<{
    chartId: string
    metaId: string
    title: string
    metaText: string
    summaryItems?: readonly string[]
    legendItems?: readonly LightweightChartLegendItem[]
    stateLabel?: string
    stateKind?: ChartStateKind | ''
    renderReady?: boolean
    viewLabel?: string
    navigation?: boolean
    qaPrefix?: string
    advisory?: string
    frameAriaLabel?: string
    hostAriaLabel?: string
    describedBy?: string
    paneCount?: number
    nativeStrategy?: string
    nativeSeriesVisible?: number
    nativeSeriesTotal?: number
    pointInspection?: boolean
  }>(),
  {
    summaryItems: () => [],
    legendItems: () => [],
    stateLabel: '',
    stateKind: '',
    renderReady: false,
    viewLabel: '',
    navigation: false,
    qaPrefix: '',
    advisory: '',
    frameAriaLabel: '',
    hostAriaLabel: 'Price chart',
    describedBy: '',
    paneCount: 1,
    nativeStrategy: '',
    pointInspection: false,
  },
)

const emit = defineEmits<{
  'host-ready': [element: HTMLElement | null]
  'fit-content': []
  'pan-left': []
  'pan-right': []
}>()

const chartHost = ref<HTMLElement | null>(null)
const legendTheme = ref<LightweightChartTheme>(resolveLightweightChartTheme())
let legendThemeObserver: MutationObserver | null = null

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

function legendItemPriority(item: LightweightChartLegendItem): number {
  const normalized = item.label.toLowerCase()
  if (normalized === 'entries' || normalized === 'exits') return 0
  if (['fast ma', 'slow ma', 'halftrend', 'ott', 'mac-z'].includes(normalized)) return 1
  if (normalized === 'candles') return 20
  return 10
}

function legendItemColor(item: LightweightChartLegendItem): string {
  if (item.paletteKey) return lightweightChartLegendColor(legendTheme.value, item.paletteKey)
  if (item.color && NAMED_HISTOGRAM_COLOR_TOKEN.test(item.color)) {
    return lightweightChartLegendColor(legendTheme.value, 'halftrend')
  }
  const namedColor = item.color?.match(NAMED_SERIES_COLOR_TOKEN)
  if (namedColor) return namedChartSeriesColor(legendTheme.value, Number(namedColor[1]))
  return item.color || 'transparent'
}

function legendItemSeriesToken(item: LightweightChartLegendItem): string | undefined {
  if (item.paletteKey) return item.paletteKey
  if (
    item.color &&
    (
      NAMED_SERIES_COLOR_TOKEN.test(item.color) ||
      NAMED_HISTOGRAM_COLOR_TOKEN.test(item.color)
    )
  ) {
    return item.color
  }
  return undefined
}

const visibleSummaryItems = computed(() => selectCompactItems(props.summaryItems, SUMMARY_ITEM_LIMIT, summaryItemPriority))
const hiddenSummaryCount = computed(() => Math.max(0, props.summaryItems.length - visibleSummaryItems.value.length))
const visibleLegendItems = computed(() => selectCompactItems(props.legendItems, LEGEND_ITEM_LIMIT, legendItemPriority))
const hiddenLegendCount = computed(() => Math.max(0, props.legendItems.length - visibleLegendItems.value.length))
const resolvedStateKind = computed<ChartStateKind>(() =>
  props.stateKind || (props.renderReady ? 'ready' : 'idle'),
)
const chartIsLoading = computed(() => resolvedStateKind.value === 'loading')
const chartHasError = computed(() => resolvedStateKind.value === 'error')
const navigationEnabled = computed(() =>
  props.renderReady &&
  !chartIsLoading.value &&
  !chartHasError.value &&
  resolvedStateKind.value !== 'idle',
)
const chartDescriptionId = computed(() => `${props.metaId}Description`)
const chartLifecycleStatusId = computed(() => `${props.metaId}LifecycleStatus`)
const chartKeyboardHintId = computed(() => `${props.metaId}KeyboardHint`)
const pointInspectionEnabled = computed(() => props.pointInspection)
const chartDescriptionIds = computed(() => {
  const ids = [
    chartDescriptionId.value,
    ...(pointInspectionEnabled.value ? [chartKeyboardHintId.value] : []),
    ...props.describedBy.split(/\s+/u),
  ]
    .map((id) => id.trim())
    .filter(Boolean)
  return [...new Set(ids)].join(' ')
})
const chartDescription = computed(() => {
  const parts = [
    props.stateLabel,
    props.metaText,
    props.summaryItems.length ? `Evidence: ${props.summaryItems.join(', ')}.` : '',
    props.legendItems.length
      ? `Series: ${props.legendItems.map((item) => `${item.label}${item.value ? ` ${item.value}` : ''}`).join(', ')}.`
      : '',
    props.viewLabel ? `Visible logical range ${props.viewLabel}.` : '',
  ]
  return parts.map((part) => part.trim()).filter(Boolean).join(' ')
})
const lifecycleAnnouncement = computed(() => {
  if (resolvedStateKind.value === 'ready' || resolvedStateKind.value === 'idle') {
    return props.stateLabel
  }
  return [props.stateLabel, props.metaText]
    .map((part) => part.trim())
    .filter(Boolean)
    .join('. ')
})
const normalizedPaneCount = computed(() =>
  Math.max(1, Math.floor(Number(props.paneCount) || 1)),
)
const chartHostStyle = computed<Record<string, string>>(() => {
  if (normalizedPaneCount.value <= 1) return {}
  if (normalizedPaneCount.value === 2) {
    return { '--price-chart-pane-height': 'clamp(420px, 52vh, 600px)' }
  }
  if (normalizedPaneCount.value === 3) {
    return { '--price-chart-pane-height': 'clamp(480px, 60vh, 680px)' }
  }
  return { '--price-chart-pane-height': 'clamp(520px, 66vh, 760px)' }
})
const pointInspector = computed(() => {
  const match = INSPECTOR_META_PATTERN.exec(props.metaText)
  if (!match) return null
  const source = match[1] as 'pointer' | 'keyboard'
  const parts = (match[2] ?? '')
    .split(' · ')
    .map((part) => part.trim())
    .filter(Boolean)
  if (!parts.length) return null
  return {
    source,
    time: parts[0] ?? '',
    values: parts.slice(1),
  }
})
const restingMetaText = computed(() =>
  props.metaText.replace(INSPECTOR_META_PATTERN, ''),
)
const keyboardAnnouncement = computed(() =>
  pointInspector.value?.source === 'keyboard'
    ? `Chart point ${pointInspector.value.time}. ${pointInspector.value.values.join('. ')}.`
    : '',
)

onMounted(() => {
  legendTheme.value = resolveLightweightChartTheme()
  if (typeof MutationObserver !== 'undefined') {
    try {
      legendThemeObserver = new MutationObserver(() => {
        legendTheme.value = resolveLightweightChartTheme()
      })
      legendThemeObserver.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ['data-theme'],
      })
    } catch {
      legendThemeObserver = null
    }
  }
  emit('host-ready', chartHost.value)
})

onBeforeUnmount(() => {
  legendThemeObserver?.disconnect()
  legendThemeObserver = null
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
        :class="resolvedStateKind"
        :data-chart-state="resolvedStateKind"
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
          :disabled="!navigationEnabled"
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
          :disabled="!navigationEnabled"
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
          :disabled="!navigationEnabled"
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
      :aria-describedby="chartDescriptionIds"
      :aria-busy="chartIsLoading"
      :data-chart-ready="navigationEnabled ? 'true' : 'false'"
      :data-chart-state="resolvedStateKind"
      :data-chart-meta="metaText"
      :data-chart-view="viewLabel"
      :data-native-strategy="nativeStrategy || undefined"
      :data-native-series-visible="nativeStrategy ? nativeSeriesVisible : undefined"
      :data-native-series-total="nativeStrategy ? nativeSeriesTotal : undefined"
      :data-native-pane-count="nativeStrategy ? normalizedPaneCount : undefined"
    >
      <div
        v-if="pointInspector"
        class="chartPointInspector"
        :data-inspector-source="pointInspector.source"
        aria-hidden="true"
      >
        <div class="chartPointInspectorHeader">
          <Crosshair :size="13" aria-hidden="true" />
          <span>{{ pointInspector.source === 'keyboard' ? 'Keyboard point' : 'Crosshair point' }}</span>
          <strong class="mono">{{ pointInspector.time }}</strong>
        </div>
        <div class="chartPointInspectorValues">
          <span
            v-for="value in pointInspector.values"
            :key="value"
            class="chartPointInspectorValue mono"
          >
            {{ value }}
          </span>
        </div>
      </div>
      <div v-if="legendItems.length" class="chartLegendOverlay" aria-label="Chart legend">
        <span
        v-for="item in visibleLegendItems"
        :key="`chart_legend_${item.label}_${item.value || ''}`"
        class="chartLegendItem"
      >
        <span
          class="chartLegendSwatch"
          :style="{ background: legendItemColor(item) }"
          :data-chart-series="legendItemSeriesToken(item)"
          aria-hidden="true"
        ></span>
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
        :style="chartHostStyle"
        :data-chart-pane-count="normalizedPaneCount"
        :data-chart-multi-pane="normalizedPaneCount > 1 ? 'true' : undefined"
        role="group"
        aria-roledescription="price chart"
        :aria-label="hostAriaLabel"
        :aria-describedby="chartDescriptionIds"
        :aria-busy="chartIsLoading"
        :tabindex="pointInspectionEnabled && navigationEnabled ? 0 : undefined"
        :aria-keyshortcuts="pointInspectionEnabled ? 'ArrowLeft ArrowRight Home End Escape' : undefined"
      ></div>
      <div
        v-if="pointInspectionEnabled"
        :id="chartKeyboardHintId"
        class="chartInteractionHint mono"
      >
        <span aria-hidden="true">← →</span> inspect
        <span aria-hidden="true">·</span> Home/End jump
        <span aria-hidden="true">·</span> Esc clear
      </div>
    </div>
    <div :id="chartDescriptionId" class="chartAccessibleDescription">
      {{ chartDescription }}
    </div>
    <div
      :id="chartLifecycleStatusId"
      class="chartAccessibleDescription"
      :role="chartHasError ? 'alert' : 'status'"
      :aria-live="chartHasError ? 'assertive' : 'polite'"
      aria-atomic="true"
    >
      {{ lifecycleAnnouncement }}
    </div>
    <div
      class="chartAccessibleDescription"
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      {{ keyboardAnnouncement }}
    </div>
    <div
      :id="metaId"
      class="panelStatus mono panelNumeric chartMeta"
      aria-live="off"
    >
      {{ viewLabel ? `${restingMetaText} view=${viewLabel}` : restingMetaText }}
    </div>
  </div>
</template>

<style scoped>
.priceChartPanel {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  min-width: 0;
  max-width: 100%;
  gap: 0.42rem;
}

.tvChartFrame,
.tvChartHost {
  min-width: 0;
  max-width: 100%;
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

.chartStateBadge.warning,
.chartStateBadge.stale,
.chartStateBadge.provenance {
  color: var(--warning-ink);
  background: color-mix(in oklab, var(--surface-warning) 62%, var(--surface-working));
  border-color: color-mix(in oklab, var(--warning) 48%, var(--border));
}

.chartStateBadge.error {
  color: var(--danger-ink);
  background: color-mix(in oklab, var(--surface-danger) 58%, var(--surface-working));
  border-color: color-mix(in oklab, var(--danger) 48%, var(--border));
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

.chartPointInspector {
  position: absolute;
  z-index: 14;
  top: 0.68rem;
  left: 0.68rem;
  max-width: min(38rem, calc(100% - 1.36rem));
  display: grid;
  gap: 0.34rem;
  border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border-strong));
  border-radius: 10px;
  background:
    linear-gradient(
      135deg,
      color-mix(in oklab, var(--surface-overlay) 95%, var(--accent) 5%),
      color-mix(in oklab, var(--surface-overlay) 88%, transparent)
    );
  box-shadow: var(--shadow-md);
  padding: 0.42rem 0.5rem;
  pointer-events: none;
  backdrop-filter: blur(12px);
}

.chartPointInspector[data-inspector-source='keyboard'] {
  border-color: color-mix(in oklab, var(--accent) 58%, var(--border-strong));
  box-shadow:
    var(--shadow-md),
    0 0 0 1px color-mix(in oklab, var(--accent) 18%, transparent);
}

.chartPointInspectorHeader {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.32rem;
  color: var(--muted-strong);
  font-size: 0.66rem;
  line-height: 1.2;
}

.chartPointInspectorHeader strong {
  min-width: 0;
  color: var(--text);
  font-size: 0.69rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chartPointInspectorValues {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.24rem;
}

.chartPointInspectorValue {
  border: 1px solid color-mix(in oklab, var(--border) 74%, transparent);
  border-radius: 6px;
  background: color-mix(in oklab, var(--surface-inline) 82%, transparent);
  color: var(--text);
  padding: 0.12rem 0.28rem;
  font-size: 0.66rem;
  line-height: 1.15;
}

.chartInteractionHint {
  position: absolute;
  z-index: 13;
  right: 0.58rem;
  bottom: 0.46rem;
  border: 1px solid color-mix(in oklab, var(--border) 65%, transparent);
  border-radius: 7px;
  background: color-mix(in oklab, var(--surface-overlay) 80%, transparent);
  color: var(--muted-strong);
  padding: 0.14rem 0.32rem;
  font-size: 0.61rem;
  line-height: 1.15;
  pointer-events: none;
  backdrop-filter: blur(8px);
}

.chartLegendItem {
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.24rem;
  border: 1px solid color-mix(in oklab, var(--border-strong) 72%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-overlay) 88%, transparent);
  color: var(--text);
  box-shadow: var(--shadow-sm);
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
  color: var(--text);
  font-weight: 760;
}

.chartLegendSwatch {
  width: 0.56rem;
  height: 0.56rem;
  flex: 0 0 auto;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--text) 42%, transparent);
}

.chartMeta {
  margin-top: 0;
}

.tvChartHost[data-chart-multi-pane='true'] {
  height: var(--price-chart-pane-height);
}

.chartAccessibleDescription {
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

.chartSurface:focus-within {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.mono {
  font-family: var(--mono);
}
</style>
