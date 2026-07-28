import { $, $$, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { invokeNative } from '../support/nativeIpc.js'
import {
  isReleaseProofRun,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type ArtifactLibrarySnapshot = {
  runs?: unknown[]
  meta?: unknown[]
  stage5?: unknown[]
  policyEvals?: unknown[]
  optionsExpression?: unknown[]
}

type PolygonCacheInfo = {
  has_api_key?: unknown
  datasets?: unknown[]
  credential_status?: {
    has_api_key?: unknown
  }
}

type AlpacaCredentialStatus = {
  has_api_key?: unknown
  has_api_secret?: unknown
  has_credentials?: unknown
}

const STATEFUL_STORAGE_KEYS = [
  'vta_data_startup_snapshot_v1',
  'vectorgrid_history_v1',
  'vectorbt_history_v1',
  'vectorgrid_recent_csv_v1',
  'vectorbt_recent_csv_v1',
  'vectorgrid_last_batch_summary_path_v1',
  'vectorbt_last_batch_summary_path_v1',
  'vectorgrid_exec_keys_v1',
  'vectorbt_exec_keys_v1',
] as const

function normalized(value: string): string {
  let resolved = path.resolve(value)
  if (process.platform !== 'win32') return resolved
  resolved = resolved.replace(/\//g, '\\')
  if (resolved.startsWith('\\\\?\\UNC\\')) resolved = `\\\\${resolved.slice(8)}`
  else if (resolved.startsWith('\\\\?\\')) resolved = resolved.slice(4)
  return resolved.toLowerCase()
}

function assertHarnessIsIsolated(): void {
  const root = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT || '',
  ).trim()
  const localAppData = String(process.env.LOCALAPPDATA || '').trim()
  const roamingAppData = String(process.env.APPDATA || '').trim()
  const temp = String(process.env.TEMP || '').trim()
  const runId = String(process.env.VECTORGRID_DESKTOP_SMOKE_RUN_ID || '').trim()
  const appDataDir = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR || '',
  ).trim()
  if (
    !root ||
    !localAppData ||
    !roamingAppData ||
    !temp ||
    !runId ||
    !appDataDir ||
    process.env.VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE !== '1'
  ) {
    throw new Error(
      'Clean-start proof requires the isolated profile environment.',
    )
  }
  const rootValue = normalized(root)
  for (const [name, candidate] of [
    ['LOCALAPPDATA', localAppData],
    ['APPDATA', roamingAppData],
    ['TEMP', temp],
  ] as const) {
    const value = normalized(candidate)
    if (value !== rootValue && !value.startsWith(`${rootValue}${path.sep}`)) {
      throw new Error(`${name} escaped the isolated profile root: ${candidate}`)
    }
  }
  const expectedAppDataDir = path.resolve(
    root,
    'LocalAppData',
    'dev.vectoralpha.vectorgrid',
  )
  if (normalized(appDataDir) !== normalized(expectedAppDataDir)) {
    throw new Error(
      `QA app-data override does not match the owned profile: ${appDataDir}`,
    )
  }
  const attestationPath = path.resolve(
    appDataDir,
    '.vectorgrid-qa-profile-attestation.json',
  )
  if (
    !fs.existsSync(attestationPath) ||
    !fs.lstatSync(attestationPath).isFile()
  ) {
    throw new Error(
      `The exact package did not attest its isolated app-data paths: ${attestationPath}`,
    )
  }
  const attestation = JSON.parse(fs.readFileSync(attestationPath, 'utf8')) as {
    schemaVersion?: unknown
    kind?: unknown
    runId?: unknown
    originalIdentifier?: unknown
    effectiveIdentifier?: unknown
    isolationRoot?: unknown
    appDataDir?: unknown
    resolvedPaths?: Record<string, unknown>
  }
  if (
    attestation.schemaVersion !== 2 ||
    attestation.kind !== 'vectorgrid-qa-profile-attestation' ||
    attestation.runId !== runId ||
    attestation.originalIdentifier !== 'dev.vectoralpha.vectorgrid' ||
    normalized(String(attestation.effectiveIdentifier || '')) !==
      normalized(appDataDir) ||
    normalized(String(attestation.isolationRoot || '')) !== rootValue ||
    normalized(String(attestation.appDataDir || '')) !== normalized(appDataDir)
  ) {
    throw new Error(
      'The exact package QA profile attestation does not match this clean-start run.',
    )
  }
  for (const candidate of Object.values(attestation.resolvedPaths || {})) {
    const value = String(candidate || '')
    if (
      !path.isAbsolute(value) ||
      !normalized(value).startsWith(`${rootValue}${path.sep}`)
    ) {
      throw new Error(
        `An attested exact-package path escaped the isolation root: ${value}`,
      )
    }
  }
}

async function assertEmptyFirstRunState(
  pass: 'first' | 'second',
): Promise<void> {
  await waitForShellReady()
  await openWorkspace('workflow')
  await (await $(nativeWorkflowSelectors.dataStep)).click()

  const csvPath = await $(nativeWorkflowSelectors.csvPath)
  await csvPath.waitForDisplayed({ timeout: 10_000 })
  expect(String(await csvPath.getValue()).trim()).toBe('')
  await expect(
    await $(nativeWorkflowSelectors.clearDatasetButton),
  ).toBeDisabled()

  const library = await invokeNative<ArtifactLibrarySnapshot>(
    'list_artifact_library',
  )
  for (const key of [
    'runs',
    'meta',
    'stage5',
    'policyEvals',
    'optionsExpression',
  ] as const) {
    expect(Array.isArray(library[key]) ? library[key] : []).toHaveLength(0)
  }
  await expect(
    await invokeNative<unknown[]>('list_live_session_artifacts'),
  ).toHaveLength(0)
  await expect(
    await invokeNative<unknown[]>('list_options_expression_artifacts'),
  ).toHaveLength(0)
  expect(await invokeNative('load_latest_run_artifact_summary')).toBeNull()

  const polygon = await invokeNative<PolygonCacheInfo>('polygon_get_cache_info')
  expect(Boolean(polygon.has_api_key)).toBe(false)
  expect(Boolean(polygon.credential_status?.has_api_key)).toBe(false)
  expect(Array.isArray(polygon.datasets) ? polygon.datasets : []).toHaveLength(
    0,
  )

  const alpaca = await invokeNative<AlpacaCredentialStatus>(
    'alpaca_get_credential_status',
  )
  expect(Boolean(alpaca.has_api_key)).toBe(false)
  expect(Boolean(alpaca.has_api_secret)).toBe(false)
  expect(Boolean(alpaca.has_credentials)).toBe(false)

  const statefulStorage = (await browser.execute((keys: readonly string[]) => {
    const out: Record<string, string> = {}
    for (const key of keys) {
      const value = globalThis.localStorage.getItem(key)
      if (value != null && value.trim() !== '') out[key] = value
    }
    return out
  }, STATEFUL_STORAGE_KEYS)) as unknown as Record<string, string>
  expect(Object.keys(statefulStorage)).toEqual([])

  await openWorkspace('library')
  const refresh = await $(nativeWorkflowSelectors.savedRunsRefresh)
  await refresh.waitForEnabled({ timeout: 10_000 })
  await refresh.click()
  await browser.waitUntil(
    async () => {
      const rows = await $$(
        `${nativeWorkflowSelectors.savedRunList} [data-saved-run="1"]`,
      )
      return (await rows.length) === 0
    },
    {
      timeout: 5_000,
      interval: 100,
      timeoutMsg: `${pass} isolated launch exposed a saved run.`,
    },
  )
}

describe('desktop isolated clean start', () => {
  it('starts clean twice without inherited catalog, credentials, dataset, or durable UI state', async () => {
    assertHarnessIsIsolated()
    await assertEmptyFirstRunState('first')
    await browser.reloadSession()
    assertHarnessIsIsolated()
    await assertEmptyFirstRunState('second')
    if (isReleaseProofRun()) {
      const environment = requireReleaseProofEnvironment()
      writeReleaseProofEvidence('clean-start-proof.json', {
        contract: 'vectorgrid-exact-package-clean-start-v1',
        launch_count: 2,
        isolation_root: environment.isolationRoot,
        app_data_dir: environment.appDataDir,
        verified_empty_surfaces: [
          'dataset',
          'artifact_library',
          'live_sessions',
          'options_expressions',
          'polygon_credentials_and_cache',
          'alpaca_credentials',
          'stateful_local_storage',
        ],
      })
    }
  })
})
