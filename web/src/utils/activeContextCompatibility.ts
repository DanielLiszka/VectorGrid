import type { ShellMode } from './core/workflowModel'

export type ActiveRunCompatibilityCode =
  | 'compatible'
  | 'missing_active_dataset'
  | 'missing_run'
  | 'missing_run_dataset_identity'
  | 'dataset_mismatch'
  | 'mode_mismatch'
  | 'timeframe_mismatch'

export interface ActiveRunCompatibility {
  compatible: boolean
  code: ActiveRunCompatibilityCode
  reason: string
  activeDataId: string
  runDataId: string
  activeDatasetPath: string
  runDatasetPath: string
  activeMode: ShellMode
  runMode: ShellMode
}

export interface ActiveRunCompatibilityInput {
  activeDataId: unknown
  activeDatasetPath?: unknown
  activeMode: unknown
  activeTimeframe?: unknown
  run: unknown
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {}
}

function text(value: unknown): string {
  const next = String(value ?? '').trim()
  return next === '—' || next === '-' ? '' : next
}

function firstText(...values: unknown[]): string {
  for (const value of values) {
    const next = text(value)
    if (next) return next
  }
  return ''
}

function normalizedMode(value: unknown): ShellMode {
  const next = text(value).toLowerCase()
  if (next === 'options') return 'options'
  if (next === 'ml-signal' || next === 'ml_signal' || next === 'mlsignal') return 'ml-signal'
  return 'conventional'
}

function modeFromStrategyKind(value: unknown): ShellMode {
  const next = text(value).toLowerCase().replace(/[^a-z0-9]+/g, '')
  if (next.includes('option')) return 'options'
  if (next.includes('mlsignal')) return 'ml-signal'
  return 'conventional'
}

function runRequest(run: Record<string, unknown>): Record<string, unknown> {
  return asRecord(run.req ?? run.request)
}

function runResult(run: Record<string, unknown>): Record<string, unknown> {
  return asRecord(run.res ?? run.result)
}

function runDatasetMeta(run: Record<string, unknown>): Record<string, unknown> {
  return asRecord(run.datasetMeta ?? run.dataset_meta)
}

export function workflowRunDataId(value: unknown): string {
  const run = asRecord(value)
  const req = runRequest(run)
  const res = runResult(run)
  const datasetMeta = runDatasetMeta(run)
  return firstText(
    req.data_id,
    req.dataId,
    res.data_id,
    res.dataId,
    datasetMeta.data_id,
    datasetMeta.dataId,
    datasetMeta.loaded_id,
    datasetMeta.loadedId,
  )
}

export function workflowRunDatasetPath(value: unknown): string {
  const run = asRecord(value)
  const req = runRequest(run)
  const res = runResult(run)
  const datasetMeta = runDatasetMeta(run)
  return firstText(
    run.csv_path,
    run.csvPath,
    req.csv_path,
    req.csvPath,
    res.csv_path,
    res.csvPath,
    datasetMeta.csv_path,
    datasetMeta.csvPath,
    datasetMeta.path,
  )
}

function normalizedDatasetPath(value: unknown): string {
  return text(value)
    .replace(/^\\\\\?\\/u, '')
    .replace(/\//gu, '\\')
    .toLowerCase()
}

export function workflowRunMode(value: unknown): ShellMode {
  const run = asRecord(value)
  const req = runRequest(run)
  const res = runResult(run)
  const mlSignal = asRecord(run.mlSignal ?? run.ml_signal)
  if (Object.keys(mlSignal).length > 0) return 'ml-signal'
  return modeFromStrategyKind(
    firstText(
      req.strategy_kind,
      req.strategyKind,
      res.strategy_kind,
      res.strategyKind,
      run.artifactKind,
      run.artifact_kind,
    ),
  )
}

export function workflowRunTimeframe(value: unknown): string {
  const run = asRecord(value)
  const req = runRequest(run)
  const res = runResult(run)
  const datasetMeta = runDatasetMeta(run)
  return firstText(
    req.timeframe,
    res.timeframe,
    datasetMeta.timeframe,
    datasetMeta.derived_timeframe,
    datasetMeta.derivedTimeframe,
    datasetMeta.base_timeframe,
    datasetMeta.baseTimeframe,
  )
}

function result(
  input: ActiveRunCompatibilityInput,
  code: ActiveRunCompatibilityCode,
  reason: string,
): ActiveRunCompatibility {
  const activeMode = normalizedMode(input.activeMode)
  const runMode = workflowRunMode(input.run)
  return {
    compatible: code === 'compatible',
    code,
    reason,
    activeDataId: text(input.activeDataId),
    runDataId: workflowRunDataId(input.run),
    activeDatasetPath: text(input.activeDatasetPath),
    runDatasetPath: workflowRunDatasetPath(input.run),
    activeMode,
    runMode,
  }
}

export function activeRunCompatibility(
  input: ActiveRunCompatibilityInput,
): ActiveRunCompatibility {
  const activeDataId = text(input.activeDataId)
  if (!activeDataId) {
    return result(
      input,
      'missing_active_dataset',
      'Load the dataset used by this result before running or exporting it.',
    )
  }

  const run = asRecord(input.run)
  if (!Object.keys(run).length) {
    return result(
      input,
      'missing_run',
      'Run an optimization for the active dataset before using result actions.',
    )
  }

  const runDataId = workflowRunDataId(run)
  if (!runDataId) {
    return result(
      input,
      'missing_run_dataset_identity',
      'This result does not identify its source dataset. Re-run it before using result actions.',
    )
  }
  if (runDataId !== activeDataId) {
    return result(
      input,
      'dataset_mismatch',
      'This result belongs to a different dataset. Re-run optimization for the active dataset.',
    )
  }

  const activeDatasetPath = normalizedDatasetPath(input.activeDatasetPath)
  const runDatasetPath = normalizedDatasetPath(workflowRunDatasetPath(run))
  if (
    activeDatasetPath &&
    runDatasetPath &&
    activeDatasetPath !== runDatasetPath
  ) {
    return result(
      input,
      'dataset_mismatch',
      'This result belongs to a different dataset path. Re-run optimization for the active dataset.',
    )
  }

  const activeMode = normalizedMode(input.activeMode)
  const runMode = workflowRunMode(run)
  if (activeMode !== runMode) {
    return result(
      input,
      'mode_mismatch',
      `This is a ${runMode} result, not a ${activeMode} result. Return to the matching mode or run the current mode.`,
    )
  }

  const activeTimeframe = text(input.activeTimeframe)
  const runTimeframe = workflowRunTimeframe(run)
  if (activeTimeframe && runTimeframe && activeTimeframe !== runTimeframe) {
    return result(
      input,
      'timeframe_mismatch',
      'This result uses a different timeframe. Re-run optimization for the active timeframe.',
    )
  }

  return result(input, 'compatible', '')
}
