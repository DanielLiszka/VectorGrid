import { expect, test } from '@playwright/test'
import { shellSelectors, workspaceTabSelector } from '../desktop/support/selectors'
import { gotoMockedShell } from './support/appHarness'

test.describe('browser shell ux', () => {
  test('supports keyboard workspace and workflow-step navigation without losing orientation', async ({ context, page }) => {
    await gotoMockedShell(page, context)

    const workflowTab = page.locator(workspaceTabSelector('workflow'))
    const libraryTab = page.locator(workspaceTabSelector('library'))
    const aggregateTab = page.locator(workspaceTabSelector('aggregate'))
    const proTab = page.locator(workspaceTabSelector('pro'))
    const settingsTab = page.locator(workspaceTabSelector('settings'))

    await workflowTab.focus()
    await expect(workflowTab).toBeFocused()
    await workflowTab.press('ArrowRight')
    await expect(libraryTab).toHaveAttribute('aria-selected', 'true')
    await expect(libraryTab).toBeFocused()
    await expect(page.locator('#results_panel_history')).toBeVisible()

    await libraryTab.press('ArrowRight')
    await expect(aggregateTab).toHaveAttribute('aria-selected', 'true')
    await expect(aggregateTab).toBeFocused()
    await expect(page.locator('#results_panel_meta')).toBeVisible()

    await aggregateTab.press('ArrowRight')
    await expect(proTab).toHaveAttribute('aria-selected', 'true')
    await expect(proTab).toBeFocused()

    await proTab.press('ArrowRight')
    await expect(settingsTab).toHaveAttribute('aria-selected', 'true')
    await expect(settingsTab).toBeFocused()
    await expect(page.locator(shellSelectors.settingsGrid)).toBeVisible()

    await settingsTab.press('Home')
    await expect(workflowTab).toHaveAttribute('aria-selected', 'true')
    await expect(workflowTab).toBeFocused()

    const dataStep = page.locator('#workflowStepBar [data-workflow-step="data"]')
    const optimizeStep = page.locator('#workflowStepBar [data-workflow-step="optimize"]')
    const validateStep = page.locator('#workflowStepBar [data-workflow-step="validate"]')
    const analyzeStep = page.locator('#workflowStepBar [data-workflow-step="analyze"]')

    await expect(dataStep).toHaveAttribute('aria-selected', 'true')
    await dataStep.focus()
    await expect(dataStep).toBeFocused()

    await dataStep.press('ArrowRight')
    await expect(optimizeStep).toHaveAttribute('aria-selected', 'true')
    await expect(optimizeStep).toBeFocused()
    await expect(page.locator('#controlsTabBar [data-tab="opt"]')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#results_panel_overview')).toBeVisible()

    await optimizeStep.press('ArrowRight')
    await expect(validateStep).toHaveAttribute('aria-selected', 'true')
    await expect(validateStep).toBeFocused()
    await expect(page.locator('#results_panel_validate')).toBeVisible()

    await validateStep.press('ArrowRight')
    await expect(analyzeStep).toHaveAttribute('aria-selected', 'true')
    await expect(analyzeStep).toBeFocused()
    await expect(page.locator('#resultsTabBar [aria-selected="true"]')).toBeVisible()
  })
})
