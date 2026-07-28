import { $, browser, expect } from '@wdio/globals'
import { captureNativeFailure, invokeNative } from '../support/nativeIpc.js'
import {
  isReleaseProofRun,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type NativeCommand =
  | 'run_double_ma_optimization'
  | 'run_composite_cpu_optimization'
  | 'run_composite_mo_optimization'

type RouteCase = {
  strategyKind:
    | 'DoubleMa'
    | 'Pit'
    | 'CompositeTrend'
    | 'RegimeHybrid'
    | 'BreakoutMomentum'
    | 'Rocb'
    | 'CompositeCpu'
    | 'CompositeMo'
  command: NativeCommand
  family: string
  routeKind: 'backend' | 'fixed-cpu-mo'
}

type RunArtifactSummary = {
  path?: unknown
  strategy_kind?: unknown
  route_provenance?: unknown
}

type RunArtifact = {
  kind?: unknown
  route_provenance?: unknown
  run?: {
    req?: unknown
    res?: unknown
  }
}

type RouteProvenance = {
  requestedRoute: string
  resolvedRoute: string
  realizedRoute: string
}

type BrowserStorageEntry = [string, string]

const ROUTE_CASES: readonly RouteCase[] = [
  {
    strategyKind: 'DoubleMa',
    command: 'run_double_ma_optimization',
    family: 'double-ma baseline',
    routeKind: 'backend',
  },
  {
    strategyKind: 'Pit',
    command: 'run_double_ma_optimization',
    family: 'standalone double-ma profile',
    routeKind: 'backend',
  },
  {
    strategyKind: 'CompositeTrend',
    command: 'run_double_ma_optimization',
    family: 'composite trend profile',
    routeKind: 'backend',
  },
  {
    strategyKind: 'RegimeHybrid',
    command: 'run_double_ma_optimization',
    family: 'regime hybrid profile',
    routeKind: 'backend',
  },
  {
    strategyKind: 'BreakoutMomentum',
    command: 'run_double_ma_optimization',
    family: 'breakout momentum profile',
    routeKind: 'backend',
  },
  {
    strategyKind: 'Rocb',
    command: 'run_composite_cpu_optimization',
    family: 'standalone composite-CPU profile',
    routeKind: 'backend',
  },
  {
    strategyKind: 'CompositeCpu',
    command: 'run_composite_cpu_optimization',
    family: 'composite-CPU baseline',
    routeKind: 'backend',
  },
  {
    strategyKind: 'CompositeMo',
    command: 'run_composite_mo_optimization',
    family: 'fixed-CPU multi-objective profile',
    routeKind: 'fixed-cpu-mo',
  },
] as const

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {}
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeBackendUsed(requested: string, value: unknown): string {
  const raw = text(value)
  if (raw === 'Cpu') return requested.startsWith('Cpu') ? requested : 'CpuOnly'
  if (raw === 'Gpu') return requested.startsWith('Gpu') ? requested : 'Gpu'
  return raw
}

function requestedRoute(value: unknown): string {
  const row = asRecord(value)
  if (Object.keys(row).length > 0) {
    return text(row.backend ?? row.kind ?? row.label)
  }
  return text(value)
}

function readRouteProvenance(value: unknown): RouteProvenance {
  const row = asRecord(value)
  return {
    requestedRoute: text(row.requested_route ?? row.requestedRoute),
    resolvedRoute: text(row.resolved_route ?? row.resolvedRoute),
    realizedRoute: text(row.realized_route ?? row.realizedRoute),
  }
}

function realizedRoute(reqValue: unknown, resValue: unknown): string {
  const req = asRecord(reqValue)
  const res = asRecord(resValue)
  const requested = requestedRoute(req.backend)
  const route = text(res.backend_route ?? res.backendRoute)
  const segments = route
    .split('->')
    .map((part) => part.trim())
    .filter(Boolean)
  return segments.at(-1)
    || normalizeBackendUsed(requested, res.backend_used ?? res.backendUsed)
    || requested
}

async function selectValueIfDisplayed(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  if (!(await field.isExisting()) || !(await field.isDisplayed())) return
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

async function selectValueRequired(selector: string, value: string): Promise<void> {
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
        querySelector: (value: string) => {
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

async function prepareSampleDataset(): Promise<void> {
  await waitForShellReady()
  await openWorkspace('workflow')
  const conventionalMode = await $(nativeWorkflowSelectors.conventionalMode)
  await conventionalMode.waitForDisplayed({ timeout: 10_000 })
  if ((await conventionalMode.getAttribute('aria-selected')) !== 'true') {
    await conventionalMode.click()
  }

  await (await $(nativeWorkflowSelectors.dataStep)).click()
  const sampleButton = await $(nativeWorkflowSelectors.sampleDatasetButton)
  await sampleButton.waitForClickable({ timeout: 15_000 })
  await sampleButton.click()
  await (await $(nativeWorkflowSelectors.clearDatasetButton))
    .waitForEnabled({ timeout: 20_000 })

  // A route proof must remain bounded and must not fan out into validation.
  await (await $(nativeWorkflowSelectors.validationStep)).click()
  await setCheckbox(nativeWorkflowSelectors.validationAutoRun, false)
  await setCheckbox(nativeWorkflowSelectors.validationAutoExportXlsx, false)
  await setCheckbox(nativeWorkflowSelectors.validationAutoRevealReport, false)
  await (await $(nativeWorkflowSelectors.optimizeStep)).click()
}

async function configureOnePairCpuCase(routeCase: RouteCase): Promise<void> {
  await selectValueRequired(
    nativeWorkflowSelectors.strategyKind,
    routeCase.strategyKind,
  )
  if (!['CompositeCpu', 'CompositeMo'].includes(routeCase.strategyKind)) {
    await selectValueRequired(nativeWorkflowSelectors.backend, 'CpuOnly')
  }
  if (!['Rocb', 'CompositeCpu', 'CompositeMo'].includes(routeCase.strategyKind)) {
    await selectValueRequired(nativeWorkflowSelectors.mode, 'Grid')
  }
  await setInput(nativeWorkflowSelectors.fastStart, '3')
  await setInput(nativeWorkflowSelectors.fastEnd, '3')
  await setInput(nativeWorkflowSelectors.fastStep, '1')
  await setInput(nativeWorkflowSelectors.slowStart, '8')
  await setInput(nativeWorkflowSelectors.slowEnd, '8')
  await setInput(nativeWorkflowSelectors.slowStep, '1')
  if (routeCase.routeKind === 'fixed-cpu-mo') {
    const advancedTab = await $(nativeWorkflowSelectors.advancedControlsTab)
    await advancedTab.waitForClickable({ timeout: 10_000 })
    await advancedTab.click()
    await (await $('#vueAdvanced_moSampler')).waitForDisplayed({ timeout: 10_000 })
    await selectValueIfDisplayed('#vueAdvanced_moSampler', 'Random')
    await setInput('#vueAdvanced_moRandomTrials', '1')
    await setInput('#vueAdvanced_moSearchSpace', '{"params":[]}')
    await setCheckbox('#vueAdvanced_moPrescanEnable', false)
    await setCheckbox('#vueAdvanced_moUseRangeBuilder', false)
    const optimizeTab = await $('#controlsTabBar [data-tab="opt"]')
    await optimizeTab.waitForClickable({ timeout: 10_000 })
    await optimizeTab.click()
  }
}

async function snapshotBrowserStorage(): Promise<BrowserStorageEntry[]> {
  const serialized = await browser.execute(() => {
    const entries: BrowserStorageEntry[] = []
    for (let index = 0; index < globalThis.localStorage.length; index += 1) {
      const key = globalThis.localStorage.key(index)
      if (key != null) entries.push([key, globalThis.localStorage.getItem(key) || ''])
    }
    return JSON.stringify(entries)
  }) as unknown as string
  return JSON.parse(serialized) as BrowserStorageEntry[]
}

async function restoreBrowserStorage(entries: BrowserStorageEntry[]): Promise<void> {
  await browser.execute((serialized: string) => {
    const saved = JSON.parse(serialized) as BrowserStorageEntry[]
    globalThis.localStorage.clear()
    for (const [key, value] of saved) globalThis.localStorage.setItem(key, value)
  }, JSON.stringify(entries))
}

async function waitForNewArtifact(
  existingPaths: ReadonlySet<string>,
  strategyKind: RouteCase['strategyKind'],
): Promise<RunArtifactSummary> {
  let match: RunArtifactSummary | null = null
  await browser.waitUntil(
    async () => {
      const rows = await invokeNative<RunArtifactSummary[]>('list_run_artifacts')
      match = rows.find((row) => (
        text(row.strategy_kind) === strategyKind
        && text(row.path).length > 0
        && !existingPaths.has(text(row.path))
      )) ?? null
      return match != null
    },
    {
      timeout: 60_000,
      interval: 250,
      timeoutMsg: `${strategyKind} completed without a new persisted run artifact.`,
    },
  )
  if (!match) throw new Error(`${strategyKind} artifact lookup ended without a match.`)
  return match
}

async function visibleRouteText(): Promise<string> {
  const overviewTab = await $(nativeWorkflowSelectors.overviewResultsTab)
  await overviewTab.waitForClickable({ timeout: 10_000 })
  await overviewTab.click()
  const card = await $(nativeWorkflowSelectors.overviewRouteCard)
  await card.waitForDisplayed({ timeout: 10_000 })
  const content = await card.getText()
  const routeLine = content
    .split(/\r?\n/g)
    .map((line) => line.trim())
    .find((line) => line.startsWith('Route:'))
  if (!routeLine) {
    throw new Error(`Overview did not expose route provenance. Card content: ${content}`)
  }
  return routeLine
}

describe('desktop native conventional CPU route matrix', () => {
  it('runs each conventional route family with invalid-request recovery and matching visible provenance', async () => {
    await waitForShellReady()
    const storageBefore = await snapshotBrowserStorage()
    const createdArtifactPaths: string[] = []
    const routeEvidence: Array<Record<string, unknown>> = []
    try {
      await prepareSampleDataset()

      for (const routeCase of ROUTE_CASES) {
        await configureOnePairCpuCase(routeCase)
        const beforeRows = await invokeNative<RunArtifactSummary[]>('list_run_artifacts')
        const beforePaths = new Set(beforeRows.map((row) => text(row.path)).filter(Boolean))

        const runButton = await $(nativeWorkflowSelectors.runOptimization)
        await runButton.waitForEnabled({ timeout: 10_000 })
        await runButton.click()
        const summary = await waitForNewArtifact(beforePaths, routeCase.strategyKind)
        const artifactPath = text(summary.path)
        createdArtifactPaths.push(artifactPath)
        const artifact = await invokeNative<RunArtifact>('load_run_artifact', {
          path: artifactPath,
        })
        expect(artifact.kind).toBe('vectorgrid_run_artifact')

        const req = asRecord(artifact.run?.req)
        const res = asRecord(artifact.run?.res)
        expect(text(req.strategy_kind ?? req.strategyKind)).toBe(routeCase.strategyKind)
        const provenance = readRouteProvenance(
          artifact.route_provenance ?? summary.route_provenance,
        )
        let visibleRoute = ''
        if (routeCase.routeKind === 'backend') {
          expect(requestedRoute(req.backend)).toBe('CpuOnly')
          expect(provenance.requestedRoute).toBe('CpuOnly')
          expect(provenance.realizedRoute).toBe('CpuOnly')
          expect(realizedRoute(req, res)).toBe('CpuOnly')

          const routeText = await visibleRouteText()
          expect(routeText).toContain('requested=CpuOnly')
          expect(routeText).toContain('actual=CpuOnly')
          visibleRoute = routeText
        } else {
          // Composite MO is a fixed CPU product route with no backend selector or
          // backend_route field. Its exact command contract and disabled GPU
          // prescan are the fail-closed route evidence for this family.
          expect(req.backend).toBeUndefined()
          expect(req.gpu_prescan).toBeNull()
          expect(provenance.requestedRoute).toBe('')
          expect(Array.isArray(res.pareto)).toBe(true)
        }

        const invalidReq = {
          ...req,
          data_id: `__desktop_route_matrix_missing_${routeCase.strategyKind}__`,
        }
        const invalidError = await captureNativeFailure(routeCase.command, {
          req: invalidReq,
        })
        expect(invalidError.toLowerCase()).toMatch(/unknown data id|missing data|data id/)

        const recovered = await invokeNative<Record<string, unknown>>(routeCase.command, {
          req,
        })
        if (routeCase.routeKind === 'backend') {
          expect(realizedRoute(req, recovered)).toBe(provenance.realizedRoute)
        } else {
          expect(recovered.data_id).toBe(req.data_id)
          expect(asRecord(recovered.telemetry)).not.toEqual({})
        }
        const hasResult = Array.isArray(recovered.top)
          || Array.isArray(recovered.pareto)
          || Object.keys(asRecord(recovered.best_params ?? recovered.bestParams)).length > 0
        expect(hasResult).toBe(true)
        routeEvidence.push({
          strategy_kind: routeCase.strategyKind,
          command: routeCase.command,
          family: routeCase.family,
          route_kind: routeCase.routeKind,
          artifact_path: artifactPath,
          requested_route: provenance.requestedRoute,
          resolved_route: provenance.resolvedRoute,
          realized_route: provenance.realizedRoute,
          visible_route: visibleRoute,
          invalid_request_error: invalidError,
          recovered_after_invalid_request: hasResult,
        })

        // Restore Optimize after the Overview evidence check before configuring the next family.
        await (await $(nativeWorkflowSelectors.optimizeStep)).click()
        await expect(await $(nativeWorkflowSelectors.runOptimization)).toBeEnabled()
      }
      if (isReleaseProofRun()) {
        writeReleaseProofEvidence('route-matrix-proof.json', {
          contract: 'vectorgrid-exact-package-route-matrix-v1',
          case_count: routeEvidence.length,
          cases: routeEvidence,
        })
      }
    } finally {
      for (const artifactPath of createdArtifactPaths.reverse()) {
        await invokeNative('delete_run_artifact', { path: artifactPath }).catch(() => undefined)
      }
      const clearDataset = await $(nativeWorkflowSelectors.clearDatasetButton)
      if (await clearDataset.isEnabled().catch(() => false)) {
        await clearDataset.click().catch(() => undefined)
      }
      await restoreBrowserStorage(storageBefore).catch(() => undefined)
    }
  })
})
