<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import OptionsExpressionPanel from './OptionsExpressionPanel.vue'

const { mountReady } = useShellMount('#optionsVueShellMount')
const workflowArtifactsStore = useWorkflowArtifactsStore()
const { currentRun } = storeToRefs(workflowArtifactsStore)

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
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
  normalizeText(currentRun.value?.artifactPath),
)
const parentStrategyKind = computed(
  () =>
    normalizeText(
      currentRunRequest.value.strategy_kind ??
        currentRunRequest.value.strategyKind ??
        currentRunResult.value.strategy_kind ??
        currentRunResult.value.strategyKind,
    ),
)
const parentDatasetLabel = computed(
  () =>
    normalizeText(
      currentRunDatasetMeta.value.label ??
        currentRunDatasetMeta.value.dataset_label ??
        currentRunDatasetMeta.value.datasetLabel ??
        currentRunDatasetMeta.value.symbol,
    ) || normalizeText(currentRun.value?.csvPath),
)
const parentCompatible = computed(() => parentArtifactPath.value.length > 0)
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
