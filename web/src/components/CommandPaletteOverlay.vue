<script setup lang="ts">
import Fuse from 'fuse.js'
import type { FuseResult } from 'fuse.js'
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
  group: 'actions' | 'data' | 'exports' | 'layout' | 'navigation'
  name: string
  desc: string
  keys: string[]
  keywords: string[]
  enabled: boolean
  run: () => void
}

interface PaletteCommandSearchEntry {
  command: PaletteCommand
  shortcut: string
}

interface PaletteCommandGroup {
  id: PaletteCommand['group']
  label: string
  items: Array<{
    command: PaletteCommand
    index: number
  }>
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
const commandListId = 'commandPaletteListbox'

const commandGroupMeta: Record<PaletteCommand['group'], string> = {
  actions: 'Run control',
  data: 'Data sources',
  exports: 'Exports and reports',
  layout: 'Layout',
  navigation: 'Navigation',
}
const commandGroupOrder: PaletteCommand['group'][] = ['actions', 'data', 'exports', 'layout', 'navigation']

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
      group: 'actions',
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
      group: 'actions',
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
      group: 'actions',
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
      group: 'data',
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
      group: 'data',
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
      group: 'exports',
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
      group: 'exports',
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
      group: 'exports',
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
      group: 'exports',
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
      group: 'layout',
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
      group: 'navigation',
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
      group: 'navigation',
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
      group: 'navigation',
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
      group: 'navigation',
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
      group: 'navigation',
      name: 'Go to Workflow Step 5 (Finalizer)',
      desc: 'Switch to the post-meta ML confidence and Stage 5 workflow step.',
      keys: [],
      keywords: ['navigate', 'workflow', 'finalize', 'stage5', 'stage4', 'ml', 'meta'],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded('finalize')
      },
    },
    {
      id: 'go_workspace_library',
      group: 'navigation',
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
      group: 'navigation',
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
      group: 'navigation',
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
      group: 'navigation',
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

const commandSearchIndex = computed<PaletteCommandSearchEntry[]>(() => commands.value.map((command) => ({
  command,
  shortcut: command.keys.join(' '),
})))
const commandOrder = computed(() => new Map(commands.value.map((command, index) => [command.id, index])))

const commandFuse = computed(() => new Fuse(commandSearchIndex.value, {
  includeScore: true,
  ignoreLocation: true,
  threshold: 0.34,
  keys: [
    { name: 'command.name', weight: 0.38 },
    { name: 'command.keywords', weight: 0.28 },
    { name: 'command.desc', weight: 0.18 },
    { name: 'shortcut', weight: 0.1 },
    { name: 'command.group', weight: 0.06 },
  ],
}))

function rankingBucket(command: PaletteCommandSearchEntry, tokens: string[]): number {
  if (!tokens.length) return 3

  const nameWords = command.command.name.toLowerCase().split(/\s+/g)
  const keywordValues = command.command.keywords.map((keyword) => keyword.toLowerCase())
  const shortcut = command.shortcut.toLowerCase()
  const exactNamePrefix = tokens.every((token) => nameWords.some((word) => word.startsWith(token)))
  if (exactNamePrefix) return 0

  const keywordPrefix = tokens.every((token) => keywordValues.some((keyword) => keyword.startsWith(token)))
  if (keywordPrefix) return 1

  const shortcutPrefix = tokens.every((token) => shortcut.includes(token))
  if (shortcutPrefix) return 2

  return 3
}

const filteredCommands = computed<PaletteCommand[]>(() => {
  const q = String(query.value || '').trim()
  if (!q) return commands.value
  const tokens = q.toLowerCase().split(/\s+/g).filter((token) => token.length > 0)

  const prefixMatches = commandSearchIndex.value
    .filter((entry) => rankingBucket(entry, tokens) <= 1)
    .sort((a: PaletteCommandSearchEntry, b: PaletteCommandSearchEntry) =>
      rankingBucket(a, tokens) - rankingBucket(b, tokens)
      || (commandOrder.value.get(a.command.id) ?? 0) - (commandOrder.value.get(b.command.id) ?? 0),
    )
  const seen = new Set(prefixMatches.map((entry) => entry.command.id))

  const fuzzyMatches = commandFuse.value
    .search(q)
    .sort((a: FuseResult<PaletteCommandSearchEntry>, b: FuseResult<PaletteCommandSearchEntry>) => {
      const bucketDiff = rankingBucket(a.item, tokens) - rankingBucket(b.item, tokens)
      if (bucketDiff !== 0) return bucketDiff
      return (a.score ?? Number.POSITIVE_INFINITY) - (b.score ?? Number.POSITIVE_INFINITY) || a.refIndex - b.refIndex
    })
    .map((entry: FuseResult<PaletteCommandSearchEntry>) => entry.item)
    .filter((entry: PaletteCommandSearchEntry) => !seen.has(entry.command.id))

  return [...prefixMatches, ...fuzzyMatches].map((entry) => entry.command)
})

const groupedFilteredCommands = computed<PaletteCommandGroup[]>(() => {
  const groups = new Map<PaletteCommand['group'], PaletteCommandGroup['items']>()
  const q = String(query.value || '').trim()
  const orderedGroupIds: PaletteCommand['group'][] = []
  filteredCommands.value.forEach((command, index) => {
    const items = groups.get(command.group) ?? []
    items.push({ command, index })
    groups.set(command.group, items)
    if (q && !orderedGroupIds.includes(command.group)) {
      orderedGroupIds.push(command.group)
    }
  })
  const visibleGroupOrder = q ? orderedGroupIds : commandGroupOrder
  return visibleGroupOrder
    .map((groupId) => ({
      id: groupId,
      label: commandGroupMeta[groupId],
      items: groups.get(groupId) ?? [],
    }))
    .filter((group) => group.items.length > 0)
})

const searchSummaryId = 'commandPaletteSummary'
const searchSummary = computed(() => {
  const total = commands.value.length
  const matches = filteredCommands.value.length
  const q = String(query.value || '').trim()
  if (!q) return `${total} commands. Search by action, view, report, or shortcut.`
  const label = matches === 1 ? 'match' : 'matches'
  return `${matches} ${label} for “${q}”.`
})

function commandOptionId(commandId: string): string {
  return `command_option_${String(commandId || '').trim()}`
}

const selectedCommandId = computed(() => {
  const command = filteredCommands.value[selectedIndex.value] ?? null
  return command ? commandOptionId(command.id) : undefined
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

function onKeydown(event: KeyboardEvent): void {
  if (!open.value) return
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
  if (event.key === 'Home') {
    event.preventDefault()
    commandPaletteStore.setSelection(0, filteredCommands.value.length)
    return
  }
  if (event.key === 'End') {
    event.preventDefault()
    commandPaletteStore.setSelection(filteredCommands.value.length - 1, filteredCommands.value.length)
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
          <button type="button" class="closeBtn" aria-label="Close command palette" @click="closePalette">Close</button>
        </header>
        <DialogDescription class="srOnly">
          Search commands, navigate workflow views, and execute supported actions.
        </DialogDescription>
        <input
          ref="inputRef"
          type="text"
          class="query"
          role="combobox"
          aria-autocomplete="list"
          aria-haspopup="listbox"
          :aria-expanded="open ? 'true' : 'false'"
          :aria-controls="commandListId"
          :aria-activedescendant="selectedCommandId"
          :aria-describedby="searchSummaryId"
          :value="query"
          placeholder="Type a command…"
          @input="commandPaletteStore.setQuery(String(($event.target as HTMLInputElement).value || ''))"
        />
        <div :id="searchSummaryId" class="summary" aria-live="polite">{{ searchSummary }}</div>
        <div :id="commandListId" class="list overflowShadowY" role="listbox" aria-label="Available commands">
          <section
            v-for="group in groupedFilteredCommands"
            :key="group.id"
            class="group"
            role="group"
            :aria-labelledby="`commandPaletteGroup_${group.id}`"
          >
            <div :id="`commandPaletteGroup_${group.id}`" class="groupLabel">{{ group.label }}</div>
            <div
              v-for="{ command, index } in group.items"
              :key="command.id"
              :id="commandOptionId(command.id)"
              class="row"
              :class="{ selected: selectedIndex === index, disabled: !command.enabled }"
              role="option"
              :aria-selected="selectedIndex === index"
              :aria-disabled="!command.enabled"
              @mouseenter="commandPaletteStore.setSelection(index, filteredCommands.length)"
              @mousedown.prevent
              @click="runCommand(command)"
            >
              <div class="meta">
                <div class="name">{{ command.name }}</div>
                <div class="desc">{{ command.desc }}</div>
              </div>
              <div class="keys">
                <kbd v-for="key in command.keys" :key="`${command.id}_${key}`">{{ key }}</kbd>
              </div>
            </div>
          </section>
          <div v-if="!filteredCommands.length" class="empty">No matching commands. Try workflow, report, or shortcut terms.</div>
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
  background: color-mix(in oklab, var(--backdrop) 88%, var(--bg));
}

.cmdPaletteDialog {
  position: fixed;
  top: 16%;
  left: 50%;
  transform: translateX(-50%);
  z-index: 9999;
  width: min(760px, calc(100vw - 24px));
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 72%, var(--border));
  border-radius: 14px;
  background: linear-gradient(
    145deg,
    color-mix(in oklab, var(--surface-focus) 92%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 94%, var(--surface-working))
  );
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
  background: color-mix(in oklab, var(--surface-inline) 92%, var(--panel));
  padding: 0.15rem 0.4rem;
}

.query {
  width: 100%;
}

.summary {
  font-size: 0.75rem;
  color: var(--muted);
}

.list {
  max-height: min(56vh, 480px);
  overflow: auto;
  display: grid;
  gap: 0.75rem;
}

.group {
  display: grid;
  gap: 0.35rem;
}

.groupLabel {
  font-size: 0.72rem;
  font-weight: 650;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: color-mix(in oklab, var(--muted) 86%, var(--ink));
}

.row {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
  color: inherit;
  cursor: pointer;
  text-align: left;
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.45rem 0.55rem;
}

.row.selected {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 74%, var(--accent));
  background: color-mix(in oklab, var(--selection-fill-strong) 82%, var(--surface-working));
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.row.disabled {
  opacity: 0.58;
  cursor: default;
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
