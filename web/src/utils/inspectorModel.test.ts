import { describe, expect, it } from 'vitest'
import { resolveWorkbenchInspectorState } from './inspectorModel'

describe('inspectorModel', () => {
  it('defaults to selection context without leaking undefined values', () => {
    const state = resolveWorkbenchInspectorState({})

    expect(state.activeMode).toBe('selection')
    expect(state.sections.map((section) => section.value)).toEqual(['No dataset', 'No strategy', 'No candidate'])
    expect(JSON.stringify(state)).not.toContain('undefined')
    expect(state.actions.map((action) => action.id)).toContain('selection.openOptimize')
  })

  it('prioritizes active task context', () => {
    const state = resolveWorkbenchInspectorState({
      workspace: 'workflow',
      workflowStep: 'optimize',
      activeTaskCount: 2,
      taskSummary: 'Optimization running.',
    })

    expect(state.activeMode).toBe('tasks')
    expect(state.attention).toBe(true)
    expect(state.sections[0]).toMatchObject({
      id: 'active_tasks',
      value: '2 active',
      tone: 'accent',
    })
  })

  it('selects validation, history, and diagnostics from routing context', () => {
    expect(resolveWorkbenchInspectorState({ workflowStep: 'validate', validationStage: 'ready' }).activeMode).toBe('validation')
    expect(resolveWorkbenchInspectorState({ workspace: 'library' }).activeMode).toBe('history')
    expect(resolveWorkbenchInspectorState({ workspace: 'pro' }).activeMode).toBe('diagnostics')
  })

  it('honors a requested mode override', () => {
    const state = resolveWorkbenchInspectorState({
      requestedMode: 'history',
      workspace: 'workflow',
      workflowStep: 'data',
      historySummary: '3 saved runs',
      historyCanRefreshSaved: true,
      historyCanClearSession: true,
    })

    expect(state.activeMode).toBe('history')
    expect(state.modes.find((mode) => mode.id === 'history')?.current).toBe(true)
    expect(state.sections[0]?.value).toBe('3 saved runs')
    expect(state.actions.find((action) => action.id === 'history.refreshSaved')?.disabled).toBe(false)
    expect(state.actions.find((action) => action.id === 'history.clearSession')).toMatchObject({
      disabled: false,
      actionId: 'workbench.inspector.history.clearSession',
    })
  })

  it('projects validation compatibility, rows, and gates into validation mode', () => {
    const state = resolveWorkbenchInspectorState({
      requestedMode: 'validation',
      validationStage: 'ready',
      validationCompatibilitySummary: 'Compatible',
      validationCompatibilityDetail: 'Dataset and timeframe match.',
      validationTableRows: 12,
      validationGatesLabel: 'PSR>=0.8 · DSR>=0.6',
    })

    expect(state.sections.find((section) => section.id === 'compatibility')).toMatchObject({
      value: 'Compatible',
      tone: 'success',
    })
    expect(state.sections.find((section) => section.id === 'validation_rows')?.value).toBe('12 rows')
    expect(state.sections.find((section) => section.id === 'gates')?.value).toContain('PSR')
    expect(state.actions.map((action) => action.id)).toEqual(['validation.open'])
  })
})
