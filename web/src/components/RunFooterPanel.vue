<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useCommandActionStore } from '../stores/commandActionStore'

const commandActionStore = useCommandActionStore()
const optimizeStore = useOptimizeStore()
const {
  runDisabled,
  canCancel,
  progressMeta,
  runPlanMeta,
  runReadinessHint,
  runReadinessBadges,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(optimizeStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function runOptimizationAction(): void {
  commandActionStore.runOptimization()
}

function cancelRunAction(): void {
  commandActionStore.cancelOptimization()
}

function resolveMount(): void {
  const panel = document.getElementById('runFooterShell')
  const mount = document.getElementById('runFooterVueShellMount')
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueRunFooter = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueRunFooter
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#runFooterVueShellMount">
    <section class="runFooterVueRoot">
      <header class="head">
        <div class="title">Run Control</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <div class="actions">
        <button type="button" class="primary" :disabled="runDisabled" @click="runOptimizationAction">Run optimization</button>
        <button type="button" class="danger" :disabled="!canCancel" @click="cancelRunAction">Cancel</button>
      </div>

      <div v-if="progressMeta" class="metaLine mono">{{ progressMeta }}</div>
      <div class="metaLine mono">{{ runPlanMeta }}</div>

      <div class="badges" aria-live="polite" aria-label="Run readiness">
        <span v-for="badge in runReadinessBadges" :key="`run_badge_${badge}`" class="badge">{{ badge }}</span>
      </div>

      <div class="metaLine mono">{{ runReadinessHint }}</div>
      <div class="status mono">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.runFooterVueRoot {
  display: grid;
  gap: 0.5rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
}

.title {
  font-size: 0.85rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted);
}

.meta {
  font-size: 0.75rem;
  color: var(--muted);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.45rem 0.68rem;
  font-size: 0.78rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 72%, var(--border));
}

.actions .danger {
  border-color: color-mix(in oklab, var(--bad) 66%, var(--border));
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.badge {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.2rem 0.48rem;
  font-size: 0.72rem;
  background: color-mix(in oklab, var(--panel) 88%, var(--panel-footer));
}

.status {
  font-size: 0.76rem;
  color: var(--muted);
}

.mono {
  font-family: var(--mono);
}
</style>

<style>
#runFooterShell[data-vue-run-footer='1'] > :not(#runFooterVueShellMount) {
  display: none !important;
}
</style>

