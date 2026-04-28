import { describe, expect, it } from 'vitest'
import { buildWorkbenchNavigationState, workbenchWorkspaceLabel } from './workbenchNavigationModel'

describe('workbenchNavigationModel', () => {
  it('builds the default workbench navigation contract', () => {
    const state = buildWorkbenchNavigationState({
      workspace: 'workflow',
      mode: 'conventional',
      workflowStep: 'optimize',
      controlsTab: 'strategy',
      resultsTab: 'top',
    })

    expect(state.activeWorkspaceId).toBe('workflow')
    expect(state.activeWorkflowStep).toBe('optimize')
    expect(state.activeControlsTab).toBe('strategy')
    expect(state.activeResultsTab).toBe('top')
    expect(state.workspaceItems.map((item) => item.label)).toEqual([
      'Research',
      'Library',
      'Runs',
      'Live',
      'Settings',
      'Diagnostics',
    ])
    expect(state.workspaceItems.find((item) => item.id === 'aggregate')?.actionId).toBe('shell.workspace.aggregate')
    expect(state.workspaceItems.find((item) => item.id === 'pro')?.actionId).toBe('shell.workspace.diagnostics')
    expect(state.stageItems.map((item) => item.label)).toEqual([
      '1 Data Source',
      '2 Strategy Setup',
      '3 Validation',
      '4 Summary',
      '5 Finalizer',
    ])
    expect(state.controlsItems.map((item) => item.actionId)).toEqual([
      'shell.controlsTab.optimize',
      'shell.controlsTab.strategy',
      'shell.controlsTab.advanced',
    ])
    expect(state.resultsItems.map((item) => item.actionId)).toEqual([
      'shell.resultsTab.overview',
      'shell.resultsTab.top',
      'shell.resultsTab.pareto',
      'shell.resultsTab.heatmap',
    ])
    expect(state.selectedShellTabId).toBe('workflow_step_tab_optimize')
    expect(state.selectionAnnouncement).toContain('Workspace Research.')
    expect(state.selectionAnnouncement).toContain('Setup panel Strategy.')
  })

  it('uses premium labels while preserving internal workspace ids', () => {
    expect(workbenchWorkspaceLabel('aggregate')).toBe('Runs')
    expect(workbenchWorkspaceLabel('pro')).toBe('Diagnostics')
    expect(workbenchWorkspaceLabel('unknown')).toBe('Research')
  })

  it('normalizes unknown selections to safe defaults', () => {
    const state = buildWorkbenchNavigationState({
      workspace: 'wat',
      mode: 'wat',
      workflowStep: 'wat',
      controlsTab: 'wat',
      resultsTab: 'wat',
    })

    expect(state.mode).toBe('conventional')
    expect(state.activeWorkspaceId).toBe('workflow')
    expect(state.activeWorkflowStep).toBe('data')
    expect(state.activeControlsTab).toBe('data')
    expect(state.activeResultsTab).toBe('overview')
  })

  it('keeps history and settings out of workflow-step navigation while preserving live workspace navigation', () => {
    const state = buildWorkbenchNavigationState({
      workspace: 'live',
      workflowStep: 'settings',
      controlsTab: 'debug',
      resultsTab: 'history',
    })

    expect(state.activeWorkspaceId).toBe('live')
    expect(state.activeWorkflowStep).toBe('data')
    expect(state.workspaceItems.find((item) => item.id === 'live')?.selected).toBe(true)
    expect(state.stageItems.map((item) => item.id)).toEqual([
      'data',
      'optimize',
      'validate',
      'analyze',
      'finalize',
    ])
    expect(state.stageItems.map((item) => item.id)).not.toContain('settings')
    expect(state.stageItems.map((item) => item.id)).not.toContain('history')
  })

  it('hides conventional result chrome for ML Signal workflow steps', () => {
    const state = buildWorkbenchNavigationState({
      workspace: 'workflow',
      mode: 'ml-signal',
      workflowStep: 'validate',
      controlsTab: 'opt',
      resultsTab: 'validate',
    })

    expect(state.stageItems.map((item) => item.label)).toEqual([
      '1 Data Setup',
      '2 Score Cache',
      '3 Policy Validation',
      '4 Policy Sweep',
      '5 Artifacts',
    ])
    expect(state.controlsItems.map((item) => item.id)).toEqual(['opt'])
    expect(state.resultsItems).toEqual([])
  })

  it('routes library, runs, and diagnostics to their preserved panel ids', () => {
    expect(
      buildWorkbenchNavigationState({ workspace: 'library', workflowStep: 'data' }).resultsItems.map((item) => item.panelId),
    ).toEqual(['results_panel_history'])
    expect(
      buildWorkbenchNavigationState({ workspace: 'aggregate', workflowStep: 'data' }).resultsItems.map((item) => item.panelId),
    ).toEqual(['results_panel_meta'])
    expect(buildWorkbenchNavigationState({ workspace: 'pro', workflowStep: 'data' }).controlsItems.map((item) => item.panelId)).toEqual([
      'controls_panel_advanced',
    ])
    expect(buildWorkbenchNavigationState({ workspace: 'pro', workflowStep: 'data' }).resultsItems.map((item) => item.panelId)).toEqual([
      'results_panel_debug',
    ])
  })
})
