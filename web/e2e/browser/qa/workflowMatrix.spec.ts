import { expect } from '@playwright/test'
import { emptyAppScenario, liveReadyScenario, staleShellStateScenario } from '../fixtures'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow, expectNoStuckBusyState } from '../support/qaAssertions'

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

    await action('shell.workspace.portfolio').click()
    await expect(page.locator('#portfolioShellGrid')).toBeVisible()
    await expect(action('portfolio.refreshSources')).toBeVisible()
    await expect(action('portfolio.runFeasibility')).toBeVisible()
    await expect(action('portfolio.saved.selectLatest')).toBeVisible()
    await expect(action('portfolio.saved.clear')).toBeVisible()

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
    await action('shell.mode.options').click()
    await expect(action('shell.mode.options')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#results_panel_options')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Options Expression' })).toBeVisible()

    await action('shell.mode.conventional').click()
    await expect(action('shell.mode.conventional')).toHaveAttribute('aria-selected', 'true')

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
    await expect(action('heatmap.exportPng')).toBeVisible()

    await action('shell.resultsTab.pareto').click()
    await expect(page.locator('#results_panel_pareto')).toBeVisible()
    await expect(action('pareto.reviewTop')).toBeVisible()

    await action('shell.resultsTab.overview').click()
    await expect(page.locator('#results_panel_overview')).toBeVisible()

    await action('shell.workflowStep.validate').click()
    await expect(page.locator('#results_panel_validate')).toBeVisible()
    await expect(action('validation.cross.refresh')).toBeVisible()
    await expect(action('validation.cross.selectAll')).toBeVisible()
    await expect(action('validation.cross.selectNone')).toBeVisible()
    await expect(action('validation.cross.selectMatch')).toBeVisible()

    await action('shell.workflowStep.analyze').click()
    await expect(page.locator('#resultsTabBar')).toBeVisible()

    await action('shell.workflowStep.finalize').click()
    await expect(page.locator('#results_panel_final')).toBeVisible()
    await expect(page.locator('#results_panel_final').getByRole('heading', { name: 'Final Review' })).toBeVisible()
    await expect(action('shell.resultsTab.meta')).toBeVisible()
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
    await expect(action('data.timeframe.apply')).toBeDisabled()
    await expect(action('data.storage.openFolder')).toBeVisible()
    await expect(action('data.storage.manageSettings')).toBeVisible()
    await action('data.polygon.toggle').click()
    await expect(action('data.polygon.saveApiKey')).toBeDisabled()
    await expect(action('data.polygon.clearApiKey')).toBeDisabled()
    await expect(action('data.polygon.download')).toBeDisabled()
    await expect(action('data.polygon.loadSelected')).toBeDisabled()
    await expect(action('data.polygon.validateSelected')).toBeDisabled()
    await expect(action('data.polygon.repairSelected')).toBeDisabled()
    await expect(action('data.polygon.refresh')).toBeVisible()
    await expect(action('data.polygon.close')).toBeVisible()
    await expect(page.getByText(/Load a dataset/i).first()).toBeVisible()
    await expect(page.getByText(/TauriRuntimeError:/)).toHaveCount(0)
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
    await expect(action('optimize.run')).toBeDisabled()
    await expect(action('shell.primaryAction.run')).toBeDisabled()
    await expect(action('footer.primaryAction.run')).toBeDisabled()
    await expect(page.getByText(/Load a dataset before running optimization/i).first()).toBeVisible()

    await expect(page.getByText(/Load data first, then configure/i).first()).toHaveCount(0)

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
    await expect(action('commandPalette.command.run')).toHaveAttribute('aria-disabled', 'true')
    await page.keyboard.press('Enter')
    await expect(page.locator('.cmdPaletteDialog')).toBeVisible()

    await page.keyboard.press('Escape')
    await expect(page.locator('.cmdPaletteDialog')).toBeHidden()
    await expect(action('optimize.run')).toBeDisabled()
    await expectNoStuckBusyState(page)
    await expectNoHorizontalDocumentOverflow(page)
  },
})

defineWorkflow({
  id: 'shell.advancedOnly',
  title: 'keeps the shell on the advanced workbench without UI mode switching',
  tags: ['@workflow', '@responsive', '@regression'],
  scenario: emptyAppScenario,
  run: async ({ page }) => {
    await expect(page.locator('#uiModeBar')).toHaveCount(0)
    await expect(page.locator('#guidedShellGrid')).toHaveCount(0)
    await expect(page.locator('#workspaceBar')).toBeVisible()
    await expect(page.locator('#runFooterShell')).toBeVisible()
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

defineWorkflow({
  id: 'live.ready',
  title: 'loads local-paper broker review evidence in the live workspace',
  tags: ['@workflow'],
  scenario: liveReadyScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.live').click()
    await page.getByLabel('Execution target').selectOption('local_paper')
    await page.getByLabel('Symbol').fill('AAPL')
    await page.getByLabel('Symbol').press('Tab')

    await expect(action('live.account.refreshOrders')).toBeEnabled()
    await action('live.account.refreshOrders').click()
    await expect(page.getByText(/Loaded 0 recent orders for AAPL/i).first()).toBeVisible()
    await expect(page.getByText(/local-paper/i).first()).toBeVisible()
    await expect(page.getByText(/get local paper execution snapshot/i).first()).toBeVisible()
    await expectNoStuckBusyState(page)
    await expectNoHorizontalDocumentOverflow(page)
  },
})
