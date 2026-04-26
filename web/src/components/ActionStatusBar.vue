<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import {
  TASK_CENTER_PANEL_ID,
  openTaskCenter as requestTaskCenterOpen,
  useTaskCenterVisibility,
} from '../composables/useTaskCenterVisibility'
import { useTaskLifecycleSummary } from '../composables/useTaskLifecycleSummary'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { taskPostureSnapshot, useTaskStore } from '../stores/taskStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { AppButton, AppStatusChip } from './ui'
import { mlSignalPrimaryActionForStep, workflowStepLabelForMode } from '../utils/core/workflowModel'
import { formatOverallProgressLabel, taskLifecycleLabel } from '../utils/statusPresentation'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysisStore = useAnalysisStore()
const commandActionStore = useCommandActionStore()
const dataStore = useDataStore()
const optimizeStore = useOptimizeStore()
const taskStore = useTaskStore()
const validationStore = useValidationStore()
const workflowStore = useWorkflowStore()
const taskCenterOpen = useTaskCenterVisibility()
const {
  datasetReady,
  hasOptimizationResults,
  validationStage,
} = useUiReadiness()
const {
  runDisabled,
  canCancel,
  validateDisabled,
  progressMeta,
  runPlanMeta,
  runReadinessHint,
  runReadinessBadges,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(optimizeStore)
const { bestMeta, bestTitle, drillStatus, heatmapMeta, topRows } = storeToRefs(analysisStore)
const {
  validationHint,
  validationMeta,
  validationWarningSummary,
} = storeToRefs(validationStore)
const { overallProgress } = storeToRefs(taskStore)
const { mode, resultsTab, workflowStep, workspace } = storeToRefs(workflowStore)
const {
  activeTaskCount,
  centerStatusLabel,
  overallProgressLabel: sharedOverallProgressLabel,
  taskHeadline,
} = useTaskLifecycleSummary()

const { mountReady } = useShellMount('#runFooterVueShellMount')
const footerVisible = computed(() => String(workspace.value || '').trim() === 'workflow')
const mlSignalModeActive = computed(() => String(mode.value || '').trim() === 'ml-signal')
const stageLabel = computed(() => workflowStepLabelForMode(workflowStep.value, mode.value))
const mlSignalPrimaryAction = computed(() => mlSignalPrimaryActionForStep(workflowStep.value))
const dataLoaded = computed(() => String(dataStore.dataId || '').trim().length > 0)
const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))
const hasRunActivity = computed(() => canCancel.value || String(progressMeta.value || '').trim().length > 0)

const mlSignalDisabledReason = computed(() => {
  if (canCancel.value) return ''
  if (!dataLoaded.value) return 'Load a dataset before running this ML Signal stage.'
  switch (mlSignalPrimaryAction.value.kind) {
    case 'dataset_export':
      return String(optimizeStore.mlSignalDatasetExportPath || '').trim()
        ? ''
        : 'Choose a dataset export path before exporting.'
    case 'score_cache':
      if (!String(optimizeStore.mlSignalScoreInputPath || '').trim()) return 'Choose a score input path before building the cache.'
      return String(optimizeStore.mlSignalScoreCachePath || '').trim()
        ? ''
        : 'Choose a score-cache output path before building the cache.'
    case 'validation':
    case 'policy_sweep':
      return String(optimizeStore.mlSignalScoreCachePath || '').trim()
        ? ''
        : 'Build or select a score cache before this stage.'
    case 'artifacts':
      return 'Current ML artifacts are inspect-only from this stage.'
  }
})

const validationDisabledReason = computed(() => {
  const hint = String(validationHint.value || '').trim()
  if (!datasetReady.value) return 'Load a dataset before running validation.'
  if (!hasOptimizationResults.value) return 'Run an optimization before running validation.'
  if (hint && hint !== '—' && hint !== '-') return hint
  if (validationStage.value !== 'ready') return 'Select or review an optimization candidate before validation.'
  return 'Validation is not available for the current state.'
})
const optimizationDisabledReason = computed(() => {
  if (!datasetReady.value) return 'Load a dataset before running optimization.'
  return String(runReadinessHint.value || '').trim() || 'Configure data and strategy inputs before running.'
})

const primaryActionLabel = computed(() => {
  if (canCancel.value) return 'Stop run'
  if (mlSignalModeActive.value) return mlSignalPrimaryAction.value.label
  if (String(workflowStep.value || '').trim() === 'validate') return 'Run validation'
  return 'Run optimization'
})

const primaryActionDisabled = computed(() => {
  if (canCancel.value) return false
  if (mlSignalModeActive.value) {
    if (!mlSignalPrimaryAction.value.actionable) return true
    if (mlSignalDisabledReason.value) return true
    return mlSignalPrimaryAction.value.kind === 'validation' || mlSignalPrimaryAction.value.kind === 'policy_sweep'
      ? !!validateDisabled.value
      : !!runDisabled.value
  }
  if (String(workflowStep.value || '').trim() === 'validate') return !!validateDisabled.value
  return !!runDisabled.value || !datasetReady.value
})

const primaryDisabledReason = computed(() => {
  if (!primaryActionDisabled.value) return ''
  if (mlSignalModeActive.value) {
    const reason = mlSignalDisabledReason.value
    return reason || String(runReadinessHint.value || '').trim() || 'This ML Signal stage is not ready.'
  }
  if (String(workflowStep.value || '').trim() === 'validate') return validationDisabledReason.value
  return optimizationDisabledReason.value
})

const footerSummary = computed(() => {
  const progress = String(progressMeta.value || '').trim()
  if (hasRunActivity.value && progress) return progress
  if (String(workflowStep.value || '').trim() === 'validate') {
    const meta = String(validationMeta.value || '').trim()
    if (meta && meta !== '-' && meta !== '—') return meta
  }
  const badges = runReadinessBadges.value.filter((badge) => String(badge || '').trim().length > 0)
  if (badges.length > 0) return badges.slice(0, 2).join(' · ')
  return String(runReadinessHint.value || '').trim() || 'Ready once data and strategy inputs are configured.'
})

const analysisSummary = computed(() => {
  if (mlSignalModeActive.value) return 'ML Signal stages use dataset export, score cache, validation, and sweep artifacts.'
  const title = String(bestTitle.value || '').trim()
  const meta = String(bestMeta.value || '').trim()
  if (title && title !== '—' && title !== '-') return `${title}${meta && meta !== '—' && meta !== '-' ? ` · ${meta}` : ''}`
  return 'No scored run yet.'
})

const taskPosture = computed(() => taskPostureSnapshot(taskStore.$state, 6))
const taskPostureChips = computed(() => {
  const counts = taskPosture.value.counts
  return [
    counts.running ? `${counts.running} running` : '',
    counts.queued ? `${counts.queued} queued` : '',
    counts.cancelling ? `${counts.cancelling} cancelling` : '',
    counts.failed ? `${counts.failed} failed` : '',
    counts.cancelled ? `${counts.cancelled} cancelled` : '',
    counts.completed ? `${counts.completed} completed` : '',
  ].filter((chip) => chip.length > 0)
})
const latestAttentionTask = computed(() => taskPosture.value.failed[0] ?? taskPosture.value.cancelled[0] ?? null)
const latestCompletedTask = computed(() => taskPosture.value.completed[0] ?? null)
const latestOutcome = computed(() => {
  if (latestAttentionTask.value) return `Attention: ${latestAttentionTask.value.label} ${taskLifecycleLabel(latestAttentionTask.value.status)}`
  if (latestCompletedTask.value) return `Latest complete: ${latestCompletedTask.value.label}`
  return taskHeadline.value
})
const overallProgressLabel = computed(() => {
  const shared = String(sharedOverallProgressLabel.value || '').trim()
  if (shared && shared !== 'No active progress') return shared
  return formatOverallProgressLabel(overallProgress.value)
})
const statusTone = computed<'neutral' | 'accent' | 'success' | 'warning' | 'danger'>(() => {
  if (latestAttentionTask.value) return 'danger'
  if (canCancel.value || activeTaskCount.value > 0) return 'accent'
  if (primaryActionDisabled.value) return 'warning'
  if (hasOptimizationResults.value) return 'success'
  return 'neutral'
})
const validationTone = computed<'neutral' | 'warning'>(() => {
  const warning = String(validationWarningSummary.value || '').trim()
  return warning && warning !== '-' && warning !== '—' ? 'warning' : 'neutral'
})

function runPrimaryAction(): void {
  if (canCancel.value) {
    commandActionStore.cancelOptimization()
    return
  }
  if (mlSignalModeActive.value) {
    commandActionStore.runOptimization()
    return
  }
  if (String(workflowStep.value || '').trim() === 'validate') {
    commandActionStore.validateOptimization()
    return
  }
  commandActionStore.runOptimization()
}

function openTaskCenterDetails(): void {
  requestTaskCenterOpen(taskCenterOpen)
}
</script>

<template>
  <Teleport v-if="mountReady && footerVisible" to="#runFooterVueShellMount">
    <section class="actionStatusBar" aria-label="Action status bar">
      <div class="statusLead">
        <div class="statusHeading">
          <span class="eyebrow">{{ stageLabel }}</span>
          <h2 class="title">Execution readiness</h2>
          <span class="meta mono">Updated {{ stamp }}</span>
        </div>
        <div class="summaryPill mono" role="status" aria-live="polite">{{ footerSummary }}</div>
        <div v-if="primaryActionDisabled" class="blockedReason mono">
          {{ primaryDisabledReason }}
        </div>
      </div>

      <div class="statusBody" aria-label="Current run and task summary">
        <AppStatusChip :tone="statusTone">{{ centerStatusLabel }}</AppStatusChip>
        <AppStatusChip v-if="taskPostureChips.length === 0" tone="muted">{{ overallProgressLabel }}</AppStatusChip>
        <AppStatusChip
          v-for="chip in taskPostureChips"
          :key="`action_status_task_${chip}`"
          :tone="chip.includes('failed') || chip.includes('cancelled') ? 'danger' : 'accent'"
        >
          {{ chip }}
        </AppStatusChip>
        <AppStatusChip v-if="workflowStep === 'validate'" :tone="validationTone">
          {{ validationMeta }}
        </AppStatusChip>
        <span class="statusCopy mono">{{ latestOutcome }}</span>
        <span class="statusCopy mono">{{ analysisSummary }}</span>
        <span v-if="!mlSignalModeActive" class="statusCopy mono">
          Tab {{ resultsTab || 'overview' }} · Top {{ topRows }} · Drill {{ drillStatus }} · Heatmap {{ heatmapMeta }}
        </span>
        <span class="statusCopy mono">{{ runPlanMeta || statusLine }}</span>
      </div>

      <div class="statusActions">
        <AppButton
          variant="subtle"
          size="sm"
          data-qa-action="footer.taskCenter.details"
          :aria-controls="TASK_CENTER_PANEL_ID"
          :aria-expanded="taskCenterOpen ? 'true' : 'false'"
          aria-haspopup="dialog"
          @click="openTaskCenterDetails"
        >
          Details
        </AppButton>
        <AppButton
          :variant="canCancel ? 'danger' : 'primary'"
          size="sm"
          data-qa-action="footer.primaryAction.run"
          :disabled="primaryActionDisabled"
          :disabled-reason="primaryDisabledReason"
          @click="runPrimaryAction"
        >
          {{ primaryActionLabel }}
        </AppButton>
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.actionStatusBar {
  display: grid;
  grid-template-columns: minmax(18rem, 0.95fr) minmax(18rem, 1.35fr) auto;
  align-items: center;
  gap: 0.5rem;
}

.statusLead {
  min-width: 0;
  display: grid;
  gap: 0.18rem;
}

.statusHeading {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.36rem 0.55rem;
  min-width: 0;
}

.eyebrow {
  font-size: 0.67rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.title {
  margin: 0;
  font-size: 0.92rem;
  line-height: 1.2;
  font-weight: 650;
}

.meta {
  font-size: 0.72rem;
  color: var(--muted);
}

.summaryPill {
  max-width: min(72ch, 100%);
  border: 1px solid color-mix(in oklab, var(--border) 80%, transparent);
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface2) 88%, transparent);
  color: var(--muted-strong);
  font-size: 0.74rem;
  line-height: 1.2;
  padding: 0.2rem 0.5rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.blockedReason {
  color: var(--warning-ink);
  font-size: 0.73rem;
  line-height: 1.3;
}

.statusBody {
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.26rem 0.36rem;
}

.statusCopy {
  min-width: 0;
  max-width: min(34ch, 100%);
  color: var(--muted);
  font-size: 0.73rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.statusActions {
  display: flex;
  justify-content: flex-end;
  gap: 0.45rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1180px) {
  .actionStatusBar {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .statusBody {
    grid-column: 1 / -1;
    order: 3;
    flex-wrap: nowrap;
    overflow: hidden;
    max-height: 1.75rem;
  }
}

@media (max-width: 760px) {
  .actionStatusBar {
    grid-template-columns: 1fr;
    align-items: stretch;
  }

  .summaryPill,
  .statusCopy {
    white-space: nowrap;
  }

  .statusBody {
    display: none;
  }

  .statusActions {
    justify-content: flex-start;
  }
}
</style>
