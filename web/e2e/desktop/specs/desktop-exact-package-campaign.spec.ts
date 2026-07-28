import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import {
  sha256File,
} from '../support/nativeFeatureFixtures.js'
import {
  isPathInside,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { invokeNative } from '../support/nativeIpc.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

const LAST_BATCH_SUMMARY_KEY = 'vectorgrid_last_batch_summary_path_v1'
const FIXTURE_PREFIX = 'release-proof-campaign'

type SampleCsv = {
  path?: unknown
  id?: unknown
}

type PolygonCacheInfo = {
  cache_dir?: unknown
}

type CampaignCounts = {
  total?: unknown
  optimized?: unknown
  validated?: unknown
  skipped?: unknown
  failed?: unknown
  cancelled?: unknown
}

type CampaignItem = {
  item_id?: unknown
  csv_path?: unknown
  status?: unknown
  artifact_path?: unknown
  error?: unknown
}

type CampaignSummary = {
  version?: unknown
  kind?: unknown
  state?: unknown
  resume_state?: unknown
  counts?: CampaignCounts
  items?: CampaignItem[]
  request?: Record<string, unknown>
}

type CampaignCheckpoint = {
  path: string
  summary: CampaignSummary
}

type PolygonManifestDataset = {
  market: string
  display_symbol: string
  polygon_ticker: string
  timeframe: string
  csv_path: string
  min_ts: number
  max_ts: number
  last_updated_utc_ms: number
  row_count: number
  adjusted: boolean
  integrity: {
    checked_utc_ms: number
    parse_errors: number
    duplicates: number
    out_of_order: number
    same_day_gaps_gt_2min: number
    csv_size_bytes: number
    csv_sha256: string
  }
}

type PolygonManifest = {
  datasets: Record<string, PolygonManifestDataset>
}

type CampaignFixtures = {
  rootDir: string
  manifestPath: string
  manifest: PolygonManifest
  paths: string[]
  invalidPath: string
  packagedSamplePath: string
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizedPath(value: string): string {
  const full = path.resolve(value)
  return process.platform === 'win32' ? full.toLowerCase() : full
}

function numberValue(value: unknown): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function writeLongOhlcv(targetPath: string, rows: number): void {
  const out = ['timestamp,open,high,low,close,volume']
  for (let index = 0; index < rows; index += 1) {
    const timestamp = index * 60
    const close = 100 + Math.sin(index / 37) * 4 + index * 0.0001
    const open = close - 0.04
    const high = close + 0.18
    const low = close - 0.19
    out.push([
      String(timestamp),
      open.toFixed(6),
      high.toFixed(6),
      low.toFixed(6),
      close.toFixed(6),
      String(1_000 + (index % 700)),
    ].join(','))
  }
  fs.writeFileSync(targetPath, `${out.join('\n')}\n`, 'utf8')
}

function datasetManifestEntry(
  csvPath: string,
  displaySymbol: string,
  rowCount: number,
  maxTimestamp: number,
): PolygonManifestDataset {
  const stat = fs.statSync(csvPath)
  return {
    market: 'stocks',
    display_symbol: displaySymbol,
    polygon_ticker: displaySymbol,
    timeframe: '1m',
    csv_path: csvPath,
    min_ts: 0,
    max_ts: maxTimestamp,
    last_updated_utc_ms: Math.floor(stat.mtimeMs),
    row_count: rowCount,
    adjusted: true,
    integrity: {
      checked_utc_ms: Math.floor(stat.mtimeMs),
      parse_errors: 0,
      duplicates: 0,
      out_of_order: 0,
      same_day_gaps_gt_2min: 0,
      csv_size_bytes: stat.size,
      csv_sha256: sha256File(csvPath),
    },
  }
}

function writePolygonManifest(
  manifestPath: string,
  manifest: PolygonManifest,
): void {
  fs.writeFileSync(
    manifestPath,
    `${JSON.stringify(manifest, null, 2)}\n`,
    'utf8',
  )
}

async function seedCampaignFixtures(): Promise<CampaignFixtures> {
  const environment = requireReleaseProofEnvironment()
  const cacheInfo = await invokeNative<PolygonCacheInfo>(
    'polygon_get_cache_info',
  )
  const cacheDir = text(cacheInfo.cache_dir)
  if (
    !cacheDir
    || !isPathInside(cacheDir, environment.isolationRoot)
  ) {
    throw new Error(`Polygon cache escaped the isolated profile: ${cacheDir}`)
  }
  fs.mkdirSync(cacheDir, { recursive: true })
  const rootDir = path.resolve(cacheDir, FIXTURE_PREFIX)
  if (!isPathInside(rootDir, environment.isolationRoot)) {
    throw new Error(`Campaign fixture root escaped isolation: ${rootDir}`)
  }
  fs.mkdirSync(rootDir, { recursive: true })

  const samples = await invokeNative<SampleCsv[]>('list_sample_csvs')
  const packaged = samples.find((row) => text(row.id) === 'synthetic-ohlcv')
    ?? samples.find((row) => text(row.path))
  const packagedSamplePath = text(packaged?.path)
  if (!packagedSamplePath || !fs.existsSync(packagedSamplePath)) {
    throw new Error('The installed package did not expose its bundled sample CSV.')
  }

  const longPath = path.resolve(rootDir, `${FIXTURE_PREFIX}-00-long.csv`)
  const validAPath = path.resolve(rootDir, `${FIXTURE_PREFIX}-01-valid.csv`)
  const invalidPath = path.resolve(rootDir, `${FIXTURE_PREFIX}-02-invalid.csv`)
  const validBPath = path.resolve(rootDir, `${FIXTURE_PREFIX}-03-valid.csv`)
  writeLongOhlcv(longPath, 50_000)
  fs.copyFileSync(packagedSamplePath, validAPath)
  fs.writeFileSync(invalidPath, Buffer.from([0xff, 0xfe, 0xfd, 0x00]))
  fs.copyFileSync(packagedSamplePath, validBPath)

  const manifestPath = path.resolve(cacheDir, 'polygon_manifest.json')
  let manifest: PolygonManifest = { datasets: {} }
  if (fs.existsSync(manifestPath)) {
    const parsed = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as {
      datasets?: unknown
    }
    if (parsed.datasets && typeof parsed.datasets === 'object') {
      manifest = {
        datasets: parsed.datasets as Record<string, PolygonManifestDataset>,
      }
    }
  }
  manifest.datasets[`${FIXTURE_PREFIX}:00-long`] = datasetManifestEntry(
    longPath,
    `${FIXTURE_PREFIX}-00-long`,
    50_000,
    (50_000 - 1) * 60,
  )
  manifest.datasets[`${FIXTURE_PREFIX}:01-valid`] = datasetManifestEntry(
    validAPath,
    `${FIXTURE_PREFIX}-01-valid`,
    160,
    9_540,
  )
  manifest.datasets[`${FIXTURE_PREFIX}:02-invalid`] = datasetManifestEntry(
    invalidPath,
    `${FIXTURE_PREFIX}-02-invalid`,
    1,
    0,
  )
  manifest.datasets[`${FIXTURE_PREFIX}:03-valid`] = datasetManifestEntry(
    validBPath,
    `${FIXTURE_PREFIX}-03-valid`,
    160,
    9_540,
  )
  writePolygonManifest(manifestPath, manifest)

  return {
    rootDir,
    manifestPath,
    manifest,
    paths: [longPath, validAPath, invalidPath, validBPath],
    invalidPath,
    packagedSamplePath,
  }
}

function repairInvalidFixture(fixtures: CampaignFixtures): void {
  fs.copyFileSync(fixtures.packagedSamplePath, fixtures.invalidPath)
  fixtures.manifest.datasets[`${FIXTURE_PREFIX}:02-invalid`] =
    datasetManifestEntry(
      fixtures.invalidPath,
      `${FIXTURE_PREFIX}-02-invalid`,
      160,
      9_540,
    )
  writePolygonManifest(fixtures.manifestPath, fixtures.manifest)
}

async function selectValue(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await field.selectByAttribute('value', value)
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

async function configureBoundedDoubleMaCampaign(): Promise<void> {
  await waitForShellReady()
  await openWorkspace('workflow')
  const conventional = await $(nativeWorkflowSelectors.conventionalMode)
  await conventional.waitForClickable({ timeout: 10_000 })
  if ((await conventional.getAttribute('aria-selected')) !== 'true') {
    await conventional.click()
  }
  await (await $(nativeWorkflowSelectors.dataStep)).click()
  const sample = await $(nativeWorkflowSelectors.sampleDatasetButton)
  await sample.waitForClickable({ timeout: 15_000 })
  await sample.click()
  await (await $(nativeWorkflowSelectors.clearDatasetButton))
    .waitForEnabled({ timeout: 20_000 })
  await (await $(nativeWorkflowSelectors.optimizeStep)).click()
  await selectValue(nativeWorkflowSelectors.strategyKind, 'DoubleMa')
  await selectValue(nativeWorkflowSelectors.backend, 'CpuOnly')
  await selectValue(nativeWorkflowSelectors.mode, 'Grid')
  await setInput(nativeWorkflowSelectors.fastStart, '3')
  await setInput(nativeWorkflowSelectors.fastEnd, '20')
  await setInput(nativeWorkflowSelectors.fastStep, '1')
  await setInput(nativeWorkflowSelectors.slowStart, '30')
  await setInput(nativeWorkflowSelectors.slowEnd, '60')
  await setInput(nativeWorkflowSelectors.slowStep, '1')
}

async function openCampaignPanel(): Promise<void> {
  await openWorkspace('library')
  const refreshRuns = await $(nativeWorkflowSelectors.savedRunsRefresh)
  await refreshRuns.waitForEnabled({ timeout: 10_000 })
  await refreshRuns.click()
  await (await $(nativeWorkflowSelectors.firstSavedRun))
    .waitForDisplayed({ timeout: 15_000 })

  await openWorkspace('aggregate')
  const batchTab = await $('[data-qa-action="aggregate.tab.batch"]')
  await batchTab.waitForClickable({ timeout: 15_000 })
  await batchTab.click()
  await (await $('#meta_aggregate_panel_batch'))
    .waitForDisplayed({ timeout: 10_000 })
}

async function configureCampaignPanel(runId: string): Promise<void> {
  await selectValue('#vueBatchExecutionMode', 'optimize_only')
  await selectValue('#vueBatchPreloadPolicy', 'host_only')
  await setInput('#vueBatchTags', `release-proof-campaign-${runId}`)
  await setInput('#vueBatchHoldoutPct', '0')
  await setInput('#vueBatchHoldoutTag', 'holdout')
  await setCheckbox(
    '//label[contains(., "Skip completed")]//input[@type="checkbox"]',
    true,
  )
  await setInput('#vueBatchDatasetSearch', FIXTURE_PREFIX)

  const refresh = await $('[data-qa-action="aggregate.batch.refresh"]')
  await refresh.waitForEnabled({ timeout: 10_000 })
  await refresh.click()
  const datasetGroup = await $('[aria-label="Batch generation datasets"]')
  await datasetGroup.waitForDisplayed({ timeout: 10_000 })
  await browser.waitUntil(
    async () => {
      const content = await datasetGroup.getText()
      return [
        `${FIXTURE_PREFIX}-00-long`,
        `${FIXTURE_PREFIX}-01-valid`,
        `${FIXTURE_PREFIX}-02-invalid`,
        `${FIXTURE_PREFIX}-03-valid`,
      ].every((label) => content.includes(label))
    },
    {
      timeout: 10_000,
      interval: 100,
      timeoutMsg: 'The isolated campaign fixtures were absent from Batch discovery.',
    },
  )
  const none = await $('[data-qa-action="aggregate.batch.selectNone"]')
  await none.waitForEnabled({ timeout: 10_000 })
  await none.click()
  const all = await $('[data-qa-action="aggregate.batch.selectAll"]')
  await all.waitForEnabled({ timeout: 10_000 })
  await all.click()
  await browser.waitUntil(
    async () => (await datasetGroup.getText()).includes(FIXTURE_PREFIX)
      && (await (await $('#meta_aggregate_panel_batch')).getText())
        .includes('Selected 4/'),
    {
      timeout: 5_000,
      interval: 100,
      timeoutMsg: 'Batch did not select exactly the four filtered fixtures.',
    },
  )
}

async function lastSummaryPath(): Promise<string> {
  return text(await browser.execute((key: string) =>
    globalThis.localStorage.getItem(key) || '', LAST_BATCH_SUMMARY_KEY))
}

async function clearLastSummaryPath(): Promise<void> {
  await browser.execute((key: string) => {
    globalThis.localStorage.removeItem(key)
  }, LAST_BATCH_SUMMARY_KEY)
}

async function waitForSummaryState(
  expectedState: string,
  previousPath = '',
): Promise<CampaignCheckpoint> {
  let checkpoint: CampaignCheckpoint = {
    path: '',
    summary: {},
  }
  await browser.waitUntil(
    async () => {
      const nextPath = await lastSummaryPath()
      if (!nextPath || (previousPath && nextPath === previousPath)) return false
      try {
        const summary = await invokeNative<CampaignSummary>(
          'load_artifact',
          { path: nextPath },
        )
        if (text(summary.state) !== expectedState) return false
        checkpoint = { path: nextPath, summary }
        return true
      } catch {
        return false
      }
    },
    {
      timeout: 180_000,
      interval: 250,
      timeoutMsg: `Campaign did not reach ${expectedState}.`,
    },
  )
  return checkpoint
}

function artifactPathsByDataset(
  summary: CampaignSummary,
): Map<string, string> {
  const out = new Map<string, string>()
  for (const item of summary.items ?? []) {
    const csvPath = text(item.csv_path)
    const artifactPath = text(item.artifact_path)
    if (csvPath && artifactPath) {
      out.set(normalizedPath(csvPath), artifactPath)
    }
  }
  return out
}

describe('desktop exact-package aggregate campaign interruption and retry proof', () => {
  it('cancels durably, resumes after a cold launch, then repairs and retries only the failed dataset', async () => {
    const environment = requireReleaseProofEnvironment()
    const fixtures = await seedCampaignFixtures()
    await configureBoundedDoubleMaCampaign()
    await openCampaignPanel()
    await configureCampaignPanel(environment.runId)
    await clearLastSummaryPath()

    const run = await $('[data-qa-action="aggregate.batch.run"]')
    const stop = await $('[data-qa-action="aggregate.batch.stop"]')
    await run.waitForEnabled({ timeout: 10_000 })
    await run.click()
    await stop.waitForEnabled({ timeout: 10_000 })
    await browser.waitUntil(
      async () => (await lastSummaryPath()).length > 0,
      {
        timeout: 30_000,
        interval: 100,
        timeoutMsg: 'Campaign did not publish its initial durable checkpoint.',
      },
    )
    await stop.click()
    const cancelled = await waitForSummaryState('cancelled')
    expect(text(cancelled.summary.kind)).toBe('vectorgrid_campaign_summary')
    expect(numberValue(cancelled.summary.version)).toBe(2)
    expect(Boolean(cancelled.summary.counts?.cancelled)).toBe(true)
    expect(text(cancelled.summary.resume_state)).toBe('resumable')
    expect(cancelled.summary.items?.length ?? 0).toBe(fixtures.paths.length)
    expect(new Set(
      (cancelled.summary.items ?? []).map((item) => text(item.item_id)),
    ).size).toBe(fixtures.paths.length)

    await browser.reloadSession()
    await waitForShellReady()
    await openCampaignPanel()
    const refresh = await $('[data-qa-action="aggregate.batch.refresh"]')
    await refresh.waitForEnabled({ timeout: 10_000 })
    await refresh.click()
    const resume = await $('[data-qa-action="aggregate.batch.resumeIncomplete"]')
    await resume.waitForEnabled({ timeout: 15_000 })
    await resume.click()
    const resumed = await waitForSummaryState(
      'completed_with_failures',
      cancelled.path,
    )
    expect(numberValue(resumed.summary.counts?.total)).toBe(fixtures.paths.length)
    expect(numberValue(resumed.summary.counts?.failed)).toBe(1)
    expect(Boolean(resumed.summary.counts?.cancelled)).toBe(false)
    const failedItems = (resumed.summary.items ?? [])
      .filter((item) => text(item.status) === 'failed')
    expect(failedItems.length).toBe(1)
    expect(normalizedPath(text(failedItems[0]?.csv_path)))
      .toBe(normalizedPath(fixtures.invalidPath))
    const successfulBeforeRetry = artifactPathsByDataset(resumed.summary)
    expect(successfulBeforeRetry.size).toBe(3)

    repairInvalidFixture(fixtures)
    await refresh.waitForEnabled({ timeout: 10_000 })
    await refresh.click()
    const retry = await $('[data-qa-action="aggregate.batch.retryFailed"]')
    await retry.waitForEnabled({ timeout: 15_000 })
    await retry.click()
    const retried = await waitForSummaryState('completed', resumed.path)
    expect(numberValue(retried.summary.counts?.total)).toBe(fixtures.paths.length)
    expect(numberValue(retried.summary.counts?.optimized)).toBe(1)
    expect(numberValue(retried.summary.counts?.skipped)).toBe(3)
    expect(numberValue(retried.summary.counts?.failed)).toBe(0)
    expect(Boolean(retried.summary.counts?.cancelled)).toBe(false)
    expect(text(retried.summary.resume_state)).toBe('complete')

    const successfulAfterRetry = artifactPathsByDataset(retried.summary)
    expect(successfulAfterRetry.size).toBe(fixtures.paths.length)
    for (const [csvPath, artifactPath] of successfulBeforeRetry) {
      expect(successfulAfterRetry.get(csvPath)).toBe(artifactPath)
    }
    const repairedArtifact = successfulAfterRetry.get(
      normalizedPath(fixtures.invalidPath),
    )
    expect(text(repairedArtifact)).not.toBe('')
    expect(fs.existsSync(text(repairedArtifact))).toBe(true)
    expect(isPathInside(text(repairedArtifact), environment.isolationRoot))
      .toBe(true)

    writeReleaseProofEvidence('campaign-lifecycle-proof.json', {
      contract: 'vectorgrid-exact-package-campaign-lifecycle-v1',
      fixtures: fixtures.paths.map((csvPath) => ({
        csv_path: csvPath,
        sha256: sha256File(csvPath),
      })),
      cancelled: {
        summary_path: cancelled.path,
        summary_sha256: sha256File(cancelled.path),
        state: text(cancelled.summary.state),
        resume_state: text(cancelled.summary.resume_state),
        counts: cancelled.summary.counts,
        item_ids: (cancelled.summary.items ?? [])
          .map((item) => text(item.item_id)),
      },
      resumed: {
        summary_path: resumed.path,
        summary_sha256: sha256File(resumed.path),
        state: text(resumed.summary.state),
        counts: resumed.summary.counts,
        failed_csv_path: text(failedItems[0]?.csv_path),
        successful_artifacts: Object.fromEntries(successfulBeforeRetry),
      },
      retried: {
        summary_path: retried.path,
        summary_sha256: sha256File(retried.path),
        state: text(retried.summary.state),
        resume_state: text(retried.summary.resume_state),
        counts: retried.summary.counts,
        artifact_paths: Object.fromEntries(successfulAfterRetry),
      },
    })
  })
})
