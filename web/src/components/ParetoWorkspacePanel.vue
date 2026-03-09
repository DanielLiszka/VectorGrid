<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useUiReadiness } from '../composables/useUiReadiness'
import { refreshParetoFromWorkflowRun } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

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
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const xDraft = ref('max_dd')
const yDraft = ref('pnl')

const visible = computed(() => String(resultsTab.value || '').trim() === 'pareto')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

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

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="pareto"]') as HTMLElement | null
  const mount = document.querySelector('#paretoVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vuePareto = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  await nextTick()
  refreshParetoFromWorkflowRun()
})

watch(visible, async (nextVisible) => {
  if (!nextVisible) return
  await nextTick()
  refreshParetoFromWorkflowRun()
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vuePareto
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#paretoVueShellMount">
    <section class="paretoPanelVueRoot paretoVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Pareto</div>
        <div class="meta mono">{{ stamp }}</div>
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
        <div class="metaLine mono">{{ paretoMeta }}</div>
      </section>

      <section class="pane">
        <h3>Pareto Chart</h3>
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
          <div class="muted mono">{{ statusLine }}</div>
        </div>
        <div v-else id="paretoWrapVue" style="margin-top: 0.75rem; display: none;">
          <div class="muted" style="margin: 0.25rem 0 0.6rem; font-size: 0.85rem;" id="paretoMetaVue">-</div>
          <canvas id="paretoCanvasVue" class="chart" width="1024" height="240"></canvas>
          <div class="muted" style="margin-top: 0.5rem; font-size: 0.8rem;">Click a point to drill down that parameter pair.</div>
        </div>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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

.title {
  font-size: 0.95rem;
  font-weight: 650;
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
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

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
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

<style>
[data-vue-panel='pareto'][data-vue-pareto='1'] > :not(#paretoVueShellMount) {
  display: none !important;
}
</style>


