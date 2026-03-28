<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import {
  ArrowRightLeft,
  Check,
  Database,
  FileSpreadsheet,
  Folder,
  FolderOpen,
  FolderSearch,
  PanelLeftClose,
  RotateCcw,
  SlidersHorizontal,
} from 'lucide-vue-next'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const settings = useSettingsStore()
const workflowState = useWorkflowStore()
const {
  theme,
  effect,
  helpMode,
  workflowAdvanced,
  controlsCollapsed,
  marketDataDir,
  moveExisting,
  marketDataDirHint,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(settings)
const { workspace } = storeToRefs(workflowState)

const visible = computed(() => String(workspace.value || '').trim() === 'settings')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const marketDataLabel = computed(() => marketDataDir.value || '(default)')

function setMarketDataDir(): void {
  settings.setMarketDataDir(String(marketDataDir.value || '').trim(), !!moveExisting.value)
}

function setCheckboxValue(id: string, checked: boolean): void {
  settings.setCheckboxField(id, checked)
}

function updateSelect(id: string, nextValue: string): void {
  settings.setSelectField(id, nextValue)
}
</script>

<template>
  <aside v-if="visible" class="settingsDock">
    <header class="dockHeader">
      <h2 class="title" style="margin: 0;">Settings Context</h2>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row">
        <span>Theme</span>
        <select
          :value="theme"
          aria-label="Theme"
          @change="updateSelect('settingsThemeSelect', String(($event.target as HTMLSelectElement).value || ''))"
        >
          <option value="light">light</option>
          <option value="dark">dark</option>
          <option value="system">system</option>
        </select>
      </div>
      <div class="row">
        <span>Effect</span>
        <select
          :value="effect"
          aria-label="Window effect"
          @change="updateSelect('settingsEffectSelect', String(($event.target as HTMLSelectElement).value || ''))"
        >
          <option value="none">none</option>
          <option value="mica">mica</option>
          <option value="acrylic">acrylic</option>
          <option value="vibrancy">vibrancy</option>
        </select>
      </div>
      <div class="row"><span>Help</span><strong class="mono">{{ helpMode }}</strong></div>
      <div class="row"><span>Advanced</span><strong class="mono">{{ workflowAdvanced ? 'on' : 'off' }}</strong></div>
      <div class="row"><span>Collapsed</span><strong class="mono">{{ controlsCollapsed ? 'yes' : 'no' }}</strong></div>
      <div class="row"><span>Move cache</span><strong class="mono">{{ moveExisting ? 'yes' : 'no' }}</strong></div>
      <div class="row"><span>Data dir</span><strong class="mono">{{ marketDataLabel }}</strong></div>
      <div class="row"><span>Hint</span><strong class="mono">{{ marketDataDirHint }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" @click="setCheckboxValue('settingsWorkflowAdvanced', !workflowAdvanced)">
        <SlidersHorizontal class="actionIcon" aria-hidden="true" />
        <span>Toggle advanced</span>
      </button>
      <button type="button" @click="setCheckboxValue('settingsControlsCollapsed', !controlsCollapsed)">
        <PanelLeftClose class="actionIcon" aria-hidden="true" />
        <span>Toggle panel</span>
      </button>
      <button type="button" @click="setCheckboxValue('settingsMoveExisting', !moveExisting)">
        <ArrowRightLeft class="actionIcon" aria-hidden="true" />
        <span>Toggle move cache</span>
      </button>
      <button type="button" @click="settings.browseMarketDataDir()">
        <FolderSearch class="actionIcon" aria-hidden="true" />
        <span>Browse dir</span>
      </button>
      <button type="button" @click="setMarketDataDir">
        <Check class="actionIcon" aria-hidden="true" />
        <span>Set dir</span>
      </button>
      <button type="button" @click="settings.resetMarketDataDir()">
        <RotateCcw class="actionIcon" aria-hidden="true" />
        <span>Reset dir</span>
      </button>
      <button type="button" @click="settings.openMarketDataDir()">
        <FolderOpen class="actionIcon" aria-hidden="true" />
        <span>Open dir</span>
      </button>
      <button type="button" @click="settings.openRunsDir()">
        <Folder class="actionIcon" aria-hidden="true" />
        <span>Runs</span>
      </button>
      <button type="button" @click="settings.openReportsDir()">
        <FileSpreadsheet class="actionIcon" aria-hidden="true" />
        <span>Reports</span>
      </button>
      <button type="button" @click="settings.openWorkflowData()">
        <Database class="actionIcon" aria-hidden="true" />
        <span>Workflow data</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.settingsDock {
  --dock-accent: var(--warning);
  --dock-accent-ink: var(--warning-ink);
  --dock-accent-soft: var(--warning-soft);
  --dock-surface: linear-gradient(
    158deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--warning-soft) 34%, var(--surface-sticky))
  );
  --dock-border: color-mix(in oklab, var(--dock-accent) 40%, var(--border));
  --dock-field-border: color-mix(in oklab, var(--dock-accent) 34%, var(--border));
  --dock-field-surface: color-mix(in oklab, var(--dock-accent-soft) 48%, var(--surface-inline));
  --dock-text: var(--text);
  --dock-title: color-mix(in oklab, var(--dock-accent) 76%, var(--dock-text));
  --dock-stamp: color-mix(in oklab, var(--dock-accent-ink) 72%, var(--dock-text));
  --dock-label: color-mix(in oklab, var(--dock-accent) 64%, var(--dock-text));
  --dock-action-text: color-mix(in oklab, var(--dock-accent-ink) 82%, var(--dock-text));
  position: fixed;
  left: 10px;
  top: 134px;
  z-index: 4037;
  width: min(440px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid var(--dock-border);
  background: var(--dock-surface);
  color: var(--dock-text);
  box-shadow: var(--shadow-focus);
  backdrop-filter: blur(12px);
  padding: 10px;
  display: grid;
  gap: 8px;
  pointer-events: auto;
}

.dockHeader {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--dock-title);
}

.stamp {
  font-size: 0.72rem;
  color: var(--dock-stamp);
}

.rows {
  display: grid;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 82px 1fr;
  gap: 6px;
  align-items: baseline;
}

.row span {
  font-size: 0.72rem;
  color: var(--dock-label);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.row strong {
  font-size: 0.78rem;
  color: var(--dock-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row select {
  width: 100%;
  border: 1px solid var(--dock-field-border);
  border-radius: 8px;
  padding: 3px 6px;
  font-size: 0.75rem;
  background: var(--dock-field-surface);
  color: var(--dock-action-text);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.actions button {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border: 1px solid var(--dock-field-border);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: var(--dock-field-surface);
  color: var(--dock-action-text);
}

.actionIcon {
  width: 0.82rem;
  height: 0.82rem;
  flex: none;
  stroke-width: 2;
}

@media (max-width: 760px) {
  .settingsDock {
    left: 8px;
    right: 8px;
    width: auto;
    top: auto;
    bottom: 8px;
  }
}

@media (max-width: 1180px) {
  .settingsDock {
    display: none;
  }
}
</style>

