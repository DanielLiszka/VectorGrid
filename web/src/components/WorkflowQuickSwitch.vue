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
] as const

const stepItems = [
  { key: 'data', label: '1' },
  { key: 'optimize', label: '2' },
  { key: 'validate', label: '3' },
  { key: 'analyze', label: '4' },
] as const

const subtitle = computed(() => workflowSubtitle.value || '—')

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
            <div class="title">Workflow Nav</div>
            <div class="headRight">
              <div class="mono state">ws={{ workspace }} · wf={{ workflow }} · step={{ workflowStep }}</div>
              <button type="button" class="closeBtn" @click="close">Close</button>
            </div>
          </header>
          <div class="subtitle mono">{{ subtitle }}</div>
          <div class="group">
            <button
              v-for="item in wsItems"
              :key="`ws_${item.key}`"
              type="button"
              :class="{ active: workspace === item.key }"
              @click="clickWorkspace(item.key)"
            >
              {{ item.label }}
            </button>
          </div>
          <div class="group">
            <button
              v-for="item in wfItems"
              :key="`wf_${item.key}`"
              type="button"
              :class="{ active: workflow === item.key }"
              @click="clickWorkflow(item.key)"
            >
              {{ item.label }}
            </button>
            <button
              v-for="item in stepItems"
              :key="`step_${item.key}`"
              type="button"
              class="step"
              :class="{ active: workflowStep === item.key }"
              @click="clickWorkflowStep(item.key)"
            >
              {{ item.label }}
            </button>
          </div>
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
