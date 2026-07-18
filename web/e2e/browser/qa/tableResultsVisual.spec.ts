import { expect } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import {
  aggregateMetaReadyScenario,
  optimizationSuccessScenario,
  validationSuccessScenario,
} from '../fixtures'
import { defineWorkflow, readTauriInvokeLog, type WorkflowContext } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow, expectNoStuckBusyState } from '../support/qaAssertions'

type ActionLookup = WorkflowContext['action']

const evidenceDir = path.resolve(process.cwd(), '../output/playwright/table-results')
const aggregateMetaTableScenario = {
  ...aggregateMetaReadyScenario,
  name: 'aggregate-meta-table-results',
  expectedInvokes: ['list_run_artifacts', 'run_meta_selection'],
}

async function ensureEvidenceDir(): Promise<void> {
  await fs.mkdir(evidenceDir, { recursive: true })
}

async function loadFixtureSampleData(
  page: import('@playwright/test').Page,
  action: ActionLookup,
): Promise<void> {
  await action('shell.workflowStep.data').click()
  await action('data.sample.load').first().click()
  await expect
    .poll(async () =>
      (await readTauriInvokeLog(page)).some(
        (entry) => entry.command === 'load_price_data',
      ),
    )
    .toBe(true)
}

defineWorkflow({
  id: 'tables.optimization.resultsPresentation',
  title: 'keeps optimization result tables dense, encoded, and visually inspectable',
  tags: ['@workflow', '@tables', '@visual'],
  scenario: optimizationSuccessScenario,
  viewport: { width: 1600, height: 1000 },
  run: async ({ page, action }) => {
    await ensureEvidenceDir()
    await loadFixtureSampleData(page, action)

    await action('shell.workflowStep.optimize').click()
    await expect(action('optimize.run')).toBeEnabled()
    await action('optimize.run').click()
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === 'run_double_ma_optimization',
        ),
      )
      .toBe(true)

    await action('shell.resultsTab.top').click()
    const shell = page.locator('[aria-label="Top optimization results table shell"]')
    await expect(shell).toHaveAttribute('data-table-has-rows', 'true')
    await expect(shell).toHaveAttribute('data-table-density', 'comfortable')
    await expect(page.locator('.topTableVue [data-top-row="1"]').first()).toBeVisible()
    await expect(page.locator('.topTableVue .appMetricCell[data-visual-kind="bar"]').first()).toBeVisible()

    await page.locator('[data-qa="top.table.density.compact"]').click()
    await expect(shell).toHaveAttribute('data-table-density', 'compact')
    await expectNoHorizontalDocumentOverflow(page)
    await page.locator('#results_panel_top').screenshot({
      path: path.join(evidenceDir, 'optimization-results-table.png'),
    })
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'tables.validation.resultsPresentation',
  title: 'keeps validation result tables readable with row detail and metric encoding',
  tags: ['@workflow', '@tables', '@visual'],
  scenario: validationSuccessScenario,
  viewport: { width: 1600, height: 1000 },
  run: async ({ page, action }) => {
    await ensureEvidenceDir()
    await loadFixtureSampleData(page, action)

    await action('shell.workflowStep.validate').click()
    await expect(action('validation.run')).toBeEnabled()
    await action('validation.run').click()

    const shell = page.locator('[aria-label="Validation results table shell"]')
    await expect(shell).toHaveAttribute('data-table-has-rows', 'true')
    await expect(shell).toHaveAttribute('data-table-density', 'comfortable')
    await expect(page.locator('.validationTableVue tbody tr').first()).toBeVisible()
    await expect(page.locator('.validationTableVue .appMetricCell[data-visual-kind="bar"]').first()).toBeVisible()

    await page.locator('[data-qa="validation.table.view.gates"]').click()
    await expect(page.locator('[data-qa-panel="validation.rowDetail"]')).toContainText(/Row 1 of 1/)
    await page.locator('[data-qa="validation.table.density.spacious"]').click()
    await expect(shell).toHaveAttribute('data-table-density', 'spacious')
    await expectNoHorizontalDocumentOverflow(page)
    await page.locator('#results_panel_validate').screenshot({
      path: path.join(evidenceDir, 'validation-results-table.png'),
    })
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'tables.meta.resultsPresentation',
  title: 'keeps meta-analysis result tables encoded, resizable, and density-controlled',
  tags: ['@workflow', '@tables', '@visual'],
  scenario: aggregateMetaTableScenario,
  viewport: { width: 1600, height: 1000 },
  run: async ({ page, action }) => {
    await ensureEvidenceDir()
    await action('shell.workspace.library').click()
    await action('library.savedRun.refresh').click()
    await expect(page.getByRole('button', { name: /CompositeMo/i }).first()).toBeVisible()

    await action('shell.workspace.aggregate').click()
    await expect(action('aggregate.meta.run')).toBeEnabled()
    await action('aggregate.meta.run').click()

    const shell = page.locator('[aria-label="Meta analysis results table shell"]')
    await expect(shell).toHaveAttribute('data-table-row-count', '1')
    await expect(shell).toHaveAttribute('data-table-density', 'comfortable')
    await expect(page.locator('.metaTableVue tbody tr').first()).toBeVisible()
    await expect(page.locator('.metaTableVue td[data-column-key="meta_score"] .appMetricCell[data-visual-kind="bar"]')).toBeVisible()

    await page.locator('[data-qa="meta.table.density.compact"]').click()
    await expect(shell).toHaveAttribute('data-table-density', 'compact')
    await expectNoHorizontalDocumentOverflow(page)
    await page.locator('#results_panel_meta').screenshot({
      path: path.join(evidenceDir, 'meta-results-table.png'),
    })
    await expectNoStuckBusyState(page)
  },
})
