import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import {
  OFFICIAL_CATBOOST_NATIVE_SHA256,
  RELEASE_PROOF_ML_BUNDLE_ID,
  RELEASE_PROOF_TINY_MODEL_SHA256,
  seedMlSignalBundle,
  sha256File,
} from '../support/nativeFeatureFixtures.js'
import {
  isPathInside,
  isolatedAppDataPath,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { invokeNative } from '../support/nativeIpc.js'
import { startSaveDialogHarness } from '../support/nativeWindowHarness.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type BundleSummary = {
  bundle_id?: unknown
  manifest_path?: unknown
  state?: unknown
  issues?: unknown[]
}

type BundleStatusResponse = {
  status?: {
    bundle_id?: unknown
    manifest_path?: unknown
    state?: unknown
    issues?: unknown[]
  }
}

type RunArtifactSummary = {
  path?: unknown
  strategy_kind?: unknown
}

type ScoreCacheBuildResult = {
  output_path?: unknown
  manifest_path?: unknown
  matched_rows?: unknown
  bars_scored?: unknown
  score_scope?: unknown
  model_bundle_id?: unknown
  source_bundle_manifest_path?: unknown
  model_long_sha256?: unknown
  model_short_sha256?: unknown
  native_backend_requested?: unknown
  native_backend_used?: unknown
  native_library_path?: unknown
  native_library_sha256?: unknown
  native_library_discovery_policy?: unknown
}

type RunArtifact = {
  kind?: unknown
  run?: {
    req?: Record<string, unknown>
    res?: ScoreCacheBuildResult
  }
  ml_signal?: Record<string, unknown>
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizedPath(value: string): string {
  const full = path.resolve(value)
  return process.platform === 'win32' ? full.toLowerCase() : full
}

function issueText(value: { issues?: unknown[] } | undefined): string {
  return Array.isArray(value?.issues)
    ? value.issues.map((entry) => text(entry)).join(' • ')
    : ''
}

function readArtifact(artifactPath: string): RunArtifact {
  return JSON.parse(fs.readFileSync(artifactPath, 'utf8')) as RunArtifact
}

async function selectValue(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await field.selectByAttribute('value', value)
  await browser.waitUntil(
    async () => text(await field.getValue()) === value,
    {
      timeout: 3_000,
      interval: 50,
      timeoutMsg: `${selector} did not retain ${JSON.stringify(value)}.`,
    },
  )
}

async function setInput(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await browser.execute((targetSelector: string, nextValue: string) => {
    const page = globalThis as unknown as {
      document: {
        querySelector: (inputSelector: string) => {
          value: string
          dispatchEvent: (event: unknown) => boolean
        } | null
      }
      Event: new (type: string, init: { bubbles: boolean }) => unknown
    }
    const element = page.document.querySelector(targetSelector)
    if (!element) throw new Error(`Expected an input for ${targetSelector}.`)
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

async function setCheckbox(selector: string, checked: boolean): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  if ((await field.isSelected()) !== checked) await field.click()
}

async function loadSampleAndOpenMlOptimize(): Promise<void> {
  await waitForShellReady()
  await openWorkspace('workflow')
  await (await $(nativeWorkflowSelectors.dataStep)).click()
  const sample = await $(nativeWorkflowSelectors.sampleDatasetButton)
  await sample.waitForClickable({ timeout: 15_000 })
  await sample.click()
  await (await $(nativeWorkflowSelectors.clearDatasetButton))
    .waitForEnabled({ timeout: 20_000 })

  const mlMode = await $('[data-qa-action="shell.mode.mlSignal"]')
  await mlMode.waitForClickable({ timeout: 10_000 })
  if ((await mlMode.getAttribute('aria-selected')) !== 'true') {
    await mlMode.click()
  }
  await (await $(nativeWorkflowSelectors.optimizeStep)).click()
  await (await $('[data-qa-action="mlSignal.bundles.refresh"]'))
    .waitForDisplayed({ timeout: 10_000 })
}

async function refreshBundleInventory(): Promise<void> {
  const refresh = await $('[data-qa-action="mlSignal.bundles.refresh"]')
  await refresh.waitForEnabled({ timeout: 10_000 })
  await refresh.click()
  const option = await $(
    `#mlSignalBundleId option[value="${RELEASE_PROOF_ML_BUNDLE_ID}"]`,
  )
  await option.waitForExist({ timeout: 10_000 })
}

async function refreshBundleStatus(): Promise<void> {
  const refresh = await $('[data-qa-action="mlSignal.bundleStatus.refresh"]')
  await refresh.waitForEnabled({ timeout: 10_000 })
  await refresh.click()
}

async function chooseScoreCachePath(
  targetPath: string,
  evidenceName: string,
): Promise<void> {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true })
  const dialog = startSaveDialogHarness({
    action: 'save',
    evidenceName,
    targetPath,
  })
  await dialog.ready()
  const browse = await $('#mlSignalScoreCachePath + button')
  await browse.waitForEnabled({ timeout: 10_000 })
  await browse.click()
  await dialog.complete()
  await browser.waitUntil(
    async () => normalizedPath(text(
      await (await $('#mlSignalScoreCachePath')).getValue(),
    )) === normalizedPath(targetPath),
    {
      timeout: 10_000,
      interval: 100,
      timeoutMsg: 'The native JSON picker did not retain the isolated ML cache path.',
    },
  )
}

async function mlStatusText(): Promise<string> {
  const status = await $('.mlSignalRoot [role="status"]')
  const hiddenStatus = await status.isExisting()
    ? text(await status.getProperty('textContent'))
    : ''
  const visibleBody = text(await (await $('body')).getText())
  return `${hiddenStatus}\n${visibleBody}`.trim()
}

async function waitForBuildFailure(): Promise<string> {
  const build = await $('[data-qa-action="mlSignal.scoreCache.build"]')
  let observed = ''
  await browser.waitUntil(
    async () => {
      observed = (await mlStatusText()).toLowerCase()
      return await build.isEnabled()
        && observed.includes('ml score cache generation failed')
    },
    {
      timeout: 60_000,
      interval: 100,
      timeoutMsg: 'The corrupt ML native library did not return to an actionable failure state.',
    },
  )
  return observed
}

async function waitForNewMlArtifact(
  pathsBefore: ReadonlySet<string>,
): Promise<string> {
  let artifactPath = ''
  await browser.waitUntil(
    async () => {
      const rows = await invokeNative<RunArtifactSummary[]>('list_run_artifacts')
      artifactPath = rows
        .filter((row) => text(row.strategy_kind) === 'MlSignal')
        .map((row) => text(row.path))
        .find((candidate) => candidate && !pathsBefore.has(candidate)) || ''
      return artifactPath.length > 0 && fs.existsSync(artifactPath)
    },
    {
      timeout: 60_000,
      interval: 250,
      timeoutMsg: 'Native ML scoring did not persist its run artifact.',
    },
  )
  return artifactPath
}

async function coldLoadMlArtifact(
  artifactPath: string,
  scoreCachePath: string,
): Promise<void> {
  await openWorkspace('library')
  const refresh = await $(nativeWorkflowSelectors.savedRunsRefresh)
  await refresh.waitForEnabled({ timeout: 10_000 })
  await refresh.click()
  await setInput(
    nativeWorkflowSelectors.savedRunsSearch,
    path.basename(artifactPath),
  )
  const savedRun = await $(nativeWorkflowSelectors.firstSavedRun)
  await savedRun.waitForDisplayed({ timeout: 15_000 })
  await savedRun.click()
  const load = await $(nativeWorkflowSelectors.savedRunLoad)
  await load.waitForEnabled({ timeout: 10_000 })
  await load.click()
  await browser.waitUntil(
    async () => (await (await $('body')).getText())
      .toLowerCase()
      .includes('ml signal artifact loaded'),
    {
      timeout: 15_000,
      interval: 100,
      timeoutMsg: 'The cold Library surface did not hydrate the ML run artifact.',
    },
  )

  await openWorkspace('workflow')
  const mlMode = await $('[data-qa-action="shell.mode.mlSignal"]')
  await mlMode.waitForClickable({ timeout: 10_000 })
  if ((await mlMode.getAttribute('aria-selected')) !== 'true') {
    await mlMode.click()
  }
  await (await $(nativeWorkflowSelectors.optimizeStep)).click()
  await browser.waitUntil(
    async () => normalizedPath(text(
      await (await $('#mlSignalScoreCachePath')).getValue(),
    )) === normalizedPath(scoreCachePath),
    {
      timeout: 10_000,
      interval: 100,
      timeoutMsg: 'The cold ML artifact did not restore its score-cache path.',
    },
  )
  expect(text(await (await $('#mlSignalBundleId')).getValue()))
    .toBe(RELEASE_PROOF_ML_BUNDLE_ID)
}

describe('desktop exact-package ML native resource recovery proof', () => {
  it('detects a broken bundle, fails closed on a corrupt local DLL, recovers to the packaged DLL, and cold-reloads', async () => {
    const environment = requireReleaseProofEnvironment()
    seedMlSignalBundle({
      includeModel: false,
      corruptLibraryOverride: false,
    })

    const missingCatalog = await invokeNative<BundleSummary[]>(
      'list_ml_signal_model_bundles',
    )
    const missingSummary = missingCatalog.find(
      (row) => text(row.bundle_id) === RELEASE_PROOF_ML_BUNDLE_ID,
    )
    if (!missingSummary) {
      throw new Error('The seeded release-proof ML bundle was absent from the native catalog.')
    }
    expect(issueText(missingSummary).toLowerCase()).toContain('model file missing')
    const missingStatus = await invokeNative<BundleStatusResponse>(
      'ml_signal_model_status',
      { bundleId: RELEASE_PROOF_ML_BUNDLE_ID },
    )
    expect(issueText(missingStatus.status).toLowerCase()).toContain('model file missing')

    await loadSampleAndOpenMlOptimize()
    await refreshBundleInventory()
    await selectValue('#mlSignalScoreSource', 'native')
    await selectValue('#mlSignalBundleId', RELEASE_PROOF_ML_BUNDLE_ID)
    await refreshBundleStatus()

    const corruptBundle = seedMlSignalBundle({
      includeModel: true,
      corruptLibraryOverride: true,
    })
    expect(sha256File(corruptBundle.modelPath))
      .toBe(RELEASE_PROOF_TINY_MODEL_SHA256)
    await refreshBundleInventory()
    await selectValue('#mlSignalBundleId', RELEASE_PROOF_ML_BUNDLE_ID)
    await selectValue('#mlSignalNativeBackend', 'native_cpu')
    await selectValue('#mlSignalNativeScoreScope', 'full_dataset')
    await setCheckbox('#mlSignalAllowNonWalkforwardScoreScope', true)

    const corruptOutputPath = isolatedAppDataPath(
      'ml-signal',
      'corrupt-library-score-cache.json',
    )
    await chooseScoreCachePath(
      corruptOutputPath,
      'ml-corrupt-library-save-dialog.json',
    )
    const build = await $('[data-qa-action="mlSignal.scoreCache.build"]')
    await build.waitForEnabled({ timeout: 10_000 })
    await build.click()
    const failureStatus = await waitForBuildFailure()
    expect(failureStatus).toMatch(/catboost|library|dll|load/)
    expect(fs.existsSync(corruptOutputPath)).toBe(false)

    const repairedBundle = seedMlSignalBundle({
      includeModel: true,
      corruptLibraryOverride: false,
    })
    expect(fs.existsSync(repairedBundle.corruptLibraryPath)).toBe(false)
    const repairedStatus = await invokeNative<BundleStatusResponse>(
      'ml_signal_model_status',
      { bundleId: RELEASE_PROOF_ML_BUNDLE_ID },
    )
    expect(issueText(repairedStatus.status).toLowerCase())
      .not.toContain('model file missing')
    await refreshBundleInventory()
    await selectValue('#mlSignalBundleId', RELEASE_PROOF_ML_BUNDLE_ID)
    await refreshBundleStatus()

    const pathsBefore = new Set(
      (await invokeNative<RunArtifactSummary[]>('list_run_artifacts'))
        .map((row) => text(row.path))
        .filter(Boolean),
    )
    const scoreCachePath = isolatedAppDataPath(
      'ml-signal',
      'packaged-native-score-cache.json',
    )
    await chooseScoreCachePath(
      scoreCachePath,
      'ml-packaged-native-save-dialog.json',
    )
    await build.waitForEnabled({ timeout: 10_000 })
    await build.click()
    await browser.waitUntil(
      async () => {
        const status = (await mlStatusText()).toLowerCase()
        return fs.existsSync(scoreCachePath)
          && await build.isEnabled()
          && status.includes('ml native score cache generated')
      },
      {
        timeout: 60_000,
        interval: 100,
        timeoutMsg: 'The packaged CatBoost resource did not generate a score cache.',
      },
    )

    const artifactPath = await waitForNewMlArtifact(pathsBefore)
    const artifact = readArtifact(artifactPath)
    expect(text(artifact.kind)).toBe('vectorgrid_run_artifact')
    const result = artifact.run?.res
    if (!result) throw new Error('The ML run artifact was missing run.res.')
    const scoreManifestPath = text(result.manifest_path)
    const nativeLibraryPath = text(result.native_library_path)
    expect(text(result.output_path)).toBe(scoreCachePath)
    expect(text(result.model_bundle_id)).toBe(RELEASE_PROOF_ML_BUNDLE_ID)
    expect(normalizedPath(text(result.source_bundle_manifest_path)))
      .toBe(normalizedPath(repairedBundle.manifestPath))
    expect(Number(result.matched_rows)).toBeGreaterThan(0)
    expect(Number(result.bars_scored)).toBeGreaterThan(0)
    expect(text(result.score_scope)).toBe('full_dataset')
    expect(text(result.native_backend_requested)).toBe('native_cpu')
    expect(text(result.native_backend_used)).toBe('native_cpu')
    expect(text(result.native_library_discovery_policy)).toBe('production')
    expect(text(result.model_long_sha256))
      .toBe(RELEASE_PROOF_TINY_MODEL_SHA256)
    expect(text(result.model_short_sha256))
      .toBe(RELEASE_PROOF_TINY_MODEL_SHA256)
    expect(text(result.native_library_sha256))
      .toBe(OFFICIAL_CATBOOST_NATIVE_SHA256)
    expect(fs.existsSync(scoreCachePath)).toBe(true)
    expect(fs.existsSync(scoreManifestPath)).toBe(true)
    expect(fs.existsSync(nativeLibraryPath)).toBe(true)
    expect(isPathInside(scoreCachePath, environment.isolationRoot)).toBe(true)
    expect(isPathInside(scoreManifestPath, environment.isolationRoot)).toBe(true)
    expect(isPathInside(artifactPath, environment.isolationRoot)).toBe(true)
    expect(isPathInside(nativeLibraryPath, repairedBundle.bundleDir)).toBe(false)
    expect(sha256File(nativeLibraryPath)).toBe(OFFICIAL_CATBOOST_NATIVE_SHA256)

    await browser.reloadSession()
    await waitForShellReady()
    const coldCatalog = await invokeNative<BundleSummary[]>(
      'list_ml_signal_model_bundles',
    )
    const coldSummary = coldCatalog.find(
      (row) => text(row.bundle_id) === RELEASE_PROOF_ML_BUNDLE_ID,
    )
    if (!coldSummary) {
      throw new Error('The ML model catalog did not survive a cold process relaunch.')
    }
    expect(issueText(coldSummary).toLowerCase()).not.toContain('model file missing')
    const coldStatus = await invokeNative<BundleStatusResponse>(
      'ml_signal_model_status',
      { bundleId: RELEASE_PROOF_ML_BUNDLE_ID },
    )
    expect(issueText(coldStatus.status).toLowerCase())
      .not.toContain('model file missing')

    await loadSampleAndOpenMlOptimize()
    await refreshBundleInventory()
    await selectValue('#mlSignalScoreSource', 'native')
    await selectValue('#mlSignalBundleId', RELEASE_PROOF_ML_BUNDLE_ID)
    await refreshBundleStatus()
    await coldLoadMlArtifact(artifactPath, scoreCachePath)

    writeReleaseProofEvidence('ml-native-catalog-proof.json', {
      contract: 'vectorgrid-exact-package-ml-native-v1',
      bundle_id: RELEASE_PROOF_ML_BUNDLE_ID,
      missing_bundle: {
        state: text(missingStatus.status?.state),
        issues: missingStatus.status?.issues ?? [],
      },
      repaired_bundle: {
        state: text(repairedStatus.status?.state),
        manifest_path: text(repairedStatus.status?.manifest_path),
        manifest_sha256: sha256File(repairedBundle.manifestPath),
        model_sha256: sha256File(repairedBundle.modelPath),
        feature_schema_sha256: sha256File(repairedBundle.featureSchemaPath),
      },
      native_recovery: {
        corrupt_library_path: corruptBundle.corruptLibraryPath,
        corrupt_failure_status: failureStatus,
        backend_requested: text(result.native_backend_requested),
        backend_used: text(result.native_backend_used),
        library_path: nativeLibraryPath,
        library_sha256: sha256File(nativeLibraryPath),
        discovery_policy: text(result.native_library_discovery_policy),
      },
      outputs: {
        score_cache_path: scoreCachePath,
        score_cache_sha256: sha256File(scoreCachePath),
        score_manifest_path: scoreManifestPath,
        score_manifest_sha256: sha256File(scoreManifestPath),
        run_artifact_path: artifactPath,
        run_artifact_sha256: sha256File(artifactPath),
      },
      cold_reload: {
        catalog_state: text(coldSummary.state),
        status_state: text(coldStatus.status?.state),
        issues: coldStatus.status?.issues ?? [],
        library_sha256_reverified: sha256File(nativeLibraryPath),
      },
    })
  })
})
