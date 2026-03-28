<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useDataStore } from '../stores/dataStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const settingsStore = useSettingsStore()
const dataStore = useDataStore()
const workflowStore = useWorkflowStore()
const {
  theme,
  effect,
  helpMode,
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

const { mountReady } = useShellMount('#settingsVueShellMount', {
  paneSelector: '#settingsShellGrid',
  paneDatasetKey: 'vueSettings',
})
let runtimeObserver: MutationObserver | null = null

const themeDraft = ref('system')
const effectDraft = ref('none')
const helpModeDraft = ref('full')
const marketDataDirDraft = ref('')

const visible = computed(() => String(workspace.value || '').trim() === 'settings')
const settingsSummaryChips = computed(() => [
  `Theme ${String(themeDraft.value || 'system').trim() || 'system'}`,
  `Effect ${String(effectDraft.value || 'none').trim() || 'none'}`,
  `Help ${String(helpModeDraft.value || 'full').trim() || 'full'}`,
  workflowAdvanced.value ? 'Advanced controls on' : 'Advanced controls off',
  controlsCollapsed.value ? 'Controls collapsed first' : 'Controls expanded first',
  marketDataDirDraft.value.trim().length > 0 ? 'Custom data folder' : 'Default data folder',
])

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

watch(
  () => [
    String(theme.value || 'system').trim() || 'system',
    String(effect.value || 'none').trim() || 'none',
    String(helpMode.value || 'full').trim() || 'full',
    String(marketDataDir.value || ''),
  ] as const,
  ([nextTheme, nextEffect, nextHelpMode, nextMarketDataDir]) => {
    if (themeDraft.value !== nextTheme) themeDraft.value = nextTheme
    if (effectDraft.value !== nextEffect) effectDraft.value = nextEffect
    if (helpModeDraft.value !== nextHelpMode) helpModeDraft.value = nextHelpMode
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

onMounted(async () => {
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
})
</script>

<template>
  <Teleport v-if="mountReady" to="#settingsVueShellMount">
    <section class="settingsPanelVueRoot settingsVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Workspace</div>
          <h2 class="panelTitle">Settings</h2>
          <div class="panelSubtitle">
            Review appearance, storage paths, and local utility shortcuts without leaving the desktop workflow.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>
      <div class="panelSummaryChips" aria-label="Settings summary">
        <span v-for="chip in settingsSummaryChips" :key="`settings_summary_${chip}`" class="panelSummaryChip mono">
          {{ chip }}
        </span>
      </div>

      <div class="grid">
        <section class="pane panelProminentPane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Appearance</div>
              <h3 class="panelPaneTitle">Appearance</h3>
              <div class="panelSectionLead">
                Keep the shell readable first. Theme and window effects should support dense workflows rather than compete with them.
              </div>
            </div>
          </div>
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
          <div class="rowSingle">
            <label for="vueSettingsHelpMode">Help and guidance</label>
            <select
              id="vueSettingsHelpMode"
              :value="helpModeDraft"
              @change="helpModeDraft = String(($event.target as HTMLSelectElement).value || 'full'); setSelectValue('settingsHelpMode', helpModeDraft)"
            >
              <option value="full">Full help (tooltips + long-hover callouts)</option>
              <option value="concise">Concise help (tooltips only)</option>
              <option value="off">Off (hide non-essential help overlays)</option>
            </select>
            <div class="panelMuted">
              Use this as the global policy for optional in-app help. Safety notices, errors, and runtime status still stay visible.
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
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Storage</div>
              <h3 class="panelPaneTitle">Data and storage</h3>
              <div class="panelSectionLead">
                Point the app at the right market-data cache and decide whether existing local data should move with it.
              </div>
            </div>
          </div>
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
          <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ marketDataDirHint }}</div>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Utilities</div>
              <h3 class="panelPaneTitle">Utilities</h3>
              <div class="panelSectionLead">
                Jump to folders and adjacent workspaces when you need to inspect artifacts outside the main workflow view.
              </div>
            </div>
          </div>
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
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Navigation</div>
              <h3 class="panelPaneTitle">Keyboard shortcuts</h3>
              <div class="panelSectionLead">
                These shortcuts cover the highest-frequency commands so the main authoring flow stays keyboard-friendly.
              </div>
            </div>
          </div>
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

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
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
  align-items: flex-start;
  gap: 0.75rem;
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

.rowTwo {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.7rem;
}

.rowSingle {
  margin-top: 0.7rem;
  display: grid;
  gap: 0.35rem;
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



