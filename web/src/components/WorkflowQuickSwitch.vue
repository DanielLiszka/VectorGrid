<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import {
  PopoverContent,
  PopoverPortal,
  PopoverRoot,
  PopoverTrigger,
  ToolbarButton,
  ToolbarRoot,
} from 'reka-ui'
import { useWorkflowStore } from '../stores/workflowStore'

const store = useWorkflowStore()
const { workspace, workflow, workflowStep, workflowSubtitle } = storeToRefs(store)
const open = useStorage<boolean>('VTA_WORKFLOW_NAV_OPEN_V1', false)

const wsItems = [
  { key: 'workflow', label: 'Workflow' },
  { key: 'library', label: 'Library' },
  { key: 'aggregate', label: 'Aggregate' },
  { key: 'pro', label: 'Pro' },
  { key: 'live', label: 'Live' },
  { key: 'settings', label: 'Settings' },
] as const

const wfItems = [
  { key: 'data', label: 'Data' },
  { key: 'optimize', label: 'Optimize' },
  { key: 'validate', label: 'Validate' },
  { key: 'analyze', label: 'Analyze' },
  { key: 'finalize', label: 'Finalizer' },
] as const

const stepItems = [
  { key: 'data', label: '1' },
  { key: 'optimize', label: '2' },
  { key: 'validate', label: '3' },
  { key: 'analyze', label: '4' },
  { key: 'finalize', label: '5' },
] as const

const subtitle = computed(() => workflowSubtitle.value || '—')
const currentWorkspace = computed(() => String(workspace.value || '').trim() || 'workflow')
const currentWorkflow = computed(() => String(workflow.value || '').trim() || 'data')
const currentWorkflowStep = computed(() => String(workflowStep.value || '').trim() || 'data')

function close(): void {
  open.value = false
}

function clickWorkspace(key: string): void {
  if (String(workspace.value || '').trim() === key) return
  store.selectWorkspace(key)
}

function clickWorkflow(key: string): void {
  if (String(workflow.value || '').trim() === key) return
  store.selectWorkflow(key)
}

function clickWorkflowStep(key: string): void {
  if (String(workspace.value || '').trim() === 'workflow' && String(workflowStep.value || '').trim() === key) return
  store.selectWorkflowStep(key)
}

function switchItemId(group: string, key: string): string {
  return `workflow_switch_${group}_${key}`
}

type SwitchItem = { key: string }

function resolveFocusedKey(event: KeyboardEvent): string | null {
  const target = event.target instanceof Element ? (event.target.closest('[data-switch-key]') as HTMLElement | null) : null
  return String(target?.dataset.switchKey || '').trim() || null
}

function moveGroupSelection(
  event: KeyboardEvent,
  items: readonly SwitchItem[],
  currentKey: string,
  selectValue: (key: string) => void,
): void {
  if (items.length <= 1) return
  const originKey = resolveFocusedKey(event) ?? currentKey
  const currentIndex = items.findIndex((item) => item.key === originKey)
  if (currentIndex < 0) return

  let nextIndex = currentIndex
  if (event.key === 'ArrowRight' || event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % items.length
  else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') nextIndex = (currentIndex - 1 + items.length) % items.length
  else if (event.key === 'Home') nextIndex = 0
  else if (event.key === 'End') nextIndex = items.length - 1
  else return

  const nextKey = items[nextIndex]?.key
  if (!nextKey) return
  selectValue(nextKey)
}

function onWorkspaceKeydown(event: KeyboardEvent): void {
  moveGroupSelection(event, wsItems, currentWorkspace.value, clickWorkspace)
}

function onWorkflowKeydown(event: KeyboardEvent): void {
  moveGroupSelection(event, wfItems, currentWorkflow.value, clickWorkflow)
}

function onStepKeydown(event: KeyboardEvent): void {
  moveGroupSelection(event, stepItems, currentWorkflowStep.value, clickWorkflowStep)
}
</script>

<template>
  <div class="workflowSwitchRoot">
    <PopoverRoot v-model:open="open">
      <PopoverTrigger as-child>
        <button class="workflowSwitchToggle" type="button">Workflow Nav</button>
      </PopoverTrigger>
      <PopoverPortal>
        <PopoverContent class="workflowSwitch" side="bottom" align="start" :side-offset="10" :collision-padding="10">
          <header class="head">
            <h2 class="title">Workflow Nav</h2>
            <div class="headRight">
              <div class="mono state">ws={{ workspace }} · wf={{ workflow }} · step={{ workflowStep }}</div>
              <button type="button" class="closeBtn" @click="close">Close</button>
            </div>
          </header>
          <div class="subtitle mono">{{ subtitle }}</div>
          <nav class="groupBlock" aria-label="Workspace quick switch">
            <div class="groupLabel">Workspace</div>
            <ToolbarRoot
              class="group"
              data-switch-group="workspace"
              as="div"
              aria-label="Workspace choices"
              :loop="true"
              @keydown="onWorkspaceKeydown"
            >
              <ToolbarButton
                v-for="item in wsItems"
                :key="`ws_${item.key}`"
                :id="switchItemId('workspace', item.key)"
                :class="{ active: currentWorkspace === item.key }"
                :aria-current="currentWorkspace === item.key ? 'page' : undefined"
                :data-active="currentWorkspace === item.key ? '' : undefined"
                :data-switch-key="item.key"
                :data-workspace="item.key"
                @click="clickWorkspace(item.key)"
                @focus="clickWorkspace(item.key)"
              >
                {{ item.label }}
              </ToolbarButton>
            </ToolbarRoot>
          </nav>
          <nav class="groupBlock" aria-label="Workflow quick switch">
            <div class="groupLabel">Workflow</div>
            <ToolbarRoot
              class="group"
              data-switch-group="workflow"
              as="div"
              aria-label="Workflow choices"
              :loop="true"
              @keydown="onWorkflowKeydown"
            >
              <ToolbarButton
                v-for="item in wfItems"
                :key="`wf_${item.key}`"
                :id="switchItemId('workflow', item.key)"
                :class="{ active: currentWorkflow === item.key }"
                :aria-current="currentWorkflow === item.key ? 'page' : undefined"
                :data-active="currentWorkflow === item.key ? '' : undefined"
                :data-switch-key="item.key"
                :data-workflow="item.key"
                @click="clickWorkflow(item.key)"
                @focus="clickWorkflow(item.key)"
              >
                {{ item.label }}
              </ToolbarButton>
            </ToolbarRoot>
          </nav>
          <nav class="groupBlock" aria-label="Workflow step quick switch">
            <div class="groupLabel">Step</div>
            <ToolbarRoot
              class="group"
              data-switch-group="step"
              as="div"
              aria-label="Workflow step choices"
              :loop="true"
              @keydown="onStepKeydown"
            >
              <ToolbarButton
                v-for="item in stepItems"
                :key="`step_${item.key}`"
                :id="switchItemId('step', item.key)"
                class="step"
                :class="{ active: currentWorkflowStep === item.key }"
                :aria-current="currentWorkflowStep === item.key ? 'step' : undefined"
                :data-active="currentWorkflowStep === item.key ? '' : undefined"
                :data-switch-key="item.key"
                :data-workflow-step="item.key"
                @click="clickWorkflowStep(item.key)"
                @focus="clickWorkflowStep(item.key)"
              >
                {{ item.label }}
              </ToolbarButton>
            </ToolbarRoot>
          </nav>
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
  </div>
</template>

<style scoped>
.workflowSwitchRoot {
  --switch-accent: var(--accent);
  --switch-accent-ink: var(--selection-text);
  --switch-accent-soft: var(--selection-fill);
  --switch-border: color-mix(in oklab, var(--switch-accent) 40%, var(--border));
  --switch-surface: color-mix(in oklab, var(--surface-sticky) 94%, var(--panel-footer));
  --switch-surface-strong: linear-gradient(
    150deg,
    color-mix(in oklab, var(--surface-focus) 92%, var(--panel-footer)),
    color-mix(in oklab, var(--selection-fill) 34%, var(--surface-sticky))
  );
  --switch-text: var(--text);
  --switch-muted: var(--muted);
  position: fixed;
  top: 84px;
  left: 12px;
  z-index: 4039;
  pointer-events: none;
}

.workflowSwitchRoot > * {
  pointer-events: auto;
}

.workflowSwitchToggle {
  border: 1px solid var(--switch-border);
  border-radius: 999px;
  background: var(--switch-surface);
  color: var(--switch-text);
  padding: 6px 11px;
  font-size: 0.76rem;
  font-weight: 650;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  box-shadow: var(--shadow-inline);
  backdrop-filter: blur(10px);
}

.workflowSwitch {
  z-index: 4040;
  width: min(880px, calc(100vw - 24px));
  border: 1px solid var(--switch-border);
  border-radius: 12px;
  background: var(--switch-surface-strong);
  color: var(--switch-text);
  padding: 8px;
  display: grid;
  gap: 6px;
  pointer-events: auto;
  transform-origin: top left;
  box-shadow: var(--shadow-focus);
  backdrop-filter: blur(12px);
}

.workflowSwitch[data-state='open'] {
  animation: workflowNavIn 140ms ease;
}

@keyframes workflowNavIn {
  from {
    opacity: 0;
    transform: scale(0.98);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.head {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: center;
}

.headRight {
  display: flex;
  align-items: center;
  gap: 6px;
}

.title {
  margin: 0;
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: color-mix(in oklab, var(--switch-accent) 74%, var(--switch-text));
}

.state {
  font-size: 0.72rem;
  color: color-mix(in oklab, var(--switch-accent-ink) 78%, var(--switch-text));
}

.closeBtn {
  border: 1px solid color-mix(in oklab, var(--switch-border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-inline) 94%, var(--panel));
  color: var(--switch-text);
  font-size: 0.7rem;
  line-height: 1;
  padding: 4px 8px;
}

.subtitle {
  font-size: 0.72rem;
  color: var(--switch-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.groupBlock {
  display: grid;
  gap: 4px;
}

.groupLabel {
  font-size: 0.67rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: color-mix(in oklab, var(--switch-accent-ink) 78%, var(--switch-text));
}

.group {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.group button {
  border: 1px solid color-mix(in oklab, var(--switch-accent) 34%, var(--border));
  border-radius: 8px;
  padding: 3px 8px;
  font-size: 0.72rem;
  background: color-mix(in oklab, var(--switch-accent-soft) 48%, var(--surface-inline));
  color: var(--switch-text);
}

.group button.step {
  border-style: dashed;
}

.group button.active {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 82%, var(--switch-accent));
  background: color-mix(in oklab, var(--selection-fill-strong) 84%, var(--surface-working));
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

@media (max-width: 760px) {
  .workflowSwitchRoot {
    top: auto;
    bottom: 8px;
    left: 8px;
  }

  .workflowSwitch {
    width: min(96vw, 520px);
  }

  .head {
    align-items: flex-start;
    flex-direction: column;
  }

  .headRight {
    width: 100%;
    justify-content: space-between;
  }
}
</style>
