import { expect } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import { emptyAppScenario } from '../fixtures'
import { workbenchSelectors } from '../../desktop/support/selectors'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow } from '../support/qaAssertions'

defineWorkflow({
  id: 'workbench.shell.firstSlice',
  title: 'exercises the default premium workbench shell without regressing shell contracts',
  tags: ['@workflow', '@smoke'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    const visualEvidenceDir = path.resolve(process.cwd(), '../output/playwright/workbench-visual')
    await fs.mkdir(visualEvidenceDir, { recursive: true })

    await expect(page.locator(workbenchSelectors.root)).toBeVisible()
    await expect(page.locator(workbenchSelectors.rail)).toBeVisible()
    await expect(page.locator(workbenchSelectors.inspector)).toBeVisible()
    await expect(page.locator(workbenchSelectors.actionBar)).toBeVisible()
    await expect(page.locator(workbenchSelectors.researchCockpit)).toBeVisible()
    await expect(page.locator(workbenchSelectors.cockpitCard('dataset'))).toContainText('No dataset')
    await expect(page.locator(workbenchSelectors.cockpitCard('strategy'))).toContainText('DoubleMa')
    await expect(page.locator('#workspaceBar [data-workspace="workflow"]')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#dataVueShellMount')).toBeAttached()
    await expect(page.locator('#runFooterVueShellMount')).toBeAttached()

    await page.locator('#workspaceBar [data-workspace="workflow"]').focus()
    await page.keyboard.press('ArrowDown')
    await expect(page.locator('#workspaceBar [data-workspace="library"]')).toHaveAttribute('aria-selected', 'true')
    await page.keyboard.press('Home')
    await expect(page.locator('#workspaceBar [data-workspace="workflow"]')).toHaveAttribute('aria-selected', 'true')

    await page.locator('#workflowStepBar [data-workflow-step="data"]').focus()
    await page.keyboard.press('End')
    await expect(page.locator('#workflowStepBar [data-workflow-step="finalize"]')).toHaveAttribute('aria-selected', 'true')
    await page.keyboard.press('Home')
    await expect(page.locator('#workflowStepBar [data-workflow-step="data"]')).toHaveAttribute('aria-selected', 'true')

    await action('workbench.cockpit.step.configure').click()
    await expect(page.locator('#workflowStepBar [data-workflow-step="optimize"]')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#controls_panel_strategy')).toBeVisible()
    await action('workbench.cockpit.step.data').click()
    await expect(page.locator('#workflowStepBar [data-workflow-step="data"]')).toHaveAttribute('aria-selected', 'true')

    await action('shell.commandPalette.open').click()
    await expect(page.locator('.cmdPaletteDialog')).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(page.locator('.cmdPaletteDialog')).toBeHidden()

    await action('shell.workflowStep.optimize').click()
    await expect(page.locator('#workflowStepBar [data-workflow-step="optimize"]')).toHaveAttribute('aria-selected', 'true')
    await action('shell.controlsTab.optimize').click()
    await expect(page.locator('#controls_panel_opt')).toBeVisible()
    await expect(action('optimize.run')).toBeVisible()

    await action('shell.controlsTab.strategy').click()
    await expect(page.locator('#controls_panel_strategy')).toBeVisible()

    await action('shell.resultsTab.top').click()
    await expect(page.locator('#results_panel_top')).toBeVisible()

    await action('workbench.inspector.mode.validation').click()
    await expect(page.locator(workbenchSelectors.inspectorActiveMode)).toContainText('Validation')
    await expect(page.locator(workbenchSelectors.inspectorSection('compatibility'))).toBeVisible()
    await action('workbench.inspector.validation.open').click()
    await expect(page.locator('#workflowStepBar [data-workflow-step="validate"]')).toHaveAttribute('aria-selected', 'true')

    await action('workbench.inspector.mode.selection').click()
    await expect(page.locator(workbenchSelectors.inspectorActiveMode)).toContainText('Selection')
    await expect(page.locator(workbenchSelectors.inspectorSection('dataset'))).toBeVisible()

    await action('workbench.inspector.mode.history').click()
    await expect(page.locator(workbenchSelectors.inspectorActiveMode)).toContainText('History')
    await expect(page.locator(workbenchSelectors.inspectorSection('saved_runs'))).toBeVisible()
    await expect(action('workbench.inspector.history.refreshSaved')).toBeVisible()
    await expect(action('workbench.inspector.history.clearSession')).toBeVisible()

    await action('workbench.inspector.mode.tasks').click()
    await expect(page.locator(workbenchSelectors.inspectorActiveMode)).toContainText('Tasks')
    await expect(page.locator(workbenchSelectors.inspectorSection('active_tasks'))).toBeVisible()

    await action('workbench.inspector.mode.diagnostics').click()
    await expect(page.locator(workbenchSelectors.inspectorActiveMode)).toContainText('Diagnostics')
    await expect(page.locator(workbenchSelectors.inspectorSection('route'))).toBeVisible()

    await page.locator(workbenchSelectors.inspectorToggle).click()
    await expect(page.locator(workbenchSelectors.inspector)).toHaveAttribute('data-collapsed', 'true')
    await page.locator(workbenchSelectors.inspectorToggle).click()
    await expect(page.locator(workbenchSelectors.inspector)).toHaveAttribute('data-collapsed', 'false')

    await action('shell.workspace.live').click()
    await expect(page.locator('#liveShellGrid')).toBeVisible()
    await expect(page.getByText('Current live support envelope')).toBeVisible()

    await action('shell.workspace.workflow').click()
    await expect(page.locator('#workflowStepBar')).toBeVisible()

    for (const viewport of [
      { width: 1720, height: 1000 },
      { width: 1366, height: 768 },
      { width: 1024, height: 768 },
      { width: 390, height: 844 },
    ]) {
      await page.setViewportSize(viewport)
      await expect(page.locator(workbenchSelectors.root)).toBeVisible()
      await expect(page.locator('#workspaceBar')).toBeVisible()
      await expect(page.locator('#runFooterShell')).toBeVisible()
      if (viewport.width > 1180) {
        await expect(page.locator(workbenchSelectors.inspector)).toBeVisible()
      } else {
        await expect(page.locator(workbenchSelectors.inspector)).toBeHidden()
      }
      await expectNoHorizontalDocumentOverflow(page)
      await page.screenshot({
        path: path.join(visualEvidenceDir, `workbench-${viewport.width}x${viewport.height}.png`),
        fullPage: true,
      })
    }
  },
})
