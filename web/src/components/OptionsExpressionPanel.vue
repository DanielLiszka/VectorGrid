<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useOptionsExpressionStore, OPTIONS_EXIT_TEMPLATE_OPTIONS, OPTIONS_FILL_MODEL_OPTIONS, OPTIONS_MONEYNESS_BUCKET_OPTIONS } from '../stores/optionsExpressionStore'

const props = defineProps<{
  parentArtifactPath: string
  parentStrategyKind: string
  parentDatasetLabel: string
}>()

const store = useOptionsExpressionStore()
const {
  activeArtifactPath,
  activeArtifactTimestamp,
  activeBackendUsed,
  activePriceSemantics,
  datasets,
  datasetsBusy,
  artifactsBusy,
  feasibilityBusy,
  filteredArtifacts,
  importBusy,
  importRootSymbol,
  loadBusy,
  latestFeasibility,
  parentRootHint,
  runBusy,
  selectedArtifactPath,
  selectedDataset,
  selectedDatasetId,
  statusLine,
  supportedParent,
  activeDiagnostics,
  activeTupleInspection,
  activeOptionsDatasetHash,
  activeRequest,
  activeStaging,
  activeRankedResults,
  inspectionBusy,
  delayMinutesInput,
  dteBucketsInput,
  holdMinutesInput,
  targetReturnInput,
  stopReturnInput,
  selectedMoneynessBuckets,
  exitTemplate,
  fillModel,
  topKInput,
  minValidTradesInput,
  splitRatioSearchInput,
  splitRatioHoldoutInput,
  maxEntryLagMinutesInput,
  maxPremiumInput,
  minPremiumInput,
  minVolumeInput,
  minOpenInterestInput,
} = storeToRefs(store)

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function formatList(value: Array<number | string> | null | undefined): string {
  return Array.isArray(value) && value.length > 0 ? value.join(', ') : '—'
}

function formatTimestamp(value: number | null | undefined): string {
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed <= 0) return '—'
  try {
    return new Date(parsed).toISOString()
  } catch {
    return '—'
  }
}

function formatOptionalDecimal(value: number | null | undefined, digits = 4): string {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : '—'
}

function formatPercent(value: number | null | undefined, digits = 1): string {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? `${(parsed * 100).toFixed(digits)}%` : '—'
}

watch(
  () => [props.parentArtifactPath, props.parentStrategyKind, props.parentDatasetLabel] as const,
  ([path, strategyKind, datasetLabel]) => {
    store.attachParent(path, strategyKind, datasetLabel)
    if (normalizeText(path) && normalizeText(strategyKind) === 'DoubleMa') {
      void store.bootstrap()
    }
  },
  { immediate: true },
)

const hasParentSelection = computed(() => normalizeText(props.parentArtifactPath).length > 0)

const datasetSummary = computed(() => {
  const selected = selectedDataset.value
  if (!selected) return ''
  return [
    selected.rootSymbol || 'unknown root',
    `${selected.contractCount} contracts`,
    `${selected.rowCount} rows`,
    selected.priceSemantics || 'unknown semantics',
  ].join(' | ')
})

const datasetProvenanceSummary = computed(() => {
  const selected = selectedDataset.value
  if (!selected) return ''
  const segments = [
    normalizeText(selected.providerName),
    normalizeText(selected.providerSourceSurface),
    normalizeText(selected.quoteResolution),
    normalizeText(selected.aggregatePriceField),
    normalizeText(selected.contractReferenceAsOfUtc),
  ].filter((entry) => entry.length > 0)
  return segments.join(' | ')
})

const rootMismatch = computed(() => {
  const selected = selectedDataset.value
  if (!selected) return false
  const selectedRoot = normalizeText(selected.rootSymbol).toUpperCase()
  const parentRoot = normalizeText(parentRootHint.value).toUpperCase()
  return !!selectedRoot && !!parentRoot && selectedRoot !== parentRoot
})

const activeRows = computed(() => activeRankedResults.value.slice(0, 10))
const hasHoldoutMetrics = computed(() => activeRows.value.some((row) => row.holdoutMetrics != null))
const hasSealedMetrics = computed(() => activeRows.value.some((row) => row.sealedEvaluationMetrics != null))
const hasStressMetrics = computed(() => activeRows.value.some((row) => row.stressMetrics != null))
const selectedTupleIndex = ref(0)
const inspectionSplitFilter = ref<'all' | 'search' | 'holdout' | 'sealed_test'>('all')
const showAllInspectionRows = ref(false)

watch(
  activeRows,
  (rows) => {
    if (!rows.length) {
      selectedTupleIndex.value = 0
      store.clearTupleInspection()
      return
    }
    if (selectedTupleIndex.value >= rows.length) {
      selectedTupleIndex.value = 0
    }
  },
  { immediate: true },
)

watch(selectedTupleIndex, () => {
  showAllInspectionRows.value = false
  inspectionSplitFilter.value = 'all'
  store.clearTupleInspection()
})

const selectedTuple = computed(() => activeRows.value[selectedTupleIndex.value] || null)
const selectedInspection = computed(() => {
  const inspection = activeTupleInspection.value
  if (!inspection) return null
  return inspection.tupleIndex === selectedTupleIndex.value ? inspection : null
})
const inspectionRows = computed(() => {
  const inspection = selectedInspection.value
  if (!inspection) return []
  const filter = inspectionSplitFilter.value
  return inspection.rows.filter((row) => filter === 'all' || row.split === filter)
})
const displayedInspectionRows = computed(() => {
  if (showAllInspectionRows.value) return inspectionRows.value
  return inspectionRows.value.slice(0, 50)
})

const resultSummaryChips = computed(() => {
  const diagnostics = activeDiagnostics.value
  if (!diagnostics) return []
  const chips = [
    `${diagnostics.totalEvents} events`,
    `${activeRankedResults.value.length} tuples`,
  ]
  if (activeBackendUsed.value) chips.push(activeBackendUsed.value)
  if (activePriceSemantics.value) chips.push(activePriceSemantics.value)
  if (diagnostics.holdoutEvents > 0) chips.push(`${diagnostics.holdoutEvents} holdout`)
  if (diagnostics.sealedTestEvents > 0) chips.push(`${diagnostics.sealedTestEvents} sealed`)
  if (activeStaging.value) {
    chips.push(`${activeStaging.value.compiledCandidateCount}/${activeStaging.value.candidateSlotCount} candidates`)
    chips.push(`${activeStaging.value.baselineExitSummaryCount} baseline summaries`)
  }
  return chips
})

const reportingMetricCards = computed(() => {
  const diagnostics = activeDiagnostics.value
  if (!diagnostics) return []
  const cards = [
    {
      key: 'bull-bear',
      label: 'Bull / Bear',
      value: `${diagnostics.bullishEvents} / ${diagnostics.bearishEvents}`,
    },
    {
      key: 'search-holdout-sealed',
      label: 'Search / Holdout / Sealed',
      value: `${diagnostics.searchEvents} / ${diagnostics.holdoutEvents} / ${diagnostics.sealedTestEvents}`,
    },
  ]
  if (activeStaging.value && activeStaging.value.candidateSlotCount > 0) {
    cards.push({
      key: 'candidate-coverage',
      label: 'Candidates',
      value: `${activeStaging.value.compiledCandidateCount} / ${activeStaging.value.candidateSlotCount}`,
    })
  }
  if (activeStaging.value) {
    cards.push({
      key: 'exit-summaries',
      label: 'Exit summaries',
      value: `${activeStaging.value.baselineExitSummaryCount} / ${activeStaging.value.conservativeExitSummaryCount}`,
    })
  }
  return cards
})

const feasibilitySummaryRows = computed(() => {
  const preview = latestFeasibility.value
  if (!preview) return []
  return [
    {
      key: 'candidate-density',
      label: 'Candidate density',
      value: `${preview.compiledCandidateCount} / ${preview.candidateSlotCount} (${formatPercent(preview.candidateDensity)})`,
    },
    {
      key: 'invalid-slots',
      label: 'Invalid slots',
      value: `${preview.invalidSlotCount} (${formatPercent(preview.invalidSlotRate)})`,
    },
    {
      key: 'event-coverage',
      label: 'Event coverage',
      value: `${preview.eventCoverage.totalEventsWithCandidates} candidate-ready / ${preview.eventCoverage.totalEventsWithBaselineOutcomes} baseline outcomes / ${preview.eventCoverage.totalEventsWithConservativeOutcomes} conservative outcomes`,
    },
    {
      key: 'tuple-grid',
      label: 'Tuple grid',
      value: `${preview.compiledTupleCount} tuples · ${preview.projectedEventTupleEvaluations} event-tuples`,
    },
    {
      key: 'preview-runtime',
      label: 'Replay / Total preview',
      value: `${preview.replayBuildMs} ms / ${preview.totalPreviewMs} ms`,
    },
    {
      key: 'compile-runtime',
      label: 'Compile / Rank',
      value: `${preview.timings.baselineCandidateCompileMs + preview.timings.baselineExitSummaryCompileMs + preview.timings.conservativeCandidateCompileMs + preview.timings.conservativeExitSummaryCompileMs} ms compile · ${preview.timings.cpuRankingMs} ms rank`,
    },
  ]
})

const feasibilityInvalidReasons = computed(() => {
  const preview = latestFeasibility.value
  if (!preview || preview.invalidReasons.length === 0) return ''
  return preview.invalidReasons
    .slice(0, 3)
    .map((entry) => `${entry.reason} (${entry.count})`)
    .join(' | ')
})

const requestSummaryRows = computed(() => {
  const request = activeRequest.value
  if (!request) return []
  return [
    {
      key: 'artifact-ts',
      label: 'Artifact timestamp',
      value: normalizeText(activeArtifactTimestamp.value) || '—',
    },
    {
      key: 'dataset-hash',
      label: 'Dataset hash',
      value: normalizeText(activeOptionsDatasetHash.value) || '—',
    },
    {
      key: 'delays-dte-holds',
      label: 'Delays / DTE / Holds',
      value: `${formatList(request.delayMinutes)} | ${formatList(request.dteBuckets)} | ${formatList(request.holdMinutes)}`,
    },
    {
      key: 'targets-stops',
      label: 'Targets / Stops',
      value: `${formatList(request.targetReturnBuckets)} | ${formatList(request.stopReturnBuckets)}`,
    },
    {
      key: 'exit-fill',
      label: 'Exit / Fill',
      value: `${normalizeText(request.exitTemplate) || '—'} | ${normalizeText(request.fillModel) || '—'}`,
    },
    {
      key: 'splits',
      label: 'Search / Holdout',
      value: `${request.splitRatioSearch.toFixed(2)} / ${request.splitRatioHoldout.toFixed(2)}`,
    },
  ]
})

function toggleMoneyness(bucket: string, checked: boolean): void {
  const next = new Set(selectedMoneynessBuckets.value)
  if (checked) next.add(bucket)
  else next.delete(bucket)
  selectedMoneynessBuckets.value = Array.from(next)
}

function onMoneynessToggle(bucket: string, event: Event): void {
  const target = event.target
  const checked = target instanceof HTMLInputElement ? target.checked : false
  toggleMoneyness(bucket, checked)
}

function selectTuple(index: number): void {
  selectedTupleIndex.value = index
}

function loadSelectedTupleInspection(): void {
  if (!selectedTuple.value) return
  void store.loadTupleInspection(selectedTupleIndex.value)
}
</script>

<template>
  <section class="optionsPanel surfaceFocus" aria-label="Options expression follow-up">
    <div class="panelSectionHeader">
      <div>
        <div class="panelEyebrow">Follow-up workflow</div>
        <h3 class="panelPaneTitle" style="margin: 0;">Options Expression</h3>
        <div class="panelSectionLead">
          Express a saved validated directional artifact through a constrained options search without mutating the main workflow state.
        </div>
      </div>
    </div>

    <section v-if="!hasParentSelection" class="emptyState">
      <div class="emptyTitle">Select a saved run first.</div>
      <div class="emptyBody">
        Options expression optimization is launched from a saved artifact in the History workspace.
      </div>
    </section>

    <section v-else-if="!supportedParent" class="emptyState">
      <div class="emptyTitle">Saved run not supported yet.</div>
      <div class="emptyBody">
        The current vertical slice supports saved <span class="mono">DoubleMa</span> artifacts only. Other strategy families stay read-only until their manifest projector is implemented.
      </div>
    </section>

    <template v-else>
      <div class="panelSummaryChips" aria-label="Options expression summary">
        <span v-for="chip in resultSummaryChips" :key="chip" class="panelSummaryChip mono">{{ chip }}</span>
      </div>

      <div class="optionsGrid">
        <div class="controlBlock">
          <label for="optionsDatasetSelect">Options dataset</label>
          <div class="row inlineControls">
            <select id="optionsDatasetSelect" v-model="selectedDatasetId">
              <option value="">Select imported dataset…</option>
              <option
                v-for="entry in datasets"
                :key="entry.optionsDatasetId"
                :value="entry.optionsDatasetId"
              >
                {{ entry.optionsDatasetId }} · {{ entry.rootSymbol || 'unknown root' }} · {{ entry.priceSemantics }}
              </option>
            </select>
            <button type="button" :disabled="datasetsBusy" @click="store.refreshDatasets()">
              Refresh
            </button>
          </div>
          <div class="panelMuted">{{ datasetSummary || 'Import or select a normalized options dataset.' }}</div>
          <div v-if="datasetProvenanceSummary" class="panelMuted mono">{{ datasetProvenanceSummary }}</div>
          <div v-if="rootMismatch" class="warningText">
            Selected dataset root does not match the current saved run root hint ({{ parentRootHint || 'unknown' }}). Continue only if that mismatch is intentional.
          </div>
        </div>

        <div class="controlBlock">
          <label for="optionsImportRoot">Import root hint</label>
          <div class="row inlineControls">
            <input
              id="optionsImportRoot"
              v-model="importRootSymbol"
              type="text"
              placeholder="SPY"
              spellcheck="false"
            />
            <button type="button" :disabled="importBusy" @click="store.importDatasetFromPicker()">
              Import CSV
            </button>
          </div>
          <div class="panelMuted">
            Uses the existing native CSV picker and stores a normalized options dataset under the app market-data root.
          </div>
        </div>
      </div>

      <details class="panelDetails" open>
        <summary>Search grid and execution settings</summary>
        <div class="panelDetailsBody optionsFields">
          <label>
            Delay minutes
            <input v-model="delayMinutesInput" type="text" placeholder="0,1,5" spellcheck="false" />
          </label>
          <label>
            DTE buckets
            <input v-model="dteBucketsInput" type="text" placeholder="1,3,5" spellcheck="false" />
          </label>
          <label>
            Hold minutes
            <input v-model="holdMinutesInput" type="text" placeholder="5,15,30,60" spellcheck="false" />
          </label>
          <label>
            Target buckets
            <input v-model="targetReturnInput" type="text" placeholder="0.25,0.5,1" spellcheck="false" />
          </label>
          <label>
            Stop buckets
            <input v-model="stopReturnInput" type="text" placeholder="0.25,0.5" spellcheck="false" />
          </label>
          <label>
            Exit template
            <select v-model="exitTemplate">
              <option v-for="entry in OPTIONS_EXIT_TEMPLATE_OPTIONS" :key="entry" :value="entry">
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Fill model
            <select v-model="fillModel">
              <option v-for="entry in OPTIONS_FILL_MODEL_OPTIONS" :key="entry" :value="entry">
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Top K
            <input v-model="topKInput" type="text" inputmode="numeric" spellcheck="false" />
          </label>
          <label>
            Min valid trades
            <input v-model="minValidTradesInput" type="text" inputmode="numeric" spellcheck="false" />
          </label>
          <label>
            Search split
            <input v-model="splitRatioSearchInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Holdout split
            <input v-model="splitRatioHoldoutInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Max entry lag
            <input v-model="maxEntryLagMinutesInput" type="text" inputmode="numeric" spellcheck="false" />
          </label>
          <label>
            Max premium
            <input v-model="maxPremiumInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Min premium
            <input v-model="minPremiumInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Min volume
            <input v-model="minVolumeInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Min open interest
            <input v-model="minOpenInterestInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <div class="full">
            <div class="panelMetricKey">Moneyness buckets</div>
            <div class="moneynessList">
              <label v-for="entry in OPTIONS_MONEYNESS_BUCKET_OPTIONS" :key="entry" class="bucketToggle">
                <input
                  :checked="selectedMoneynessBuckets.includes(entry)"
                  type="checkbox"
                  @change="onMoneynessToggle(entry, $event)"
                />
                <span class="mono">{{ entry }}</span>
              </label>
            </div>
          </div>
        </div>
      </details>

      <div class="actions">
        <button
          type="button"
          :disabled="feasibilityBusy || !selectedDatasetId"
          @click="store.previewCurrentFeasibility()"
        >
          {{ feasibilityBusy ? 'Previewing…' : 'Preview feasibility' }}
        </button>
        <button
          type="button"
          :disabled="runBusy || !selectedDatasetId"
          @click="store.runCurrentExpression()"
        >
          {{ runBusy ? 'Running…' : 'Optimize expression' }}
        </button>
        <button type="button" :disabled="artifactsBusy" @click="store.refreshArtifacts()">
          Refresh artifacts
        </button>
        <button
          type="button"
          :disabled="loadBusy || !selectedArtifactPath"
          @click="store.loadArtifact()"
        >
          Load selected artifact
        </button>
        <button
          type="button"
          :disabled="!activeArtifactPath"
          @click="store.revealActiveArtifact()"
        >
          Reveal artifact
        </button>
        <button
          type="button"
          :disabled="activeRankedResults.length === 0"
          @click="store.exportActiveCsv()"
        >
          Export CSV
        </button>
      </div>

      <div class="controlBlock">
        <label for="optionsArtifactSelect">Saved options expression artifacts</label>
        <select id="optionsArtifactSelect" v-model="selectedArtifactPath">
          <option value="">Select saved options expression artifact…</option>
          <option
            v-for="entry in filteredArtifacts"
            :key="entry.path"
            :value="entry.path"
          >
            {{ entry.ts || entry.path }} · {{ entry.optionsDatasetId }} · {{ entry.tupleCount }} tuples · robust {{ entry.bestRobustScore.toFixed(3) }}<template v-if="entry.hasSealedEvaluationMetrics"> · sealed</template><template v-else-if="entry.hasHoldoutMetrics"> · holdout</template><template v-if="entry.hasStressMetrics"> · stress</template>
          </option>
        </select>
        <div class="panelMuted">
          Filtered to the currently selected saved run.
        </div>
      </div>

      <section v-if="latestFeasibility" class="resultCard">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Phase 0.5 preview</div>
            <div class="panelSectionLead">
              Review candidate density, event coverage, and replay/compile timing before committing to a full optimization run or GPU work.
            </div>
          </div>
        </div>
        <div class="detailGrid detailGridTight">
          <div v-for="row in feasibilitySummaryRows" :key="row.key">
            <span class="panelMetricKey">{{ row.label }}</span>
            <span class="panelMetricValue mono panelNumeric">{{ row.value }}</span>
          </div>
        </div>
        <div v-if="feasibilityInvalidReasons" class="panelMuted mono">
          Invalid reasons: {{ feasibilityInvalidReasons }}
        </div>
      </section>

      <section v-if="activeDiagnostics" class="resultCard">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Current result</div>
            <div class="panelSectionLead">
              Review the current top tuples before deciding whether this expression family is worth promoting into a wider options workflow.
            </div>
          </div>
        </div>
        <div class="detailGrid">
          <div>
            <span class="panelMetricKey">Root</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.rootSymbol }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Dataset</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.optionsDatasetId }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Events</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.totalEvents }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Search / Holdout / Sealed</span>
            <span class="panelMetricValue mono panelNumeric">
              {{ activeDiagnostics.searchEvents }} / {{ activeDiagnostics.holdoutEvents }} / {{ activeDiagnostics.sealedTestEvents }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Contracts</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.datasetContracts }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Rows</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.datasetRows }}</span>
          </div>
        </div>

        <div class="detailGrid detailGridTight">
          <div v-for="card in reportingMetricCards" :key="card.key">
            <span class="panelMetricKey">{{ card.label }}</span>
            <span class="panelMetricValue mono panelNumeric">{{ card.value }}</span>
          </div>
        </div>

        <details v-if="requestSummaryRows.length > 0" class="panelDetails">
          <summary>Run provenance</summary>
          <div class="panelDetailsBody detailGrid detailGridTight">
            <div v-for="row in requestSummaryRows" :key="row.key">
              <span class="panelMetricKey">{{ row.label }}</span>
              <span class="panelMetricValue mono panelNumeric">{{ row.value }}</span>
            </div>
          </div>
        </details>

        <div v-if="activeRows.length > 0" class="tableWrap">
          <table class="resultTable">
            <thead>
              <tr>
                <th>DTE</th>
                <th>Moneyness</th>
                <th>Delay</th>
                <th>Hold</th>
                <th>Target / Stop</th>
                <th>Robust</th>
                <th>EV / Premium</th>
                <th>Invalid %</th>
                <th v-if="hasHoldoutMetrics">Holdout EV / Premium</th>
                <th v-if="hasSealedMetrics">Sealed EV / Premium</th>
                <th v-if="hasStressMetrics">Stress EV / Premium</th>
                <th>Win</th>
                <th>Trades</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, index) in activeRows"
                :key="`${row.dteDays}-${row.moneynessBucket}-${row.delayMinutes}-${row.holdMinutes}-${row.targetReturn}-${row.stopReturn}`"
                :class="{ isSelected: index === selectedTupleIndex }"
                @click="selectTuple(index)"
              >
                <td class="mono">{{ row.dteDays }}</td>
                <td class="mono">{{ row.moneynessBucket }}</td>
                <td class="mono">{{ row.delayMinutes }}</td>
                <td class="mono">{{ row.holdMinutes }}</td>
                <td class="mono">
                  {{ row.targetReturn == null ? '—' : row.targetReturn }} /
                  {{ row.stopReturn == null ? '—' : row.stopReturn }}
                </td>
                <td class="mono">{{ row.robustScore.toFixed(3) }}</td>
                <td class="mono">{{ row.selectionMetrics.expectancyPerPremium.toFixed(4) }}</td>
                <td class="mono">{{ row.selectionMetrics.invalidRate.toFixed(3) }}</td>
                <td v-if="hasHoldoutMetrics" class="mono">
                  {{ row.holdoutMetrics ? row.holdoutMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td v-if="hasSealedMetrics" class="mono">
                  {{ row.sealedEvaluationMetrics ? row.sealedEvaluationMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td v-if="hasStressMetrics" class="mono">
                  {{ row.stressMetrics ? row.stressMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td class="mono">{{ row.selectionMetrics.winRate.toFixed(3) }}</td>
                <td class="mono">{{ row.selectionMetrics.validTrades }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="selectedTuple" class="detailInspector">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Selected tuple</div>
              <div class="panelSectionLead">
                Inspect the current top-row configuration and robustness metrics before promoting it into a wider options workflow.
              </div>
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              :disabled="inspectionBusy || !activeArtifactPath"
              @click="loadSelectedTupleInspection()"
            >
              {{ inspectionBusy ? 'Loading events…' : 'Load event drilldown' }}
            </button>
          </div>
          <div class="detailGrid detailGridTight">
            <div>
              <span class="panelMetricKey">Delay / DTE / Hold</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.delayMinutes }} / {{ selectedTuple.dteDays }} / {{ selectedTuple.holdMinutes }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Moneyness</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.moneynessBucket }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Target / Stop</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.targetReturn == null ? '—' : selectedTuple.targetReturn }} /
                {{ selectedTuple.stopReturn == null ? '—' : selectedTuple.stopReturn }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Exit template</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.exitTemplate }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Fill model</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.fillModel }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Robust score</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.robustScore.toFixed(3) }}</span>
            </div>
            <div>
              <span class="panelMetricKey">EV / Trade</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.expectancyPerTrade.toFixed(4) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">EV / Premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Median return / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.medianReturnOnPremium.toFixed(4) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Win rate</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.winRate.toFixed(3) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Valid / Invalid</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.validTrades }} / {{ selectedTuple.selectionMetrics.invalidEvents }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">PnL / contract</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.totalPnlPerContract.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.holdoutMetrics">
              <span class="panelMetricKey">Holdout EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.holdoutMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedEvaluationMetrics">
              <span class="panelMetricKey">Sealed EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.sealedEvaluationMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.stressMetrics">
              <span class="panelMetricKey">Stress EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.stressMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
          </div>
          <details v-if="selectedInspection" class="panelDetails" open>
            <summary>Tuple event drilldown</summary>
            <div class="panelDetailsBody">
              <div class="detailGrid detailGridTight">
                <div>
                  <span class="panelMetricKey">Events</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.totalEvents }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Executed</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.executedEvents }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Candidate missing</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.candidateMissingEvents }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Outcome missing</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.outcomeMissingEvents }}</span>
                </div>
              </div>

              <div class="row inlineControls">
                <label>
                  Split filter
                  <select v-model="inspectionSplitFilter">
                    <option value="all">all</option>
                    <option value="search">search</option>
                    <option value="holdout">holdout</option>
                    <option value="sealed_test">sealed_test</option>
                  </select>
                </label>
                <button
                  v-if="inspectionRows.length > 50 && !showAllInspectionRows"
                  type="button"
                  @click="showAllInspectionRows = true"
                >
                  Show all {{ inspectionRows.length }}
                </button>
              </div>

              <div class="panelMuted">
                Showing {{ displayedInspectionRows.length }} of {{ inspectionRows.length }} event rows for tuple #{{ selectedInspection.tupleIndex + 1 }}.
              </div>

              <div class="tableWrap">
                <table class="resultTable inspectionTable">
                  <thead>
                    <tr>
                      <th>Event</th>
                      <th>Split</th>
                      <th>Side</th>
                      <th>Status</th>
                      <th>Contract</th>
                      <th>Requested entry</th>
                      <th>Entry</th>
                      <th>Exit</th>
                      <th>Exit reason</th>
                      <th>Return / premium</th>
                      <th>PnL / contract</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="row in displayedInspectionRows" :key="row.eventId">
                      <td class="mono">{{ row.eventId }}</td>
                      <td class="mono">{{ row.split }}</td>
                      <td class="mono">{{ row.side }}</td>
                      <td class="mono">{{ row.status }}</td>
                      <td class="mono">
                        <template v-if="row.contractId">
                          {{ row.contractId }}
                          <span v-if="row.strike != null"> @ {{ formatOptionalDecimal(row.strike, 2) }}</span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">{{ formatTimestamp(row.requestedEntryTimestamp) }}</td>
                      <td class="mono">
                        <template v-if="row.entryTimestamp != null">
                          {{ formatTimestamp(row.entryTimestamp) }} · {{ formatOptionalDecimal(row.entryOptionPrice) }}
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">
                        <template v-if="row.exitTimestamp != null">
                          {{ formatTimestamp(row.exitTimestamp) }} · {{ formatOptionalDecimal(row.exitOptionPrice) }}
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">{{ row.exitReason || '—' }}</td>
                      <td class="mono">{{ formatOptionalDecimal(row.returnOnPremium) }}</td>
                      <td class="mono">{{ formatOptionalDecimal(row.pnlPerContract) }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </details>
        </div>
        <div v-else class="emptyBody">
          No ranked tuples available yet. Run an options expression job or load a saved artifact.
        </div>
      </section>
    </template>

    <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
  </section>
</template>

<style scoped>
.optionsPanel {
  display: grid;
  gap: 0.85rem;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--panel-footer);
  padding: 0.9rem;
}

.optionsGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.controlBlock {
  display: grid;
  gap: 0.35rem;
}

.inlineControls {
  display: flex;
  gap: 0.55rem;
  align-items: center;
}

.inlineControls > * {
  min-width: 0;
}

.optionsFields {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.optionsFields label,
.controlBlock label {
  display: grid;
  gap: 0.3rem;
  font-size: 0.82rem;
}

.optionsFields .full {
  grid-column: 1 / -1;
}

.moneynessList {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.bucketToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.3rem 0.45rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--panel);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
}

.warningText {
  color: var(--warning, #f0b429);
  font-size: 0.78rem;
}

.resultCard {
  display: grid;
  gap: 0.75rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.detailGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.detailGridTight {
  gap: 0.55rem;
}

.tableWrap {
  overflow: auto;
}

.resultTable {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.78rem;
}

.resultTable th,
.resultTable td {
  border-bottom: 1px solid var(--border);
  padding: 0.45rem 0.4rem;
  text-align: left;
}

.resultTable tbody tr {
  cursor: pointer;
}

.resultTable tbody tr.isSelected {
  background: color-mix(in srgb, var(--accent, #4da3ff) 12%, transparent);
}

.emptyState {
  display: grid;
  gap: 0.35rem;
  padding: 0.7rem 0.2rem 0.1rem;
}

.emptyTitle {
  font-weight: 600;
}

.emptyBody {
  color: var(--muted);
  font-size: 0.84rem;
}

.detailInspector {
  display: grid;
  gap: 0.7rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

input,
select {
  width: 100%;
}

@media (max-width: 980px) {
  .optionsGrid,
  .optionsFields,
  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>
