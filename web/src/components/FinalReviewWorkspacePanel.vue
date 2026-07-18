<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { useShellMount } from '../composables/useShellMount'
import { useUiReadiness } from '../composables/useUiReadiness'

const { mountReady } = useShellMount('#finalReviewVueShellMount', {
  paneSelector: '[data-vue-panel="final"]',
  paneDatasetKey: 'finalReview',
})

const workflowStore = useWorkflowStore()
const dataStore = useDataStore()
const optimizeStore = useOptimizeStore()
const analysisStore = useAnalysisStore()
const validationStore = useValidationStore()
const workflowArtifactsStore = useWorkflowArtifactsStore()
const { datasetReady, hasOptimizationResults, hasValidationResults } = useUiReadiness()

const { datasetLabel, dataLen, dataTimeframe, datasetRange } = storeToRefs(dataStore)
const { backend, gpuStatus, objective, progressMeta, strategyKind } = storeToRefs(optimizeStore)
const { bestMeta, bestTitle, selectedTopTitle } = storeToRefs(analysisStore)
const {
  statusLine: validationStatusLine,
  tableRowCount,
  validationMeta,
  validationWarningSummary,
} = storeToRefs(validationStore)
const { currentRun, currentValidation } = storeToRefs(workflowArtifactsStore)

function clean(value: unknown, fallback = '-'): string {
  const text = String(value ?? '').trim()
  return text.length > 0 && text !== 'undefined' && text !== 'NaN'
    ? text
    : fallback
}

const datasetSummary = computed(() =>
  [
    clean(datasetLabel.value, ''),
    clean(dataTimeframe.value, ''),
    clean(dataLen.value, '') ? `${clean(dataLen.value)} rows` : '',
    clean(datasetRange.value, ''),
  ].filter(Boolean).join(' · ') || 'No dataset loaded',
)

const strategySummary = computed(() =>
  [
    clean(strategyKind.value, ''),
    clean(objective.value, ''),
    clean(backend.value, ''),
    clean(gpuStatus.value, ''),
  ].filter(Boolean).join(' · ') || 'No strategy run selected',
)

const optimizationSummary = computed(() =>
  clean(bestTitle.value, '') ||
  clean(selectedTopTitle.value, '') ||
  clean(bestMeta.value, '') ||
  clean(progressMeta.value, '') ||
  'No ranked candidate selected',
)

const validationSummary = computed(() =>
  clean(validationMeta.value, '') ||
  clean(validationWarningSummary.value, '') ||
  clean(validationStatusLine.value, '') ||
  `${Math.max(0, Number(tableRowCount.value || 0))} validation rows`,
)

const checklist = computed(() => [
  {
    key: 'dataset',
    label: 'Dataset',
    status: datasetReady.value ? 'Ready' : 'Missing',
    detail: datasetSummary.value,
    complete: datasetReady.value,
  },
  {
    key: 'optimization',
    label: 'Optimization',
    status: hasOptimizationResults.value || currentRun.value ? 'Ready' : 'Missing',
    detail: optimizationSummary.value,
    complete: hasOptimizationResults.value || !!currentRun.value,
  },
  {
    key: 'validation',
    label: 'Validation',
    status: hasValidationResults.value || currentValidation.value ? 'Ready' : 'Missing',
    detail: validationSummary.value,
    complete: hasValidationResults.value || !!currentValidation.value,
  },
  {
    key: 'artifacts',
    label: 'Artifacts',
    status: currentRun.value?.artifactPath ? 'Ready' : 'Pending',
    detail: clean(currentRun.value?.artifactPath, 'Run artifact not selected'),
    complete: !!currentRun.value?.artifactPath,
  },
])

const incompleteCount = computed(
  () => checklist.value.filter((item) => !item.complete).length,
)
const verdict = computed(() =>
  incompleteCount.value === 0
    ? 'Evidence packet ready for export or replay.'
    : `${incompleteCount.value} evidence item${incompleteCount.value === 1 ? '' : 's'} still need review.`,
)

function openCandidates(): void {
  workflowStore.selectWorkflowStep('analyze')
}

function openValidation(): void {
  workflowStore.selectWorkflowStep('validate')
}

function openMeta(): void {
  workflowStore.selectWorkspace('aggregate')
}

function openLibrary(): void {
  workflowStore.selectWorkspace('library')
}
</script>

<template>
  <Teleport v-if="mountReady" to="#finalReviewVueShellMount">
    <section class="finalReviewPanel" aria-label="Final review evidence checklist">
      <div class="finalReviewHeader">
        <div>
          <span class="eyebrow">Decision packet</span>
          <h2>Final Review</h2>
        </div>
        <strong class="finalReviewVerdict">{{ verdict }}</strong>
      </div>

      <div class="finalReviewSummary">
        <div>
          <span>Strategy</span>
          <strong>{{ strategySummary }}</strong>
        </div>
        <div>
          <span>Latest evidence</span>
          <strong>{{ validationSummary }}</strong>
        </div>
      </div>

      <div class="finalReviewChecklist" aria-label="Final evidence checklist">
        <article
          v-for="item in checklist"
          :key="item.key"
          class="finalReviewItem"
          :class="{ complete: item.complete }"
        >
          <div>
            <span>{{ item.label }}</span>
            <strong>{{ item.status }}</strong>
          </div>
          <p>{{ item.detail }}</p>
        </article>
      </div>

      <div class="finalReviewActions" aria-label="Final review actions">
        <button type="button" data-qa-action="finalReview.openCandidates" @click="openCandidates">
          Review candidates
        </button>
        <button type="button" data-qa-action="finalReview.openValidation" @click="openValidation">
          Open validation
        </button>
        <button type="button" data-qa-action="finalReview.openMeta" @click="openMeta">
          Open meta
        </button>
        <button type="button" data-qa-action="finalReview.openLibrary" @click="openLibrary">
          Open library
        </button>
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.finalReviewPanel {
  display: grid;
  gap: 0.85rem;
}

.finalReviewHeader {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 1rem;
}

.finalReviewHeader h2 {
  margin: 0.1rem 0 0;
  font-size: 1.05rem;
}

.finalReviewVerdict {
  max-width: 22rem;
  color: var(--text-strong);
  text-align: right;
}

.finalReviewSummary,
.finalReviewChecklist {
  display: grid;
  gap: 0.65rem;
}

.finalReviewSummary {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.finalReviewSummary > div,
.finalReviewItem {
  border: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 96%, var(--surface-working));
  padding: 0.7rem;
}

.finalReviewSummary span,
.finalReviewItem span {
  display: block;
  color: var(--text-muted);
  font-size: 0.78rem;
}

.finalReviewSummary strong,
.finalReviewItem strong {
  color: var(--text-strong);
}

.finalReviewItem.complete {
  border-color: color-mix(in oklab, var(--success, #16a34a) 45%, var(--border));
}

.finalReviewItem p {
  margin: 0.35rem 0 0;
  color: var(--text-muted);
}

.finalReviewActions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.finalReviewActions button {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-inline) 92%, var(--panel));
  color: var(--text-strong);
  padding: 0.5rem 0.72rem;
  font: inherit;
  cursor: pointer;
}

.finalReviewActions button:hover {
  background: color-mix(in oklab, var(--selection-fill) 72%, var(--surface-working));
}

@media (max-width: 840px) {
  .finalReviewHeader,
  .finalReviewSummary {
    grid-template-columns: 1fr;
  }

  .finalReviewHeader {
    display: grid;
  }

  .finalReviewVerdict {
    max-width: none;
    text-align: left;
  }
}
</style>
