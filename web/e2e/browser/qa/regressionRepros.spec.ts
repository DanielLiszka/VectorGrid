import { expect } from '@playwright/test'
import { optionsValidationReadyScenario, validationDatasetMismatchScenario } from '../fixtures/scenarios'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoStuckBusyState } from '../support/qaAssertions'

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
    await page.getByRole('button', { name: 'Refresh saved' }).click()
    await expect(page.getByRole('button', { name: /DoubleMa/i }).first()).toBeVisible()
    await page.getByRole('button', { name: /DoubleMa/i }).first().click()

    await expect(page.locator('[aria-label="Options expression follow-up"]')).toBeVisible()
    await expect(page.getByText(/Saved options expression artifacts/i)).toBeVisible()
    await expect(page.getByRole('button', { name: 'Load selected artifact' })).toBeEnabled()
    await page.getByRole('button', { name: 'Load selected artifact' }).click()
    await expect(page.getByText(/Loaded saved options expression artifact/i).first()).toBeVisible()

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

    const invokes = await page.evaluate(() =>
      ((window as typeof window & { __VTA_TEST_INVOKES?: string[] }).__VTA_TEST_INVOKES || []).slice(),
    )
    expect(invokes).toContain('analyze_options_expression_validation_feasibility')
    expect(invokes).toContain('validate_options_expression_artifact')
    expect(invokes).toContain('reveal_path')
    await expectNoStuckBusyState(page)
  },
})
