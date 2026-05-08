#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const repoRoot = path.resolve(webRoot, '..')
const outputDir = path.resolve(repoRoot, 'output', 'qa')
const jsonPath = path.resolve(outputDir, 'local-qa-summary.json')
const markdownPath = path.resolve(outputDir, 'local-qa-summary.md')

const existsCache = new Map()
const statCache = new Map()
const listAllFilesCache = new Map()

function fileExists(fullPath) {
  let exists = existsCache.get(fullPath)
  if (exists === undefined) {
    exists = fs.existsSync(fullPath)
    existsCache.set(fullPath, exists)
  }
  return exists
}

function fileStat(fullPath) {
  let stat = statCache.get(fullPath)
  if (!stat) {
    stat = fs.statSync(fullPath)
    statCache.set(fullPath, stat)
  }
  return stat
}

function readJson(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath)
  if (!fileExists(fullPath)) return null
  return JSON.parse(fs.readFileSync(fullPath, 'utf8'))
}

function listAllFiles(dir) {
  const cached = listAllFilesCache.get(dir)
  if (cached) return cached
  if (!fileExists(dir)) {
    listAllFilesCache.set(dir, [])
    return []
  }
  const files = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...listAllFiles(fullPath))
    } else {
      files.push(fullPath)
    }
  }
  listAllFilesCache.set(dir, files)
  return files
}

function listFiles(dir, predicate) {
  const files = listAllFiles(dir)
  if (!files.length) return files
  return files.filter(predicate)
}

function newestMtimeMs(entries) {
  let newest = 0
  for (const entry of entries) {
    const fullPath = path.resolve(repoRoot, entry)
    if (!fileExists(fullPath)) continue
    const stat = fileStat(fullPath)
    if (stat.isDirectory()) {
      const files = listFiles(
        fullPath,
        (item) => /\.(json|mjs|ts|tsx|vue)$/.test(item),
      )
      for (const file of files) {
        newest = Math.max(newest, fileStat(file).mtimeMs)
      }
      continue
    }
    newest = Math.max(newest, stat.mtimeMs)
  }
  return newest
}

function reportFreshnessIssue(label, relativePath, sourceEntries, required = true) {
  const fullPath = path.resolve(repoRoot, relativePath)
  if (!fileExists(fullPath)) {
    return required ? `${label} report is missing: ${relativePath}` : null
  }
  let generatedAtMs = 0
  try {
    const report = JSON.parse(fs.readFileSync(fullPath, 'utf8'))
    generatedAtMs = Date.parse(report.generatedAt || '')
  } catch {
    return `${label} report is not valid JSON: ${relativePath}`
  }
  const reportMs = Number.isFinite(generatedAtMs) ? generatedAtMs : fileStat(fullPath).mtimeMs
  const sourceMs = newestMtimeMs(sourceEntries)
  if (sourceMs > 0 && sourceMs - reportMs > 1000) {
    return `${label} report is stale: ${relativePath}`
  }
  return null
}

function collectFreshnessIssues() {
  return [
    reportFreshnessIssue('static action coverage', 'output/qa/action-coverage-static.json', [
      'web/src',
      'web/e2e/browser/qa/actionCoverageManifest.ts',
      'web/e2e/browser/qa',
      'web/scripts/check-action-coverage-manifest.ts',
    ]),
    reportFreshnessIssue('artifact fixture', 'output/qa/artifact-fixtures.json', [
      'web/e2e/browser/fixtures/artifacts',
      'web/scripts/check-e2e-artifact-fixtures.mjs',
    ]),
    reportFreshnessIssue('fixture matrix', 'output/qa/e2e-fixture-matrix.json', [
      'web/e2e/browser/fixtures',
      'web/e2e/browser/qa',
      'web/scripts/check-e2e-fixture-matrix.ts',
    ]),
    reportFreshnessIssue('selector inventory', 'output/qa/qa-selector-inventory.json', [
      'web/src',
      'web/scripts/check-qa-selector-inventory.mjs',
    ]),
    reportFreshnessIssue('Tauri capability', 'output/qa/tauri-capabilities.json', [
      'src-tauri/capabilities',
      'src-tauri/tauri.conf.json',
      'web/scripts/check-tauri-capabilities.mjs',
    ]),
    reportFreshnessIssue('UI perf budget', 'output/qa/ui-perf-budget.json', [
      'scripts/baselines/ui_capture/perf_metrics.json',
      'web/scripts/check-ui-perf-budget.mjs',
    ]),
    reportFreshnessIssue('runtime action coverage', 'output/playwright/action-coverage.json', [
      'web/src',
      'web/e2e/browser/qa',
    ], false),
    reportFreshnessIssue('runtime options action coverage', 'output/playwright/action-coverage-options.json', [
      'web/src',
      'web/e2e/browser/qa',
    ], false),
  ].filter(Boolean)
}

function countWorkflowIds() {
  const qaDir = path.resolve(webRoot, 'e2e', 'browser', 'qa')
  const ids = new Set()
  for (const file of listFiles(qaDir, (item) => item.endsWith('.ts'))) {
    const content = fs.readFileSync(file, 'utf8')
    for (const match of content.matchAll(/\bdefineWorkflow\s*\(\s*\{\s*id:\s*['"]([^'"]+)['"]/g)) {
      ids.add(match[1])
    }
  }
  return [...ids].sort()
}

function formatNumber(value) {
  return typeof value === 'number' && Number.isFinite(value) ? String(value) : 'n/a'
}

function bulletList(items) {
  if (!items.length) return '- none'
  return items.map((item) => `- ${item}`).join('\n')
}

function fileSha256(fullPath) {
  return crypto.createHash('sha256').update(fs.readFileSync(fullPath)).digest('hex')
}

function collectVisualBaselineEvidence() {
  const specRelativePath = 'web/e2e/browser/qa/visualBaselines.spec.ts'
  const snapshotDirRelativePath = 'web/e2e/browser/qa/visualBaselines.spec.ts-snapshots'
  const specPath = path.resolve(repoRoot, specRelativePath)
  const snapshotDir = path.resolve(repoRoot, snapshotDirRelativePath)
  const projectName = 'chromium'
  const platformName = process.platform
  const specContent = fileExists(specPath) ? fs.readFileSync(specPath, 'utf8') : ''
  const baselineNames = [
    ...new Set(
      [...specContent.matchAll(/\btoHaveScreenshot\s*\(\s*['"`]([^'"`]+\.png)['"`]/g)]
        .map((match) => String(match[1] || '').trim())
        .filter(Boolean),
    ),
  ].sort()
  const expectedFiles = baselineNames.map((name) => {
    const parsed = path.parse(name)
    return `${parsed.name}-${projectName}-${platformName}${parsed.ext || '.png'}`
  })
  const actualFiles = listFiles(snapshotDir, (item) => item.endsWith('.png'))
    .map((file) => path.basename(file))
    .sort()
  const actualSet = new Set(actualFiles)
  const expectedSet = new Set(expectedFiles)
  const files = expectedFiles.map((file, index) => {
    const fullPath = path.resolve(snapshotDir, file)
    const exists = fileExists(fullPath)
    const stat = exists ? fileStat(fullPath) : null
    return {
      baselineName: baselineNames[index],
      file,
      path: path.join(snapshotDirRelativePath, file).replace(/\\/g, '/'),
      present: exists,
      bytes: stat?.size ?? 0,
      modifiedAt: stat ? stat.mtime.toISOString() : '',
      sha256: exists ? fileSha256(fullPath) : '',
    }
  })
  const specMs = fileExists(specPath) ? fileStat(specPath).mtimeMs : 0
  const stale = files
    .filter((file) => {
      if (!file.present || specMs <= 0) return false
      const snapshotMs = Date.parse(file.modifiedAt)
      return Number.isFinite(snapshotMs) && specMs - snapshotMs > 1000
    })
    .map((file) => file.file)
  return {
    specPath: specRelativePath,
    snapshotDir: snapshotDirRelativePath,
    projectName,
    platformName,
    expectedCount: expectedFiles.length,
    actualCount: actualFiles.length,
    snapshotCount: actualFiles.length,
    missing: expectedFiles.filter((file) => !actualSet.has(file)),
    extra: actualFiles.filter((file) => !expectedSet.has(file)),
    stale,
    files,
  }
}

function visualBaselineIssues(evidence) {
  const issues = []
  if (!evidence.expectedCount) {
    issues.push('visual baseline spec has no toHaveScreenshot baselines')
  }
  issues.push(...evidence.missing.map((file) => `visual baseline snapshot missing: ${file}`))
  issues.push(...evidence.extra.map((file) => `visual baseline snapshot extra or stale: ${file}`))
  return issues
}

const actionStatic = readJson('output/qa/action-coverage-static.json')
const artifactFixtures = readJson('output/qa/artifact-fixtures.json')
const fixtureMatrix = readJson('output/qa/e2e-fixture-matrix.json')
const selectorInventory = readJson('output/qa/qa-selector-inventory.json')
const tauriCapabilities = readJson('output/qa/tauri-capabilities.json')
const perfBudget = readJson('output/qa/ui-perf-budget.json')
const a11ySmoke = readJson('output/qa/a11y-smoke.json')
const actionCoverageReports = ['action-coverage.json', 'action-coverage-options.json']
  .map((file) => readJson(path.join('output', 'playwright', file)))
  .filter(Boolean)
const visualBaselineEvidence = collectVisualBaselineEvidence()
const workflowIds = countWorkflowIds()

const issues = []
issues.push(...collectFreshnessIssues())
issues.push(...visualBaselineIssues(visualBaselineEvidence))
if (actionStatic) {
  issues.push(...(actionStatic.missingFromManifest || []).map((id) => `action missing from manifest: ${id}`))
  issues.push(...(actionStatic.unknownCoverageReferences || []).map((id) => `coverage references unknown workflow id: ${id}`))
  issues.push(...(actionStatic.duplicateIds || []).map((id) => `duplicate action manifest id: ${id}`))
  issues.push(...(actionStatic.invalidSourceIds || []).map((item) => `invalid source action id: ${item.id}`))
  issues.push(...(actionStatic.manifestIssues || []).map((item) => `${item.id}: ${item.message}`))
}
if (artifactFixtures) {
  for (const report of artifactFixtures.reports || []) {
    if (report.issueCount > 0) issues.push(`artifact fixture ${report.file} has ${report.issueCount} issue(s)`)
  }
}
if (fixtureMatrix) {
  issues.push(...(fixtureMatrix.issues || []).map((issue) => `fixture matrix: ${issue}`))
}
if (tauriCapabilities) {
  issues.push(...(tauriCapabilities.issues || []).map((issue) => `tauri capability/CSP: ${issue}`))
}
if (perfBudget) {
  issues.push(...(perfBudget.issues || []).map((issue) => `ui perf budget: ${issue}`))
}
for (const report of actionCoverageReports) {
  issues.push(...(report.unknown || []).map((id) => `runtime action missing from manifest: ${id}`))
  issues.push(...(report.requiredUncovered || []).map((id) => `required runtime action uncovered: ${id}`))
  issues.push(...(report.expiredWaivers || []).map((id) => `expired runtime action waiver: ${id}`))
}

const summary = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  status: issues.length > 0 ? 'failed' : 'passed',
  workflows: {
    count: workflowIds.length,
    ids: workflowIds,
  },
  actions: actionStatic
    ? {
        sourceActionCount: actionStatic.sourceActionCount,
        manifestEntryCount: actionStatic.manifestEntryCount,
        requiredEntryCount: actionStatic.requiredEntryCount,
        requiredCoveredCount: actionStatic.requiredCoveredCount,
        waiverCount: actionStatic.waiverCount,
        unknownCoverageReferenceCount: actionStatic.unknownCoverageReferences?.length ?? 0,
        unknownCoverageReferences: actionStatic.unknownCoverageReferences ?? [],
        priorityCounts: actionStatic.priorityCounts,
        surfaceCounts: actionStatic.surfaceCounts,
        dynamicOrNonLiteralManifestEntries: actionStatic.notFoundAsLiteralSource?.length ?? 0,
      }
    : null,
  selectors: selectorInventory
    ? {
        controlCount: selectorInventory.controlCount,
        annotatedControlCount: selectorInventory.annotatedControlCount,
        unannotatedControlCount: selectorInventory.unannotatedControlCount,
        actionSelectorCount: selectorInventory.actionSelectorCount,
        panelSelectorCount: selectorInventory.panelSelectorCount,
        stateSelectorCount: selectorInventory.stateSelectorCount,
        annotationRatio: selectorInventory.annotationRatio,
      }
    : null,
  fixtures: {
    artifactPackCount: artifactFixtures?.reports?.length ?? 0,
    matrixStatus: fixtureMatrix?.status ?? 'not present',
    scenarioCount: fixtureMatrix?.scenarioCount ?? 0,
    registeredWorkflowCount: fixtureMatrix?.registeredWorkflowCount ?? 0,
    localWorkflowScenarioCount: fixtureMatrix?.localWorkflowScenarioCount ?? 0,
    fixtureArtifactCount: fixtureMatrix?.artifactReports?.length ?? 0,
    requiredBacklogTotal: fixtureMatrix?.requiredWorkflowBacklog?.length ?? 0,
    requiredBacklogCovered:
      fixtureMatrix?.requiredWorkflowBacklog?.filter((item) => item.status === 'covered').length ?? 0,
    requiredBacklogDeferred:
      fixtureMatrix?.requiredWorkflowBacklog?.filter((item) => item.status === 'deferred').length ?? 0,
    requiredBacklogMissing:
      fixtureMatrix?.requiredWorkflowBacklog?.filter((item) => item.status === 'missing').length ?? 0,
    families: fixtureMatrix?.families ?? {},
  },
  visualBaselines: {
    ...visualBaselineEvidence,
  },
  perfBudget: perfBudget
    ? {
        source: perfBudget.source,
        observations: perfBudget.observations,
        budgets: perfBudget.budgets,
      }
    : null,
  a11ySmoke: a11ySmoke
    ? {
        status: a11ySmoke.status,
        generatedAt: a11ySmoke.generatedAt,
        scannedSurfaceCount: Array.isArray(a11ySmoke.scannedSurfaces)
          ? a11ySmoke.scannedSurfaces.length
          : 0,
        includeWcag22: !!a11ySmoke.includeWcag22,
      }
    : null,
  reportsSeen: {
    staticActionCoverage: !!actionStatic,
    artifactFixtures: !!artifactFixtures,
    fixtureMatrix: !!fixtureMatrix,
    selectorInventory: !!selectorInventory,
    tauriCapabilities: !!tauriCapabilities,
    perfBudget: !!perfBudget,
    a11ySmoke: !!a11ySmoke,
    runtimeActionCoverageReports: actionCoverageReports.length,
  },
  issues,
}

const lines = [
  '# Local QA Summary',
  '',
  `Generated: ${summary.generatedAt}`,
  `Status: ${summary.status}`,
  '',
  '## Coverage',
  '',
  `- Browser workflow IDs: ${summary.workflows.count}`,
  `- Source QA actions: ${formatNumber(summary.actions?.sourceActionCount)}`,
  `- Manifest entries: ${formatNumber(summary.actions?.manifestEntryCount)}`,
  `- Required manifest entries covered: ${formatNumber(summary.actions?.requiredCoveredCount)} / ${formatNumber(summary.actions?.requiredEntryCount)}`,
  `- Manifest waivers: ${formatNumber(summary.actions?.waiverCount)}`,
  `- Coverage references without a matching workflow id: ${formatNumber(summary.actions?.unknownCoverageReferenceCount)}`,
  `- Visual baseline snapshots: ${summary.visualBaselines.actualCount} actual / ${summary.visualBaselines.expectedCount} expected`,
  `- Visual baseline missing/extra: ${summary.visualBaselines.missing.length} missing / ${summary.visualBaselines.extra.length} extra`,
  `- Visual baseline older-than-spec snapshots: ${summary.visualBaselines.stale.length}`,
  `- Artifact fixture packs: ${summary.fixtures.artifactPackCount}`,
  `- Fixture matrix: ${summary.fixtures.matrixStatus}`,
  `- Registered scenarios: ${summary.fixtures.scenarioCount}`,
  `- Workflows using registered scenarios: ${summary.fixtures.registeredWorkflowCount}`,
  `- Workflows using local scenarios: ${summary.fixtures.localWorkflowScenarioCount}`,
  `- Fixture JSON artifacts checked: ${summary.fixtures.fixtureArtifactCount}`,
  `- Required workflow backlog: ${summary.fixtures.requiredBacklogCovered} covered / ${summary.fixtures.requiredBacklogTotal} total (${summary.fixtures.requiredBacklogDeferred} deferred, ${summary.fixtures.requiredBacklogMissing} missing)`,
  `- Accessibility smoke: ${summary.a11ySmoke ? summary.a11ySmoke.status : 'not present'}`,
  '',
  '## Selector Inventory',
  '',
  `- Annotated controls: ${formatNumber(summary.selectors?.annotatedControlCount)} / ${formatNumber(summary.selectors?.controlCount)}`,
  `- Action selector declarations: ${formatNumber(summary.selectors?.actionSelectorCount)}`,
  `- Panel markers: ${formatNumber(summary.selectors?.panelSelectorCount)}`,
  `- State markers: ${formatNumber(summary.selectors?.stateSelectorCount)}`,
  '',
  '## Perf Budget',
  '',
  perfBudget
    ? Object.entries(perfBudget.observations || {})
        .map(([key, value]) => `- ${key}: ${value ?? 'missing'} / ${(perfBudget.budgets || {})[key] ?? 'n/a'}`)
        .join('\n')
    : '- perf budget report not available',
  '',
  '## Issues',
  '',
  bulletList(issues),
  '',
]

fs.mkdirSync(outputDir, { recursive: true })
fs.writeFileSync(jsonPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8')
fs.writeFileSync(markdownPath, `${lines.join('\n')}\n`, 'utf8')

console.log(`[qa-summary] ${summary.status}. Summary written to ${path.relative(webRoot, markdownPath).replace(/\\/g, '/')}`)
if (issues.length > 0) {
  for (const issue of issues) console.error(`[qa-summary] ${issue}`)
  process.exit(1)
}
