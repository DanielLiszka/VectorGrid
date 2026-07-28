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
      'Portfolio',
      'Live',
      'Settings',
      'Diagnostics',
    ])
    expect(state.workspaceItems.find((item) => item.id === 'aggregate')?.actionId).toBe('shell.workspace.aggregate')
    expect(state.workspaceItems.find((item) => item.id === 'portfolio')?.actionId).toBe('shell.workspace.portfolio')
    expect(state.workspaceItems.find((item) => item.id === 'pro')?.actionId).toBe('shell.workspace.diagnostics')
    expect(state.stageItems.map((item) => item.label)).toEqual([
      '1 Data Source',
      '2 Strategy Setup',
      '3 Validation',
      '4 Summary',
      '5 Final Review',
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
    expect(workbenchWorkspaceLabel('portfolio')).toBe('Portfolio')
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

  it('routes portfolio as a dedicated workspace without local setup/result tabs', () => {
    const state = buildWorkbenchNavigationState({
      workspace: 'portfolio',
      workflowStep: 'validate',
      controlsTab: 'opt',
      resultsTab: 'validate',
    })

    expect(state.activeWorkspaceId).toBe('portfolio')
    expect(state.workspaceItems.find((item) => item.id === 'portfolio')?.selected).toBe(true)
    expect(state.controlsItems).toEqual([])
    expect(state.resultsItems).toEqual([])
    expect(state.selectedShellTabId).toBe('workspace_tab_portfolio')
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

  it('keeps the ML dataset-export surface mounted during Data Setup', () => {
    const state = buildWorkbenchNavigationState({
      workspace: 'workflow',
      mode: 'ml-signal',
      workflowStep: 'data',
      controlsTab: 'data',
      resultsTab: 'overview',
    })

    expect(state.activeControlsTab).toBe('opt')
    expect(state.controlsItems.map((item) => item.id)).toEqual(['opt'])
    expect(state.resultsItems).toEqual([])
  })

  it('routes Options mode to the options research result panel', () => {
    const state = buildWorkbenchNavigationState({
      workspace: 'workflow',
      mode: 'options',
      workflowStep: 'validate',
      controlsTab: 'data',
      resultsTab: 'options',
    })

    expect(state.modeLabel).toBe('Options')
    expect(state.stageItems.map((item) => item.label)).toEqual([
      '1 Underlying Data',
      '2 Options Data',
      '3 Expression Run',
      '4 Validation',
      '5 Final Review',
    ])
    expect(state.controlsItems).toEqual([])
    expect(state.resultsItems.map((item) => item.panelId)).toEqual([
      'results_panel_options',
    ])
    expect(state.activeResultsTab).toBe('options')
  })

  it('keeps later conventional stages focused on evidence instead of optimize setup', () => {
    const validation = buildWorkbenchNavigationState({
      workspace: 'workflow',
      mode: 'conventional',
      workflowStep: 'validate',
      controlsTab: 'opt',
      resultsTab: 'validate',
    })

    expect(validation.controlsItems).toEqual([])
    expect(validation.activeControlsTab).toBe('')
    expect(validation.resultsItems.map((item) => item.id)).toEqual(['validate'])
    expect(validation.activeResultsTab).toBe('validate')

    const analyze = buildWorkbenchNavigationState({
      workspace: 'workflow',
      mode: 'conventional',
      workflowStep: 'analyze',
      controlsTab: 'opt',
      resultsTab: 'top',
    })

    expect(analyze.controlsItems).toEqual([])
    expect(analyze.resultsItems.map((item) => item.id)).toEqual([
      'drill',
      'top',
      'pareto',
      'heatmap',
      'overview',
    ])
    expect(analyze.activeResultsTab).toBe('top')

    const finalize = buildWorkbenchNavigationState({
      workspace: 'workflow',
      mode: 'conventional',
      workflowStep: 'finalize',
      controlsTab: 'advanced',
      resultsTab: 'meta',
    })

    expect(finalize.controlsItems).toEqual([])
    expect(finalize.resultsItems.map((item) => item.id)).toEqual(['final', 'meta'])
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
