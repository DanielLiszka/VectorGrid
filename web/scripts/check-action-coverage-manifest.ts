import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { actionCoverageManifest, type QaActionCoverageEntry } from '../e2e/browser/qa/actionCoverageManifest'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const sourceRoot = path.resolve(webRoot, 'src')
const browserQaRoot = path.resolve(webRoot, 'e2e', 'browser', 'qa')
const reportPath = path.resolve(webRoot, '..', 'output', 'qa', 'action-coverage-static.json')

const ACTION_ID_PATTERN = /^[a-z][A-Za-z0-9]*(?:\.[A-Za-z0-9]+)+$/
const VALID_PRIORITIES = new Set(['critical', 'important', 'extended'])

interface SourceAction {
  id: string
  file: string
  line: number
  source: 'attribute' | 'actionId'
}

interface ManifestIssue {
  id: string
  message: string
}

function walkFiles(dir: string): string[] {
  const entries = fs.readdirSync(dir, { withFileTypes: true })
  const files: string[] = []
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist') continue
      files.push(...walkFiles(fullPath))
      continue
    }
    if (/\.(vue|ts|tsx)$/.test(entry.name)) files.push(fullPath)
  }
  return files
}

function lineNumber(content: string, index: number): number {
  let line = 1
  for (let i = 0; i < index; i += 1) {
    if (content.charCodeAt(i) === 10) line += 1
  }
  return line
}

function relative(file: string): string {
  return path.relative(webRoot, file).replace(/\\/g, '/')
}

function collectSourceActions(): SourceAction[] {
  const actions: SourceAction[] = []
  for (const file of walkFiles(sourceRoot)) {
    const content = fs.readFileSync(file, 'utf8')
    for (const match of content.matchAll(/(?<!:)\bdata-qa-action\s*=\s*["']([^"']+)["']/g)) {
      actions.push({
        id: match[1],
        file: relative(file),
        line: lineNumber(content, match.index ?? 0),
        source: 'attribute',
      })
    }
    for (const match of content.matchAll(/\bactionId:\s*["']([^"']+)["']/g)) {
      actions.push({
        id: match[1],
        file: relative(file),
        line: lineNumber(content, match.index ?? 0),
        source: 'actionId',
      })
    }
  }
  return actions
}

function collectWorkflowIds(): string[] {
  if (!fs.existsSync(browserQaRoot)) return []
  const workflowIds = new Set<string>()
  for (const file of walkFiles(browserQaRoot)) {
    const content = fs.readFileSync(file, 'utf8')
    for (const match of content.matchAll(/\bdefineWorkflow\s*\(\s*\{\s*id:\s*["']([^"']+)["']/g)) {
      workflowIds.add(match[1])
    }
  }
  return [...workflowIds].sort()
}

function validateEntry(entry: QaActionCoverageEntry, index: number): ManifestIssue[] {
  const prefix = entry.actionId || `entry[${index}]`
  const issues: ManifestIssue[] = []
  if (!ACTION_ID_PATTERN.test(entry.actionId)) {
    issues.push({ id: prefix, message: 'actionId must use dotted lower/camel-case segments' })
  }
  if (!entry.surface.trim()) {
    issues.push({ id: prefix, message: 'surface is required' })
  }
  if (!VALID_PRIORITIES.has(entry.priority)) {
    issues.push({ id: prefix, message: `priority must be one of ${[...VALID_PRIORITIES].join(', ')}` })
  }
  if (!Array.isArray(entry.coveredBy)) {
    issues.push({ id: prefix, message: 'coveredBy must be an array' })
  }
  if (entry.required && entry.coveredBy.length === 0 && !entry.waiver) {
    issues.push({ id: prefix, message: 'required actions need coverage or an explicit waiver' })
  }
  if (entry.waiver) {
    if (!entry.waiver.owner.trim()) {
      issues.push({ id: prefix, message: 'waiver.owner is required' })
    }
    if (!entry.waiver.reason.trim()) {
      issues.push({ id: prefix, message: 'waiver.reason is required' })
    }
    if (entry.waiver.expires && Number.isNaN(Date.parse(entry.waiver.expires))) {
      issues.push({ id: prefix, message: 'waiver.expires must be an ISO-compatible date' })
    }
    if (entry.waiver.expires && Date.parse(entry.waiver.expires) < Date.now()) {
      issues.push({ id: prefix, message: `waiver expired on ${entry.waiver.expires}` })
    }
  }
  return issues
}

const sourceActions = collectSourceActions()
const manifestIds = new Map<string, QaActionCoverageEntry>()
const duplicateIds: string[] = []
for (const entry of actionCoverageManifest) {
  if (manifestIds.has(entry.actionId)) duplicateIds.push(entry.actionId)
  manifestIds.set(entry.actionId, entry)
}

const sourceActionIds = [...new Set(sourceActions.map((action) => action.id))].sort()
const workflowIds = collectWorkflowIds()
const workflowIdSet = new Set(workflowIds)
const coveredByIds = [...new Set(actionCoverageManifest.flatMap((entry) => entry.coveredBy))].sort()
const unknownCoverageReferences = coveredByIds.filter((id) => !workflowIdSet.has(id))
const missingFromManifest = sourceActionIds.filter((id) => !manifestIds.has(id))
const notFoundAsLiteralSource = [...manifestIds.keys()].filter((id) => !sourceActionIds.includes(id)).sort()
const invalidSourceIds = sourceActions.filter((action) => !ACTION_ID_PATTERN.test(action.id))
const manifestIssues = actionCoverageManifest.flatMap((entry, index) => validateEntry(entry, index))
const requiredEntries = actionCoverageManifest.filter((entry) => entry.required)
const waivedEntries = actionCoverageManifest.filter((entry) => !!entry.waiver)
const requiredCoveredEntries = requiredEntries.filter((entry) => entry.coveredBy.length > 0)
const priorityCounts = actionCoverageManifest.reduce<Record<string, number>>((acc, entry) => {
  acc[entry.priority] = (acc[entry.priority] ?? 0) + 1
  return acc
}, {})
const surfaceCounts = actionCoverageManifest.reduce<Record<string, number>>((acc, entry) => {
  acc[entry.surface] = (acc[entry.surface] ?? 0) + 1
  return acc
}, {})

const report = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  sourceActionCount: sourceActionIds.length,
  manifestEntryCount: actionCoverageManifest.length,
  requiredEntryCount: requiredEntries.length,
  requiredCoveredCount: requiredCoveredEntries.length,
  waiverCount: waivedEntries.length,
  workflowIds,
  coveredByIds,
  unknownCoverageReferences,
  priorityCounts,
  surfaceCounts,
  missingFromManifest,
  notFoundAsLiteralSource,
  duplicateIds: [...new Set(duplicateIds)].sort(),
  invalidSourceIds,
  manifestIssues,
  sourceActions,
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true })
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')

const failures = [
  ...missingFromManifest.map((id) => `source action is missing from manifest: ${id}`),
  ...[...new Set(duplicateIds)].map((id) => `duplicate manifest actionId: ${id}`),
  ...invalidSourceIds.map((item) => `invalid source action id ${item.id} at ${item.file}:${item.line}`),
  ...manifestIssues.map((issue) => `${issue.id}: ${issue.message}`),
]

if (failures.length > 0) {
  console.error('[qa-actions] Action coverage manifest drift detected:')
  for (const failure of failures) console.error(`- ${failure}`)
  console.error(`[qa-actions] Static report written to ${path.relative(webRoot, reportPath).replace(/\\/g, '/')}`)
  process.exit(1)
}

console.log(
  `[qa-actions] ${sourceActionIds.length} source QA actions represented by ${actionCoverageManifest.length} manifest entries.`,
)
console.log(`[qa-actions] Static report written to ${path.relative(webRoot, reportPath).replace(/\\/g, '/')}`)
