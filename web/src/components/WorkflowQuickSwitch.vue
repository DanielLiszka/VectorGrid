<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui'
import { useWorkflowStore } from '../stores/workflowStore'

const store = useWorkflowStore()
const { workspace, workflow, workflowStep, workflowSubtitle } = storeToRefs(store)
const open = useStorage<boolean>('VTA_WORKFLOW_NAV_OPEN_V1', false)

const wsItems = [
  { key: 'workflow', label: 'Workflow' },
  { key: 'library', label: 'Library' },
  { key: 'aggregate', label: 'Aggregate' },
  { key: 'pro', label: 'Pro' },
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

type SwitchItem = { key: string }

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

function focusSelector(selector: string): void {
  const element = document.querySelector<HTMLElement>(selector)
  element?.focus()
}

function resolveFocusedKey(event: KeyboardEvent): string | null {
  const target = event.target instanceof Element ? (event.target.closest('[data-switch-key]') as HTMLElement | null) : null
  return String(target?.dataset.switchKey || '').trim() || null
}

function moveGroupFocus(
  event: KeyboardEvent,
  items: readonly SwitchItem[],
  currentKey: string,
  selectValue: (key: string) => void,
  selectorForKey: (key: string) => string,
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

  event.preventDefault()
  const nextKey = items[nextIndex]?.key
  if (!nextKey) return
  selectValue(nextKey)
  focusSelector(selectorForKey(nextKey))
}

function onWorkspaceKeydown(event: KeyboardEvent): void {
  moveGroupFocus(event, wsItems, currentWorkspace.value, clickWorkspace, (key) => `[data-switch-group="workspace"] [data-switch-key="${key}"]`)
}

function onWorkflowKeydown(event: KeyboardEvent): void {
  moveGroupFocus(event, wfItems, currentWorkflow.value, clickWorkflow, (key) => `[data-switch-group="workflow"] [data-switch-key="${key}"]`)
}

function onStepKeydown(event: KeyboardEvent): void {
  moveGroupFocus(event, stepItems, currentWorkflowStep.value, clickWorkflowStep, (key) => `[data-switch-group="step"] [data-switch-key="${key}"]`)
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
            <div class="group" data-switch-group="workspace" @keydown="onWorkspaceKeydown">
              <button
                v-for="item in wsItems"
                :key="`ws_${item.key}`"
                :id="switchItemId('workspace', item.key)"
                type="button"
                :class="{ active: currentWorkspace === item.key }"
                :aria-current="currentWorkspace === item.key ? 'page' : undefined"
                :tabindex="currentWorkspace === item.key ? 0 : -1"
                :data-switch-key="item.key"
                @click="clickWorkspace(item.key)"
              >
                {{ item.label }}
              </button>
            </div>
          </nav>
          <nav class="groupBlock" aria-label="Workflow quick switch">
            <div class="groupLabel">Workflow</div>
            <div class="group" data-switch-group="workflow" @keydown="onWorkflowKeydown">
              <button
                v-for="item in wfItems"
                :key="`wf_${item.key}`"
                :id="switchItemId('workflow', item.key)"
                type="button"
                :class="{ active: currentWorkflow === item.key }"
                :aria-current="currentWorkflow === item.key ? 'page' : undefined"
                :tabindex="currentWorkflow === item.key ? 0 : -1"
                :data-switch-key="item.key"
                @click="clickWorkflow(item.key)"
              >
                {{ item.label }}
              </button>
            </div>
          </nav>
          <nav class="groupBlock" aria-label="Workflow step quick switch">
            <div class="groupLabel">Step</div>
            <div class="group" data-switch-group="step" @keydown="onStepKeydown">
              <button
                v-for="item in stepItems"
                :key="`step_${item.key}`"
                :id="switchItemId('step', item.key)"
                type="button"
                class="step"
                :class="{ active: currentWorkflowStep === item.key }"
                :aria-current="currentWorkflowStep === item.key ? 'step' : undefined"
                :tabindex="currentWorkflowStep === item.key ? 0 : -1"
                :data-switch-key="item.key"
                @click="clickWorkflowStep(item.key)"
              >
                {{ item.label }}
              </button>
            </div>
          </nav>
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
  </div>
</template>

<style scoped>
.workflowSwitchRoot {
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
  border: 1px solid rgba(129, 140, 248, 0.5);
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.88);
  color: #e0e7ff;
  padding: 6px 11px;
  font-size: 0.76rem;
  font-weight: 650;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  box-shadow: 0 10px 24px rgba(2, 6, 23, 0.35);
}

.workflowSwitch {
  z-index: 4040;
  width: min(880px, calc(100vw - 24px));
  border: 1px solid rgba(99, 102, 241, 0.4);
  border-radius: 12px;
  background: linear-gradient(150deg, rgba(15, 23, 42, 0.95), rgba(30, 27, 75, 0.92));
  color: #e2e8f0;
  padding: 8px;
  display: grid;
  gap: 6px;
  pointer-events: auto;
  transform-origin: top left;
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
  color: #a5b4fc;
}

.state {
  font-size: 0.72rem;
  color: #c4b5fd;
}

.closeBtn {
  border: 1px solid rgba(148, 163, 184, 0.34);
  border-radius: 8px;
  background: rgba(15, 23, 42, 0.7);
  color: #e2e8f0;
  font-size: 0.7rem;
  line-height: 1;
  padding: 4px 8px;
}

.subtitle {
  font-size: 0.72rem;
  color: #cbd5e1;
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
  color: #c4b5fd;
}

.group {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.group button {
  border: 1px solid rgba(129, 140, 248, 0.38);
  border-radius: 8px;
  padding: 3px 8px;
  font-size: 0.72rem;
  background: rgba(30, 27, 75, 0.82);
  color: #e0e7ff;
}

.group button.step {
  border-style: dashed;
}

.group button.active {
  border-color: rgba(196, 181, 253, 0.9);
  background: rgba(79, 70, 229, 0.35);
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
