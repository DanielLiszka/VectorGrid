<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useDataStore } from '../stores/dataStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'

const settingsStore = useSettingsStore()
const dataStore = useDataStore()
const workflowStore = useWorkflowStore()
const {
  theme,
  effect,
  workflowAdvanced,
  controlsCollapsed,
  marketDataDir,
  moveExisting,
  marketDataDirHint,
  canBrowseMarketDataDir,
  canSetMarketDataDir,
  canResetMarketDataDir,
  canOpenMarketDataDir,
  canOpenWorkflowData,
  canOpenRunsDir,
  canOpenReportsDir,
  canOpenPolygonModal,
  canGotoLibrary,
  canGotoAggregate,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(settingsStore)
const { updatedAtUnixMs: dataUpdatedAtUnixMs } = storeToRefs(dataStore)
const { workspace } = storeToRefs(workflowStore)

const mountReady = ref(false)
const gridRoot = ref<HTMLElement | null>(null)
let runtimeObserver: MutationObserver | null = null

const themeDraft = ref('system')
const effectDraft = ref('none')
const marketDataDirDraft = ref('')

const visible = computed(() => String(workspace.value || '').trim() === 'settings')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

watch(
  () => [
    String(theme.value || 'system').trim() || 'system',
    String(effect.value || 'none').trim() || 'none',
    String(marketDataDir.value || ''),
  ] as const,
  ([nextTheme, nextEffect, nextMarketDataDir]) => {
    if (themeDraft.value !== nextTheme) themeDraft.value = nextTheme
    if (effectDraft.value !== nextEffect) effectDraft.value = nextEffect
    if (marketDataDirDraft.value !== nextMarketDataDir) marketDataDirDraft.value = nextMarketDataDir
  },
  { immediate: true },
)

watch(
  dataUpdatedAtUnixMs,
  () => {
    settingsStore.syncFromDataStore()
  },
  { immediate: true },
)

function setMarketDataDirFromDraft(): void {
  settingsStore.setMarketDataDir(String(marketDataDirDraft.value || '').trim(), !!moveExisting.value)
}

function setCheckboxValue(id: string, checked: boolean): void {
  settingsStore.setCheckboxField(id, checked)
}

function setSelectValue(id: string, nextValue: string): void {
  settingsStore.setSelectField(id, nextValue)
}

function setInputValue(id: string, nextValue: string): void {
  settingsStore.setInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function resolveMount(): void {
  const settingsGrid = document.querySelector('#settingsShellGrid') as HTMLElement | null
  const mount = document.querySelector('#settingsVueShellMount') as HTMLElement | null
  if (!settingsGrid || !mount) return
  gridRoot.value = settingsGrid
  settingsGrid.dataset.vueSettings = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  settingsStore.syncFromDataStore()
  if (typeof document !== 'undefined') {
    runtimeObserver = new MutationObserver(() => settingsStore.syncFromDataStore())
    runtimeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-running'],
    })
  }
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  if (runtimeObserver) {
    runtimeObserver.disconnect()
    runtimeObserver = null
  }
  if (gridRoot.value) {
    delete gridRoot.value.dataset.vueSettings
    gridRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#settingsVueShellMount">
    <section class="settingsPanelVueRoot settingsVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Settings</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <div class="grid">
        <section class="pane">
          <h3>Appearance</h3>
          <div class="rowTwo">
            <div>
              <label for="vueSettingsTheme">Theme</label>
              <select
                id="vueSettingsTheme"
                :value="themeDraft"
                @change="themeDraft = String(($event.target as HTMLSelectElement).value || 'system'); setSelectValue('settingsThemeSelect', themeDraft)"
              >
                <option value="light">Light</option>
                <option value="dark">Dark</option>
                <option value="system">System</option>
              </select>
            </div>
            <div>
              <label for="vueSettingsEffect">Window effect</label>
              <select
                id="vueSettingsEffect"
                :value="effectDraft"
                @change="effectDraft = String(($event.target as HTMLSelectElement).value || 'none'); setSelectValue('settingsEffectSelect', effectDraft)"
              >
                <option value="none">None</option>
                <option value="mica">Mica</option>
                <option value="acrylic">Acrylic</option>
                <option value="vibrancy">Vibrancy</option>
              </select>
            </div>
          </div>
          <div class="checks">
            <label>
              <input
                type="checkbox"
                :checked="workflowAdvanced"
                @change="setCheckboxValue('settingsWorkflowAdvanced', ($event.target as HTMLInputElement).checked)"
              />
              Show advanced controls by default
            </label>
            <label>
              <input
                type="checkbox"
                :checked="controlsCollapsed"
                @change="setCheckboxValue('settingsControlsCollapsed', ($event.target as HTMLInputElement).checked)"
              />
              Start with controls panel collapsed
            </label>
          </div>
        </section>

        <section class="pane">
          <h3>Data and storage</h3>
          <label for="vueSettingsMarketDataDir">Market data folder</label>
          <div class="inputWithActions">
            <input
              id="vueSettingsMarketDataDir"
              type="text"
              :value="marketDataDirDraft"
              placeholder="Default (AppData)"
              @input="marketDataDirDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('settingsMarketDataDir', marketDataDirDraft)"
              @change="commitInputValue('settingsMarketDataDir', marketDataDirDraft)"
            />
            <div class="actions">
              <button type="button" :disabled="!canBrowseMarketDataDir" @click="settingsStore.browseMarketDataDir()">Browse</button>
              <button type="button" :disabled="!canSetMarketDataDir" @click="setMarketDataDirFromDraft">Set</button>
            </div>
          </div>
          <label class="inlineCheck">
            <input
              type="checkbox"
              :checked="moveExisting"
              @change="setCheckboxValue('settingsMoveExisting', ($event.target as HTMLInputElement).checked)"
            />
            Move existing Polygon cache
          </label>
          <div class="actions">
            <button type="button" :disabled="!canResetMarketDataDir" @click="settingsStore.resetMarketDataDir()">Reset to default</button>
            <button type="button" :disabled="!canOpenMarketDataDir" @click="settingsStore.openMarketDataDir()">Open folder</button>
            <button type="button" :disabled="!canOpenWorkflowData" @click="settingsStore.openWorkflowData()">Open workflow data panel</button>
          </div>
          <div class="muted mono">{{ marketDataDirHint }}</div>
        </section>

        <section class="pane">
          <h3>Utilities</h3>
          <div class="actions">
            <button type="button" :disabled="!canOpenRunsDir" @click="settingsStore.openRunsDir()">Open runs folder</button>
            <button type="button" :disabled="!canOpenReportsDir" @click="settingsStore.openReportsDir()">Open reports folder</button>
            <button type="button" :disabled="!canOpenPolygonModal" @click="settingsStore.openPolygonModal()">Open Polygon downloader</button>
          </div>
          <div class="actions">
            <button type="button" :disabled="!canGotoLibrary" @click="settingsStore.gotoLibrary()">Go to Run Library</button>
            <button type="button" :disabled="!canGotoAggregate" @click="settingsStore.gotoAggregate()">Go to Aggregate Analysis</button>
          </div>
        </section>

        <section class="pane">
          <h3>Keyboard shortcuts</h3>
          <div class="shortcutGrid">
            <div class="mono">Ctrl+K</div>
            <div>Command palette</div>
            <div class="mono">Ctrl+Enter</div>
            <div>Run optimization</div>
            <div class="mono">Ctrl+Shift+V</div>
            <div>Run validation</div>
            <div class="mono">Ctrl+Shift+C</div>
            <div>Export top rows CSV</div>
            <div class="mono">Ctrl+O</div>
            <div>Browse CSV file</div>
          </div>
        </section>
      </div>

      <div class="status mono">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.settingsPanelVueRoot {
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

.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
}

.rowTwo {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.7rem;
}

.checks {
  margin-top: 0.65rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.inputWithActions {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.actions {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.inlineCheck {
  margin-top: 0.6rem;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.shortcutGrid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.35rem 0.8rem;
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
  margin-top: 0.45rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1040px) {
  .grid {
    grid-template-columns: 1fr;
  }

  .rowTwo {
    grid-template-columns: 1fr;
  }

  .inputWithActions {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
#settingsShellGrid[data-vue-settings='1'] > :not(#settingsVueShellMount):not(#executionShellPane) {
  display: none !important;
}
</style>



