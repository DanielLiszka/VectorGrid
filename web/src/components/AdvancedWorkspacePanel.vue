<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useAdvancedStore, type AdvancedOption } from '../stores/advancedStore'
import { useWorkflowStore } from '../stores/workflowStore'

const advancedStore = useAdvancedStore()
const workflowStore = useWorkflowStore()
const {
  maSource,
  maSourceOptions,
  objective,
  objectiveOptions,
  mode,
  modeOptions,
  vdpsVisible,
  vdpsTelemetryHint,
  compositeMoVisible,
  heatmapBins,
  topK,
  includeAll,
  includeAllOptions,
  exportAllCsvPath,
  canBrowseExportAll,
  canResetMoSearchSpace,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(advancedStore)
const { controlsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)
const adoptedNodes: { node: HTMLElement; parent: Node; next: ChildNode | null }[] = []

const maSourceDraft = ref('close')
const objectiveDraft = ref('Sharpe')
const modeDraft = ref('Grid')
const heatmapBinsDraft = ref('128')
const topKDraft = ref('20')
const includeAllDraft = ref('false')
const exportPathDraft = ref('')

const visible = computed(() => String(controlsTab.value || '').trim() === 'advanced')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function resolvedOptions(options: AdvancedOption[], fallbackValue: string, fallbackLabel: string): AdvancedOption[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const maSourceOptionItems = computed(() => resolvedOptions(maSourceOptions.value, maSource.value, maSource.value || 'close'))
const objectiveOptionItems = computed(() => resolvedOptions(objectiveOptions.value, objective.value, objective.value || 'Sharpe'))
const modeOptionItems = computed(() => resolvedOptions(modeOptions.value, mode.value, mode.value || 'Grid'))
const includeAllOptionItems = computed(() => resolvedOptions(includeAllOptions.value, includeAll.value, includeAll.value || 'false'))

watch(maSource, (v) => { const next = String(v || 'close'); if (maSourceDraft.value !== next) maSourceDraft.value = next }, { immediate: true })
watch(objective, (v) => { const next = String(v || 'Sharpe'); if (objectiveDraft.value !== next) objectiveDraft.value = next }, { immediate: true })
watch(mode, (v) => { const next = String(v || 'Grid'); if (modeDraft.value !== next) modeDraft.value = next }, { immediate: true })
watch(heatmapBins, (v) => { const next = String(v || '128'); if (heatmapBinsDraft.value !== next) heatmapBinsDraft.value = next }, { immediate: true })
watch(topK, (v) => { const next = String(v || '20'); if (topKDraft.value !== next) topKDraft.value = next }, { immediate: true })
watch(includeAll, (v) => { const next = String(v || 'false'); if (includeAllDraft.value !== next) includeAllDraft.value = next }, { immediate: true })
watch(exportAllCsvPath, (v) => { const next = String(v || ''); if (exportPathDraft.value !== next) exportPathDraft.value = next }, { immediate: true })

function setInputValue(id: string, nextValue: string): void {
  advancedStore.setInputField(id, nextValue)
}

function setSelectValue(id: string, nextValue: string): void {
  advancedStore.setSelectField(id, nextValue)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null
  const mount = document.getElementById('advancedVueShellMount')
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueAdvanced = '1'
  mountReady.value = true
}

function adoptLegacyAdvancedBlocks(): void {
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null)
  const host = document.getElementById('advancedCompatHostVue')
  if (!panel || !host) return
  const nodes = Array.from(panel.querySelectorAll('.advancedLegacyPassthrough')) as HTMLElement[]
  for (const node of nodes) {
    if (node.parentElement === host) continue
    const parent = node.parentNode
    if (!parent) continue
    adoptedNodes.push({ node, parent, next: node.nextSibling })
    host.appendChild(node)
  }
}

function restoreLegacyAdvancedBlocks(): void {
  while (adoptedNodes.length > 0) {
    const entry = adoptedNodes.pop()
    if (!entry) continue
    if (entry.next && entry.next.parentNode === entry.parent) {
      entry.parent.insertBefore(entry.node, entry.next)
    } else {
      entry.parent.appendChild(entry.node)
    }
  }
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  window.setTimeout(() => {
    adoptLegacyAdvancedBlocks()
  }, 0)
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueAdvanced
    panelRoot.value = null
  }
  restoreLegacyAdvancedBlocks()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#advancedVueShellMount">
    <section class="advancedPanelVueRoot advancedVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Advanced Controls</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <h3>Optimization setup</h3>
        <div class="gridThree">
          <div>
            <label for="vueMaSource">MA price source</label>
            <select
              id="vueMaSource"
              :value="maSourceDraft"
              @change="maSourceDraft = String(($event.target as HTMLSelectElement).value || 'close'); setSelectValue('maSource', maSourceDraft)"
            >
              <option
                v-for="option in maSourceOptionItems"
                :key="`ma_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div>
            <label for="vueObjective">Objective</label>
            <select
              id="vueObjective"
              :value="objectiveDraft"
              @change="objectiveDraft = String(($event.target as HTMLSelectElement).value || 'Sharpe'); setSelectValue('objective', objectiveDraft)"
            >
              <option
                v-for="option in objectiveOptionItems"
                :key="`objective_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div>
            <label for="vueMode">Mode</label>
            <select
              id="vueMode"
              :value="modeDraft"
              @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'Grid'); setSelectValue('mode', modeDraft)"
            >
              <option
                v-for="option in modeOptionItems"
                :key="`mode_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
      </section>

      <section class="pane">
        <h3>Output shaping</h3>
        <div class="gridThree">
          <div>
            <label for="vueHeatmapBins">Heatmap bins (0 = off)</label>
            <input
              id="vueHeatmapBins"
              type="number"
              min="0"
              step="1"
              :value="heatmapBinsDraft"
              @input="heatmapBinsDraft = String(($event.target as HTMLInputElement).value || '128'); setInputValue('heatmapBins', heatmapBinsDraft)"
            />
          </div>
          <div>
            <label for="vueTopK">Return top K (0 = best only)</label>
            <input
              id="vueTopK"
              type="number"
              min="0"
              step="1"
              :value="topKDraft"
              @input="topKDraft = String(($event.target as HTMLInputElement).value || '20'); setInputValue('topK', topKDraft)"
            />
          </div>
          <div>
            <label for="vueIncludeAll">Return all results</label>
            <select
              id="vueIncludeAll"
              :value="includeAllDraft"
              @change="includeAllDraft = String(($event.target as HTMLSelectElement).value || 'false'); setSelectValue('includeAll', includeAllDraft)"
            >
              <option
                v-for="option in includeAllOptionItems"
                :key="`include_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>

        <label for="vueExportAllCsvPath">Export per-pair metrics CSV path (optional)</label>
        <div class="pathRow">
          <input
            id="vueExportAllCsvPath"
            type="text"
            :value="exportPathDraft"
            placeholder="C:\\path\\to\\all_pairs.csv"
            @input="exportPathDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('exportAllCsvPath', exportPathDraft)"
          />
          <button type="button" :disabled="!canBrowseExportAll" @click="advancedStore.browseExportAll()">Browse</button>
        </div>
      </section>

      <section class="pane">
        <h3>Strategy samplers</h3>
        <div class="metaLine mono">VDPS panel: {{ vdpsVisible ? 'visible' : 'hidden' }}</div>
        <div class="metaLine mono">Composite-MO panel: {{ compositeMoVisible ? 'visible' : 'hidden' }}</div>
        <div class="muted">{{ vdpsTelemetryHint }}</div>
        <div class="pathRow" style="margin-top: 0.55rem;">
          <div></div>
          <button type="button" :disabled="!canResetMoSearchSpace" @click="advancedStore.resetMoSearchSpace()">
            Reset MO search-space JSON
          </button>
        </div>
        <div id="advancedCompatHostVue"></div>
      </section>

      <div class="status mono">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.advancedPanelVueRoot {
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

.gridThree {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.65rem;
}

.pathRow {
  margin-top: 0.4rem;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  margin-top: 0.4rem;
  color: var(--muted);
  font-size: 0.8rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .gridThree,
  .pathRow {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='advanced'][data-vue-advanced='1'] > :not(#advancedVueShellMount) {
  display: none !important;
}
</style>


