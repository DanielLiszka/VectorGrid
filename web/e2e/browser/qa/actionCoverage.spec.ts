import { expect, test } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import { actionCoverageManifest } from './actionCoverageManifest'
import { emptyAppScenario } from '../fixtures/scenarios'
import { gotoMockedShellWithScenario } from '../support/workflowRunner'
import { installQaGuards } from '../support/qaAssertions'

async function visibleQaActions(page: import('@playwright/test').Page): Promise<string[]> {
  return page.locator('[data-qa-action]').evaluateAll((nodes) =>
    nodes
      .filter((node) => {
        const element = node as HTMLElement
        const rect = element.getBoundingClientRect()
        const style = window.getComputedStyle(element)
        return rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden' && style.display !== 'none'
      })
      .map((node) => String((node as HTMLElement).dataset.qaAction || '').trim())
      .filter(Boolean),
  )
}

test('visible QA actions are represented in the action coverage manifest @workflow', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await gotoMockedShellWithScenario(page, context, emptyAppScenario)

  const discovered = new Set<string>()
  async function collect(): Promise<void> {
    for (const id of await visibleQaActions(page)) discovered.add(id)
  }

  await collect()
  await page.locator('[data-qa-action="shell.commandPalette.open"]').click()
  await expect(page.locator('.cmdPaletteDialog')).toBeVisible()
  await collect()
  await page.keyboard.press('Escape')
  await expect(page.locator('.cmdPaletteDialog')).toBeHidden()
  await collect()
  await page.locator('[data-qa-action="shell.mode.mlSignal"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.mode.conventional"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workflowStep.optimize"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.controlsTab.strategy"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.controlsTab.advanced"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.controlsTab.optimize"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workflowStep.validate"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workflowStep.analyze"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.resultsTab.top"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.resultsTab.heatmap"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.resultsTab.overview"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workflowStep.finalize"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workspace.library"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workspace.aggregate"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workspace.diagnostics"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workspace.live"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.workspace.settings"]').click()
  await collect()
  await page.locator('[data-qa-action="shell.uiMode.guided"]').click()
  await collect()

  const entries = new Map(actionCoverageManifest.map((entry) => [entry.actionId, entry]))
  const unknown = [...discovered].filter((id) => !entries.has(id)).sort()
  const requiredUncovered = [...entries.values()]
    .filter((entry) => entry.required && entry.coveredBy.length === 0 && !entry.waiver)
    .map((entry) => entry.actionId)
    .sort()
  const expiredWaivers = [...entries.values()]
    .filter((entry) => entry.waiver?.expires && Date.parse(entry.waiver.expires) < Date.now())
    .map((entry) => entry.actionId)
    .sort()

  const report = {
    schemaVersion: 1,
    discovered: [...discovered].sort(),
    manifestEntries: actionCoverageManifest.length,
    unknown,
    requiredUncovered,
    expiredWaivers,
  }
  const reportPath = path.resolve(process.cwd(), '../output/playwright/action-coverage.json')
  await fs.mkdir(path.dirname(reportPath), { recursive: true })
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`)

  expect(unknown, 'visible actions missing from actionCoverageManifest').toEqual([])
  expect(requiredUncovered, 'required manifest entries without coverage').toEqual([])
  expect(expiredWaivers, 'expired action coverage waivers').toEqual([])
  await guard.assertClean()
})
