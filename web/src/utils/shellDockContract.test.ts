import { describe, expect, it } from 'vitest'
import {
  isAnalysisDockVisible,
  isHistoryDockVisible,
  isOptimizeDockVisible,
  isValidationDockVisible,
} from './shellDockContract'

describe('shellDockContract', () => {
  it('hides the analysis dock in ml-signal workflow postures', () => {
    expect(isAnalysisDockVisible({ mode: 'ml-signal', workspace: 'workflow', workflow: 'optimize' })).toBe(false)
    expect(isAnalysisDockVisible({ mode: 'ml-signal', workspace: 'aggregate', workflow: 'optimize' })).toBe(true)
  })

  it('matches history dock visibility to library and history postures', () => {
    expect(isHistoryDockVisible({ workspace: 'library', workflow: 'data' })).toBe(true)
    expect(isHistoryDockVisible({ workspace: 'workflow', workflow: 'history' })).toBe(true)
    expect(isHistoryDockVisible({ workspace: 'workflow', workflow: 'optimize' })).toBe(false)
  })

  it('limits optimize and validation docks to conventional workflow postures', () => {
    expect(isOptimizeDockVisible({ mode: 'conventional', workspace: 'workflow', controlsTab: 'opt' })).toBe(true)
    expect(isOptimizeDockVisible({ mode: 'ml-signal', workspace: 'workflow', controlsTab: 'opt' })).toBe(false)
    expect(isValidationDockVisible({ mode: 'conventional', workspace: 'workflow', resultsTab: 'validate' })).toBe(true)
    expect(isValidationDockVisible({ mode: 'ml-signal', workspace: 'workflow', resultsTab: 'validate' })).toBe(false)
  })
})
