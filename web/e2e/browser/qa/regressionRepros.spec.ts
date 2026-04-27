import { expect } from '@playwright/test'
import {
  dataLoadFailureScenario,
  gpuStatusFailureScenario,
  liveSafetyReadyScenario,
  optionsValidationInputFailureScenario,
  optionsValidationPreviewFailureScenario,
  optionsValidationReadyScenario,
  savedOptionsArtifactFixturePack,
  validationDatasetMismatchScenario,
  validationSuccessScenario,
} from '../fixtures/scenarios'
import { defineWorkflow, readTauriInvokeLog, type WorkflowContext } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow, expectNoStuckBusyState } from '../support/qaAssertions'

type ActionLookup = WorkflowContext['action']

async function openSavedOptionsArtifact(page: import('@playwright/test').Page, action: ActionLookup): Promise<void> {
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
}

const savedRunRepairScenario = {
  ...optionsValidationReadyScenario,
  name: 'saved-run-repair-ready',
  description: 'A saved DoubleMa run can run a dry-run artifact repair check.',
  expectedInvokes: ['list_run_artifacts', 'load_run_artifact_detail', 'repair_run_artifact'],
}

const savedRunRepairWriteScenario = {
  ...optionsValidationReadyScenario,
  name: 'saved-run-repair-write-ready',
  description: 'A saved DoubleMa run can write a repaired artifact copy.',
  commandResults: {
    ...optionsValidationReadyScenario.commandResults,
    list_run_artifacts: savedOptionsArtifactFixturePack.repairedRunSummaries,
    repair_run_artifact: savedOptionsArtifactFixturePack.repairWrite,
  },
  expectedInvokes: ['list_run_artifacts', 'load_run_artifact_detail', 'repair_run_artifact'],
}

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
    const robustHeader = page.locator('.validationTableVue th', {
      has: page.getByRole('button', { name: /^Robust$/ }),
    }).first()
    await robustHeader.getByRole('button', { name: /^Robust$/ }).click()
    await expect(robustHeader).toHaveAttribute('aria-sort', 'ascending')
    await robustHeader.getByRole('button', { name: /^Resize Robust column$/ }).focus()
    await page.keyboard.press('ArrowRight')
    await expect
      .poll(async () => page.evaluate(() => window.localStorage.getItem('vta_validation_results_table_state_v1') || ''))
      .toContain('"robust":144')
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
  id: 'keyboard.core.validation',
  title: 'runs the core data-to-validation path with keyboard activation only',
  tags: ['@workflow', '@regression'],
  scenario: validationSuccessScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.data').press('Enter')
    await expect(action('data.sample.load').first()).toBeVisible()
    await action('data.sample.load').first().press('Enter')
    await expect
      .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'load_price_data'))
      .toBe(true)

    await action('shell.workflowStep.validate').press('Enter')
    await expect(action('validation.run')).toBeEnabled()
    await action('validation.run').press('Enter')

    await expect(page.getByText(/Validation completed in/i).first()).toBeVisible()
    await expect(page.locator('.validationTableVue tbody tr').first()).toBeVisible()
    await expectNoStuckBusyState(page)
    await expectNoHorizontalDocumentOverflow(page)
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
  id: 'data.load.failure',
  title: 'surfaces sample CSV load failure after the user activates the sample action',
  tags: ['@workflow', '@regression'],
  scenario: dataLoadFailureScenario,
  run: async ({ page, action }) => {
    await action('data.sample.load').first().click()
    await expect(page.getByText(/Fixture CSV read failed/i).first()).toBeVisible()
    await expect(action('data.sample.load').first()).toBeEnabled()
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'optimize.gpuStatus.failure',
  title: 'surfaces GPU status refresh failure without leaving optimize busy',
  tags: ['@workflow', '@regression'],
  scenario: gpuStatusFailureScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.optimize').click()
    const beforeRefreshCount = (await readTauriInvokeLog(page)).filter((entry) => entry.command === 'get_gpu_status').length

    await action('optimize.gpu.refreshStatus').first().click()
    await expect(page.getByText(/GPU status refresh failed:.*GPU probe failed in browser QA fixture/i).first()).toBeVisible()

    const afterRefreshCount = (await readTauriInvokeLog(page)).filter((entry) => entry.command === 'get_gpu_status').length
    expect(afterRefreshCount).toBeGreaterThan(beforeRefreshCount)
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'options.validation.controls',
  title: 'validates a saved options expression artifact from the Library panel',
  tags: ['@workflow', '@regression'],
  scenario: optionsValidationReadyScenario,
  run: async ({ page, action }) => {
    await openSavedOptionsArtifact(page, action)
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

defineWorkflow({
  id: 'library.savedRun.repairCheck',
  title: 'checks saved-run artifact repair from the Library panel',
  tags: ['@workflow', '@regression'],
  scenario: savedRunRepairScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await action('library.savedRun.refresh').click()
    await expect(page.getByRole('button', { name: /DoubleMa/i }).first()).toBeVisible()
    await page.getByRole('button', { name: /DoubleMa/i }).first().click()

    await expect(action('library.savedRun.repairCheck')).toBeEnabled()
    await action('library.savedRun.repairCheck').click()
    await page.locator('summary', { hasText: 'Artifact repair' }).click()
    await expect(page.getByText(/Dry-run found repairable changes/i).first()).toBeVisible()
    await expect(page.getByText(/canonicalized request payload/i).first()).toBeVisible()

    const repairInvokes = (await readTauriInvokeLog(page)).filter((entry) => entry.command === 'repair_run_artifact')
    expect(repairInvokes).toHaveLength(1)
    expect(repairInvokes[0]?.args).toMatchObject({
      path: 'C:/runs/vectorgrid_options_parent.json',
      dry_run: true,
    })
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'library.savedRun.repairWrite',
  title: 'writes a repaired saved-run artifact copy from the Library panel',
  tags: ['@workflow', '@regression'],
  scenario: savedRunRepairWriteScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await action('library.savedRun.refresh').click()
    await expect(page.getByRole('button', { name: /DoubleMa/i }).first()).toBeVisible()
    await page.getByRole('button', { name: /DoubleMa/i }).first().click()

    await expect(action('library.savedRun.repairWrite')).toBeEnabled()
    await action('library.savedRun.repairWrite').click()
    await page.locator('summary', { hasText: 'Artifact repair' }).click()
    await expect(page.getByText(/Wrote repaired artifact copy/i).first()).toBeVisible()
    await expect(page.getByText(/vectorgrid_options_parent\.repaired\.json/i).first()).toBeVisible()

    const repairInvokes = (await readTauriInvokeLog(page)).filter((entry) => entry.command === 'repair_run_artifact')
    expect(repairInvokes).toHaveLength(1)
    expect(repairInvokes[0]?.args).toMatchObject({
      path: 'C:/runs/vectorgrid_options_parent.json',
      dry_run: false,
      write_target: 'C:/runs/vectorgrid_options_parent.repaired.json',
    })
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'options.validation.inputFailure',
  title: 'blocks options validation preview on invalid date-range input before invoking Tauri',
  tags: ['@workflow', '@regression'],
  scenario: optionsValidationInputFailureScenario,
  run: async ({ page, action }) => {
    await openSavedOptionsArtifact(page, action)

    await page.getByLabel('Validation plan').selectOption('date_range_holdout')
    await page.getByLabel('Holdout start').fill('2030-04-26')
    await page.getByLabel('Holdout end').fill('2030-04-25')
    const beforePreviewCount = (await readTauriInvokeLog(page))
      .filter((entry) => entry.command === 'analyze_options_expression_validation_feasibility').length

    await action('options.validation.preview').click()
    await expect(page.getByText(/Validation start date must be before validation end date/i).first()).toBeVisible()

    const afterPreviewCount = (await readTauriInvokeLog(page))
      .filter((entry) => entry.command === 'analyze_options_expression_validation_feasibility').length
    expect(afterPreviewCount).toBe(beforePreviewCount)
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'options.validation.previewFailure',
  title: 'surfaces options validation feasibility preview failures from mocked Tauri',
  tags: ['@workflow', '@regression'],
  scenario: optionsValidationPreviewFailureScenario,
  run: async ({ page, action }) => {
    await openSavedOptionsArtifact(page, action)

    await page.getByLabel('Tuple indexes').fill('0')
    await page.getByLabel('Validation plan').selectOption('date_range_holdout')
    await page.getByLabel('Holdout start').fill('2030-04-25')
    await page.getByLabel('Holdout end').fill('2030-04-26')
    await action('options.validation.preview').click()

    await expect(page.getByText(/Options validation feasibility fixture failed/i).first()).toBeVisible()
    const invokes = (await readTauriInvokeLog(page)).map((entry) => entry.command)
    expect(invokes).toContain('analyze_options_expression_validation_feasibility')
    await expectNoStuckBusyState(page)
  },
})

defineWorkflow({
  id: 'live.safety.confirmations',
  title: 'keeps live safety actions behind explicit confirmation flows',
  tags: ['@workflow', '@regression'],
  scenario: liveSafetyReadyScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.live').click()
    await page.getByLabel('Execution target').selectOption('local_paper')
    await page.getByLabel('Symbol').fill('AAPL')
    await page.getByLabel('Symbol').press('Tab')

    await expect(action('live.account.refreshOrders')).toBeEnabled()
    await expect(action('live.safety.cancelOpenOrders')).toBeEnabled()
    await expect(action('live.safety.flattenPosition')).toBeEnabled()

    await expect(action('live.stream.start')).toBeEnabled()
    await action('live.stream.start').click()
    await expect(page.getByRole('alertdialog', { name: /Start live candle feed/i })).toBeVisible()
    await page
      .locator('.confirmationDialog')
      .getByRole('button', { name: /^Start live feed$/ })
      .click()
    await expect(page.getByText(/Live candles started for AAPL/i).first()).toBeVisible()
    await expect(action('live.safety.emergencyHalt')).toBeEnabled()

    await action('live.safety.cancelOpenOrders').click()
    await expect(page.getByRole('alertdialog', { name: /Cancel current-symbol open orders/i })).toBeVisible()
    await page
      .locator('.confirmationDialog')
      .getByRole('button', { name: /^Cancel open orders$/ })
      .click()
    await expect(page.getByText(/Cancelled 1 open broker order for AAPL/i).first()).toBeVisible()

    await action('live.safety.flattenPosition').click()
    await expect(page.getByRole('alertdialog', { name: /Flatten the current broker position/i })).toBeVisible()
    await page
      .locator('.confirmationDialog')
      .getByRole('button', { name: /^Flatten position$/ })
      .click()
    await expect(page.getByText(/Submitted SELL flatten order \(id=lp_flat_1\) for AAPL/i).first()).toBeVisible()

    await action('live.safety.emergencyHalt').click()
    await expect(page.getByRole('alertdialog', { name: /Emergency halt live automation/i })).toBeVisible()
    await page
      .locator('.confirmationDialog')
      .getByRole('button', { name: /^Emergency halt$/ })
      .click()
    await expect(page.getByText(/Emergency halt applied/i).first()).toBeVisible()
    await expect(action('live.safety.emergencyHalt')).toBeDisabled()

    const invokes = (await readTauriInvokeLog(page)).map((entry) => entry.command)
    expect(invokes).toContain('polygon_start_live_candle_stream')
    expect(invokes).toContain('cancel_local_paper_open_orders')
    expect(invokes).toContain('flatten_local_paper_position')
    expect(invokes).toContain('emergency_halt_live_execution')
    await expectNoStuckBusyState(page)
    await expectNoHorizontalDocumentOverflow(page)
  },
})
