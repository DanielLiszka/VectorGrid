import { describe, expect, it } from 'vitest'
import {
  normalizeShellMode,
  shellModeLabel,
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
})
