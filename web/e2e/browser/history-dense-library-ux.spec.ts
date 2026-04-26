import { expect, test } from '@playwright/test'
import { workspaceTabSelector } from '../desktop/support/selectors'
import { waitForShellReady } from './support/appHarness'
import { dismissStartupOverlay, installMockedTauri } from './support/mockTauri'

function buildDenseSavedRunFixture(count: number): Array<Record<string, unknown>> {
  return Array.from({ length: count }, (_value, index) => ({
    ts: `2026-04-11T${String(Math.floor(index / 6)).padStart(2, '0')}:${String((index % 6) * 10).padStart(2, '0')}:00Z`,
    path: `C:/runs/dense_run_${String(index + 1).padStart(3, '0')}.json`,
    bytes: 2048 + index,
    strategy_kind: index % 5 === 0 ? 'MlSignal' : 'DoubleMa',
    has_validation: index % 2 === 0,
    report_xlsx_path: index % 3 === 0 ? `C:/runs/dense_run_${String(index + 1).padStart(3, '0')}.xlsx` : '',
    csv_path: `C:/data/ticker_${String((index % 9) + 1).padStart(2, '0')}.csv`,
    tags: [`tag${index % 4}`, `bucket${index % 7}`],
  }))
}

test.describe('browser dense history library', () => {
  test('keeps the saved-run library bound to its own virtualized scroll host under load', async ({ context, page }) => {
    const denseSavedRuns = buildDenseSavedRunFixture(180)
    await installMockedTauri(context, {
      holdStartupOverlay: true,
      commandResults: {
        list_run_artifacts: denseSavedRuns,
      },
    })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)
    await waitForShellReady(page)

    await page.locator(workspaceTabSelector('library')).click()
    await page.getByRole('button', { name: 'Refresh' }).click()

    const savedRunsList = page.locator('ul[aria-label="Saved run artifacts"]')
    await expect(savedRunsList).toHaveClass(/historyVirtualListViewport/)
    await expect(page.getByText('2026-04-11T00:00:00Z')).toBeVisible()
    await expect(page.getByText('2026-04-11T20:00:00Z')).toHaveCount(0)

    const initialRenderedButtons = await page.locator('button[data-saved-run="1"]').count()
    expect(initialRenderedButtons).toBeGreaterThan(0)
    expect(initialRenderedButtons).toBeLessThan(denseSavedRuns.length)

    await savedRunsList.evaluate((element) => {
      element.scrollTop = element.scrollHeight
      element.dispatchEvent(new Event('scroll'))
    })
    await page.waitForTimeout(150)

    await expect(page.getByText('2026-04-11T29:50:00Z')).toBeVisible()
    const scrolledRenderedButtons = await page.locator('button[data-saved-run="1"]').count()
    expect(scrolledRenderedButtons).toBeGreaterThan(0)
    expect(scrolledRenderedButtons).toBeLessThan(denseSavedRuns.length)
  })
})
