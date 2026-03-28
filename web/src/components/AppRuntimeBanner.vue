<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useRuntimeStore } from '../stores/runtimeStore'

const runtime = useRuntimeStore()
const { bootPhase, bootStage, bootError, bootErrorDetail, bootElapsedMs } = storeToRefs(runtime)

const visible = computed(() => bootPhase.value === 'booting' || bootPhase.value === 'error')

const phaseLabel = computed(() => {
  if (bootPhase.value === 'error') return 'Runtime Error'
  if (bootPhase.value === 'booting') return 'Initializing'
  return 'Ready'
})

const detail = computed(() => {
  if (bootPhase.value === 'error') return bootError.value || 'Unknown startup error'
  const elapsed = Math.max(0, Math.floor(Number(bootElapsedMs.value || 0)))
  const suffix = elapsed > 0 ? ` (${elapsed}ms)` : ''
  const stage = String(bootStage.value || '').trim() || 'Loading'
  return `${stage}${suffix}`
})

const errorDetail = computed(() => {
  if (bootPhase.value !== 'error') return ''
  return String(bootErrorDetail.value || '').trim()
})
</script>

<template>
  <div v-if="visible" class="runtimeBanner" :class="bootPhase">
    <div class="runtimeLabel">{{ phaseLabel }}</div>
    <div class="runtimeDetail">{{ detail }}</div>
    <details v-if="errorDetail" class="runtimeErrorTrace">
      <summary>Stack trace</summary>
      <pre>{{ errorDetail }}</pre>
    </details>
  </div>
</template>

<style scoped>
.runtimeBanner {
  --runtime-accent: var(--accent);
  --runtime-accent-soft: var(--accent-soft);
  --runtime-border: color-mix(in oklab, var(--runtime-accent) 42%, var(--border));
  --runtime-surface: color-mix(in oklab, var(--surface-sticky) 94%, var(--panel-footer));
  --runtime-text: var(--text);
  position: fixed;
  top: 10px;
  left: 10px;
  z-index: 4050;
  border-radius: 10px;
  padding: 6px 10px;
  max-width: min(56ch, calc(100vw - 22px));
  border: 1px solid var(--runtime-border);
  background: var(--runtime-surface);
  color: var(--runtime-text);
  box-shadow: var(--shadow-focus);
  backdrop-filter: blur(8px);
}

.runtimeBanner.error {
  --runtime-accent: var(--danger);
  --runtime-accent-soft: var(--danger-soft);
  --runtime-border: color-mix(in oklab, var(--runtime-accent) 48%, var(--border));
  --runtime-surface: color-mix(in oklab, var(--danger-soft) 54%, var(--surface-sticky));
}

.runtimeLabel {
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.runtimeDetail {
  margin-top: 2px;
  font-size: 0.76rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.runtimeErrorTrace {
  margin-top: 6px;
  font-size: 0.72rem;
}

.runtimeErrorTrace summary {
  cursor: pointer;
  user-select: none;
}

.runtimeErrorTrace pre {
  margin: 4px 0 0;
  max-height: 200px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
