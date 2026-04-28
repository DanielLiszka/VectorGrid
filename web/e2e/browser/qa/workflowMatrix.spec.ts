import { expect } from '@playwright/test'
import { emptyAppScenario } from '../fixtures/scenarios'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow, expectNoStuckBusyState } from '../support/qaAssertions'

const staleShellStateScenario = {
  ...emptyAppScenario,
  name: 'stale-shell-state',
  description: 'Persisted workflow tab/step state contains stale tab vocabulary that must repair on launch.',
  localStorage: {
    ...emptyAppScenario.localStorage,
    vta_workspace_tab_v1: 'workflow',
    vta_workflow_tab_v1: 'settings',
    vta_workflow_step_v1: 'settings',
    vectorgrid_controls_tab_v1: 'advanced',
    vectorgrid_results_tab_v1: 'debug',
  },
}

defineWorkflow({
  id: 'shell.workspace.tabs',
  title: 'navigates every major workspace with stable QA actions',
  tags: ['@workflow', '@smoke'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await expect(page.locator('#results_panel_history')).toBeVisible()

    await action('shell.workspace.aggregate').click()
    await expect(page.locator('#results_panel_meta')).toBeVisible()

    await action('shell.workspace.diagnostics').click()
    await expect(page.locator('#results_panel_debug')).toBeVisible()

    await action('shell.workspace.live').click()
    await expect(page.locator('#liveShellGrid')).toBeVisible()
    await expect(page.getByText('Current live support envelope')).toBeVisible()

    await action('shell.workspace.settings').click()
    await expect(page.locator('#settingsShellGrid')).toBeVisible()

    await action('shell.workspace.workflow').click()
    await expect(page.locator('#workflowStepBar')).toBeVisible()
  },
})

defineWorkflow({
  id: 'shell.workflow.staleStateRepair',
  title: 'repairs stale persisted workflow state before projecting shell tabs',
  tags: ['@workflow', '@regression'],
  scenario: staleShellStateScenario,
  run: async ({ page, action }) => {
    await expect(action('shell.workspace.workflow')).toHaveAttribute('aria-selected', 'true')
    await expect(action('shell.workflowStep.data')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#controls_panel_data')).toBeVisible()
    await expect(page.locator('#results_panel_overview')).toBeVisible()
    await expect(page.locator('#results_panel_debug')).toBeHidden()
  },
})

defineWorkflow({
  id: 'shell.workflow.navigation',
  title: 'navigates workflow steps and local setup/result tabs',
  tags: ['@workflow', '@smoke'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.optimize').click()
    await expect(page.locator('#controls_panel_opt')).toBeVisible()
    await expect(action('optimize.run')).toBeVisible()

    await action('shell.controlsTab.strategy').click()
    await expect(page.locator('#controls_panel_strategy')).toBeVisible()

    await action('shell.controlsTab.advanced').click()
    await expect(page.locator('#controls_panel_advanced')).toBeVisible()

    await action('shell.resultsTab.top').click()
    await expect(page.locator('#results_panel_top')).toBeVisible()

    await action('shell.resultsTab.heatmap').click()
    await expect(page.locator('#results_panel_heatmap')).toBeVisible()

    await action('shell.workflowStep.validate').click()
    await expect(page.locator('#results_panel_validate')).toBeVisible()

    await action('shell.workflowStep.analyze').click()
    await expect(page.locator('#resultsTabBar')).toBeVisible()
  },
})

defineWorkflow({
  id: 'data.empty',
  title: 'shows blocked empty-data workflow actions without runtime errors',
  tags: ['@workflow'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await expect(action('data.csv.browse')).toBeVisible()
    await expect(action('data.csv.loadDraft')).toBeVisible()
    await expect(action('data.sample.load').first()).toBeVisible()
    await expect(page.getByText(/Load a dataset/i).first()).toBeVisible()
    await expectNoHorizontalDocumentOverflow(page)
  },
})

defineWorkflow({
  id: 'optimize.blocked.noData',
  title: 'keeps no-data optimization actions blocked and inspectable',
  tags: ['@workflow', '@regression'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.optimize').click()
    await expect(action('optimize.run')).toBeEnabled()
    await expect(action('shell.primaryAction.run')).toBeEnabled()
    await expect(action('footer.primaryAction.run')).toBeDisabled()
    await expect(page.getByText(/Load a dataset before running optimization/i).first()).toBeVisible()

    await action('shell.primaryAction.run').click()
    await expect(page.getByText(/Load data first, then configure/i).first()).toBeVisible()
    await action('optimize.run').click()
    await expect(page.getByText(/Load data first, then configure/i).first()).toBeVisible()

    await action('optimize.preset.apply').click()
    await expect(page.getByText(/Applied preset:/i).first()).toBeVisible()
    await action('optimize.backend.presetKernel').click()
    await expect(page.getByText(/Applied preset: GPU kernel/i).first()).toBeVisible()
    await action('optimize.gpu.refreshStatus').first().click()
    await expect(page.getByText(/GPU status refreshed/i).first()).toBeVisible()

    await action('shell.commandPalette.open').click()
    await expect(page.locator('.cmdPaletteDialog')).toBeVisible()
    await page.getByLabel('Command search').fill('run')
    await expect(action('commandPalette.command.run')).toBeVisible()
    await page.keyboard.press('Enter')
    await expect(page.locator('.cmdPaletteDialog')).toBeHidden()
    await expect(page.getByText(/Load data first, then configure/i).first()).toBeVisible()

    await expect(action('optimize.run')).toBeEnabled()
    await expectNoStuckBusyState(page)
    await expectNoHorizontalDocumentOverflow(page)
  },
})

defineWorkflow({
  id: 'shell.guided.handoff',
  title: 'switches into guided mode and back to advanced mode',
  tags: ['@workflow', '@responsive'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.uiMode.guided').click()
    await expect(page.locator('#guidedShellGrid')).toBeVisible()
    await expect(action('guided.primaryAction.run')).toBeVisible()
    await expect(action('guided.data.csv.browse')).toBeVisible()

    await action('guided.advancedMode.open').click()
    await expect(page.locator('#workspaceBar')).toBeVisible()
    await expect(action('shell.uiMode.advanced')).toHaveAttribute('aria-selected', 'true')
  },
})

defineWorkflow({
  id: 'shell.taskCenter',
  title: 'opens and closes the task center from shell and footer actions',
  tags: ['@workflow', '@smoke'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    const taskCenter = page.getByRole('dialog', { name: 'Task center' })

    await action('shell.taskCenter.open').click()
    await expect(taskCenter).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(taskCenter).toBeHidden()

    await action('footer.taskCenter.details').click()
    await expect(taskCenter).toBeVisible()
    await expect(page.getByText(/No active or recent commands/i).first()).toBeVisible()
  },
})

defineWorkflow({
  id: 'library.empty',
  title: 'keeps the empty artifact library refreshable',
  tags: ['@workflow'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await expect(page.getByText(/Library is empty/i).first()).toBeVisible()
    await expect(action('library.savedRun.refresh').first()).toBeEnabled()
    await action('library.savedRun.refresh').first().click()
    await expect(page.getByText(/Library is empty/i).first()).toBeVisible()
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'live.empty',
  title: 'keeps the empty live workspace safe and inspectable',
  tags: ['@workflow'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.live').click()
    await expect(page.locator('#liveShellGrid')).toBeVisible()
    await expect(page.getByText('Current live support envelope')).toBeVisible()
    await expect(action('live.account.refreshOrders')).toBeDisabled()
    await expect(action('live.safety.cancelOpenOrders')).toBeDisabled()
    await expect(action('live.safety.flattenPosition')).toBeDisabled()
    await expect(action('live.safety.emergencyHalt')).toBeDisabled()
    await expectNoHorizontalDocumentOverflow(page)
  },
})
