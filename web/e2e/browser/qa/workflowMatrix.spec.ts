import { expect } from '@playwright/test'
import { emptyAppScenario } from '../fixtures/scenarios'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow } from '../support/qaAssertions'

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
