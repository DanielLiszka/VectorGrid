import { expect } from '@playwright/test'
import {
  optionsValidationReadyScenario,
  validationDatasetMismatchScenario,
  validationSuccessScenario,
} from '../fixtures/scenarios'
import { defineWorkflow, readTauriInvokeLog } from '../support/workflowRunner'
import { expectNoStuckBusyState } from '../support/qaAssertions'

defineWorkflow({
  id: 'validation.success.selected',
  title: 'runs conventional validation successfully from a loaded matching dataset',
  tags: ['@workflow', '@regression'],
  scenario: validationSuccessScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.data').click()
    await action('data.sample.load').first().click()
    await expect
      .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'load_price_data'))
      .toBe(true)

    await action('shell.workflowStep.validate').click()
    await expect(action('validation.run')).toBeEnabled()
    await action('validation.run').click()

    await expect(page.getByText(/Validation completed in/i).first()).toBeVisible()
    await expect(page.getByText(/Validation complete/i).first()).toBeVisible()
    await expect(page.locator('.validationTableVue tbody tr').first()).toBeVisible()
    const validationTableShell = page.locator('[aria-label="Validation results table shell"]')
    await expect(validationTableShell).toHaveAttribute('data-table-density', 'comfortable')
    await page.getByRole('group', { name: 'Validation density' }).getByRole('button', { name: 'compact' }).click()
    await expect(validationTableShell).toHaveAttribute('data-table-density', 'compact')
    await page.locator('[data-qa="validation.table.view.gates"]').click()
    await expect(page.locator('[data-qa-panel="validation.rowDetail"]')).toContainText(/Row 1 of 1/)
    await expect(page.locator('[data-qa-state="validation.rowDetail.gate"]')).toBeVisible()
    await expect(action('validation.export.shortlistCsv')).toBeEnabled()
    await expect(action('validation.export.validationJson')).toBeEnabled()
    await expect(action('validation.export.reportJson')).toBeEnabled()
    await expect(action('validation.export.reportXlsx')).toBeEnabled()
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'validation.blocked.datasetMismatch',
  title: 'reports validation dataset mismatch in UI without page errors',
  tags: ['@workflow', '@regression'],
  scenario: validationDatasetMismatchScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.validate').click()
    await expect(action('validation.run')).toBeVisible()
    await action('validation.run').click()
    await expect(page.getByText(/Validation failed:/).first()).toBeVisible()
    await expect(page.getByText(/Load the same dataset used by the optimization artifact/).first()).toBeVisible()
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'options.validation.controls',
  title: 'validates a saved options expression artifact from the Library panel',
  tags: ['@workflow', '@regression'],
  scenario: optionsValidationReadyScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await action('library.savedRun.refresh').click()
    await expect(page.getByRole('button', { name: /DoubleMa/i }).first()).toBeVisible()
    await page.getByRole('button', { name: /DoubleMa/i }).first().click()

    await expect(page.locator('[aria-label="Options expression follow-up"]')).toBeVisible()
    await expect(page.getByText(/Saved options expression artifacts/i)).toBeVisible()
    await action('options.artifacts.refresh').click()
    await expect(action('options.artifacts.load')).toBeEnabled()
    await action('options.artifacts.load').click()
    await expect(page.getByText(/Loaded saved options expression artifact/i).first()).toBeVisible()
    await expect(action('options.artifacts.revealActive')).toBeEnabled()

    await page.getByLabel('Tuple indexes').fill('0')
    await page.getByLabel('Validation plan').selectOption('date_range_holdout')
    await page.getByLabel('Holdout start').fill('2030-04-25')
    await page.getByLabel('Holdout end').fill('2030-04-26')
    await expect(action('options.validation.preview')).toBeEnabled()
    await expect(action('options.validation.run')).toBeEnabled()

    await action('options.validation.preview').click()
    await expect(page.getByText(/1 selected tuple/i).first()).toBeVisible()
    await expect(page.locator('.panelMuted.mono', { hasText: /Date-range holdout/i }).first()).toBeVisible()

    await action('options.validation.run').click()
    await expect(page.getByText(/Subsequent validation · accepted/i).first()).toBeVisible()
    await expect(page.locator('.gateBadge', { hasText: /Date-range holdout/i }).first()).toBeVisible()
    await expect(page.getByText(/Accepted with supported holdout and stress lanes/i).first()).toBeVisible()
    await expect(action('options.validation.reveal')).toBeEnabled()
    await action('options.validation.reveal').click()
    await expect(page.getByText(/Revealed C:\/runs\/vectorgrid_options_parent_options_validation\.json/i).first()).toBeVisible()

    const invokes = (await readTauriInvokeLog(page)).map((entry) => entry.command)
    expect(invokes).toContain('analyze_options_expression_validation_feasibility')
    expect(invokes).toContain('validate_options_expression_artifact')
    expect(invokes).toContain('reveal_path')
    await expectNoStuckBusyState(page)
  },
})
