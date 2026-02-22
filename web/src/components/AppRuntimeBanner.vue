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
  position: fixed;
  top: 10px;
  left: 10px;
  z-index: 4050;
  border-radius: 10px;
  padding: 6px 10px;
  max-width: min(56ch, calc(100vw - 22px));
  border: 1px solid rgba(56, 189, 248, 0.42);
  background: rgba(15, 23, 42, 0.92);
  color: #e2e8f0;
  box-shadow: 0 8px 20px rgba(2, 6, 23, 0.35);
  backdrop-filter: blur(8px);
}

.runtimeBanner.error {
  border-color: rgba(244, 63, 94, 0.52);
  background: rgba(76, 5, 25, 0.92);
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
