import { expect, test, type BrowserContext, type Page } from '@playwright/test'
import { emptyAppScenario, validationSuccessScenario } from './fixtures'
import { shellSelectors, workspaceTabSelector } from '../desktop/support/selectors'
import { waitForShellReady } from './support/appHarness'
import {
  dismissStartupOverlay,
  installMockedTauri,
  mockTauriSequence,
} from './support/mockTauri'

type InvokeCounts = Record<string, number>

async function installBootstrapHarness(
  context: BrowserContext,
  input: {
    localStorage: Record<string, string>
    commandResults: Record<string, unknown>
  },
): Promise<void> {
  await context.addInitScript((values) => {
    for (const [key, value] of Object.entries(values)) {
      try {
        localStorage.setItem(key, value)
      } catch {
        // The app's storage fallback remains under test when storage is unavailable.
      }
    }
  }, input.localStorage)
  await installMockedTauri(context, {
    holdStartupOverlay: true,
    commandResults: input.commandResults,
  })
}

async function openShell(page: Page): Promise<void> {
  await page.goto('/', { waitUntil: 'networkidle' })
  await dismissStartupOverlay(page)
  await waitForShellReady(page)
}

async function reloadShell(page: Page): Promise<void> {
  await page.reload({ waitUntil: 'networkidle' })
  await dismissStartupOverlay(page)
  await waitForShellReady(page)
}

async function readInvokeCounts(page: Page): Promise<InvokeCounts> {
  return page.evaluate(() => {
    const counts = (
      window as typeof window & {
        __VTA_TEST_INVOKE_COUNTS?: Record<string, number>
      }
    ).__VTA_TEST_INVOKE_COUNTS
    return { ...(counts ?? {}) }
  })
}

async function expectInvokeCount(
  page: Page,
  command: string,
  expected: number,
): Promise<void> {
  await expect
    .poll(async () => Number((await readInvokeCounts(page))[command] || 0))
    .toBe(expected)
}

test.describe('cold bootstrap recovery @workflow', () => {
  test('restores workspace, dataset, and latest artifact exactly once per cold reload', async ({
    context,
    page,
  }) => {
    await installBootstrapHarness(context, {
      localStorage: {
        vta_ui_mode_v1: 'advanced',
        vta_workspace_tab_v1: 'workflow',
        vta_workflow_step_v1: 'optimize',
        vectorgrid_controls_tab_v1: 'opt',
        vectorgrid_results_tab_v1: 'overview',
        vta_data_startup_snapshot_v1: JSON.stringify({
          autoRestoreDataset: true,
          baseCsvPath: 'C:/fixtures/aapl_1m.csv',
          selectedTimeframe: '1m',
          source: 'csv',
          baseSourceSizeBytes: null,
          baseSourceMtimeUtcMs: null,
          savedAtUnixMs: 1_900_000_000_000,
        }),
      },
      commandResults: validationSuccessScenario.commandResults,
    })

    await openShell(page)

    await expect(page.locator(workspaceTabSelector('workflow'))).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#workflowStepBar [data-workflow-step="optimize"]')).toHaveAttribute(
      'aria-selected',
      'true',
    )
    await expect(page.locator('[data-qa-state="workbench.cockpit.card.dataset"]')).toContainText('aapl_1m.csv')
    await expect(page.locator('[data-qa-state="workbench.cockpit.card.results"]')).not.toContainText('No run yet')
    await expectInvokeCount(page, 'load_price_data', 1)
    await expectInvokeCount(page, 'load_latest_run_artifact_hydration', 1)

    await reloadShell(page)

    await expect(page.locator(workspaceTabSelector('workflow'))).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#workflowStepBar [data-workflow-step="optimize"]')).toHaveAttribute(
      'aria-selected',
      'true',
    )
    await expect(page.locator('[data-qa-state="workbench.cockpit.card.dataset"]')).toContainText('aapl_1m.csv')
    await expect(page.locator('[data-qa-state="workbench.cockpit.card.results"]')).not.toContainText('No run yet')
    await expectInvokeCount(page, 'load_price_data', 2)
    await expectInvokeCount(page, 'load_latest_run_artifact_hydration', 2)

    await page.waitForTimeout(200)
    const counts = await readInvokeCounts(page)
    expect(counts.load_price_data).toBe(2)
    expect(counts.load_latest_run_artifact_hydration).toBe(2)
  })

  test('falls back to a usable default shell when persisted bootstrap snapshots are malformed', async ({
    context,
    page,
  }) => {
    await installBootstrapHarness(context, {
      localStorage: {
        vta_ui_mode_v1: 'advanced',
        vta_workspace_tab_v1: 'not-a-workspace',
        vta_workflow_step_v1: 'not-a-step',
        vta_workspace_split_v1: 'not-a-number',
        vta_data_startup_snapshot_v1: '{"autoRestoreDataset":true,',
      },
      commandResults: emptyAppScenario.commandResults,
    })

    await openShell(page)

    await expect(page.locator(workspaceTabSelector('workflow'))).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#workflowStepBar [data-workflow-step="data"]')).toHaveAttribute(
      'aria-selected',
      'true',
    )
    await expect(page.locator('[data-qa-state="workbench.cockpit.card.dataset"]')).toContainText('No dataset')
    await expect(page.locator(shellSelectors.shellHeader)).toBeVisible()
    await expectInvokeCount(page, 'load_price_data', 0)
    await expectInvokeCount(page, 'load_latest_run_artifact_hydration', 1)

    const rewrittenSnapshot = await page.evaluate(() =>
      JSON.parse(localStorage.getItem('vta_data_startup_snapshot_v1') || 'null'),
    )
    expect(rewrittenSnapshot).toMatchObject({
      autoRestoreDataset: true,
      baseCsvPath: '',
    })
  })

  test('keeps the shell usable and offers retry when latest-artifact hydration fails', async ({
    context,
    page,
  }) => {
    const successfulHydration =
      validationSuccessScenario.commandResults.load_latest_run_artifact_hydration
    await installBootstrapHarness(context, {
      localStorage: {
        vta_ui_mode_v1: 'advanced',
        vta_workspace_tab_v1: 'workflow',
        vta_workflow_step_v1: 'data',
        vta_data_startup_snapshot_v1: JSON.stringify({
          autoRestoreDataset: true,
          baseCsvPath: 'C:/fixtures/aapl_1m.csv',
          selectedTimeframe: '1m',
          source: 'csv',
          baseSourceSizeBytes: null,
          baseSourceMtimeUtcMs: null,
          savedAtUnixMs: 1_900_000_000_000,
        }),
      },
      commandResults: {
        ...validationSuccessScenario.commandResults,
        load_latest_run_artifact_hydration: mockTauriSequence(
          { __mockFailure: 'Artifact store temporarily unavailable.' },
          successfulHydration,
        ),
      },
    })

    await openShell(page)

    await expect(page.locator(shellSelectors.titlebar)).toBeVisible()
    await expect(page.locator(shellSelectors.shellHeader)).toBeVisible()
    await expect(page.locator('[data-qa-state="workbench.cockpit.card.dataset"]')).toBeVisible()
    const warning = page.locator('.runtimeBanner.restoreWarning')
    await expect(warning).toBeVisible()
    await expect(warning).toContainText('The workspace remains usable')
    await expectInvokeCount(page, 'load_latest_run_artifact_hydration', 1)

    await warning.getByRole('button', { name: 'Retry latest artifact' }).click()

    await expect(warning).toBeHidden()
    await expect(page.locator('[data-qa-state="workbench.cockpit.card.results"]')).not.toContainText('No run yet')
    await expectInvokeCount(page, 'load_latest_run_artifact_hydration', 2)
    await expect(page.locator(shellSelectors.shellHeader)).toBeVisible()
  })
})
