import { expect, test, type BrowserContext, type Page } from '@playwright/test'
import { waitForShellReady } from './support/appHarness'
import { dismissStartupOverlay, installMockedTauri } from './support/mockTauri'

const visibleTooltipRole = (page: Page) => page.locator('[role="tooltip"]:visible')

async function openShellWithHelpMode(
  page: Page,
  context: BrowserContext,
  helpMode: 'full' | 'concise' | 'off',
  options: { strictUnknownInvoke?: boolean } = {},
): Promise<void> {
  await context.addInitScript((mode) => {
    try {
      localStorage.setItem('vta_help_mode_v1', mode)
      localStorage.setItem('vta_ui_mode_v1', 'advanced')
      localStorage.setItem('vta_workflow_step_v1', 'data')
    } catch {
      // Ignore storage failures in the browser harness.
    }
  }, helpMode)

  await installMockedTauri(context, {
    holdStartupOverlay: true,
    strictUnknownInvoke: options.strictUnknownInvoke ?? true,
  })
  await page.goto('/', { waitUntil: 'networkidle' })
  await dismissStartupOverlay(page)
  await waitForShellReady(page)
  await page.locator('#workflowStepBar [data-workflow-step="data"]').click()
}

async function openDataStepWithHelpMode(page: Page, context: BrowserContext, helpMode: 'full' | 'concise' | 'off'): Promise<void> {
  await openShellWithHelpMode(page, context, helpMode)
}

async function openWorkflowStep(page: Page, step: string): Promise<void> {
  const tab = page.locator(`#workflowStepBar [data-workflow-step="${step}"]`)
  await tab.click()
  await expect(tab).toHaveAttribute('aria-selected', 'true')
}

async function openWorkspace(page: Page, workspace: string): Promise<void> {
  const tab = page.locator(`#workspaceBar [data-workspace="${workspace}"]`)
  await tab.click()
  await expect(tab).toHaveAttribute('aria-selected', 'true')
}

async function expectVisibleHelpTriggersHaveOneTooltip(page: Page, surfaceName: string): Promise<void> {
  const triggers = page.locator('.contextHelpTrigger:visible')
  const triggerCount = await triggers.count()
  expect(triggerCount, `${surfaceName} visible help trigger count`).toBeGreaterThan(0)

  for (let index = 0; index < triggerCount; index += 1) {
    const trigger = triggers.nth(index)
    const label = (await trigger.getAttribute('aria-label')) || `${surfaceName} help trigger ${index + 1}`

    await page.mouse.move(8, 8)
    await expect(visibleTooltipRole(page), `${label} tooltip reset`).toHaveCount(0, { timeout: 1_000 })

    await trigger.scrollIntoViewIfNeeded()
    await trigger.hover()

    const tooltip = visibleTooltipRole(page)
    await expect(tooltip, `${label} visible tooltip role count`).toHaveCount(1, { timeout: 1_800 })
    const tooltipText = String(await tooltip.first().innerText()).trim()
    expect(tooltipText.length, `${label} tooltip text length`).toBeGreaterThan(0)

    await page.mouse.move(8, 8)
    await expect(visibleTooltipRole(page), `${label} tooltip closes after pointer leave`).toHaveCount(0, { timeout: 1_200 })
  }
}

async function expectOptionalVisibleHelpTriggersHaveOneTooltip(page: Page, surfaceName: string): Promise<void> {
  const triggerCount = await page.locator('.contextHelpTrigger:visible').count()
  if (triggerCount === 0) return
  await expectVisibleHelpTriggersHaveOneTooltip(page, surfaceName)
}

test.describe('browser help tooltip behavior', () => {
  test('opens full help on hover and closes it after pointer leave', async ({ page, context }) => {
    await openDataStepWithHelpMode(page, context, 'full')

    const trigger = page.getByRole('button', { name: 'Storage controls help' })
    await expect(trigger).toBeVisible()
    await trigger.scrollIntoViewIfNeeded()
    await trigger.hover()

    const card = page.locator('.contextHelpCard').filter({ hasText: 'Review the active market-data folder' })
    await page.waitForTimeout(300)
    await expect(card).toBeHidden({ timeout: 50 })

    await expect(card).toBeVisible({ timeout: 1_600 })
    await expect(visibleTooltipRole(page)).toHaveCount(1)
    await expect(card).toContainText('Storage controls')
    await expect(card).toContainText('Directory-level storage management')

    await page.mouse.move(8, 8)
    await expect(card).toBeHidden({ timeout: 1_000 })
  })

  test('opens concise help on hover without rendering the long detail body', async ({ page, context }) => {
    await openDataStepWithHelpMode(page, context, 'concise')

    const trigger = page.getByRole('button', { name: 'Storage controls help' })
    await expect(trigger).toBeVisible()
    await trigger.scrollIntoViewIfNeeded()
    await trigger.hover()

    const tooltip = page.locator('.contextHelpTooltip').filter({ hasText: 'Review the active market-data folder' })
    await page.waitForTimeout(200)
    await expect(tooltip).toBeHidden({ timeout: 50 })

    await expect(tooltip).toBeVisible({ timeout: 1_000 })
    await expect(visibleTooltipRole(page)).toHaveCount(1)
    await expect(tooltip).toContainText('Review the active market-data folder')
    await expect(tooltip).not.toContainText('Directory-level storage management')

    await page.mouse.move(8, 8)
    await expect(tooltip).toBeHidden({ timeout: 1_000 })
  })

  test('hides optional help triggers when help mode is off', async ({ page, context }) => {
    await openDataStepWithHelpMode(page, context, 'off')

    await expect(page.getByRole('button', { name: 'Storage controls help' })).toHaveCount(0)
  })

  test('keeps every visible app help trigger hoverable with a single tooltip role', async ({ page, context }) => {
    await openShellWithHelpMode(page, context, 'full', { strictUnknownInvoke: false })

    await test.step('data workflow help', async () => {
      await openWorkflowStep(page, 'data')
      await expectVisibleHelpTriggersHaveOneTooltip(page, 'data workflow')
    })

    await test.step('validation workflow help', async () => {
      await openWorkflowStep(page, 'validate')
      await expectVisibleHelpTriggersHaveOneTooltip(page, 'validation workflow')
    })

    await test.step('live workspace help', async () => {
      await openWorkspace(page, 'live')
      await expectOptionalVisibleHelpTriggersHaveOneTooltip(page, 'live workspace')
    })
  })
})
