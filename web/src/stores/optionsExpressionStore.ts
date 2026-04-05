import { defineStore } from 'pinia'
import type {
  AnalyzeOptionsExpressionFeasibilitySummary,
  OptionsDatasetManifestSummary,
  OptionsExpressionArtifactRecord,
  OptionsExpressionArtifactSummary,
  OptionsExpressionTupleInspectionSummary,
  RunOptionsExpressionSummary,
} from '../services/optionsExpressionService'
import {
  analyzeOptionsExpressionFeasibility,
  exportOptionsExpressionCsv,
  importOptionsDataset,
  inspectOptionsExpressionTuple,
  listOptionsDatasets,
  listOptionsExpressionArtifacts,
  loadOptionsExpressionArtifact,
  openOrRevealOptionsPath,
  pickOptionsCsvPath,
  runOptionsExpression,
} from '../services/optionsExpressionService'

export const OPTIONS_MONEYNESS_BUCKET_OPTIONS = ['atm', 'otm_1', 'otm_2', 'itm_1'] as const
export const OPTIONS_EXIT_TEMPLATE_OPTIONS = [
  'fixed_hold',
  'fixed_hold_or_target_stop',
  'end_of_day_or_target_stop',
  'base_exit_or_target_stop',
] as const
export const OPTIONS_FILL_MODEL_OPTIONS = [
  'aggregate_research',
  'quote_conservative',
  'quote_mid',
] as const

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeNumber(value: unknown, fallback = 0): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

function normalizeOptionalNumber(value: unknown): number | null {
  const raw = normalizeText(value)
  if (!raw) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

function normalizeOptionalInteger(value: unknown): number | null {
  const parsed = normalizeOptionalNumber(value)
  return parsed == null ? null : Math.max(0, Math.floor(parsed))
}

function fileStem(value: string): string {
  const trimmed = normalizeText(value)
  if (!trimmed) return ''
  const parts = trimmed.split(/[\\/]+/g)
  const leaf = parts[parts.length - 1] || ''
  return leaf.replace(/\.[^.]+$/, '').trim()
}

function deriveRootHint(datasetLabel: string): string {
  const stem = fileStem(datasetLabel)
  if (!stem) return ''
  const normalized = stem.match(/[A-Za-z0-9]+/g)?.join('') || ''
  return normalized.toUpperCase()
}

function parseIntegerList(value: string, fallback: number[]): number[] {
  const tokens = normalizeText(value)
    .split(',')
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0)
  const out = tokens
    .map((entry) => Math.floor(normalizeNumber(entry, Number.NaN)))
    .filter((entry) => Number.isFinite(entry) && entry >= 0)
  return out.length > 0 ? Array.from(new Set(out)).sort((a, b) => a - b) : fallback
}

function parseFloatList(value: string, fallback: number[]): number[] {
  const tokens = normalizeText(value)
    .split(',')
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0)
  const out = tokens
    .map((entry) => normalizeNumber(entry, Number.NaN))
    .filter((entry) => Number.isFinite(entry) && entry >= 0)
  return out.length > 0 ? Array.from(new Set(out)).sort((a, b) => a - b) : fallback
}

function pickSupported<T extends string>(
  value: string,
  allowed: readonly T[],
  fallback: T,
): T {
  const normalized = normalizeText(value)
  return (allowed.includes(normalized as T) ? normalized : fallback) as T
}

export const useOptionsExpressionStore = defineStore('optionsExpression', {
  state: () => ({
    parentArtifactPath: '',
    parentArtifactId: '',
    parentStrategyKind: '',
    parentDatasetLabel: '',
    parentRootHint: '',
    importRootSymbol: '',
    datasets: [] as OptionsDatasetManifestSummary[],
    artifacts: [] as OptionsExpressionArtifactSummary[],
    selectedDatasetId: '',
    selectedArtifactPath: '',
    activeArtifact: null as OptionsExpressionArtifactRecord | null,
    latestRun: null as RunOptionsExpressionSummary | null,
    latestFeasibility: null as AnalyzeOptionsExpressionFeasibilitySummary | null,
    activeTupleInspection: null as OptionsExpressionTupleInspectionSummary | null,
    delayMinutesInput: '0,1,5',
    dteBucketsInput: '1,3,5',
    holdMinutesInput: '5,15,30,60',
    targetReturnInput: '0.25,0.5,1',
    stopReturnInput: '0.25,0.5',
    selectedMoneynessBuckets: [...OPTIONS_MONEYNESS_BUCKET_OPTIONS] as string[],
    exitTemplate: 'fixed_hold_or_target_stop',
    fillModel: 'aggregate_research',
    topKInput: '25',
    minValidTradesInput: '',
    splitRatioSearchInput: '0.6',
    splitRatioHoldoutInput: '0.2',
    maxEntryLagMinutesInput: '5',
    maxPremiumInput: '',
    minPremiumInput: '',
    minVolumeInput: '',
    minOpenInterestInput: '',
    datasetsBusy: false,
    artifactsBusy: false,
    runBusy: false,
    loadBusy: false,
    importBusy: false,
    inspectionBusy: false,
    feasibilityBusy: false,
    statusLine: 'Select a saved DoubleMa run to configure an options expression follow-up.',
  }),

  getters: {
    supportedParent: (state) => normalizeText(state.parentStrategyKind) === 'DoubleMa',

    filteredArtifacts(state): OptionsExpressionArtifactSummary[] {
      const parentId = normalizeText(state.parentArtifactId)
      if (!parentId) return []
      return state.artifacts.filter((entry) => normalizeText(entry.parentArtifactId) === parentId)
    },

    selectedDataset(state): OptionsDatasetManifestSummary | null {
      return state.datasets.find((entry) => entry.optionsDatasetId === state.selectedDatasetId) || null
    },

    activeArtifactPath(state): string {
      return normalizeText(state.latestRun?.artifactPath) || normalizeText(state.activeArtifact?.artifactPath) || normalizeText(state.selectedArtifactPath)
    },

    activeArtifactTimestamp(state): string {
      return normalizeText(state.latestRun?.artifactTs) || normalizeText(state.activeArtifact?.ts)
    },

    activeDiagnostics(state) {
      return state.latestRun?.diagnostics || state.activeArtifact?.diagnostics || null
    },

    activeRequest(state) {
      return state.latestRun?.request || state.activeArtifact?.request || null
    },

    activeOptionsDatasetHash(state): string {
      return normalizeText(state.latestRun?.optionsDatasetHash) || normalizeText(state.activeArtifact?.optionsDatasetHash)
    },

    activeStaging(state) {
      return state.latestRun?.staging || state.activeArtifact?.staging || null
    },

    activeRankedResults(state) {
      if (state.latestRun) return state.latestRun.rankedResults
      return state.activeArtifact?.selectionMetrics || []
    },

    activeBackendUsed(state): string {
      return normalizeText(state.latestRun?.backendUsed) || normalizeText(state.activeArtifact?.backendUsed)
    },

    activePriceSemantics(state): string {
      return normalizeText(state.latestRun?.priceSemantics) || normalizeText(state.activeArtifact?.priceSemantics)
    },
  },

  actions: {
    attachParent(parentArtifactPath: string, parentStrategyKind: string, parentDatasetLabel: string): void {
      const nextPath = normalizeText(parentArtifactPath)
      const nextStrategy = normalizeText(parentStrategyKind)
      const nextDatasetLabel = normalizeText(parentDatasetLabel)
      const nextParentId = fileStem(nextPath)
      const nextRootHint = deriveRootHint(nextDatasetLabel)
      const changed =
        nextPath !== this.parentArtifactPath ||
        nextStrategy !== this.parentStrategyKind ||
        nextDatasetLabel !== this.parentDatasetLabel

      this.parentArtifactPath = nextPath
      this.parentArtifactId = nextParentId
      this.parentStrategyKind = nextStrategy
      this.parentDatasetLabel = nextDatasetLabel
      this.parentRootHint = nextRootHint
      if (changed) {
        this.importRootSymbol = nextRootHint
        this.selectedArtifactPath = ''
        this.activeArtifact = null
        this.latestRun = null
        this.latestFeasibility = null
        this.activeTupleInspection = null
      }
      if (!nextPath) {
        this.statusLine = 'Select a saved run to configure an options expression follow-up.'
        return
      }
      if (nextStrategy !== 'DoubleMa') {
        this.statusLine = 'Options expression follow-up currently supports saved DoubleMa runs only.'
        return
      }
      this.statusLine = 'Ready to import a normalized options dataset or reuse an existing one.'
    },

    async bootstrap(): Promise<void> {
      if (!this.supportedParent || !this.parentArtifactPath) return
      await Promise.all([this.refreshDatasets(), this.refreshArtifacts()])
    },

    async refreshDatasets(): Promise<void> {
      this.datasetsBusy = true
      try {
        const datasets = await listOptionsDatasets()
        this.datasets = [...datasets].sort((left, right) => {
          const byRoot = normalizeText(left.rootSymbol).localeCompare(normalizeText(right.rootSymbol))
          if (byRoot !== 0) return byRoot
          return normalizeText(left.optionsDatasetId).localeCompare(normalizeText(right.optionsDatasetId))
        })
        const selectedExists = this.datasets.some((entry) => entry.optionsDatasetId === this.selectedDatasetId)
        if (!selectedExists) {
          const preferred = this.datasets.find(
            (entry) =>
              normalizeText(entry.rootSymbol).toUpperCase() === normalizeText(this.parentRootHint).toUpperCase(),
          )
          this.selectedDatasetId = preferred?.optionsDatasetId || this.datasets[0]?.optionsDatasetId || ''
        }
        this.statusLine = this.datasets.length
          ? `Loaded ${this.datasets.length} options dataset${this.datasets.length === 1 ? '' : 's'}.`
          : 'No imported options datasets yet. Import a normalized CSV to continue.'
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Failed to refresh options datasets.'
      } finally {
        this.datasetsBusy = false
      }
    },

    async importDatasetFromPicker(): Promise<void> {
      this.importBusy = true
      try {
        const csvPath = await pickOptionsCsvPath()
        if (!csvPath) {
          this.statusLine = 'Options dataset import canceled.'
          return
        }
        const imported = await importOptionsDataset({
          csv_path: csvPath,
          root_symbol: normalizeText(this.importRootSymbol) || undefined,
          provider_name: 'manual_import',
          provider_source_surface: 'minute_aggregate',
          price_semantics: 'minute_aggregate',
          aggregate_price_field: 'close',
        })
        await this.refreshDatasets()
        this.selectedDatasetId = imported.optionsDatasetId
        this.statusLine = `Imported options dataset ${imported.optionsDatasetId}.`
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Failed to import options dataset.'
      } finally {
        this.importBusy = false
      }
    },

    async refreshArtifacts(): Promise<void> {
      this.artifactsBusy = true
      try {
        const artifacts = await listOptionsExpressionArtifacts()
        this.artifacts = [...artifacts].sort((left, right) => normalizeText(right.ts).localeCompare(normalizeText(left.ts)))
        const filtered = this.filteredArtifacts
        const selectedExists = filtered.some((entry) => entry.path === this.selectedArtifactPath)
        if (!selectedExists) this.selectedArtifactPath = filtered[0]?.path || ''
        this.statusLine = filtered.length
          ? `Loaded ${filtered.length} saved options expression artifact${filtered.length === 1 ? '' : 's'} for this run.`
          : this.statusLine
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Failed to refresh saved options expression artifacts.'
      } finally {
        this.artifactsBusy = false
      }
    },

    async runCurrentExpression(): Promise<void> {
      if (!this.parentArtifactPath) {
        this.statusLine = 'Select a saved run before starting options expression optimization.'
        return
      }
      if (!this.supportedParent) {
        this.statusLine = 'Only saved DoubleMa runs are currently supported.'
        return
      }
      if (!this.selectedDatasetId) {
        this.statusLine = 'Import or select an options dataset first.'
        return
      }

      const splitRatioSearch = normalizeNumber(this.splitRatioSearchInput, 0.6)
      const splitRatioHoldout = normalizeNumber(this.splitRatioHoldoutInput, 0.2)
      if (!(splitRatioSearch > 0 && splitRatioHoldout >= 0 && splitRatioSearch + splitRatioHoldout < 1)) {
        this.statusLine = 'Split ratios must satisfy search > 0, holdout >= 0, and search + holdout < 1.'
        return
      }

      const request = {
        parent_artifact_path: this.parentArtifactPath,
        options_dataset_id: this.selectedDatasetId,
        delay_minutes: parseIntegerList(this.delayMinutesInput, [0, 1, 5]),
        dte_buckets: parseIntegerList(this.dteBucketsInput, [1, 3, 5]),
        moneyness_buckets:
          this.selectedMoneynessBuckets.length > 0
            ? [...this.selectedMoneynessBuckets]
            : [...OPTIONS_MONEYNESS_BUCKET_OPTIONS],
        hold_minutes: parseIntegerList(this.holdMinutesInput, [5, 15, 30, 60]),
        target_return_buckets: parseFloatList(this.targetReturnInput, [0.25, 0.5, 1]),
        stop_return_buckets: parseFloatList(this.stopReturnInput, [0.25, 0.5]),
        exit_template: pickSupported(
          this.exitTemplate,
          OPTIONS_EXIT_TEMPLATE_OPTIONS,
          'fixed_hold_or_target_stop',
        ),
        fill_model: pickSupported(this.fillModel, OPTIONS_FILL_MODEL_OPTIONS, 'aggregate_research'),
        filters: {
          max_entry_lag_minutes: normalizeOptionalInteger(this.maxEntryLagMinutesInput),
          max_premium: normalizeOptionalNumber(this.maxPremiumInput),
          min_premium: normalizeOptionalNumber(this.minPremiumInput),
          min_volume: normalizeOptionalNumber(this.minVolumeInput),
          min_open_interest: normalizeOptionalNumber(this.minOpenInterestInput),
        },
        top_k: Math.max(1, Math.floor(normalizeNumber(this.topKInput, 25))),
        min_valid_trades: normalizeOptionalInteger(this.minValidTradesInput),
        split_ratio_search: splitRatioSearch,
        split_ratio_holdout: splitRatioHoldout,
      }

      this.runBusy = true
      try {
        const result = await runOptionsExpression(request)
        this.latestRun = result
        this.activeArtifact = null
        this.activeTupleInspection = null
        this.selectedArtifactPath = result.artifactPath
        await this.refreshArtifacts()
        this.statusLine = `Options expression run completed with ${result.tupleCount} ranked tuple${result.tupleCount === 1 ? '' : 's'}.`
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Options expression run failed.'
      } finally {
        this.runBusy = false
      }
    },

    async previewCurrentFeasibility(): Promise<void> {
      if (!this.parentArtifactPath) {
        this.statusLine = 'Select a saved run before previewing options-expression feasibility.'
        return
      }
      if (!this.supportedParent) {
        this.statusLine = 'Only saved DoubleMa runs are currently supported.'
        return
      }
      if (!this.selectedDatasetId) {
        this.statusLine = 'Import or select an options dataset first.'
        return
      }

      const splitRatioSearch = normalizeNumber(this.splitRatioSearchInput, 0.6)
      const splitRatioHoldout = normalizeNumber(this.splitRatioHoldoutInput, 0.2)
      if (!(splitRatioSearch > 0 && splitRatioHoldout >= 0 && splitRatioSearch + splitRatioHoldout < 1)) {
        this.statusLine = 'Split ratios must satisfy search > 0, holdout >= 0, and search + holdout < 1.'
        return
      }

      this.feasibilityBusy = true
      try {
        const preview = await analyzeOptionsExpressionFeasibility({
          parent_artifact_path: this.parentArtifactPath,
          options_dataset_id: this.selectedDatasetId,
          delay_minutes: parseIntegerList(this.delayMinutesInput, [0, 1, 5]),
          dte_buckets: parseIntegerList(this.dteBucketsInput, [1, 3, 5]),
          moneyness_buckets:
            this.selectedMoneynessBuckets.length > 0
              ? [...this.selectedMoneynessBuckets]
              : [...OPTIONS_MONEYNESS_BUCKET_OPTIONS],
          hold_minutes: parseIntegerList(this.holdMinutesInput, [5, 15, 30, 60]),
          target_return_buckets: parseFloatList(this.targetReturnInput, [0.25, 0.5, 1]),
          stop_return_buckets: parseFloatList(this.stopReturnInput, [0.25, 0.5]),
          exit_template: pickSupported(
            this.exitTemplate,
            OPTIONS_EXIT_TEMPLATE_OPTIONS,
            'fixed_hold_or_target_stop',
          ),
          fill_model: pickSupported(this.fillModel, OPTIONS_FILL_MODEL_OPTIONS, 'aggregate_research'),
          filters: {
            max_entry_lag_minutes: normalizeOptionalInteger(this.maxEntryLagMinutesInput),
            max_premium: normalizeOptionalNumber(this.maxPremiumInput),
            min_premium: normalizeOptionalNumber(this.minPremiumInput),
            min_volume: normalizeOptionalNumber(this.minVolumeInput),
            min_open_interest: normalizeOptionalNumber(this.minOpenInterestInput),
          },
          top_k: Math.max(1, Math.floor(normalizeNumber(this.topKInput, 25))),
          min_valid_trades: normalizeOptionalInteger(this.minValidTradesInput),
          split_ratio_search: splitRatioSearch,
          split_ratio_holdout: splitRatioHoldout,
        })
        this.latestFeasibility = preview
        this.statusLine = `Feasibility preview compiled ${preview.compiledTupleCount} tuple${preview.compiledTupleCount === 1 ? '' : 's'} with ${(preview.candidateDensity * 100).toFixed(1)}% candidate density.`
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Failed to preview options-expression feasibility.'
      } finally {
        this.feasibilityBusy = false
      }
    },

    async loadArtifact(path?: string): Promise<void> {
      const nextPath = normalizeText(path || this.selectedArtifactPath)
      if (!nextPath) {
        this.statusLine = 'Select a saved options expression artifact first.'
        return
      }
      this.loadBusy = true
      try {
        const artifact = await loadOptionsExpressionArtifact(nextPath)
        this.selectedArtifactPath = nextPath
        this.activeArtifact = artifact
        this.latestRun = null
        this.latestFeasibility = null
        this.activeTupleInspection = null
        this.statusLine = `Loaded saved options expression artifact ${fileStem(nextPath) || nextPath}.`
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Failed to load saved options expression artifact.'
      } finally {
        this.loadBusy = false
      }
    },

    async revealActiveArtifact(): Promise<void> {
      const nextPath = this.activeArtifactPath
      if (!nextPath) {
        this.statusLine = 'No options expression artifact is active yet.'
        return
      }
      try {
        const result = await openOrRevealOptionsPath(nextPath)
        this.statusLine = `${result.openedWith === 'reveal_path' ? 'Revealed' : 'Opened'} ${nextPath}.`
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Failed to open options expression artifact path.'
      }
    },

    exportActiveCsv(): void {
      const tuples = this.activeRankedResults
      if (!tuples.length) {
        this.statusLine = 'No ranked tuples are available to export yet.'
        return
      }

      const exported = exportOptionsExpressionCsv({
        filenameStem: normalizeText(this.parentArtifactId)
          ? `${this.parentArtifactId}_options_expression`
          : `vectorgrid_options_expression_${normalizeText(this.activeDiagnostics?.rootSymbol || this.parentRootHint).toLowerCase() || 'results'}`,
        rootSymbol: normalizeText(this.activeDiagnostics?.rootSymbol) || normalizeText(this.parentRootHint),
        backendUsed: this.activeBackendUsed,
        priceSemantics: this.activePriceSemantics,
        tuples,
      })

      this.statusLine = exported
        ? `Exported ${tuples.length} ranked tuple${tuples.length === 1 ? '' : 's'} to CSV.`
        : 'CSV export is unavailable in the current environment.'
    },

    clearTupleInspection(): void {
      this.activeTupleInspection = null
    },

    async loadTupleInspection(tupleIndex: number): Promise<void> {
      const artifactPath = this.activeArtifactPath
      if (!artifactPath) {
        this.statusLine = 'Run or load an options expression artifact before inspecting tuple events.'
        return
      }
      if (!this.activeRankedResults[tupleIndex]) {
        this.statusLine = 'Select a valid ranked tuple before loading drilldown events.'
        return
      }
      this.inspectionBusy = true
      try {
        const inspection = await inspectOptionsExpressionTuple({
          artifactPath,
          tupleIndex,
        })
        this.activeTupleInspection = inspection
        this.statusLine = `Loaded ${inspection.rows.length} event row${inspection.rows.length === 1 ? '' : 's'} for tuple #${tupleIndex + 1}.`
      } catch (error) {
        this.statusLine = String(error || '').trim() || 'Failed to load tuple inspection rows.'
      } finally {
        this.inspectionBusy = false
      }
    },
  },
})
