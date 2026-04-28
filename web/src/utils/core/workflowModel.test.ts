import { describe, expect, it } from 'vitest'
import {
  ML_SIGNAL_WORKFLOW_ALLOWED,
  WORKFLOW_ALLOWED,
  WORKFLOW_STEPS,
  WORKFLOW_TABS,
  WORKSPACE_TABS,
  mlSignalPrimaryActionForStep,
  normalizeControlsTab,
  normalizeResultsTab,
  normalizeShellMode,
  normalizeWorkflowStep,
  normalizeWorkflowTab,
  normalizeWorkspaceTab,
  shellModeLabel,
  workflowStepForMlSignalStage,
  workflowAllowedForMode,
  workflowDefaultsForMode,
  workflowStepFromWorkflow,
  workflowStepLabelForMode,
  workflowStepSubtitleForMode,
} from './workflowModel'

describe('workflowModel', () => {
  it('keeps workflow tabs, workflow steps, and workspaces distinct', () => {
    expect(WORKFLOW_TABS).toEqual([
      'data',
      'optimize',
      'validate',
      'analyze',
      'finalize',
      'history',
      'settings',
    ])
    expect(WORKFLOW_STEPS).toEqual([
      'data',
      'optimize',
      'validate',
      'analyze',
      'finalize',
    ])
    expect(WORKSPACE_TABS).toEqual([
      'workflow',
      'library',
      'aggregate',
      'pro',
      'live',
      'settings',
    ])
    expect(WORKFLOW_STEPS).not.toContain('history')
    expect(WORKFLOW_STEPS).not.toContain('settings')
    expect(workflowStepFromWorkflow('history')).toBe('data')
    expect(workflowStepFromWorkflow('settings')).toBe('data')
  })

  it('rejects unknown shell vocabulary values through the canonical normalizers', () => {
    expect(normalizeWorkflowTab(' HISTORY ')).toBe('history')
    expect(normalizeWorkflowTab('review')).toBe('')
    expect(normalizeWorkflowStep('settings')).toBe('')
    expect(normalizeWorkflowStep('validate')).toBe('validate')
    expect(normalizeWorkspaceTab('LIVE')).toBe('live')
    expect(normalizeWorkspaceTab('execution')).toBe('')
    expect(normalizeControlsTab('strategy')).toBe('strategy')
    expect(normalizeControlsTab('chart')).toBe('')
    expect(normalizeResultsTab('heatmap')).toBe('heatmap')
    expect(normalizeResultsTab('orders')).toBe('')
  })

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

  it('keeps conventional and ML Signal allowed-tab maps intentionally different', () => {
    expect(WORKFLOW_ALLOWED.data).toEqual({
      controls: ['data'],
      results: ['overview'],
    })
    expect(WORKFLOW_ALLOWED.optimize).toEqual({
      controls: ['opt', 'strategy', 'advanced'],
      results: ['overview', 'top', 'pareto', 'heatmap'],
    })
    expect(WORKFLOW_ALLOWED.settings).toEqual({
      controls: ['advanced'],
      results: ['meta', 'debug'],
    })
    expect(ML_SIGNAL_WORKFLOW_ALLOWED.optimize).toEqual({
      controls: ['opt'],
      results: ['overview'],
    })
    expect(ML_SIGNAL_WORKFLOW_ALLOWED.validate).toEqual({
      controls: ['opt'],
      results: ['overview'],
    })
    expect(ML_SIGNAL_WORKFLOW_ALLOWED.settings).toEqual({
      controls: ['advanced'],
      results: ['meta', 'debug'],
    })
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
