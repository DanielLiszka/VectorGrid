<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import AppVisualIcon from './AppVisualIcon.vue'
import { retryLatestWorkflowArtifactHydration } from '../services/workflowBootstrap'
import { useRuntimeStore } from '../stores/runtimeStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { runtimeReadyIcon } from '../utils/appSvgAssets'
import { taskLevelIcon } from '../utils/iconography'
import { describeRuntimePhase, runtimePhaseLabel } from '../utils/statusPresentation'

const runtime = useRuntimeStore()
const workflowStore = useWorkflowStore()
const {
  bootPhase,
  bootStage,
  bootError,
  bootErrorDetail,
  bootElapsedMs,
  latestArtifactHydrationPhase,
  latestArtifactHydrationError,
} = storeToRefs(runtime)

const latestArtifactRestoreFailed = computed(
  () => bootPhase.value === 'ready' && latestArtifactHydrationPhase.value === 'error',
)

const visible = computed(
  () => bootPhase.value === 'booting' || bootPhase.value === 'error' || latestArtifactRestoreFailed.value,
)

const phaseLabel = computed(() => {
  if (bootPhase.value === 'error') return 'Runtime Error'
  if (latestArtifactRestoreFailed.value) return 'Restore warning'
  return runtimePhaseLabel(bootPhase.value)
})

const detail = computed(() => {
  if (latestArtifactRestoreFailed.value) {
    const error = String(latestArtifactHydrationError.value || 'Unknown restore error').trim()
    return `Latest saved artifact could not be restored: ${error}. The workspace remains usable.`
  }
  return describeRuntimePhase({
    phase: bootPhase.value,
    stage: bootStage.value,
    elapsedMs: bootElapsedMs.value,
    error: bootError.value,
  })
})

const errorDetail = computed(() => {
  if (bootPhase.value !== 'error') return ''
  return String(bootErrorDetail.value || '').trim()
})

const bannerIcon = computed(() => {
  if (bootPhase.value === 'error') return taskLevelIcon('error')
  if (latestArtifactRestoreFailed.value) return taskLevelIcon('warn')
  return runtimeReadyIcon
})

const bannerClass = computed(() => {
  if (bootPhase.value === 'error') return 'error'
  if (latestArtifactRestoreFailed.value) return 'restoreWarning'
  return bootPhase.value
})

function openDiagnostics(): void {
  workflowStore.selectWorkspace('settings')
}

function retryLatestArtifactRestore(): void {
  void retryLatestWorkflowArtifactHydration().catch(() => undefined)
}
</script>

<template>
  <div
    v-if="visible"
    class="runtimeBanner"
    :class="bannerClass"
    role="status"
    :aria-live="bootPhase === 'error' ? 'assertive' : 'polite'"
    aria-atomic="true"
  >
    <div class="runtimeLead">
      <AppVisualIcon :icon="bannerIcon" class="runtimeIcon" />
      <div class="runtimeCopy">
        <div class="runtimeLabel">{{ phaseLabel }}</div>
        <div class="runtimeDetail">{{ detail }}</div>
      </div>
      <button v-if="bootPhase === 'error'" type="button" class="runtimeAction" @click="openDiagnostics">
        Open diagnostics
      </button>
      <button
        v-else-if="latestArtifactRestoreFailed"
        type="button"
        class="runtimeAction"
        @click="retryLatestArtifactRestore"
      >
        Retry latest artifact
      </button>
    </div>
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
  top: calc(var(--desktop-titlebar-h, 0px) + 10px);
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

.runtimeLead {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.runtimeCopy {
  min-width: 0;
}

.runtimeIcon {
  width: 1rem;
  height: 1rem;
  flex: 0 0 auto;
  margin-top: 1px;
}

.runtimeAction {
  margin-left: auto;
  align-self: center;
}

.runtimeBanner.error {
  --runtime-accent: var(--danger);
  --runtime-accent-soft: var(--danger-soft);
  --runtime-border: color-mix(in oklab, var(--runtime-accent) 48%, var(--border));
  --runtime-surface: color-mix(in oklab, var(--danger-soft) 54%, var(--surface-sticky));
}

.runtimeBanner.restoreWarning {
  --runtime-accent: var(--warning);
  --runtime-accent-soft: var(--warning-soft);
  --runtime-border: color-mix(in oklab, var(--runtime-accent) 48%, var(--border));
  --runtime-surface: color-mix(in oklab, var(--warning-soft) 52%, var(--surface-sticky));
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
