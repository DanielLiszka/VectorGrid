import { expect, test, type Page } from '@playwright/test'
import { emptyAppScenario, optionsValidationReadyScenario } from '../fixtures/scenarios'
import { gotoMockedShellWithScenario } from '../support/workflowRunner'
import { installQaGuards } from '../support/qaAssertions'

const screenshotOptions = {
  animations: 'disabled' as const,
  fullPage: true,
  maxDiffPixelRatio: 0.015,
  scale: 'css' as const,
  threshold: 0.25,
}

async function stabilizeVisualSurface(page: Page): Promise<void> {
  await page.addStyleTag({
    content: `
      *, *::before, *::after {
        animation-duration: 0s !important;
        animation-delay: 0s !important;
        transition-duration: 0s !important;
        transition-delay: 0s !important;
        caret-color: transparent !important;
      }
    `,
  })
  await page.evaluate(async () => {
    await document.fonts?.ready
  })
  await page.waitForTimeout(150)
}

test('visual baseline: desktop workflow data state @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await gotoMockedShellWithScenario(page, context, emptyAppScenario)
  await stabilizeVisualSurface(page)

  await expect(page).toHaveScreenshot('desktop-workflow-data.png', screenshotOptions)
  await guard.assertClean()
})

test('visual baseline: desktop optimize advanced controls @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await gotoMockedShellWithScenario(page, context, emptyAppScenario)

  await page.locator('[data-qa-action="shell.workflowStep.optimize"]').click()
  await page.locator('[data-qa-action="shell.controlsTab.advanced"]').click()
  await expect(page.locator('#controls_panel_advanced')).toBeVisible()
  await stabilizeVisualSurface(page)

  await expect(page).toHaveScreenshot('desktop-optimize-advanced.png', screenshotOptions)
  await guard.assertClean()
})

test('visual baseline: desktop saved options artifact state @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await gotoMockedShellWithScenario(page, context, optionsValidationReadyScenario)

  await page.locator('[data-qa-action="shell.workspace.library"]').click()
  await page.locator('[data-qa-action="library.savedRun.refresh"]').click()
  await expect(page.getByRole('button', { name: /DoubleMa/i }).first()).toBeVisible()
  await page.getByRole('button', { name: /DoubleMa/i }).first().click()
  await expect(page.locator('[aria-label="Options expression follow-up"]')).toBeVisible()
  await stabilizeVisualSurface(page)

  await expect(page).toHaveScreenshot('desktop-library-options-artifact.png', screenshotOptions)
  await guard.assertClean()
})

test('visual baseline: mobile advanced shell @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 390, height: 844 })
  await gotoMockedShellWithScenario(page, context, emptyAppScenario)

  await expect(page.locator('#uiModeBar')).toHaveCount(0)
  await expect(page.locator('#workspaceBar')).toBeVisible()
  await stabilizeVisualSurface(page)

  await expect(page).toHaveScreenshot('mobile-advanced-shell.png', screenshotOptions)
  await guard.assertClean()
})
