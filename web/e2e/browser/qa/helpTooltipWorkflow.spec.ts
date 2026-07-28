import { expect, type Page } from '@playwright/test'
import { helpTooltipReadyScenario } from '../fixtures'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow } from '../support/qaAssertions'

const visibleTooltipRole = (page: Page) => page.locator('[role="tooltip"]:visible')

async function expectVisibleHelpTriggersHaveOneTooltip(page: Page, surfaceName: string): Promise<void> {
  const triggers = page.locator('.contextHelpTrigger:visible')
  await expect
    .poll(() => triggers.count(), {
      message: `${surfaceName} visible help trigger count`,
    })
    .toBeGreaterThan(0)
  const triggerCount = await triggers.count()

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

defineWorkflow({
  id: 'help.tooltips.visibleRole',
  title: 'keeps visible contextual help triggers hoverable with one tooltip role',
  tags: ['@workflow', '@regression'],
  scenario: helpTooltipReadyScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.data').click()
    await expectVisibleHelpTriggersHaveOneTooltip(page, 'data workflow')

    await action('shell.workflowStep.validate').click()
    await expectVisibleHelpTriggersHaveOneTooltip(page, 'validation workflow')

    await action('shell.workspace.aggregate').click()
    await expectOptionalVisibleHelpTriggersHaveOneTooltip(page, 'aggregate workspace')

    await action('shell.workspace.live').click()
    await expectOptionalVisibleHelpTriggersHaveOneTooltip(page, 'live workspace')

    await expectNoHorizontalDocumentOverflow(page)
  },
})
