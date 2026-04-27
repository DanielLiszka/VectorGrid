#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const repoRoot = path.resolve(webRoot, '..')
const outputDir = path.resolve(repoRoot, 'output', 'qa')
const jsonPath = path.resolve(outputDir, 'local-qa-summary.json')
const markdownPath = path.resolve(outputDir, 'local-qa-summary.md')

function readJson(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath)
  if (!fs.existsSync(fullPath)) return null
  return JSON.parse(fs.readFileSync(fullPath, 'utf8'))
}

function listFiles(dir, predicate) {
  if (!fs.existsSync(dir)) return []
  const files = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...listFiles(fullPath, predicate))
    } else if (predicate(fullPath)) {
      files.push(fullPath)
    }
  }
  return files
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

const actionStatic = readJson('output/qa/action-coverage-static.json')
const artifactFixtures = readJson('output/qa/artifact-fixtures.json')
const selectorInventory = readJson('output/qa/qa-selector-inventory.json')
const tauriCapabilities = readJson('output/qa/tauri-capabilities.json')
const perfBudget = readJson('output/qa/ui-perf-budget.json')
const actionCoverageReports = ['action-coverage.json', 'action-coverage-options.json']
  .map((file) => readJson(path.join('output', 'playwright', file)))
  .filter(Boolean)
const visualSnapshotCount = listFiles(
  path.resolve(webRoot, 'e2e', 'browser', 'qa', 'visualBaselines.spec.ts-snapshots'),
  (item) => item.endsWith('.png'),
).length
const workflowIds = countWorkflowIds()

const issues = []
if (actionStatic) {
  issues.push(...(actionStatic.missingFromManifest || []).map((id) => `action missing from manifest: ${id}`))
  issues.push(...(actionStatic.duplicateIds || []).map((id) => `duplicate action manifest id: ${id}`))
  issues.push(...(actionStatic.invalidSourceIds || []).map((item) => `invalid source action id: ${item.id}`))
  issues.push(...(actionStatic.manifestIssues || []).map((item) => `${item.id}: ${item.message}`))
}
if (artifactFixtures) {
  for (const report of artifactFixtures.reports || []) {
    if (report.issueCount > 0) issues.push(`artifact fixture ${report.file} has ${report.issueCount} issue(s)`)
  }
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
  },
  visualBaselines: {
    snapshotCount: visualSnapshotCount,
  },
  perfBudget: perfBudget
    ? {
        source: perfBudget.source,
        observations: perfBudget.observations,
        budgets: perfBudget.budgets,
      }
    : null,
  reportsSeen: {
    staticActionCoverage: !!actionStatic,
    artifactFixtures: !!artifactFixtures,
    selectorInventory: !!selectorInventory,
    tauriCapabilities: !!tauriCapabilities,
    perfBudget: !!perfBudget,
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
  `- Visual baseline snapshots: ${summary.visualBaselines.snapshotCount}`,
  `- Artifact fixture packs: ${summary.fixtures.artifactPackCount}`,
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
