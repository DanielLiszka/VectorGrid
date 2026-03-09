<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from 'reka-ui'
import { useAnalysisStore } from '../stores/analysisStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'

interface PaletteCommand {
  id: string
  name: string
  desc: string
  keys: string[]
  keywords: string[]
  enabled: boolean
  run: () => void
}

const analysisStore = useAnalysisStore()
const commandActionStore = useCommandActionStore()
const commandPaletteStore = useCommandPaletteStore()
const dataStore = useDataStore()
const optimizeStore = useOptimizeStore()
const workflowStore = useWorkflowStore()

const { canExportCsv, canExportJson } = storeToRefs(analysisStore)
const { canOpenLatestMetaReport, canOpenLatestReport } = storeToRefs(commandActionStore)
const { open, query, selectedIndex } = storeToRefs(commandPaletteStore)
const { canBrowseCsv, canPolygonOpen } = storeToRefs(dataStore)
const { canCancel, runDisabled, validateDisabled } = storeToRefs(optimizeStore)
const { workspace, workflowStep } = storeToRefs(workflowStore)

function setWorkflowStepIfNeeded(step: string): void {
  if (String(workspace.value || '').trim() === 'workflow' && String(workflowStep.value || '').trim() === step) return
  workflowStore.selectWorkflowStep(step)
}

function setWorkspaceIfNeeded(nextWorkspace: string): void {
  if (String(workspace.value || '').trim() === nextWorkspace) return
  workflowStore.selectWorkspace(nextWorkspace)
}

const inputRef = ref<HTMLInputElement | null>(null)

const commands = computed<PaletteCommand[]>(() => {
  const canRun = !runDisabled.value
  const canValidate = !validateDisabled.value
  return [
    {
      id: 'run',
      name: 'Run optimization',
      desc: 'Start optimization for current setup.',
      keys: ['Ctrl', 'Enter'],
      keywords: ['run', 'optimize', 'backtest'],
      enabled: canRun,
      run: () => {
        commandActionStore.runOptimization()
      },
    },
    {
      id: 'cancel',
      name: 'Cancel run',
      desc: 'Cancel active optimization or validation task.',
      keys: ['Esc'],
      keywords: ['cancel', 'stop'],
      enabled: canCancel.value,
      run: () => {
        commandActionStore.cancelOptimization()
      },
    },
    {
      id: 'validate',
      name: 'Run validation',
      desc: 'Execute validation checks for shortlisted candidates.',
      keys: ['Ctrl', 'Shift', 'V'],
      keywords: ['validate', 'robustness', 'shortlist'],
      enabled: canValidate,
      run: () => {
        commandActionStore.validateOptimization()
      },
    },
    {
      id: 'browse_csv',
      name: 'Browse CSV',
      desc: 'Pick dataset CSV from disk.',
      keys: ['Ctrl', 'O'],
      keywords: ['csv', 'data', 'browse'],
      enabled: canBrowseCsv.value,
      run: () => {
        commandActionStore.browseCsv()
      },
    },
    {
      id: 'open_polygon',
      name: 'Open Polygon Downloader',
      desc: 'Open Polygon dataset downloader modal.',
      keys: ['Ctrl', 'Shift', 'O'],
      keywords: ['polygon', 'download', 'market'],
      enabled: canPolygonOpen.value,
      run: () => {
        commandActionStore.openPolygon()
      },
    },
    {
      id: 'export_csv',
      name: 'Export Top CSV',
      desc: 'Export current top table as CSV.',
      keys: ['Ctrl', 'Shift', 'C'],
      keywords: ['export', 'csv', 'top'],
      enabled: canExportCsv.value,
      run: () => {
        commandActionStore.exportTopCsv()
      },
    },
    {
      id: 'export_json',
      name: 'Export Top JSON',
      desc: 'Export current top result as JSON.',
      keys: ['Ctrl', 'Shift', 'J'],
      keywords: ['export', 'json', 'top'],
      enabled: canExportJson.value,
      run: () => {
        commandActionStore.exportTopJson()
      },
    },
    {
      id: 'open_latest_report',
      name: 'Open latest report',
      desc: 'Open most recent generated report.',
      keys: ['Ctrl', 'Shift', 'R'],
      keywords: ['report', 'xlsx'],
      enabled: canOpenLatestReport.value,
      run: () => {
        commandActionStore.openLatestReport()
      },
    },
    {
      id: 'open_latest_meta_report',
      name: 'Open latest meta report',
      desc: 'Open most recent meta-analysis report.',
      keys: ['Ctrl', 'Shift', 'M'],
      keywords: ['meta', 'report', 'xlsx'],
      enabled: canOpenLatestMetaReport.value,
      run: () => {
        commandActionStore.openLatestMetaReport()
      },
    },
    {
      id: 'toggle_controls',
      name: 'Toggle controls panel',
      desc: 'Collapse or expand controls column.',
      keys: ['Ctrl', '\\'],
      keywords: ['controls', 'sidebar', 'layout'],
      enabled: true,
      run: () => {
        commandActionStore.toggleControls()
      },
    },
    {
      id: 'go_step_data',
      name: 'Go to Workflow Step 1 (Data)',
      desc: 'Switch to data-source workflow step.',
      keys: [],
      keywords: ['navigate', 'workflow', 'data'],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded('data')
      },
    },
    {
      id: 'go_step_optimize',
      name: 'Go to Workflow Step 2 (Optimize)',
      desc: 'Switch to strategy setup workflow step.',
      keys: [],
      keywords: ['navigate', 'workflow', 'optimize'],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded('optimize')
      },
    },
    {
      id: 'go_step_validate',
      name: 'Go to Workflow Step 3 (Validate)',
      desc: 'Switch to validation workflow step.',
      keys: [],
      keywords: ['navigate', 'workflow', 'validate'],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded('validate')
      },
    },
    {
      id: 'go_step_analyze',
      name: 'Go to Workflow Step 4 (Analyze)',
      desc: 'Switch to analysis workflow step.',
      keys: [],
      keywords: ['navigate', 'workflow', 'analyze'],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded('analyze')
      },
    },
    {
      id: 'go_step_finalize',
      name: 'Go to Workflow Step 5 (Finalizer)',
      desc: 'Switch to the post-meta ML and Stage 4 workflow step.',
      keys: [],
      keywords: ['navigate', 'workflow', 'finalize', 'stage4', 'ml', 'meta'],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded('finalize')
      },
    },
    {
      id: 'go_workspace_library',
      name: 'Go to Run Library',
      desc: 'Switch to saved run library workspace.',
      keys: [],
      keywords: ['navigate', 'library', 'history'],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded('library')
      },
    },
    {
      id: 'go_workspace_aggregate',
      name: 'Go to Aggregate Analysis',
      desc: 'Switch to aggregate/meta analysis workspace.',
      keys: [],
      keywords: ['navigate', 'aggregate', 'meta'],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded('aggregate')
      },
    },
    {
      id: 'go_workspace_pro',
      name: 'Go to Pro Tools',
      desc: 'Switch to pro/debug workspace.',
      keys: [],
      keywords: ['navigate', 'pro', 'debug'],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded('pro')
      },
    },
    {
      id: 'go_workspace_settings',
      name: 'Go to Settings',
      desc: 'Switch to global settings workspace.',
      keys: [],
      keywords: ['navigate', 'settings'],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded('settings')
      },
    },
  ]
})

function commandScore(command: PaletteCommand, tokens: string[]): number | null {
  const haystack = `${command.name} ${command.desc} ${command.keywords.join(' ')}`.toLowerCase()
  let score = 0
  for (const token of tokens) {
    const idx = haystack.indexOf(token)
    if (idx < 0) return null
    score += idx
  }
  return score
}

const filteredCommands = computed<PaletteCommand[]>(() => {
  const q = String(query.value || '').trim().toLowerCase()
  const tokens = q ? q.split(/\s+/g).filter((token) => token.length > 0) : []
  if (!tokens.length) return commands.value

  const scored: Array<{ command: PaletteCommand; score: number }> = []
  for (const command of commands.value) {
    const score = commandScore(command, tokens)
    if (score == null) continue
    scored.push({ command, score })
  }
  scored.sort((a, b) => a.score - b.score)
  return scored.map((entry) => entry.command)
})

watch(filteredCommands, (commandsNow) => {
  commandPaletteStore.clampSelection(commandsNow.length)
})

watch(
  open,
  async (isOpen) => {
    if (!isOpen) return
    await nextTick()
    const input = inputRef.value
    if (!input) return
    input.focus()
    input.select()
  },
  { immediate: true },
)

function closePalette(): void {
  commandPaletteStore.closePalette()
}

function onOpenChange(nextOpen: boolean): void {
  if (nextOpen) return
  closePalette()
}

function runCommand(command: PaletteCommand): void {
  if (!command.enabled) return
  command.run()
  closePalette()
}

function runSelected(): void {
  const command = filteredCommands.value[selectedIndex.value] ?? null
  if (!command) return
  runCommand(command)
}

function focusableElements(root: HTMLElement): HTMLElement[] {
  const selectors =
    'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
  return Array.from(root.querySelectorAll<HTMLElement>(selectors)).filter((el) => !el.hasAttribute('disabled'))
}

function onKeydown(event: KeyboardEvent): void {
  if (!open.value) return
  if (event.key === 'Tab') {
    const targetRootNode = event.target instanceof HTMLElement ? event.target.closest('.cmdPaletteDialog') : null
    const currentRootNode = event.currentTarget instanceof HTMLElement ? event.currentTarget.closest('.cmdPaletteDialog') : null
    const targetRoot = targetRootNode instanceof HTMLElement ? targetRootNode : null
    const currentRoot = currentRootNode instanceof HTMLElement ? currentRootNode : null
    const root = targetRoot ?? currentRoot ?? null
    if (!root) return
    const focusables = focusableElements(root)
    if (!focusables.length) {
      event.preventDefault()
      return
    }

    const active = document.activeElement instanceof HTMLElement ? document.activeElement : null
    const currentIndex = active ? focusables.indexOf(active) : -1
    const step = event.shiftKey ? -1 : 1
    const nextIndex = currentIndex < 0 ? 0 : (currentIndex + step + focusables.length) % focusables.length
    event.preventDefault()
    focusables[nextIndex]?.focus()
    return
  }
  if (event.key === 'Escape') {
    event.preventDefault()
    closePalette()
    return
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    commandPaletteStore.moveSelection(1, filteredCommands.value.length)
    return
  }
  if (event.key === 'ArrowUp') {
    event.preventDefault()
    commandPaletteStore.moveSelection(-1, filteredCommands.value.length)
    return
  }
  if (event.key === 'Enter') {
    event.preventDefault()
    runSelected()
  }
}
</script>

<template>
  <DialogRoot :open="open" @update:open="onOpenChange">
    <DialogPortal>
      <DialogOverlay class="cmdPaletteBackdrop" />
      <DialogContent class="cmdPaletteDialog" aria-label="Command palette" @keydown.capture="onKeydown">
        <header class="head">
          <DialogTitle class="title">Command Palette</DialogTitle>
          <button type="button" class="closeBtn" @click="closePalette">x</button>
        </header>
        <DialogDescription class="srOnly">
          Search commands, navigate workflow views, and execute supported actions.
        </DialogDescription>
        <input
          ref="inputRef"
          type="text"
          class="query"
          :value="query"
          placeholder="Type a command..."
          @input="commandPaletteStore.setQuery(String(($event.target as HTMLInputElement).value || ''))"
        />
        <div class="list">
          <button
            v-for="(command, index) in filteredCommands"
            :key="command.id"
            type="button"
            class="row"
            :class="{ selected: selectedIndex === index, disabled: !command.enabled }"
            @mouseenter="commandPaletteStore.selectedIndex = index"
            @click="runCommand(command)"
          >
            <div class="meta">
              <div class="name">{{ command.name }}</div>
              <div class="desc">{{ command.desc }}</div>
            </div>
            <div class="keys">
              <kbd v-for="key in command.keys" :key="`${command.id}_${key}`">{{ key }}</kbd>
            </div>
          </button>
          <div v-if="!filteredCommands.length" class="empty">No matching commands.</div>
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<style scoped>
.cmdPaletteBackdrop {
  position: fixed;
  inset: 0;
  z-index: 9998;
  border: 0;
  background: color-mix(in oklab, var(--bg) 25%, #000);
}

.cmdPaletteDialog {
  position: fixed;
  top: 16%;
  left: 50%;
  transform: translateX(-50%);
  z-index: 9999;
  width: min(760px, calc(100vw - 24px));
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--panel);
  box-shadow: var(--shadow-lg);
  padding: 0.7rem;
  display: grid;
  gap: 0.6rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 0.82rem;
  font-weight: 650;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--muted);
}

.closeBtn {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface2);
  padding: 0.15rem 0.4rem;
}

.query {
  width: 100%;
}

.list {
  max-height: min(56vh, 480px);
  overflow: auto;
  display: grid;
  gap: 0.35rem;
}

.row {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface2);
  color: inherit;
  text-align: left;
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.45rem 0.55rem;
}

.row.selected {
  border-color: color-mix(in oklab, var(--accent) 62%, var(--border));
  background: color-mix(in oklab, var(--accent-soft) 68%, var(--surface2));
}

.row.disabled {
  opacity: 0.58;
}

.meta {
  display: grid;
  gap: 0.15rem;
}

.name {
  font-size: 0.83rem;
  font-weight: 600;
}

.desc {
  font-size: 0.75rem;
  color: var(--muted);
}

.keys {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

kbd {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.05rem 0.35rem;
  font-size: 0.7rem;
  background: var(--panel-footer);
}

.empty {
  border: 1px dashed var(--border);
  border-radius: 10px;
  color: var(--muted);
  font-size: 0.8rem;
  padding: 0.6rem;
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>
