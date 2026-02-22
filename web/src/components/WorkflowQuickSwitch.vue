<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useWorkflowStore } from '../stores/workflowStore'

const store = useWorkflowStore()
const { workspace, workflow, workflowStep, workflowSubtitle } = storeToRefs(store)

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
  <aside class="workflowSwitch">
    <header class="head">
      <div class="title">Workflow Nav</div>
      <div class="mono state">ws={{ workspace }} · wf={{ workflow }} · step={{ workflowStep }}</div>
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
  </aside>
</template>

<style scoped>
.workflowSwitch {
  position: fixed;
  top: 84px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 4039;
  width: min(880px, calc(100vw - 24px));
  border: 1px solid rgba(99, 102, 241, 0.4);
  border-radius: 12px;
  background: linear-gradient(150deg, rgba(15, 23, 42, 0.95), rgba(30, 27, 75, 0.92));
  color: #e2e8f0;
  padding: 8px;
  display: grid;
  gap: 6px;
  pointer-events: auto;
}

.head {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: center;
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
  .workflowSwitch {
    top: auto;
    bottom: 8px;
    left: 8px;
    right: 8px;
    transform: none;
    width: auto;
  }
}
</style>
