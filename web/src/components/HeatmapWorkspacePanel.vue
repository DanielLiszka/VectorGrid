<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
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
import {
  buildHeatmapSliceCanvasRenderModel,
  heatmapSelectionLegendText,
  type HeatmapCellParams,
  type HeatmapFitMode,
  type HeatmapSliceCanvasMode,
} from '../visualizations/heatmapCanvasModel'
import HeatmapCanvas from './visualizations/HeatmapCanvas.vue'
import type HeatmapSurface3DComponent from './visualizations/HeatmapSurface3D.vue'
import VisualizationToolbar from './visualizations/VisualizationToolbar.vue'
import AppMetricStrip, { type AppMetricItem } from './ui/AppMetricStrip.vue'

const HeatmapSurface3D = defineAsyncComponent(() => import('./visualizations/HeatmapSurface3D.vue'))

interface OptionItem {
  value: string
  label: string
  disabled: boolean
  disabledReason?: string
}

const HEATMAP_3D_RETAINED_ROWS_REASON = '3D slices need retained full-result rows for at least two MA-type or parameter variants. Enable Advanced > Return all results, configure at least two variants, then rerun optimization.'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  heatmapMode,
  heatmapModeOptions,
  heatmapViewMode,
  heatmapViewModeOptions,
  heatmapMeta,
  canHeatmapExportPng,
  heatmapCanvasModel,
  heatmapStatusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#heatmapVueShellMount')
const modeDraft = ref('objective')
const viewModeDraft = ref('2d')
const heatmap2dSourceDraft = ref<'aggregate' | HeatmapSliceCanvasMode>('slice')
const heatmapFitModeDraft = ref<HeatmapFitMode>('valid')
const selectedSliceKey = ref('')
const baselineSliceKey = ref('')
const heatmapCanvasRef = ref<InstanceType<typeof HeatmapCanvas> | null>(null)
const heatmapSurface3dRef = ref<InstanceType<typeof HeatmapSurface3DComponent> | null>(null)

const visible = computed(() => String(resultsTab.value || '').trim() === 'heatmap')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))
const isPlateauMode = computed(() => String(heatmapMode.value || 'objective').trim().toLowerCase() === 'plateau')

const modeOptions = computed<OptionItem[]>(() => {
  const options = heatmapModeOptions.value as OptionItem[]
  if (options.length > 0) return options
  return [{ value: 'objective', label: 'Objective score', disabled: false }]
})

const viewModeOptions = computed<OptionItem[]>(() => {
  const options = heatmapViewModeOptions.value as OptionItem[]
  const resolved = options.length > 0 ? options : [
    { value: '2d', label: '2D heatmap', disabled: false },
    {
      value: '3d',
      label: '3D slices',
      disabled: true,
      disabledReason: HEATMAP_3D_RETAINED_ROWS_REASON,
    },
  ]
  return resolved.map((option) => {
    if (option.value !== '3d') return option
    if (isPlateauMode.value) {
      return {
        ...option,
        disabled: true,
        disabledReason: '3D slices are available only in Objective score mode.',
      }
    }
    if (option.disabled && !String(option.disabledReason || '').trim()) {
      return {
        ...option,
        disabledReason: HEATMAP_3D_RETAINED_ROWS_REASON,
      }
    }
    return option
  })
})

const threeDimensionalDisabledReason = computed(() => (
  viewModeOptions.value.find((option) => option.value === '3d' && option.disabled)?.disabledReason || ''
))

const activeViewMode = computed(() => (
  !isPlateauMode.value && String(heatmapViewMode.value || '2d').trim().toLowerCase() === '3d'
    ? '3d'
    : '2d'
))
const is3dView = computed(() => activeViewMode.value === '3d')
const canExportActiveView = computed(() => {
  if (!canHeatmapExportPng.value) return false
  if (!is3dView.value) return true
  return Boolean(heatmapSurface3dRef.value?.exportReady)
})
const sliceModel = computed(() => heatmapCanvasModel.value?.slice3d ?? null)
const sliceOptions = computed(() => sliceModel.value?.slices ?? [])
const canUse2dSlices = computed(() => !is3dView.value && !isPlateauMode.value && sliceOptions.value.length >= 2)
const requested2dSource = computed<'aggregate' | HeatmapSliceCanvasMode>(() => (
  canUse2dSlices.value ? heatmap2dSourceDraft.value : 'aggregate'
))
const requested2dRenderModel = computed(() => {
  const base = heatmapCanvasModel.value
  if (!base || requested2dSource.value === 'aggregate') return null
  return buildHeatmapSliceCanvasRenderModel(base, {
    mode: requested2dSource.value,
    selectedSliceKey: selectedSliceKey.value,
    baselineSliceKey: baselineSliceKey.value,
  })
})
const active2dSource = computed<'aggregate' | HeatmapSliceCanvasMode>(() => (
  requested2dSource.value !== 'aggregate' && requested2dRenderModel.value
    ? requested2dSource.value
    : 'aggregate'
))
const sliceFallbackNotice = computed(() => (
  requested2dSource.value !== 'aggregate' && !requested2dRenderModel.value
    ? requested2dSource.value === 'delta'
      ? 'Selected and baseline slices do not share any retained heatmap cells. Showing the run aggregate instead.'
      : 'The selected slice has no retained heatmap cells. Showing the run aggregate instead.'
    : ''
))
const effective2dRenderModel = computed(() => {
  const base = heatmapCanvasModel.value
  return active2dSource.value === 'aggregate'
    ? base
    : requested2dRenderModel.value ?? base
})
const selectionSemantics = computed(() => {
  const renderModel = effective2dRenderModel.value
  return renderModel
    ? heatmapSelectionLegendText(renderModel)
    : 'Use Top Results for exact retained candidates, or click a populated heatmap cell for a representative parameter pair from that region.'
})

const chartAriaLabel = computed(() =>
  [
    is3dView.value ? '3D sliced optimization heatmap for retained parameter variants.' : 'Optimization heatmap for the selected objective.',
    isPlateauMode.value ? 'Derived plateau display mode is active.' : '',
    !is3dView.value && active2dSource.value !== 'aggregate'
      ? 'Click a populated heatmap cell for the exact retained candidate in the selected slice.'
      : selectionSemantics.value,
  ].filter(Boolean).join(' '),
)

const scaleNote = computed(() => {
  const valueNote = isPlateauMode.value
    ? 'Derived plateau scale: blue fragile, green stable. The score uses supportive 8-neighbor agreement, roughness, curvature, and bin-consistency when mean/count aggregates are available.'
    : 'Zero-centered scale: red negative, blue near zero, green positive.'
  let viewNote = selectionSemantics.value
  if (is3dView.value) {
    viewNote = '3D slices use full retained rows to compare MA type or parameter variants; cells open exact retained candidates from the selected slice.'
  } else if (active2dSource.value === 'delta') {
    viewNote = '2D delta uses retained rows: color shows selected slice minus baseline, and clicks open exact selected-slice candidates.'
  } else if (active2dSource.value === 'slice') {
    viewNote = '2D slice mode uses retained rows from the selected MA type or parameter variant; clicks open exact selected-slice candidates.'
  }
  return `${valueNote} ${viewNote}`
})

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

const activeViewLabel = computed(() => {
  const activeValue = activeViewMode.value
  return viewModeOptions.value.find((option) => option.value === activeValue)?.label || '2D heatmap'
})

const heatmapDecisionStats = computed<AppMetricItem[]>(() => {
  const renderModel = is3dView.value ? heatmapCanvasModel.value : effective2dRenderModel.value
  if (!renderModel) {
    return [
      { key: 'view', label: 'View', value: activeViewLabel.value },
      { key: 'mode', label: 'Mode', value: activeModeLabel.value },
      { key: 'canvas', label: 'Canvas', value: 'Awaiting run' },
      { key: 'export', label: 'Export', value: canExportActiveView.value ? 'Ready' : 'Unavailable' },
    ]
  }
  const binsFast = Number(renderModel.model?.binsFast ?? 0)
  const binsSlow = Number(renderModel.model?.binsSlow ?? 0)
  const cells = binsFast > 0 && binsSlow > 0 ? `${binsFast}x${binsSlow}` : ''
  const displayRange = is3dView.value
    ? renderModel.slice3d?.range ?? renderModel.range
    : renderModel.range
  const range = `${formatHeatmapNumber(displayRange?.min)} to ${formatHeatmapNumber(displayRange?.max)}`
  const viewSpecificStats: AppMetricItem[] = is3dView.value
    ? []
    : [
        { key: 'markers', label: 'Markers', value: String(renderModel.markers?.length ?? 0) },
        { key: 'fit', label: 'Fit', value: heatmapFitModeDraft.value === 'valid' ? 'Valid domain' : 'Full grid' },
      ]
  return [
    { key: 'view', label: 'View', value: activeViewLabel.value },
    {
      key: 'source',
      label: 'Source',
      value: is3dView.value
        ? 'Selected slice'
        : active2dSource.value === 'aggregate'
          ? 'Aggregate'
          : active2dSource.value === 'delta'
            ? 'Delta'
            : 'Slice',
    },
    { key: 'mode', label: 'Mode', value: activeModeLabel.value },
    { key: 'surface', label: 'Surface', value: cells },
    {
      key: 'objective',
      label: 'Objective',
      value: cleanText(is3dView.value ? renderModel.slice3d?.scoreLabel : renderModel.displayLabel, 'Score'),
    },
    { key: 'range', label: 'Range', value: range },
    ...viewSpecificStats,
    { key: 'export', label: 'Export', value: canExportActiveView.value ? 'Ready' : 'Unavailable' },
  ].filter((item) => cleanText(item.value).length > 0)
})
const showHeatmapStatusLine = computed(() => (
  !is3dView.value || Boolean(heatmapSurface3dRef.value?.exportReady)
))

async function ensureVisibleHeatmap(): Promise<void> {
  await nextTick()
  if (is3dView.value && !document.getElementById('heatmapSurface3DHostVue') && typeof window !== 'undefined') {
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()))
    await nextTick()
  }
  if (!restoreHeatmapVisible()) {
    refreshHeatmapFromWorkflowRun()
  }
  const stack = document.getElementById(is3dView.value ? 'heatmapSurface3DHostVue' : 'heatmapCanvasStackVue')
  if (!stack || typeof stack.scrollIntoView !== 'function') return
  const stackRect = stack.getBoundingClientRect()
  const chartOffscreen = stackRect.top < 0 || stackRect.top > window.innerHeight - Math.min(120, stackRect.height)
  if (chartOffscreen) {
    stack.scrollIntoView({ block: 'nearest', inline: 'nearest' })
  }
}

watch(
  () => String(heatmapMode.value || 'objective'),
  (nextMode) => {
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
  },
  { immediate: true },
)

watch(
  () => activeViewMode.value,
  (nextViewMode) => {
    if (viewModeDraft.value !== nextViewMode) viewModeDraft.value = nextViewMode
  },
  { immediate: true },
)

watch(
  isPlateauMode,
  (plateauActive) => {
    if (plateauActive && String(heatmapViewMode.value || '2d').trim().toLowerCase() === '3d') {
      analysisStore.setSelectField('heatmapViewMode', '2d')
    }
  },
  { immediate: true },
)

watch(
  () => sliceOptions.value.map((slice) => slice.key).join('|'),
  () => {
    const slices = sliceOptions.value
    if (!slices.length) {
      selectedSliceKey.value = ''
      baselineSliceKey.value = ''
      return
    }
    if (!slices.some((slice) => slice.key === selectedSliceKey.value)) {
      selectedSliceKey.value = slices[0]?.key ?? ''
    }
    if (
      !slices.some((slice) => slice.key === baselineSliceKey.value) ||
      baselineSliceKey.value === selectedSliceKey.value
    ) {
      baselineSliceKey.value = slices.find((slice) => slice.key !== selectedSliceKey.value)?.key ?? slices[0]?.key ?? ''
    }
  },
  { immediate: true },
)

watch([visible, mountReady, activeViewMode], async ([nextVisible, nextReady]) => {
  if (!nextVisible || !nextReady) return
  await ensureVisibleHeatmap()
}, { immediate: true })

function setHeatmapMode(nextMode: string): void {
  analysisStore.setSelectField('heatmapMode', nextMode)
}

function setHeatmapViewMode(nextMode: string): void {
  analysisStore.setSelectField('heatmapViewMode', nextMode)
}

function setHeatmap2dSource(nextSource: string): void {
  heatmap2dSourceDraft.value = nextSource === 'delta' || nextSource === 'slice' ? nextSource : 'aggregate'
}

function setHeatmapFitMode(nextMode: string): void {
  heatmapFitModeDraft.value = nextMode === 'full' ? 'full' : 'valid'
}

function setSelectedSlice(nextKey: string): void {
  selectedSliceKey.value = nextKey
  if (baselineSliceKey.value === nextKey) {
    baselineSliceKey.value = sliceOptions.value.find((slice) => slice.key !== nextKey)?.key ?? baselineSliceKey.value
  }
}

function setBaselineSlice(nextKey: string): void {
  baselineSliceKey.value = nextKey
}

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}

function registerCanvasController(): void {
  registerHeatmapCanvasController({
    exportPng: (fileName: string) =>
      is3dView.value
        ? heatmapSurface3dRef.value?.exportPng(fileName) ?? false
        : heatmapCanvasRef.value?.exportPng(fileName) ?? false,
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
            <h3 class="panelPaneTitle">{{ activeViewLabel }} · {{ activeModeLabel }}</h3>
          </div>
          <div class="panelMuted">
            Top Results remains the exact retained-candidate path; heatmap cells open representative parameter-region detail.
          </div>
        </div>
        <AppMetricStrip
          class="heatmapMetricStrip"
          :items="heatmapDecisionStats"
          aria-label="Heatmap state"
          min-item-width="5.8rem"
          size="compact"
          surface="sunken"
        />
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
            <div>
              <label for="vueHeatmapViewMode">View</label>
              <select
                id="vueHeatmapViewMode"
                :value="viewModeDraft"
                :aria-describedby="threeDimensionalDisabledReason ? 'heatmap3dAvailabilityReasonVue' : undefined"
                @change="viewModeDraft = String(($event.target as HTMLSelectElement).value || '2d'); setHeatmapViewMode(viewModeDraft)"
              >
                <option
                  v-for="option in viewModeOptions"
                  :key="`heatmap_view_mode_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
              <p
                v-if="threeDimensionalDisabledReason"
                id="heatmap3dAvailabilityReasonVue"
                class="heatmapControlHint"
              >
                {{ threeDimensionalDisabledReason }}
              </p>
            </div>
            <div v-if="!is3dView">
              <label for="vueHeatmapFitMode">Fit</label>
              <select
                id="vueHeatmapFitMode"
                :value="heatmapFitModeDraft"
                @change="setHeatmapFitMode(String(($event.target as HTMLSelectElement).value || 'valid'))"
              >
                <option value="valid">Valid domain</option>
                <option value="full">Full grid</option>
              </select>
            </div>
            <div v-if="canUse2dSlices">
              <label for="vueHeatmap2dSource">2D source</label>
              <select
                id="vueHeatmap2dSource"
                :value="requested2dSource"
                @change="setHeatmap2dSource(String(($event.target as HTMLSelectElement).value || 'aggregate'))"
              >
                <option value="slice">Selected slice</option>
                <option value="delta">Delta vs baseline</option>
                <option value="aggregate">Run aggregate</option>
              </select>
            </div>
            <div v-if="canUse2dSlices && requested2dSource !== 'aggregate'">
              <label for="vueHeatmapSliceSelect">Slice</label>
              <select
                id="vueHeatmapSliceSelect"
                :value="selectedSliceKey"
                @change="setSelectedSlice(String(($event.target as HTMLSelectElement).value || ''))"
              >
                <option
                  v-for="slice in sliceOptions"
                  :key="`heatmap_slice_${slice.key}`"
                  :value="slice.key"
                >
                  {{ slice.label }}
                </option>
              </select>
            </div>
            <div v-if="canUse2dSlices && requested2dSource === 'delta'">
              <label for="vueHeatmapBaselineSliceSelect">Baseline</label>
              <select
                id="vueHeatmapBaselineSliceSelect"
                :value="baselineSliceKey"
                @change="setBaselineSlice(String(($event.target as HTMLSelectElement).value || ''))"
              >
                <option
                  v-for="slice in sliceOptions"
                  :key="`heatmap_baseline_slice_${slice.key}`"
                  :value="slice.key"
                  :disabled="slice.key === selectedSliceKey"
                >
                  {{ slice.label }}
                </option>
              </select>
            </div>
          </div>
          <template #actions>
            <button
              type="button"
              :disabled="!canExportActiveView"
              :title="canExportActiveView ? 'Export the current heatmap as PNG' : 'Export becomes available after the current heatmap renderer is ready'"
              data-qa-action="heatmap.exportPng"
              @click="analysisStore.heatmapExportPng()"
            >
              Export PNG
            </button>
          </template>
        </VisualizationToolbar>
        <div class="metaLine mono">{{ heatmapMeta }}</div>
        <div
          v-if="sliceFallbackNotice"
          class="metaLine heatmapFallbackNotice"
          role="status"
          aria-live="polite"
          aria-atomic="true"
        >
          {{ sliceFallbackNotice }}
        </div>
        <details class="heatmapScaleDetails">
          <summary>Scale and selection semantics</summary>
          <div class="metaLine mono">{{ scaleNote }}</div>
        </details>
      </section>

      <section class="pane">
        <HeatmapCanvas
          v-if="!is3dView"
          ref="heatmapCanvasRef"
          :ariaLabel="chartAriaLabel"
          :render-model="effective2dRenderModel"
          :fit-mode="heatmapFitModeDraft"
          :selected-summary="heatmapStatusLine"
          @review-top-results="reviewTopResults"
          @request-candidate-detail="requestHeatmapCandidateDetail"
        />
        <HeatmapSurface3D
          v-else
          ref="heatmapSurface3dRef"
          :ariaLabel="chartAriaLabel"
          :render-model="heatmapCanvasModel"
          @review-top-results="reviewTopResults"
          @request-candidate-detail="requestHeatmapCandidateDetail"
        />
      </section>

      <div
        v-if="showHeatmapStatusLine"
        class="panelStatus mono panelNumeric"
        role="status"
        aria-live="polite"
      >
        {{ heatmapStatusLine }}
      </div>
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

.heatmapControls {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
}

.heatmapControlHint {
  max-width: 28rem;
  margin: 0.35rem 0 0;
  color: var(--muted);
  font-size: 0.74rem;
  line-height: 1.35;
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

.heatmapFallbackNotice {
  color: var(--warning-ink);
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

@media (max-width: 760px) {
  .head,
  .heatmapDecisionHeader {
    grid-template-columns: 1fr;
  }

  .head {
    display: grid;
  }

}

</style>




