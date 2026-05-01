<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import {
  refreshHeatmapFromWorkflowRun,
  registerHeatmapCanvasController,
  runHeatmapRepresentativeCandidateDetail,
  restoreHeatmapVisible,
} from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import type { HeatmapCellParams } from '../visualizations/heatmapCanvasModel'
import HeatmapCanvas from './visualizations/HeatmapCanvas.vue'
import VisualizationToolbar from './visualizations/VisualizationToolbar.vue'

interface OptionItem {
  value: string
  label: string
  disabled: boolean
}

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  heatmapMode,
  heatmapModeOptions,
  heatmapMeta,
  canHeatmapExportPng,
  heatmapCanvasModel,
  heatmapStatusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#heatmapVueShellMount')
const modeDraft = ref('objective')
const heatmapCanvasRef = ref<InstanceType<typeof HeatmapCanvas> | null>(null)

const visible = computed(() => String(resultsTab.value || '').trim() === 'heatmap')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const modeOptions = computed<OptionItem[]>(() => {
  const options = heatmapModeOptions.value as OptionItem[]
  if (options.length > 0) return options
  return [{ value: 'objective', label: 'Objective score', disabled: false }]
})

const chartAriaLabel = computed(() =>
  String(heatmapMode.value || '').trim() === 'plateau'
    ? 'Derived plateau heatmap for the selected objective. Use Top results for exact retained candidates, or click a populated heatmap cell for a representative parameter pair from that region.'
    : 'Optimization heatmap for the selected objective. Use Top results for exact retained candidates, or click a populated heatmap cell for a representative parameter pair from that region.',
)

const scaleNote = computed(() =>
  String(heatmapMode.value || '').trim() === 'plateau'
    ? 'Derived plateau scale: blue fragile, green stable. The score uses supportive 8-neighbor agreement, roughness, curvature, and bin-consistency when mean/count aggregates are available. Click opens a representative parameter pair for the selected heatmap region; Top results remains the exact retained-candidate path.'
    : 'Zero-centered scale: red negative, blue near zero, green positive. Click opens a representative parameter pair for the selected heatmap region.',
)

function cleanText(value: unknown, fallback = ''): string {
  const text = String(value ?? '').trim()
  if (!text || text === '-' || text === '—' || text === 'undefined' || text === 'NaN') return fallback
  return text
}

function formatHeatmapNumber(value: unknown, digits = 4): string {
  const next = Number(value)
  if (!Number.isFinite(next)) return ''
  return next.toFixed(Math.max(0, digits)).replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
}

const activeModeLabel = computed(() => {
  const activeValue = String(heatmapMode.value || 'objective').trim()
  return modeOptions.value.find((option) => option.value === activeValue)?.label || 'Objective score'
})

const heatmapDecisionStats = computed(() => {
  const renderModel = heatmapCanvasModel.value
  if (!renderModel) {
    return [
      { label: 'Mode', value: activeModeLabel.value },
      { label: 'Canvas', value: 'Awaiting run' },
      { label: 'Export', value: canHeatmapExportPng.value ? 'Ready' : 'Unavailable' },
    ]
  }
  const binsFast = Number(renderModel.model?.binsFast ?? 0)
  const binsSlow = Number(renderModel.model?.binsSlow ?? 0)
  const cells = binsFast > 0 && binsSlow > 0 ? `${binsFast}x${binsSlow}` : ''
  const range = `${formatHeatmapNumber(renderModel.range?.min)} to ${formatHeatmapNumber(renderModel.range?.max)}`
  return [
    { label: 'Mode', value: activeModeLabel.value },
    { label: 'Surface', value: cells },
    { label: 'Objective', value: cleanText(renderModel.displayLabel, 'Score') },
    { label: 'Range', value: range },
    { label: 'Markers', value: String(renderModel.markers?.length ?? 0) },
    { label: 'Export', value: canHeatmapExportPng.value ? 'Ready' : 'Unavailable' },
  ].filter((item) => cleanText(item.value).length > 0)
})

async function ensureVisibleHeatmap(): Promise<void> {
  await nextTick()
  if (!restoreHeatmapVisible()) {
    refreshHeatmapFromWorkflowRun()
  }
  const stack = document.getElementById('heatmapCanvasStackVue')
  const legend = document.getElementById('heatmapLegendVue')
  if (!stack || typeof stack.scrollIntoView !== 'function') return
  const stackRect = stack.getBoundingClientRect()
  const legendRect = legend?.getBoundingClientRect()
  const belowViewport =
    stackRect.top < 0 ||
    (legendRect ? legendRect.bottom > window.innerHeight : stackRect.bottom > window.innerHeight)
  if (belowViewport) {
    stack.scrollIntoView({ block: 'start', inline: 'nearest' })
  }
}

watch(
  () => String(heatmapMode.value || 'objective'),
  (nextMode) => {
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
  },
  { immediate: true },
)

watch([visible, mountReady], async ([nextVisible, nextReady]) => {
  if (!nextVisible || !nextReady) return
  await ensureVisibleHeatmap()
}, { immediate: true })

function setHeatmapMode(nextMode: string): void {
  analysisStore.setSelectField('heatmapMode', nextMode)
}

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}

function registerCanvasController(): void {
  registerHeatmapCanvasController({
    exportPng: (fileName: string) => heatmapCanvasRef.value?.exportPng(fileName) ?? false,
  })
}

function requestHeatmapCandidateDetail(params: HeatmapCellParams): void {
  runHeatmapRepresentativeCandidateDetail(params)
}

onMounted(() => {
  void nextTick(registerCanvasController)
})

onBeforeUnmount(() => {
  registerHeatmapCanvasController(null)
})
</script>

<template>
  <Teleport v-if="mountReady" to="#heatmapVueShellMount">
    <section class="heatmapPanelVueRoot heatmapVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Parameter surface</div>
          <h2 class="panelTitle">Heatmap</h2>
          <div class="panelSubtitle">
            Inspect parameter regions, then open representative candidate detail from populated cells.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <section class="pane">
        <div class="heatmapDecisionHeader">
          <div>
            <div class="panelEyebrow">Current view</div>
            <h3 class="panelPaneTitle">{{ activeModeLabel }}</h3>
          </div>
          <div class="panelMuted">
            Top Results remains the exact retained-candidate path; heatmap cells open representative parameter-region detail.
          </div>
        </div>
        <div class="heatmapMetricStrip" aria-label="Heatmap state">
          <div
            v-for="stat in heatmapDecisionStats"
            :key="`heatmap_stat_${stat.label}_${stat.value}`"
            class="heatmapMetric"
          >
            <span>{{ stat.label }}</span>
            <strong class="mono panelNumeric">{{ stat.value }}</strong>
          </div>
        </div>
        <VisualizationToolbar label="Heatmap display controls">
          <div class="heatmapControls">
            <div>
              <label for="vueHeatmapMode">Display mode</label>
              <select
                id="vueHeatmapMode"
                :value="modeDraft"
                @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'objective'); setHeatmapMode(modeDraft)"
              >
                <option
                  v-for="option in modeOptions"
                  :key="`heatmap_mode_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
          </div>
          <template #actions>
            <button type="button" :disabled="!canHeatmapExportPng" @click="analysisStore.heatmapExportPng()">Export PNG</button>
          </template>
        </VisualizationToolbar>
        <div class="metaLine mono" role="status" aria-live="polite">{{ heatmapMeta }}</div>
        <details class="heatmapScaleDetails">
          <summary>Scale and selection semantics</summary>
          <div class="metaLine mono">{{ scaleNote }}</div>
        </details>
      </section>

      <section class="pane">
        <HeatmapCanvas
          ref="heatmapCanvasRef"
          :ariaLabel="chartAriaLabel"
          :render-model="heatmapCanvasModel"
          :selected-summary="heatmapStatusLine"
          @review-top-results="reviewTopResults"
          @request-candidate-detail="requestHeatmapCandidateDetail"
        />
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ heatmapStatusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.heatmapPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.pane {
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-working) 95%, var(--panel-footer));
  padding: var(--density-pane-y) var(--density-pane-x);
  min-width: 0;
  display: grid;
  gap: 0.65rem;
}

.heatmapDecisionHeader {
  display: grid;
  grid-template-columns: minmax(0, auto) minmax(18rem, 1fr);
  gap: 0.75rem;
  align-items: start;
}

.heatmapMetricStrip {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 0.5rem;
}

.heatmapMetric {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-sunken) 72%, transparent);
  padding: 0.45rem 0.5rem;
  display: grid;
  gap: 0.12rem;
}

.heatmapMetric span {
  min-width: 0;
  font-size: 0.64rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.heatmapMetric strong {
  min-width: 0;
  font-size: 0.8rem;
  line-height: 1.15;
  overflow: hidden;
  text-overflow: ellipsis;
}

.heatmapControls {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: minmax(0, 15rem);
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.chartAssist {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.35rem;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.metaLine + .metaLine {
  margin-top: 0.3rem;
}

.heatmapScaleDetails {
  border: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-inline) 76%, transparent);
  padding: 0.48rem 0.58rem;
}

.heatmapScaleDetails summary {
  cursor: pointer;
  font-size: 0.76rem;
  font-weight: 700;
  color: var(--muted-strong);
}

.heatmapScaleDetails .metaLine {
  margin-top: 0.45rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1100px) {
  .heatmapMetricStrip {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .head,
  .heatmapDecisionHeader {
    grid-template-columns: 1fr;
  }

  .head {
    display: grid;
  }

  .heatmapMetricStrip {
    grid-template-columns: 1fr 1fr;
  }
}

</style>




