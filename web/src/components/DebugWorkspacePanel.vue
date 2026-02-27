<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  debugOutputPreview,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const visible = computed(() => String(resultsTab.value || '').trim() === 'debug')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="debug"]') as HTMLElement | null
  const mount = document.querySelector('#debugVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueDebug = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueDebug
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#debugVueShellMount">
    <section class="debugPanelVueRoot debugVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Debug JSON</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <pre class="mono output">{{ debugOutputPreview }}</pre>
      </section>

      <div class="status mono">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.debugPanelVueRoot {
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

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.output {
  margin: 0;
  max-height: min(58vh, 520px);
  overflow: auto;
  white-space: pre-wrap;
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.mono {
  font-family: var(--mono);
}
</style>

<style>
[data-vue-panel='debug'][data-vue-debug='1'] > :not(#debugVueShellMount) {
  display: none !important;
}
</style>


