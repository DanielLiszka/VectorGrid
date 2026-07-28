<script setup lang="ts">
import { computed, nextTick, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import OptionsExpressionPanel from './OptionsExpressionPanel.vue'

const { mountReady } = useShellMount('#optionsVueShellMount')
const workflowArtifactsStore = useWorkflowArtifactsStore()
const workflowStore = useWorkflowStore()
const historyStore = useHistoryStore()
const { currentRun } = storeToRefs(workflowArtifactsStore)
const { workflowStep } = storeToRefs(workflowStore)
const {
  canLoadSavedRun,
  savedRunDetailDataset,
  savedRunDetailStrategy,
  savedSelectedValue,
} = storeToRefs(historyStore)

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeHistoryText(value: unknown): string {
  const normalized = normalizeText(value)
  return normalized === '—' || normalized === '-' ? '' : normalized
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {}
}

const currentRunRequest = computed(() => asRecord(currentRun.value?.req))
const currentRunResult = computed(() => asRecord(currentRun.value?.res))
const currentRunDatasetMeta = computed(() =>
  asRecord(currentRun.value?.datasetMeta),
)

const parentArtifactPath = computed(() =>
  normalizeText(currentRun.value?.artifactPath) || normalizeText(savedSelectedValue.value),
)
const parentStrategyKind = computed(
  () =>
    normalizeText(
      currentRunRequest.value.strategy_kind ??
        currentRunRequest.value.strategyKind ??
        currentRunResult.value.strategy_kind ??
        currentRunResult.value.strategyKind,
    ) || normalizeHistoryText(savedRunDetailStrategy.value),
)
const parentDatasetLabel = computed(
  () =>
    normalizeText(
      currentRunDatasetMeta.value.label ??
        currentRunDatasetMeta.value.dataset_label ??
        currentRunDatasetMeta.value.datasetLabel ??
        currentRunDatasetMeta.value.symbol,
    ) || normalizeText(currentRun.value?.csvPath) || normalizeHistoryText(savedRunDetailDataset.value),
)
const parentCompatible = computed(() =>
  normalizeText(currentRun.value?.artifactPath).length > 0
    ? true
    : canLoadSavedRun.value,
)

watch(
  [mountReady, workflowStep],
  async ([ready, step]) => {
    if (!ready) return
    await nextTick()
    document
      .querySelector<HTMLElement>(`[data-options-stage="${String(step || 'data')}"]`)
      ?.scrollIntoView?.({ behavior: 'auto', block: 'start' })
  },
  { immediate: true, flush: 'post' },
)
</script>

<template>
  <Teleport v-if="mountReady" to="#optionsVueShellMount">
    <OptionsExpressionPanel
      :parent-artifact-path="parentArtifactPath"
      :parent-strategy-kind="parentStrategyKind"
      :parent-dataset-label="parentDatasetLabel"
      :parent-compatible="parentCompatible"
    />
  </Teleport>
</template>
