<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { applyRecommendedOptimizationDefaults } from '../services/recommendedOptimizationDefaults'
import { useAdvancedStore, type AdvancedOption } from '../stores/advancedStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useStrategyStore } from '../stores/strategyStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { describeStrategyOptimizationCapabilities } from '../utils/strategyOptimizationCapabilities'

const advancedStore = useAdvancedStore()
const optimizeStore = useOptimizeStore()
const strategyStore = useStrategyStore()
const workflowStore = useWorkflowStore()
const {
  maSource,
  maSourceOptions,
  objective,
  objectiveOptions,
  mode,
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
const compatCleanupFns: Array<() => void> = []
const compatObservers: MutationObserver[] = []

const maSourceDraft = ref('close')
const objectiveDraft = ref('Sharpe')
const modeDraft = ref('Grid')
const heatmapBinsDraft = ref('128')
const topKDraft = ref('20')
const includeAllDraft = ref('false')
const exportPathDraft = ref('')

const visible = computed(() => String(controlsTab.value || '').trim() === 'advanced')
const strategyCapabilities = computed(() => describeStrategyOptimizationCapabilities(
  optimizeStore.strategyKind,
  {
    requestedBackend: optimizeStore.backend,
    trendUseAdx: strategyStore.trendUseAdx,
    breakoutUseAdx: strategyStore.breakoutUseAdx,
  },
))
const showModeControl = computed(() => strategyCapabilities.value.usesGlobalMode)
const showHeatmapControl = computed(() => strategyCapabilities.value.supportsHeatmap)
const showTopKControl = computed(() => strategyCapabilities.value.supportsTopK)
const showIncludeAllControl = computed(() => strategyCapabilities.value.supportsIncludeAll)
const showExportControl = computed(() => strategyCapabilities.value.supportsExportAll)

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
const modeOptionItems = computed(() => [{ value: 'Grid', label: 'Grid', disabled: false }])
const includeAllOptionItems = computed(() => resolvedOptions(includeAllOptions.value, includeAll.value, includeAll.value || 'false'))

watch(
  () => [
    String(maSource.value || 'close'),
    String(objective.value || 'Sharpe'),
    String(mode.value || 'Grid'),
    String(heatmapBins.value || '128'),
    String(topK.value || '20'),
    String(includeAll.value || 'false'),
    String(exportAllCsvPath.value || ''),
  ] as const,
  ([
    nextMaSource,
    nextObjective,
    nextMode,
    nextHeatmapBins,
    nextTopK,
    nextIncludeAll,
    nextExportPath,
  ]) => {
    if (maSourceDraft.value !== nextMaSource) maSourceDraft.value = nextMaSource
    if (objectiveDraft.value !== nextObjective) objectiveDraft.value = nextObjective
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
    if (heatmapBinsDraft.value !== nextHeatmapBins) heatmapBinsDraft.value = nextHeatmapBins
    if (topKDraft.value !== nextTopK) topKDraft.value = nextTopK
    if (includeAllDraft.value !== nextIncludeAll) includeAllDraft.value = nextIncludeAll
    if (exportPathDraft.value !== nextExportPath) exportPathDraft.value = nextExportPath
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  advancedStore.setInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setSelectValue(id: string, nextValue: string): void {
  advancedStore.setSelectField(id, nextValue)
}

function clearCompatListeners(): void {
  while (compatCleanupFns.length > 0) {
    const cleanup = compatCleanupFns.pop()
    if (!cleanup) continue
    cleanup()
  }
}

function clearCompatObservers(): void {
  while (compatObservers.length > 0) {
    const observer = compatObservers.pop()
    if (!observer) continue
    observer.disconnect()
  }
}

function queryLegacyById<T extends Element>(id: string): T | null {
  const selector = `[id="${id}"]`
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null)
  if (panel) {
    const scoped = panel.querySelector(selector) as T | null
    if (scoped) return scoped
  }
  const byQuery = typeof document.querySelector === 'function'
    ? document.querySelector(selector)
    : null
  if (byQuery) return byQuery as T
  const getById = (document as unknown as { getElementById?: (nextId: string) => Element | null }).getElementById
  if (typeof getById !== 'function') return null
  return (getById.call(document, id) as T | null) ?? null
}

function legacyAdvancedControls(): Array<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement> {
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null)
  if (!panel) return []
  return Array.from(panel.querySelectorAll('input[id], select[id], textarea[id]'))
    .filter((el) => !String((el as HTMLElement).id || '').startsWith('vue'))
    .map((el) => el as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement)
}

function optionsById(id: string): AdvancedOption[] {
  const el = queryLegacyById<HTMLSelectElement>(id)
  if (!el) return []
  return Array.from(el.options)
    .map((option) => {
      const value = String(option.value || '').trim()
      const label = String(option.textContent || option.value || '').trim()
      return {
        value: value || label,
        label: label || value,
        disabled: !!option.disabled,
      }
    })
    .filter((entry) => entry.value.length > 0 || entry.label.length > 0)
}

function optionsEqual(left: AdvancedOption[], right: AdvancedOption[]): boolean {
  if (left.length !== right.length) return false
  for (let i = 0; i < left.length; i += 1) {
    const l = left[i]
    const r = right[i]
    if (!l || !r) return false
    if (l.value !== r.value || l.label !== r.label || l.disabled !== r.disabled) return false
  }
  return true
}

function syncLegacyAdvancedSelectOptions(): void {
  const maOptions = optionsById('maSource')
  const objectiveOptionsNext = optionsById('objective')
  const modeOptionsNext = optionsById('mode')
  const includeAllOptionsNext = optionsById('includeAll')
  advancedStore.$patch((state) => {
    let changed = false
    if (maOptions.length > 0 && !optionsEqual(state.maSourceOptions, maOptions)) {
      state.maSourceOptions = maOptions
      changed = true
    }
    if (objectiveOptionsNext.length > 0 && !optionsEqual(state.objectiveOptions, objectiveOptionsNext)) {
      state.objectiveOptions = objectiveOptionsNext
      changed = true
    }
    if (modeOptionsNext.length > 0 && !optionsEqual(state.modeOptions, modeOptionsNext)) {
      state.modeOptions = modeOptionsNext
      changed = true
    }
    if (includeAllOptionsNext.length > 0 && !optionsEqual(state.includeAllOptions, includeAllOptionsNext)) {
      state.includeAllOptions = includeAllOptionsNext
      changed = true
    }
    if (changed) state.updatedAtUnixMs = Date.now()
  })
}

function syncLegacyAdvancedUiMeta(): void {
  const vdpsWrap = queryLegacyById<HTMLElement>('vdpsConfigWrap')
  const compositeWrap = queryLegacyById<HTMLElement>('compositeMoWrap')
  const telemetryNode = queryLegacyById<HTMLElement>('vdpsTelemetryHint')
  const statusNode = queryLegacyById<HTMLElement>('status')
  const exportInput = queryLegacyById<HTMLInputElement>('exportAllCsvPath')
  const canReset = String(document.documentElement.dataset.advCanResetMoSearchSpace || '').trim() === '1'
  const vdpsVisibleNext = !!vdpsWrap && window.getComputedStyle(vdpsWrap).display !== 'none'
  const compositeVisibleNext = !!compositeWrap && window.getComputedStyle(compositeWrap).display !== 'none'
  const telemetryText = String(telemetryNode?.textContent || '').trim() || 'VDPS telemetry appears after a run.'
  const statusText = String(statusNode?.textContent || '').trim() || 'Ready.'
  const canBrowseNext = exportInput ? !exportInput.disabled : true

  advancedStore.$patch((state) => {
    let changed = false
    if (state.vdpsVisible !== vdpsVisibleNext) {
      state.vdpsVisible = vdpsVisibleNext
      changed = true
    }
    if (state.compositeMoVisible !== compositeVisibleNext) {
      state.compositeMoVisible = compositeVisibleNext
      changed = true
    }
    if (state.vdpsTelemetryHint !== telemetryText) {
      state.vdpsTelemetryHint = telemetryText
      changed = true
    }
    if (state.statusLine !== statusText) {
      state.statusLine = statusText
      changed = true
    }
    if (state.canBrowseExportAll !== canBrowseNext) {
      state.canBrowseExportAll = canBrowseNext
      changed = true
    }
    if (state.canResetMoSearchSpace !== canReset) {
      state.canResetMoSearchSpace = canReset
      changed = true
    }
    if (changed) state.updatedAtUnixMs = Date.now()
  })
}

function seedLegacyAdvancedInputsToStore(): void {
  const storeMap = advancedStore as unknown as Record<string, unknown>
  for (const control of legacyAdvancedControls()) {
    const id = String(control.id || '').trim()
    if (!id || !(id in storeMap)) continue
    const current = storeMap[id]
    if (typeof current === 'boolean' && control instanceof HTMLInputElement && control.type === 'checkbox') {
      advancedStore.setCheckboxField(id, !!control.checked)
      continue
    }
    if (typeof current !== 'string') continue
    const nextValue = String(control.value || '')
    if (String(current) === nextValue) continue
    if (control instanceof HTMLSelectElement) {
      advancedStore.setSelectField(id, nextValue)
    } else {
      advancedStore.setInputField(id, nextValue)
    }
  }
}

function syncLegacyAdvancedInputsFromStore(): void {
  const storeMap = advancedStore as unknown as Record<string, unknown>
  for (const control of legacyAdvancedControls()) {
    const id = String(control.id || '').trim()
    if (!id || !(id in storeMap)) continue
    const nextValue = storeMap[id]
    if (typeof nextValue === 'boolean' && control instanceof HTMLInputElement && control.type === 'checkbox') {
      if (control.checked !== nextValue) control.checked = nextValue
      continue
    }
    if (typeof nextValue !== 'string') continue
    if (String(control.value || '') === nextValue) continue
    control.value = nextValue
  }
}

function bindLegacyAdvancedInputs(): void {
  clearCompatListeners()
  for (const control of legacyAdvancedControls()) {
    const id = String(control.id || '').trim()
    if (!id) continue
    const onChange = (event: Event) => {
      const target = event.target as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
      if (!target) return
      if (target instanceof HTMLInputElement && target.type === 'checkbox') {
        advancedStore.setCheckboxField(id, !!target.checked)
      } else if (target instanceof HTMLSelectElement) {
        advancedStore.setSelectField(id, String(target.value || ''))
      } else {
        advancedStore.setInputField(id, String(target.value || ''))
      }
      syncLegacyAdvancedUiMeta()
      syncLegacyAdvancedSelectOptions()
    }
    control.addEventListener('input', onChange)
    control.addEventListener('change', onChange)
    compatCleanupFns.push(() => {
      control.removeEventListener('input', onChange)
      control.removeEventListener('change', onChange)
    })
  }
}

function bindLegacyAdvancedObservers(): void {
  clearCompatObservers()

  const textObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  for (const id of ['vdpsTelemetryHint', 'status'] as const) {
    const node = queryLegacyById<HTMLElement>(id)
    if (!node) continue
    textObserver.observe(node, { childList: true, characterData: true, subtree: true })
  }
  compatObservers.push(textObserver)

  const rootObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  rootObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-controls-tab', 'data-running', 'data-adv-can-reset-mo-search-space'],
  })
  compatObservers.push(rootObserver)

  const visibilityObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  for (const id of ['vdpsConfigWrap', 'compositeMoWrap'] as const) {
    const node = queryLegacyById<HTMLElement>(id)
    if (!node) continue
    visibilityObserver.observe(node, {
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }
  compatObservers.push(visibilityObserver)

  const optionObserver = new MutationObserver(() => {
    syncLegacyAdvancedSelectOptions()
  })
  for (const id of ['maSource', 'objective', 'mode', 'includeAll'] as const) {
    const node = queryLegacyById<HTMLElement>(id)
    if (!node) continue
    optionObserver.observe(node, {
      attributes: true,
      childList: true,
      subtree: true,
      attributeFilter: ['disabled', 'selected', 'value', 'class'],
    })
  }
  compatObservers.push(optionObserver)

  const exportObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  const exportInput = queryLegacyById<HTMLElement>('exportAllCsvPath')
  if (exportInput) {
    exportObserver.observe(exportInput, {
      attributes: true,
      attributeFilter: ['disabled', 'class'],
    })
  }
  compatObservers.push(exportObserver)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null
  const mount = document.querySelector('#advancedVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueAdvanced = '1'
  mountReady.value = true
}

function adoptLegacyAdvancedBlocks(): void {
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null)
  const host = document.querySelector('#advancedCompatHostVue') as HTMLElement | null
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
    seedLegacyAdvancedInputsToStore()
    applyRecommendedOptimizationDefaults()
    syncLegacyAdvancedInputsFromStore()
    bindLegacyAdvancedInputs()
    syncLegacyAdvancedSelectOptions()
    syncLegacyAdvancedUiMeta()
    bindLegacyAdvancedObservers()
  }, 0)
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  clearCompatObservers()
  clearCompatListeners()
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
            <template v-if="showModeControl">
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
            </template>
            <template v-else>
              <label>Mode</label>
              <div class="muted mono">{{ strategyCapabilities.modeSummary }}</div>
            </template>
          </div>
        </div>
      </section>

      <section class="pane">
        <h3>Output shaping</h3>
        <div class="gridThree">
          <div v-if="showHeatmapControl">
            <label for="vueHeatmapBins">Heatmap bins (0 = off)</label>
            <input
              id="vueHeatmapBins"
              type="number"
              min="0"
              step="1"
              :value="heatmapBinsDraft"
              @input="heatmapBinsDraft = String(($event.target as HTMLInputElement).value || '128'); queueInputValue('heatmapBins', heatmapBinsDraft)"
              @change="commitInputValue('heatmapBins', heatmapBinsDraft)"
            />
          </div>
          <div v-if="showTopKControl">
            <label for="vueTopK">Return top K (0 = best only)</label>
            <input
              id="vueTopK"
              type="number"
              min="0"
              step="1"
              :value="topKDraft"
              @input="topKDraft = String(($event.target as HTMLInputElement).value || '20'); queueInputValue('topK', topKDraft)"
              @change="commitInputValue('topK', topKDraft)"
            />
          </div>
          <div v-if="showIncludeAllControl">
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

        <template v-if="showExportControl">
          <label for="vueExportAllCsvPath">Export per-pair metrics CSV path (optional)</label>
          <div class="pathRow">
            <input
              id="vueExportAllCsvPath"
              type="text"
              :value="exportPathDraft"
              placeholder="C:\\path\\to\\all_pairs.csv"
              @input="exportPathDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('exportAllCsvPath', exportPathDraft)"
              @change="commitInputValue('exportAllCsvPath', exportPathDraft)"
            />
            <button type="button" :disabled="!canBrowseExportAll" @click="advancedStore.browseExportAll()">Browse</button>
          </div>
        </template>
        <div v-if="!showHeatmapControl || !showTopKControl || !showIncludeAllControl || !showExportControl" class="muted mono">
          {{ strategyCapabilities.outputSummary }}
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


