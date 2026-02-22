<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'

const settings = useSettingsStore()
const workflowState = useWorkflowStore()
const {
  theme,
  effect,
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

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

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
      <div class="title">Settings Context</div>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row">
        <span>Theme</span>
        <select :value="theme" @change="updateSelect('settingsThemeSelect', String(($event.target as HTMLSelectElement).value || ''))">
          <option value="light">light</option>
          <option value="dark">dark</option>
          <option value="system">system</option>
        </select>
      </div>
      <div class="row">
        <span>Effect</span>
        <select :value="effect" @change="updateSelect('settingsEffectSelect', String(($event.target as HTMLSelectElement).value || ''))">
          <option value="none">none</option>
          <option value="mica">mica</option>
          <option value="acrylic">acrylic</option>
          <option value="vibrancy">vibrancy</option>
        </select>
      </div>
      <div class="row"><span>Advanced</span><strong class="mono">{{ workflowAdvanced ? 'on' : 'off' }}</strong></div>
      <div class="row"><span>Collapsed</span><strong class="mono">{{ controlsCollapsed ? 'yes' : 'no' }}</strong></div>
      <div class="row"><span>Move cache</span><strong class="mono">{{ moveExisting ? 'yes' : 'no' }}</strong></div>
      <div class="row"><span>Data dir</span><strong class="mono">{{ marketDataLabel }}</strong></div>
      <div class="row"><span>Hint</span><strong class="mono">{{ marketDataDirHint }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" @click="setCheckboxValue('settingsWorkflowAdvanced', !workflowAdvanced)">Toggle advanced</button>
      <button type="button" @click="setCheckboxValue('settingsControlsCollapsed', !controlsCollapsed)">Toggle panel</button>
      <button type="button" @click="setCheckboxValue('settingsMoveExisting', !moveExisting)">Toggle move cache</button>
      <button type="button" @click="settings.browseMarketDataDir()">Browse dir</button>
      <button type="button" @click="setMarketDataDir">Set dir</button>
      <button type="button" @click="settings.resetMarketDataDir()">Reset dir</button>
      <button type="button" @click="settings.openMarketDataDir()">Open dir</button>
      <button type="button" @click="settings.openRunsDir()">Runs</button>
      <button type="button" @click="settings.openReportsDir()">Reports</button>
      <button type="button" @click="settings.openWorkflowData()">Workflow data</button>
    </div>
  </aside>
</template>

<style scoped>
.settingsDock {
  position: fixed;
  left: 10px;
  top: 134px;
  z-index: 4037;
  width: min(440px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(251, 191, 36, 0.42);
  background: linear-gradient(158deg, rgba(15, 23, 42, 0.94), rgba(69, 26, 3, 0.9));
  color: #e2e8f0;
  box-shadow: 0 18px 40px rgba(2, 6, 23, 0.45);
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
  color: #fcd34d;
}

.stamp {
  font-size: 0.72rem;
  color: #fde68a;
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
  color: #fbbf24;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.row strong {
  font-size: 0.78rem;
  color: #e2e8f0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row select {
  width: 100%;
  border: 1px solid rgba(251, 191, 36, 0.36);
  border-radius: 8px;
  padding: 3px 6px;
  font-size: 0.75rem;
  background: rgba(69, 26, 3, 0.82);
  color: #fef3c7;
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
  border: 1px solid rgba(251, 191, 36, 0.38);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(69, 26, 3, 0.82);
  color: #fef3c7;
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
</style>

