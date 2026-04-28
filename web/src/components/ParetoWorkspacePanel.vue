<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useUiReadiness } from '../composables/useUiReadiness'
import { refreshParetoFromWorkflowRun } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import ChartEmptyState from './visualizations/ChartEmptyState.vue'
import MetricDistributionChart from './visualizations/MetricDistributionChart.vue'
import ParetoScatter from './visualizations/ParetoScatter.vue'
import { formatParetoDisplay, type ParetoRenderPoint } from '../visualizations/paretoModel'

interface OptionItem {
  value: string
  label: string
  disabled: boolean
}

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const { paretoStage, describeStage } = useUiReadiness()
const {
  paretoX,
  paretoY,
  paretoXOptions,
  paretoYOptions,
  paretoModel,
  paretoMeta,
  paretoTruthNote,
  paretoRepresentativeSummary,
  paretoConfidenceNote,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#paretoVueShellMount')

const xDraft = ref('max_dd')
const yDraft = ref('pnl')

const visible = computed(() => String(resultsTab.value || '').trim() === 'pareto')
const paretoReadiness = computed(() => describeStage('pareto', paretoStage.value))

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

function resolvedOptions(options: OptionItem[], fallbackValue: string, fallbackLabel: string): OptionItem[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const xOptions = computed(() => resolvedOptions(paretoXOptions.value as OptionItem[], paretoX.value, paretoX.value || 'max_dd'))
const yOptions = computed(() => resolvedOptions(paretoYOptions.value as OptionItem[], paretoY.value, paretoY.value || 'pnl'))
const paretoDistributionColumns = computed(() => {
  const model = paretoModel.value
  if (!model || model.status !== 'ready') return []
  const xLabel = xOptions.value.find((option) => option.value === model.xKey)?.label || model.xKey
  const yLabel = yOptions.value.find((option) => option.value === model.yKey)?.label || model.yKey
  return [
    { key: model.xKey, label: xLabel },
    { key: model.yKey, label: yLabel },
  ]
})
const paretoDistributionRows = computed(() => {
  const model = paretoModel.value
  if (!model || model.status !== 'ready') return []
  return model.points.map((point) => ({
    cells: [
      {
        text: formatParetoDisplay(point.x, 4),
        title: String(point.x),
        className: 'mono',
      },
      {
        text: formatParetoDisplay(point.y, 4),
        title: String(point.y),
        className: 'mono',
      },
    ],
  }))
})

watch(
  () => [
    String(paretoX.value || 'max_dd'),
    String(paretoY.value || 'pnl'),
  ] as const,
  ([nextX, nextY]) => {
    if (xDraft.value !== nextX) xDraft.value = nextX
    if (yDraft.value !== nextY) yDraft.value = nextY
  },
  { immediate: true },
)

function setSelectValue(id: string, nextValue: string): void {
  analysisStore.setSelectField(id, nextValue)
}

onMounted(async () => {
  await nextTick()
  await nextTick()
  refreshParetoFromWorkflowRun()
})

watch(visible, async (nextVisible) => {
  if (!nextVisible) return
  await nextTick()
  refreshParetoFromWorkflowRun()
})

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}

function selectParetoPoint(point: ParetoRenderPoint): void {
  analysisStore.selectParetoPoint(point.rowId)
}
</script>

<template>
  <Teleport v-if="mountReady" to="#paretoVueShellMount">
    <section class="paretoPanelVueRoot paretoVueShellMountHost" v-show="visible">
      <header class="head">
        <h2 class="panelTitle">Pareto</h2>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <section v-if="paretoStage === 'ready'" class="pane">
        <div class="gridTwo">
          <div>
            <label for="vueParetoX">X axis</label>
            <select
              id="vueParetoX"
              :value="xDraft"
              @change="xDraft = String(($event.target as HTMLSelectElement).value || 'max_dd'); setSelectValue('paretoX', xDraft)"
            >
              <option
                v-for="option in xOptions"
                :key="`x_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div>
            <label for="vueParetoY">Y axis</label>
            <select
              id="vueParetoY"
              :value="yDraft"
              @change="yDraft = String(($event.target as HTMLSelectElement).value || 'pnl'); setSelectValue('paretoY', yDraft)"
            >
              <option
                v-for="option in yOptions"
                :key="`y_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div class="metaLine mono" role="status" aria-live="polite">{{ paretoMeta }}</div>
        <div class="panelMuted">{{ paretoTruthNote }}</div>
        <div class="panelMuted mono">Representative points: {{ paretoRepresentativeSummary }}</div>
        <div class="panelMuted">{{ paretoConfidenceNote }}</div>
      </section>

      <section class="pane">
        <h3 class="panelPaneTitle">Pareto Chart</h3>
        <ChartEmptyState
          v-if="paretoStage !== 'ready'"
          :title="paretoReadiness.title"
          :body="paretoReadiness.body"
          :status="String(statusLine || '')"
          action-label="Review Top results"
          @action="reviewTopResults"
        />
        <div class="chartAssist">
          <div id="paretoKeyboardHint" class="panelMuted">
            Use the Top results table as the keyboard-friendly way to move between candidates, or click frontier points if you want direct chart-driven candidate detail.
          </div>
          <div class="actions" style="margin-top: 0;">
            <button type="button" @click="reviewTopResults">Review Top results</button>
          </div>
        </div>
        <div class="panelAdvisory">
          If the frontier stays blank after a run, confirm that optimization returned more than one ranked candidate and that the current X/Y metrics are still available for this result set.
        </div>
        <ParetoScatter
          v-if="paretoStage === 'ready'"
          :model="paretoModel"
          @select-point="selectParetoPoint"
        />
        <MetricDistributionChart
          v-if="paretoStage === 'ready'"
          title="Pareto metric distribution"
          :columns="paretoDistributionColumns"
          :rows="paretoDistributionRows"
          :preferred-keys="[paretoY || 'pnl', paretoX || 'max_dd']"
        />
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.paretoPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.gridTwo {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.65rem;
}

.metaLine {
  margin-top: 0.45rem;
  font-size: 0.78rem;
  color: var(--muted);
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

.chartAssist {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.35rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 860px) {
  .gridTwo {
    grid-template-columns: 1fr;
  }
}
</style>



