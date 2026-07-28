import { expect, test } from '@playwright/test'
import { workspaceTabSelector } from '../desktop/support/selectors'
import { waitForShellReady } from './support/appHarness'
import { dismissStartupOverlay, installMockedTauri } from './support/mockTauri'

test.describe('browser execution review surface', () => {
  test('shows truthful broker review state when account data is refreshed without a loaded dataset', async ({ context, page }) => {
    await installMockedTauri(context, {
      holdStartupOverlay: true,
      commandResults: {
        alpaca_get_credential_status: {
          remember_available: true,
          has_api_key: false,
          has_api_secret: false,
          has_credentials: false,
          state: 'missing',
          storage_backend: 'keyring',
          message: '',
        },
        get_live_execution_snapshot: {
          symbol: 'AAPL',
          account_number: 'ABC123',
          account_status: 'ACTIVE',
          currency: 'USD',
          buying_power: '100000',
          cash: '25000',
          portfolio_value: '125000',
          fetched_at_unix_ms: 1_900_000_000_000,
          position: null,
          orders: [
            {
              id: 'ord_123',
              side: 'buy',
              status: 'filled',
              qty: '1',
              filled_qty: '1',
              filled_avg_price: '182.4',
              submitted_at: '2026-04-12T06:00:00Z',
            },
          ],
        },
      },
    })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)
    await waitForShellReady(page)

    await page.locator(workspaceTabSelector('live')).click()
    await page.locator('#vueExecSymbol').fill('AAPL')
    await page.locator('#vueExecApiKey').fill('key_1')
    await page.locator('#vueExecApiSecret').fill('secret_1')
    await page.getByRole('button', { name: 'Refresh orders' }).click()

    await expect(page.getByText('Account review: paper · account=ABC123 · status=ACTIVE').first()).toBeVisible()
    await expect(page.getByText('Broker-confirmed flat.').first()).toBeVisible()
    await expect(page.getByText('Execution chart unavailable: load a workflow dataset first.').first()).toBeVisible()
    await expect(page.getByText('symbol-scoped AAPL').first()).toBeVisible()
    await expect(page.getByText('ord_123').first()).toBeVisible()
  })

  test('refreshes saved live session evidence as a read-only review surface', async ({ context, page }) => {
    await installMockedTauri(context, {
      holdStartupOverlay: true,
      commandResults: {
        alpaca_get_credential_status: {
          remember_available: true,
          has_api_key: false,
          has_api_secret: false,
          has_credentials: false,
          state: 'missing',
          storage_backend: 'keyring',
          message: '',
        },
        list_live_session_artifacts: [
          {
            ts: '2026-04-12T15:40:00Z',
            path: 'C:/runs/vectorgrid_live_session_latest.json',
            bytes: 512,
            session_id: 'sess_latest',
            mode: 'paper',
            symbol: 'AAPL',
            strategy_kind: 'DoubleMa',
            data_id: 'data_aapl',
            dry_run: false,
            param_source: 'selected',
            session_state: 'completed',
            created_at_unix_ms: 1_900_000_000_000,
            finalized_at_unix_ms: 1_900_000_000_500,
            event_count: 2,
            broker_snapshot_count: 1,
          },
        ],
        load_live_session_artifact_detail: {
          ts: '2026-04-12T15:40:00Z',
          artifact_path: 'C:/runs/vectorgrid_live_session_latest.json',
          session_id: 'sess_latest',
          session_state: 'completed',
          mode: 'paper',
          symbol: 'AAPL',
          qty: 2,
          dry_run: false,
          strategy_kind: 'DoubleMa',
          data_id: 'data_aapl',
          param_source: 'selected',
          created_at_unix_ms: 1_900_000_000_000,
          finalized_at_unix_ms: 1_900_000_000_500,
          source_run_artifact_path: 'C:/runs/vectorgrid_run_2026-04-12.json',
          source_run_config_fingerprint: 'cfg1-fnv1a64-live',
          source_run_route_provenance: {
            requested_route: 'GpuOnly',
            resolved_route: 'GpuKernel',
            realized_route: 'CpuOnly',
            route_changed: true,
            fallback_reason: 'cuda unavailable',
          },
          event_count: 2,
          broker_snapshot_count: 1,
          events: [
            {
              provenance: 'local_control',
              status: 'start_runner',
              message: 'runner armed',
              symbol: 'AAPL',
              at_unix_ms: 1_900_000_000_000,
              action: '',
              order_id: '',
              delta_qty: null,
              order_submitted: null,
              warnings: [],
            },
            {
              provenance: 'submission_attempt',
              status: 'order_submitted',
              message: 'submitted buy order',
              symbol: 'AAPL',
              at_unix_ms: 1_900_000_000_250,
              action: 'buy',
              order_id: 'ord_123',
              delta_qty: 2,
              order_submitted: true,
              warnings: ['paper only'],
            },
          ],
          broker_snapshots: [
            {
              source_command: 'get_live_execution_snapshot',
              mode: 'paper',
              symbol: 'AAPL',
              at_unix_ms: 1_900_000_000_300,
              account_status: 'ACTIVE',
              currency: 'USD',
              position_state: 'flat',
              position_qty: 0,
              position_side: '',
              open_order_count: 0,
              order_count: 1,
            },
          ],
        },
      },
    })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)
    await waitForShellReady(page)

    await page.locator(workspaceTabSelector('live')).click()
    await page.getByRole('button', { name: 'Refresh saved sessions' }).click()

    await expect(page.getByText('Loaded 1 saved live session artifact.').first()).toBeVisible()
    await expect(page.getByText('sess_latest').first()).toBeVisible()
    await expect(page.getByText('submitted buy order').first()).toBeVisible()
    await expect(page.getByText('get_live_execution_snapshot').first()).toBeVisible()
    const sourceRunPath = 'C:/runs/vectorgrid_run_2026-04-12.json'
    const sourceRunEvidence = page.getByLabel(`Source run: ${sourceRunPath}`)
    await expect(sourceRunEvidence).toHaveText('vectorgrid_run_2026-04-12.json')
    await expect(sourceRunEvidence).toHaveAttribute('title', sourceRunPath)
  })

  test('shows the live safety matrix and broker-facing confirmation copy', async ({ context, page }) => {
    await installMockedTauri(context, {
      holdStartupOverlay: true,
      commandResults: {
        alpaca_get_credential_status: {
          remember_available: true,
          has_api_key: false,
          has_api_secret: false,
          has_credentials: false,
          state: 'missing',
          storage_backend: 'keyring',
          message: '',
        },
      },
    })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)
    await waitForShellReady(page)

    await page.locator(workspaceTabSelector('live')).click()
    await page.locator('#vueExecSymbol').fill('AAPL')
    await page.locator('#vueExecApiKey').fill('key_1')
    await page.locator('#vueExecApiSecret').fill('secret_1')

    await expect(page.getByText('Live execution safety control matrix')).toBeVisible()
    await expect(page.getByText('Stop runner').first()).toBeVisible()
    await expect(page.getByText('Cancel open orders').first()).toBeVisible()
    await expect(page.getByText('Flatten position').first()).toBeVisible()
    await expect(page.getByText('Emergency halt').first()).toBeVisible()

    await page.getByRole('button', { name: 'Cancel open orders' }).click()
    await expect(page.getByText('Cancel current-symbol open orders?')).toBeVisible()
    await expect(page.getByText('does not flatten any existing broker position')).toBeVisible()
  })
})

