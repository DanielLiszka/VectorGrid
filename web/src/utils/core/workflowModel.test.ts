import { describe, expect, it } from 'vitest'
import {
  mlSignalPrimaryActionForStep,
  normalizeShellMode,
  shellModeLabel,
  workflowStepForMlSignalStage,
  workflowAllowedForMode,
  workflowDefaultsForMode,
  workflowStepLabelForMode,
  workflowStepSubtitleForMode,
} from './workflowModel'

describe('workflowModel', () => {
  it('normalizes shell modes and resolves mode-scoped defaults and allowed tabs', () => {
    expect(normalizeShellMode('ML-SIGNAL')).toBe('ml-signal')
    expect(normalizeShellMode(' conventional ')).toBe('conventional')
    expect(normalizeShellMode('options')).toBe('')

    const conventionalDefaults = workflowDefaultsForMode('conventional')
    const mlDefaults = workflowDefaultsForMode('ml-signal')
    const conventionalAllowed = workflowAllowedForMode('conventional')
    const mlAllowed = workflowAllowedForMode('ml-signal')

    expect(conventionalDefaults.optimize).toEqual({
      controls: 'opt',
      results: 'overview',
    })
    expect(mlDefaults.validate).toEqual({
      controls: 'opt',
      results: 'overview',
    })
    expect(conventionalAllowed.optimize.controls).toEqual([
      'opt',
      'strategy',
      'advanced',
    ])
    expect(mlAllowed.optimize.controls).toEqual(['opt'])
    expect(mlAllowed.analyze.results).toEqual(['overview'])
  })

  it('derives mode-aware labels and subtitles while keeping conventional fallback intact', () => {
    expect(shellModeLabel('ml-signal')).toBe('ML Signal')
    expect(shellModeLabel('conventional')).toBe('Conventional')
    expect(workflowStepLabelForMode('validate', 'conventional')).toBe('Validation')
    expect(workflowStepLabelForMode('validate', 'ml-signal')).toBe('Policy Validation')
    expect(workflowStepSubtitleForMode('optimize', 'ml-signal')).toContain('score cache')
    expect(workflowStepSubtitleForMode('finalize', 'conventional')).toContain('Stage 5')
  })

  it('maps ML workflow steps and saved stages to truthful shared primary actions', () => {
    expect(mlSignalPrimaryActionForStep('data')).toMatchObject({
      kind: 'dataset_export',
      label: 'Export dataset',
      actionable: true,
    })
    expect(mlSignalPrimaryActionForStep('validate')).toMatchObject({
      kind: 'validation',
      label: 'Run ML validation',
      actionable: true,
    })
    expect(mlSignalPrimaryActionForStep('analyze')).toMatchObject({
      kind: 'policy_sweep',
      label: 'Run ML policy sweep',
      actionable: true,
    })
    expect(mlSignalPrimaryActionForStep('finalize')).toMatchObject({
      kind: 'artifacts',
      label: 'Artifacts ready',
      actionable: false,
    })

    expect(workflowStepForMlSignalStage('dataset_export')).toBe('data')
    expect(workflowStepForMlSignalStage('score_cache')).toBe('optimize')
    expect(workflowStepForMlSignalStage('validation')).toBe('validate')
    expect(workflowStepForMlSignalStage('policy_sweep')).toBe('analyze')
    expect(workflowStepForMlSignalStage('unknown')).toBe('finalize')
  })
})
