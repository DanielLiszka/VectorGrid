import { describe, expect, it } from 'vitest'
import { resolveWorkbenchPrimaryAction } from './workbenchActionModel'

describe('workbenchActionModel', () => {
  it('promotes cancellation over every normal primary action', () => {
    expect(resolveWorkbenchPrimaryAction({ canCancel: true, runDisabled: true, dataLoaded: false })).toEqual({
      kind: 'cancel',
      label: 'Stop run',
      disabled: false,
      disabledReason: '',
      tone: 'danger',
    })
  })

  it('blocks optimization until a dataset is loaded', () => {
    const action = resolveWorkbenchPrimaryAction({
      workflowStep: 'optimize',
      dataLoaded: false,
      runDisabled: false,
    })

    expect(action.label).toBe('Run optimization')
    expect(action.disabled).toBe(true)
    expect(action.disabledReason).toBe('Load a dataset before running optimization.')
  })

  it('returns a validation action on the validation step', () => {
    const action = resolveWorkbenchPrimaryAction({
      workflowStep: 'validate',
      dataLoaded: true,
      hasOptimizationResults: true,
      validateDisabled: false,
      validationStage: 'ready',
    })

    expect(action.kind).toBe('validate')
    expect(action.label).toBe('Run validation')
    expect(action.disabled).toBe(false)
  })

  it('surfaces the validation blocker when validation is disabled', () => {
    const action = resolveWorkbenchPrimaryAction({
      workflowStep: 'validate',
      dataLoaded: true,
      hasOptimizationResults: false,
      validateDisabled: true,
      validationStage: 'data_loaded_no_run',
    })

    expect(action.disabled).toBe(true)
    expect(action.disabledReason).toBe('Run an optimization before running validation.')
  })

  it('keeps ML Signal stage labels aligned with workflow step readiness', () => {
    expect(
      resolveWorkbenchPrimaryAction({
        mode: 'ml-signal',
        workflowStep: 'data',
        dataLoaded: true,
        runDisabled: false,
        mlSignalDatasetExportPath: 'C:/exports/ml.csv',
      }).label,
    ).toBe('Export dataset')

    const scoreCache = resolveWorkbenchPrimaryAction({
      mode: 'ml-signal',
      workflowStep: 'optimize',
      dataLoaded: true,
      runDisabled: false,
      mlSignalScoreInputPath: 'C:/scores.csv',
      mlSignalScoreCachePath: '',
    })
    expect(scoreCache.label).toBe('Build score cache')
    expect(scoreCache.disabled).toBe(true)
    expect(scoreCache.disabledReason).toBe('Choose a score-cache output path before building the cache.')

    const finalize = resolveWorkbenchPrimaryAction({
      mode: 'ml-signal',
      workflowStep: 'finalize',
      dataLoaded: true,
    })
    expect(finalize.kind).toBe('inspect')
    expect(finalize.label).toBe('Artifacts ready')
    expect(finalize.disabledReason).toBe('Current ML artifacts are inspect-only from this stage.')
  })

  it('allows native ML Signal score-cache actions without a score CSV input', () => {
    const action = resolveWorkbenchPrimaryAction({
      mode: 'ml-signal',
      workflowStep: 'optimize',
      dataLoaded: true,
      runDisabled: false,
      mlSignalScoreSource: 'native',
      mlSignalModelBundleId: 'bundle-a',
      mlSignalScoreInputPath: '',
      mlSignalScoreCachePath: 'C:/score_cache.json',
    })

    expect(action.label).toBe('Build score cache')
    expect(action.disabled).toBe(false)

    const missingBundle = resolveWorkbenchPrimaryAction({
      mode: 'ml-signal',
      workflowStep: 'optimize',
      dataLoaded: true,
      runDisabled: false,
      mlSignalScoreSource: 'native',
      mlSignalModelBundleId: '',
      mlSignalScoreInputPath: '',
      mlSignalScoreCachePath: 'C:/score_cache.json',
    })

    expect(missingBundle.disabled).toBe(true)
    expect(missingBundle.disabledReason).toBe('Choose an ML model bundle before native scoring.')
  })
})
