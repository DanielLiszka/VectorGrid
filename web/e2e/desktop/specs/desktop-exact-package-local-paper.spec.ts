import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import {
  sha256File,
} from '../support/nativeFeatureFixtures.js'
import { invokeNative } from '../support/nativeIpc.js'
import { startOpenDialogHarness } from '../support/nativeWindowHarness.js'
import {
  isPathInside,
  isolatedAppDataPath,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import {
  nativeWorkflowSelectors,
  shellSelectors,
} from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type LocalPaperOrder = {
  id?: unknown
  symbol?: unknown
  side?: unknown
  status?: unknown
  qty?: unknown
  filled_qty?: unknown
  filled_avg_price?: unknown
}

type LocalPaperSnapshot = {
  mode?: unknown
  symbol?: unknown
  account_id?: unknown
  account_status?: unknown
  cash?: unknown
  portfolio_value?: unknown
  fetched_at_unix_ms?: unknown
  position?: {
    symbol?: unknown
    qty?: unknown
    side?: unknown
    avg_entry_price?: unknown
  } | null
  orders?: LocalPaperOrder[]
}

type BundledSample = {
  id?: unknown
  label?: unknown
  path?: unknown
  source_kind?: unknown
}

const SYMBOL = 'VGQA'
const APP_STATUS_SELECTOR =
  '[aria-label="Execution runtime status"] .statusGrid [role="status"]'

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function finiteNumber(value: unknown): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) {
    throw new Error(`Expected a finite number, received ${JSON.stringify(value)}.`)
  }
  return parsed
}

async function setFormValue(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await browser.execute((targetSelector: string, nextValue: string) => {
    const page = globalThis as unknown as {
      document: {
        querySelector: (value: string) => {
          value: string
          dispatchEvent: (event: unknown) => boolean
        } | null
      }
      Event: new (type: string, init: { bubbles: boolean }) => unknown
    }
    const element = page.document.querySelector(targetSelector)
    if (!element) throw new Error(`Expected a form control for ${targetSelector}.`)
    element.value = nextValue
    element.dispatchEvent(new page.Event('input', { bubbles: true }))
    element.dispatchEvent(new page.Event('change', { bubbles: true }))
  }, selector, value)
  await browser.waitUntil(
    async () => text(await field.getValue()) === value,
    {
      timeout: 3_000,
      interval: 50,
      timeoutMsg: `${selector} did not retain ${JSON.stringify(value)}.`,
    },
  )
}

async function confirmExecutionAction(
  expectedTitle: string,
  confirmLabel: string,
): Promise<void> {
  const title = await $('#execution_confirmation_title')
  await title.waitForDisplayed({ timeout: 5_000 })
  await expect(title).toHaveText(expectedTitle)
  const confirm = await $(
    `//section[@role="alertdialog"]//button[normalize-space()="${confirmLabel}"]`,
  )
  await confirm.waitForEnabled({ timeout: 5_000 })
  await confirm.click()
  await browser.waitUntil(
    async () => !(await title.isDisplayed().catch(() => false)),
    {
      timeout: 5_000,
      interval: 50,
      timeoutMsg: `${expectedTitle} confirmation did not close.`,
    },
  )
}

async function waitForAppStatus(
  predicate: (value: string) => boolean,
  timeoutMsg: string,
): Promise<string> {
  const status = await $(APP_STATUS_SELECTOR)
  await status.waitForDisplayed({ timeout: 10_000 })
  let latest = ''
  await browser.waitUntil(
    async () => {
      latest = text(await status.getText())
      return predicate(latest)
    },
    {
      timeout: 30_000,
      interval: 100,
      timeoutMsg,
    },
  )
  return latest
}

async function loadBundledSampleThroughNativePicker(): Promise<{
  bundledPath: string
  selectedPath: string
  sha256: string
}> {
  const bundled = (await invokeNative<BundledSample[]>('list_sample_csvs'))
    .find((entry) => {
      const candidate = text(entry.path)
      return candidate.toLowerCase().endsWith('.csv')
        && fs.existsSync(candidate)
        && fs.statSync(candidate).isFile()
    })
  if (!bundled) {
    throw new Error('The exact package did not expose a bundled sample CSV.')
  }
  const bundledPath = text(bundled.path)
  const selectedPath = isolatedAppDataPath(
    'qa-fixtures',
    'native-open-picker-bundled-sample.csv',
  )
  fs.mkdirSync(path.dirname(selectedPath), { recursive: true })
  fs.copyFileSync(bundledPath, selectedPath)
  const bundledHash = sha256File(bundledPath)
  expect(sha256File(selectedPath)).toBe(bundledHash)

  await openWorkspace('workflow')
  await (await $(nativeWorkflowSelectors.dataStep)).click()
  const picker = startOpenDialogHarness({
    evidenceName: 'local-paper-open-csv-dialog.json',
    targetPath: selectedPath,
  })
  await picker.ready()
  const browse = await $('[data-qa-action="data.csv.browse"]')
  await browse.waitForEnabled({ timeout: 10_000 })
  await browse.click()
  const pickerResult = await picker.complete()
  expect(text(pickerResult.mode)).toBe('OpenDialog')
  expect(text(pickerResult.target_path)).toBe(selectedPath)
  await (await $(nativeWorkflowSelectors.clearDatasetButton))
    .waitForEnabled({ timeout: 20_000 })
  await browser.waitUntil(
    async () => text(await (await $(nativeWorkflowSelectors.csvPath)).getValue()) === selectedPath,
    {
      timeout: 10_000,
      interval: 100,
      timeoutMsg: 'The native Open dialog did not load its contained bundled fixture.',
    },
  )
  await (await $(nativeWorkflowSelectors.optimizeStep)).click()
  await setFormValue('#vueFastType', 'sma')
  await setFormValue('#vueSlowType', 'sma')
  return {
    bundledPath,
    selectedPath,
    sha256: bundledHash,
  }
}

async function configureLocalPaperManualExecution(): Promise<void> {
  await openWorkspace('live')
  await (await $(shellSelectors.liveGrid)).waitForDisplayed({ timeout: 15_000 })
  await setFormValue('#vueExecTarget', 'local_paper')
  await setFormValue('#vueExecSymbol', SYMBOL)
  await setFormValue('#vueExecQty', '1')
  await setFormValue('#vueExecSource', 'manual')
  await setFormValue('#vueExecManualFast', '2')
  await setFormValue('#vueExecManualSlow', '20')
  await setFormValue('#vueExecRiskSl', '')
  await setFormValue('#vueExecRiskTp', '')
  await expect(await $('#vueExecApiKey')).toHaveValue('')
  await expect(await $('#vueExecApiSecret')).toHaveValue('')
}

async function localPaperSnapshot(): Promise<LocalPaperSnapshot> {
  return invokeNative<LocalPaperSnapshot>(
    'get_local_paper_execution_snapshot',
    {
      req: {
        symbol: SYMBOL,
        limit: 200,
      },
    },
  )
}

function filledOrderIds(snapshot: LocalPaperSnapshot): string[] {
  return (Array.isArray(snapshot.orders) ? snapshot.orders : [])
    .filter((order) => (
      text(order.symbol) === SYMBOL
      && text(order.status).toLowerCase() === 'filled'
      && text(order.id)
    ))
    .map((order) => text(order.id))
    .sort()
}

describe('desktop exact-package local paper persistence proof', () => {
  it('fills locally without credentials, cold-restarts, and rehydrates the durable broker review', async () => {
    const environment = requireReleaseProofEnvironment()
    const statePath = isolatedAppDataPath(
      'execution',
      'paper_broker',
      'state.json',
    )
    expect(isPathInside(statePath, environment.appDataDir)).toBe(true)

    try {
      await waitForShellReady()
      const pickerFixture = await loadBundledSampleThroughNativePicker()
      await configureLocalPaperManualExecution()

      const reset = await $('[data-qa-action="live.paper.reset"]')
      await reset.waitForEnabled({ timeout: 10_000 })
      await reset.click()
      await confirmExecutionAction(
        'Reset local paper broker?',
        'Reset local paper',
      )
      await waitForAppStatus(
        (value) => value === 'Local paper broker reset.',
        'The local paper reset did not complete.',
      )

      const runOnce = await $('[data-qa-action="live.runner.runOnce"]')
      await runOnce.waitForEnabled({ timeout: 10_000 })
      await expect(runOnce).toHaveText('Fill local paper once')
      await runOnce.click()
      await confirmExecutionAction(
        'Confirm fill local paper once?',
        'Fill local paper once',
      )
      const executionStatus = await waitForAppStatus(
        (value) => value.startsWith('Submitted '),
        'The deterministic local-paper signal did not submit a simulated fill.',
      )

      const refresh = await $('[data-qa-action="live.account.refreshOrders"]')
      await refresh.waitForEnabled({ timeout: 10_000 })
      await refresh.click()
      await waitForAppStatus(
        (value) => value.startsWith('Loaded ') && value.includes(SYMBOL),
        'The local paper broker review did not load after the fill.',
      )

      const beforeCold = await localPaperSnapshot()
      expect(text(beforeCold.mode).toLowerCase()).toBe('paper')
      expect(text(beforeCold.symbol)).toBe(SYMBOL)
      expect(text(beforeCold.account_status)).toBe('simulated')
      expect(text(beforeCold.position?.symbol)).toBe(SYMBOL)
      expect(Math.abs(finiteNumber(beforeCold.position?.qty))).toBeCloseTo(1, 8)
      const orderIdsBeforeCold = filledOrderIds(beforeCold)
      expect(orderIdsBeforeCold.length).toBeGreaterThanOrEqual(1)
      expect(fs.existsSync(statePath)).toBe(true)
      const stateHashBeforeCold = sha256File(statePath)
      const stateSizeBeforeCold = fs.statSync(statePath).size

      await browser.reloadSession()
      await waitForShellReady()

      const afterCold = await localPaperSnapshot()
      expect(text(afterCold.account_status)).toBe('simulated')
      expect(text(afterCold.position?.symbol)).toBe(SYMBOL)
      expect(finiteNumber(afterCold.position?.qty))
        .toBeCloseTo(finiteNumber(beforeCold.position?.qty), 8)
      expect(filledOrderIds(afterCold)).toEqual(orderIdsBeforeCold)
      expect(fs.existsSync(statePath)).toBe(true)
      const stateHashAfterCold = sha256File(statePath)

      await openWorkspace('live')
      await (await $(shellSelectors.liveGrid)).waitForDisplayed({ timeout: 15_000 })
      await setFormValue('#vueExecTarget', 'local_paper')
      await setFormValue('#vueExecSymbol', SYMBOL)
      await expect(await $('#vueExecApiKey')).toHaveValue('')
      await expect(await $('#vueExecApiSecret')).toHaveValue('')
      const coldRefresh = await $('[data-qa-action="live.account.refreshOrders"]')
      await coldRefresh.waitForEnabled({ timeout: 10_000 })
      await coldRefresh.click()
      const coldUiStatus = await waitForAppStatus(
        (value) => value.startsWith('Loaded ') && value.includes(SYMBOL),
        'The cold local-paper UI did not rehydrate the durable broker review.',
      )
      const accountPanel = await $('[aria-label="Local paper account"]')
      await accountPanel.waitForDisplayed({ timeout: 10_000 })
      await expect(accountPanel).toHaveText(expect.stringContaining('simulated'))
      const orderTable = await $('[aria-label="Broker order activity table"]')
      await orderTable.waitForDisplayed({ timeout: 10_000 })
      await expect(orderTable).toHaveText(expect.stringContaining(SYMBOL))
      await expect(orderTable).toHaveText(expect.stringContaining('filled'))

      writeReleaseProofEvidence('local-paper-cold-restart-proof.json', {
        contract: 'vectorgrid-local-paper-cold-restart-v1',
        credential_free: true,
        provider_network_used: false,
        native_open_picker: {
          exercised: true,
          bundled_source_path: pickerFixture.bundledPath,
          contained_selected_path: pickerFixture.selectedPath,
          content_sha256: pickerFixture.sha256,
        },
        symbol: SYMBOL,
        manual_parameters: {
          fast_len: 2,
          slow_len: 20,
          fast_ma_type: 'sma',
          slow_ma_type: 'sma',
          quantity: 1,
        },
        execution_status: executionStatus,
        cold_ui_status: coldUiStatus,
        position_before_cold: beforeCold.position,
        position_after_cold: afterCold.position,
        filled_order_ids_before_cold: orderIdsBeforeCold,
        filled_order_ids_after_cold: filledOrderIds(afterCold),
        broker_state: {
          path: statePath,
          size_bytes: stateSizeBeforeCold,
          sha256_before_cold: stateHashBeforeCold,
          sha256_after_cold: stateHashAfterCold,
        },
      })
    } finally {
      await invokeNative('reset_local_paper_broker').catch(() => undefined)
    }
  })
})
