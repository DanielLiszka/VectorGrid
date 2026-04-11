<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useUiReadiness } from '../composables/useUiReadiness'
import { refreshParetoFromWorkflowRun } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

interface OptionItem {
  value: string
  label: string
  disabled: boolean
}

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const { paretoStage } = useUiReadiness()
const {
  paretoX,
  paretoY,
  paretoXOptions,
  paretoYOptions,
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

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

function resolvedOptions(options: OptionItem[], fallbackValue: string, fallbackLabel: string): OptionItem[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const xOptions = computed(() => resolvedOptions(paretoXOptions.value as OptionItem[], paretoX.value, paretoX.value || 'max_dd'))
const yOptions = computed(() => resolvedOptions(paretoYOptions.value as OptionItem[], paretoY.value, paretoY.value || 'pnl'))

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
        <div v-if="paretoStage !== 'ready'" class="emptyState">
          <div class="emptyTitle">
            {{ paretoStage === 'no_data' ? 'Load data before opening the Pareto view.' : 'Run an optimization to generate Pareto points.' }}
          </div>
          <div class="emptyBody">
            {{
              paretoStage === 'no_data'
                ? 'The Pareto frontier appears after a dataset is loaded and scored.'
                : 'This view compares tradeoffs between competing candidates once optimization results are available.'
            }}
          </div>
          <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
        </div>
        <div class="chartAssist">
          <div id="paretoKeyboardHint" class="panelMuted">
            Use the Top results table as the keyboard-friendly way to move between candidates, or click frontier points if you want direct chart drilldown.
          </div>
          <div class="actions" style="margin-top: 0;">
            <button type="button" @click="reviewTopResults">Review Top results</button>
          </div>
        </div>
        <div class="panelAdvisory">
          If the frontier stays blank after a run, confirm that optimization returned more than one ranked candidate and that the current X/Y metrics are still available for this result set.
        </div>
        <div v-if="paretoStage === 'ready'" id="paretoWrapVue" style="margin-top: 0.75rem; display: none;">
          <div class="panelStatus" style="margin: 0.25rem 0 0.6rem; font-size: 0.85rem;" id="paretoMetaVue" role="status" aria-live="polite">-</div>
          <div
            class="chartSurface"
            aria-label="Pareto frontier chart comparing the selected X and Y metrics. Use the Top results tab for a non-pointer way to move between candidates."
            aria-describedby="paretoKeyboardHint"
          >
            <canvas id="paretoCanvasVue" class="chart" width="1024" height="240" role="img" aria-label="Pareto frontier chart"></canvas>
          </div>
          <div class="panelMuted" style="margin-top: 0.5rem;">Select a Top result or click a point to drill down that parameter pair.</div>
          <div class="panelMuted" style="margin-top: 0.35rem;">Frontier membership visualizes tradeoffs; validation still decides whether a candidate is acceptable.</div>
        </div>
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

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
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



