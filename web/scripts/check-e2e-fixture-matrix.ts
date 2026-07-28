import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  fixtureFamilies,
  fixtureFamilyIds,
  fixtureScenarios,
  type FixtureFamilyId,
  type FixtureFamilyModule,
  type MockTauriScenario,
} from '../e2e/browser/fixtures'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const repoRoot = path.resolve(webRoot, '..')
const browserRoot = path.resolve(webRoot, 'e2e', 'browser')
const browserQaRoot = path.resolve(browserRoot, 'qa')
const fixtureArtifactRoot = path.resolve(browserRoot, 'fixtures', 'artifacts')
const reportPath = path.resolve(repoRoot, 'output', 'qa', 'e2e-fixture-matrix.json')

const SCENARIO_NAME_PATTERN = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/
const SECRET_VALUE_PATTERN =
  /\b(sk_live_[A-Za-z0-9]{12,}|sk_test_[A-Za-z0-9]{12,}|pk_live_[A-Za-z0-9]{12,}|AKIA[0-9A-Z]{16})\b/
const FIXTURE_ARTIFACT_SIZE_LIMIT_BYTES = 256 * 1024
const FIXTURE_ARTIFACT_SIZE_ALLOWLIST = new Map<string, { maxBytes: number; reason: string }>()

interface WorkflowReference {
  id: string
  file: string
  scenarioIdentifier: string
  scenarioName?: string
  family?: FixtureFamilyId
  localScenario: boolean
}

interface FamilyReport {
  description: string
  scenarioCount: number
  workflowCount: number
  scenarios: string[]
  workflows: string[]
  deferred?: {
    reason: string
  }
}

interface FixtureArtifactReport {
  file: string
  bytes: number
  maxBytes: number
  allowlisted: boolean
  reason?: string
}

interface RequiredWorkflowBacklogItem {
  id: string
  aliases?: string[]
  deferredReason?: string
}

const REQUIRED_WORKFLOW_BACKLOG: RequiredWorkflowBacklogItem[] = [
  { id: 'data.sample.load.success' },
  { id: 'data.sample.load.failure', aliases: ['data.load.failure'] },
  { id: 'optimize.loaded.success' },
  { id: 'optimize.loaded.failure' },
  { id: 'optimize.composite.routeSummary' },
  { id: 'validation.blocked.datasetMismatch' },
  { id: 'validation.success.accepted', aliases: ['validation.success.selected'] },
  { id: 'validation.success.review', aliases: ['validation.review.evidence'] },
  { id: 'validation.success.rejected', aliases: ['validation.rejected.evidence'] },
  { id: 'validation.export.actions', aliases: ['validation.success.selected'] },
  { id: 'artifact.library.repair', aliases: ['library.savedRun.repairCheck', 'library.savedRun.repairWrite'] },
  { id: 'artifact.library.detailHydration', aliases: ['library.openRun'] },
  { id: 'taskCenter.progress.lifecycle' },
  { id: 'commandPalette.workflowActions', aliases: ['shell.commandPalette.navigation'] },
  { id: 'aggregate.meta.runOpenReveal' },
  { id: 'aggregate.batch.campaignLifecycle' },
  { id: 'live.safety.confirmations' },
  { id: 'settings.diagnostics.storage' },
  { id: 'options.artifact.followup', aliases: ['options.validation.controls'] },
  { id: 'mlSignal.unavailable.failure', aliases: ['mlSignal.unavailable.emptyBundle'] },
  { id: 'responsive.mobile.primaryPath', aliases: ['responsive.mobile.workbenchParity', 'layout.mobile.workflow'] },
  { id: 'help.tooltips.visibleRole' },
]

function walkFiles(dir: string): string[] {
  if (!fs.existsSync(dir)) return []
  const files: string[] = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist') continue
      files.push(...walkFiles(fullPath))
      continue
    }
    if (entry.name.endsWith('.ts')) files.push(fullPath)
  }
  return files
}

function walkFilesByExtension(dir: string, extension: string): string[] {
  if (!fs.existsSync(dir)) return []
  const files: string[] = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...walkFilesByExtension(fullPath, extension))
      continue
    }
    if (entry.name.endsWith(extension)) files.push(fullPath)
  }
  return files
}

function relative(file: string): string {
  return path.relative(webRoot, file).replace(/\\/g, '/')
}

function inspectForSecrets(value: unknown, pathParts: string[], issues: string[]): void {
  if (typeof value === 'string') {
    if (SECRET_VALUE_PATTERN.test(value)) {
      issues.push(`${pathParts.join('.')} looks like a live/test credential and must not be committed as a fixture`)
    }
    return
  }
  if (!value || typeof value !== 'object') return
  if (Array.isArray(value)) {
    value.forEach((item, index) => inspectForSecrets(item, [...pathParts, String(index)], issues))
    return
  }
  for (const [key, nested] of Object.entries(value)) {
    inspectForSecrets(nested, [...pathParts, key], issues)
  }
}

function scenarioIssues(scenario: MockTauriScenario): string[] {
  const issues: string[] = []
  if (!SCENARIO_NAME_PATTERN.test(scenario.name)) {
    issues.push(`${scenario.name}: scenario name must be kebab-case`)
  }
  if (!String(scenario.description || '').trim()) {
    issues.push(`${scenario.name}: description is required`)
  }
  if (!scenario.commandResults || typeof scenario.commandResults !== 'object' || Array.isArray(scenario.commandResults)) {
    issues.push(`${scenario.name}: commandResults must be an object`)
  } else {
    for (const key of Object.keys(scenario.commandResults)) {
      if (!key.trim()) issues.push(`${scenario.name}: commandResults contains an empty command name`)
    }
  }
  const expected = new Set(scenario.expectedInvokes ?? [])
  const forbidden = new Set(scenario.forbiddenInvokes ?? [])
  for (const command of expected) {
    if (forbidden.has(command)) {
      issues.push(`${scenario.name}: ${command} appears in both expectedInvokes and forbiddenInvokes`)
    }
  }
  inspectForSecrets(scenario, [scenario.name], issues)
  return issues
}

function collectWorkflowReferences(): WorkflowReference[] {
  const scenarioByIdentifier = new Map<string, MockTauriScenario>()
  for (const scenario of fixtureScenarios) {
    const exportName = `${scenario.name.replace(/-([a-z0-9])/g, (_, part: string) => part.toUpperCase())}Scenario`
    scenarioByIdentifier.set(exportName, scenario)
  }
  for (const [identifier, scenario] of Object.entries({
    emptyAppScenario: fixtureScenarios.find((scenario) => scenario.name === 'empty-app'),
    dataLoadFailureScenario: fixtureScenarios.find((scenario) => scenario.name === 'data-load-failure'),
    gpuStatusFailureScenario: fixtureScenarios.find((scenario) => scenario.name === 'gpu-status-failure'),
    validationDatasetMismatchScenario: fixtureScenarios.find((scenario) => scenario.name === 'validation-dataset-mismatch'),
    validationSuccessScenario: fixtureScenarios.find((scenario) => scenario.name === 'validation-success'),
    optionsValidationReadyScenario: fixtureScenarios.find((scenario) => scenario.name === 'options-validation-ready'),
    optionsValidationInputFailureScenario: fixtureScenarios.find((scenario) => scenario.name === 'options-validation-input-failure'),
    optionsValidationPreviewFailureScenario: fixtureScenarios.find((scenario) => scenario.name === 'options-validation-preview-failure'),
    liveReadyScenario: fixtureScenarios.find((scenario) => scenario.name === 'live-ready'),
    liveSafetyReadyScenario: fixtureScenarios.find((scenario) => scenario.name === 'live-safety-ready'),
    optimizationSuccessScenario: fixtureScenarios.find((scenario) => scenario.name === 'optimization-success-ready'),
    optimizationFailureScenario: fixtureScenarios.find((scenario) => scenario.name === 'optimization-command-failure'),
    candidateDetailReadyScenario: fixtureScenarios.find((scenario) => scenario.name === 'candidate-detail-ready'),
    heatmapLayoutScenario: fixtureScenarios.find((scenario) => scenario.name === 'heatmap-layout-ready'),
    savedRunOpenScenario: fixtureScenarios.find((scenario) => scenario.name === 'saved-run-open-ready'),
    savedRunRepairScenario: fixtureScenarios.find((scenario) => scenario.name === 'saved-run-repair-ready'),
    savedRunRepairWriteScenario: fixtureScenarios.find((scenario) => scenario.name === 'saved-run-repair-write-ready'),
    // These workflow-local variants enrich a registered base fixture with
    // presentation-specific responses while retaining the base family contract.
    optionsBacktestValidationScenario: fixtureScenarios.find((scenario) => scenario.name === 'options-validation-ready'),
    aggregateMetaTableScenario: fixtureScenarios.find((scenario) => scenario.name === 'aggregate-meta-ready'),
  })) {
    if (scenario) scenarioByIdentifier.set(identifier, scenario)
  }

  const refs: WorkflowReference[] = []
  for (const file of walkFiles(browserQaRoot)) {
    const content = fs.readFileSync(file, 'utf8')
    for (const match of content.matchAll(/\bdefineWorkflow\s*\(/g)) {
      const snippet = content.slice(match.index ?? 0, (match.index ?? 0) + 2200)
      const id = snippet.match(/\bid:\s*['"]([^'"]+)['"]/)?.[1] ?? ''
      const scenarioIdentifier = snippet.match(/\bscenario:\s*([A-Za-z_$][\w$]*)/)?.[1] ?? ''
      if (!id) continue
      const scenario = scenarioByIdentifier.get(scenarioIdentifier)
      refs.push({
        id,
        file: relative(file),
        scenarioIdentifier,
        scenarioName: scenario?.name,
        family: scenario?.family,
        localScenario: !scenario,
      })
    }
  }
  return refs.sort((a, b) => a.id.localeCompare(b.id))
}

function familyReports(workflowReferences: WorkflowReference[]): Record<FixtureFamilyId, FamilyReport> {
  const out = {} as Record<FixtureFamilyId, FamilyReport>
  for (const family of fixtureFamilies) {
    const workflows = workflowReferences
      .filter((workflow) => workflow.family === family.family)
      .map((workflow) => workflow.id)
      .sort()
    out[family.family] = {
      description: family.description,
      scenarioCount: family.scenarios.length,
      workflowCount: workflows.length,
      scenarios: family.scenarios.map((scenario) => scenario.name).sort(),
      workflows,
      ...(family.deferred ? { deferred: family.deferred } : {}),
    }
  }
  return out
}

function validateFamilies(): string[] {
  const issues: string[] = []
  const validFamilies = new Set<FixtureFamilyId>(fixtureFamilyIds)
  const seenFamilies = new Set<FixtureFamilyId>()
  const scenarioMembership = new Map<string, string[]>()

  for (const family of fixtureFamilies as FixtureFamilyModule[]) {
    if (!validFamilies.has(family.family)) {
      issues.push(`${family.family}: invalid fixture family id`)
      continue
    }
    if (seenFamilies.has(family.family)) {
      issues.push(`${family.family}: duplicate fixture family module`)
    }
    seenFamilies.add(family.family)
    if (!String(family.description || '').trim()) {
      issues.push(`${family.family}: family description is required`)
    }
    if (family.scenarios.length === 0 && !family.deferred) {
      issues.push(`${family.family}: family has no scenarios and no deferred reason`)
    }
    if (family.deferred && !String(family.deferred.reason || '').trim()) {
      issues.push(`${family.family}: deferred families need a reason`)
    }
    for (const scenario of family.scenarios) {
      if (scenario.family !== family.family) {
        issues.push(`${scenario.name}: scenario family ${scenario.family} does not match module ${family.family}`)
      }
      const membership = scenarioMembership.get(scenario.name) ?? []
      membership.push(family.family)
      scenarioMembership.set(scenario.name, membership)
      issues.push(...scenarioIssues(scenario))
    }
  }

  for (const family of fixtureFamilyIds) {
    if (!seenFamilies.has(family)) issues.push(`${family}: missing fixture family module`)
  }
  for (const [name, memberships] of scenarioMembership) {
    if (memberships.length > 1) {
      issues.push(`${name}: scenario appears in multiple families (${memberships.join(', ')})`)
    }
  }
  return issues
}

function fixtureArtifactReports(): FixtureArtifactReport[] {
  return walkFilesByExtension(fixtureArtifactRoot, '.json')
    .map((file) => {
      const rel = relative(file)
      const allowlist = FIXTURE_ARTIFACT_SIZE_ALLOWLIST.get(rel)
      const stat = fs.statSync(file)
      return {
        file: rel,
        bytes: stat.size,
        maxBytes: allowlist?.maxBytes ?? FIXTURE_ARTIFACT_SIZE_LIMIT_BYTES,
        allowlisted: !!allowlist,
        ...(allowlist?.reason ? { reason: allowlist.reason } : {}),
      }
    })
    .sort((a, b) => a.file.localeCompare(b.file))
}

const workflowReferences = collectWorkflowReferences()
const workflowIds = new Set(workflowReferences.map((workflow) => workflow.id))
const requiredWorkflowBacklog = REQUIRED_WORKFLOW_BACKLOG.map((item) => {
  const matchedBy = [item.id, ...(item.aliases ?? [])].find((id) => workflowIds.has(id)) ?? ''
  return {
    id: item.id,
    aliases: item.aliases ?? [],
    status: matchedBy ? 'covered' : item.deferredReason ? 'deferred' : 'missing',
    matchedBy,
    deferredReason: item.deferredReason ?? '',
  }
})
const issues = [
  ...validateFamilies(),
  ...workflowReferences
    .filter((workflow) => !workflow.scenarioIdentifier)
    .map((workflow) => `${workflow.id}: workflow does not declare a scenario`),
]
const localScenarioReferences = workflowReferences.filter(
  (workflow) => workflow.localScenario && workflow.scenarioIdentifier,
)
issues.push(
  ...localScenarioReferences.map(
    (workflow) =>
      `${workflow.id}: scenario ${workflow.scenarioIdentifier} is not registered in the fixture family matrix`,
  ),
)
issues.push(
  ...requiredWorkflowBacklog
    .filter((item) => item.status === 'missing')
    .map((item) => `${item.id}: required workflow backlog item is not covered or explicitly deferred`),
)
const families = familyReports(workflowReferences)
const artifactReports = fixtureArtifactReports()
const referencedScenarioNames = new Set(
  workflowReferences
    .map((workflow) => workflow.scenarioName)
    .filter((name): name is string => !!name),
)
const unusedScenarios = fixtureScenarios.filter(
  (scenario) => !referencedScenarioNames.has(scenario.name),
)
issues.push(
  ...unusedScenarios.map(
    (scenario) =>
      `${scenario.name}: registered fixture scenario is not referenced by any workflow`,
  ),
)
issues.push(
  ...artifactReports
    .filter((artifact) => artifact.bytes > artifact.maxBytes)
    .map(
      (artifact) =>
        `${artifact.file}: fixture artifact is ${artifact.bytes} bytes, above ${artifact.maxBytes} byte limit without an allowlist`,
    ),
)
const scenarioNames = fixtureScenarios.map((scenario) => scenario.name).sort()
const duplicateScenarioNames = scenarioNames.filter((name, index) => index > 0 && scenarioNames[index - 1] === name)
issues.push(...duplicateScenarioNames.map((name) => `${name}: duplicate scenario name`))

const report = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  status: issues.length > 0 ? 'failed' : 'passed',
  scenarioCount: fixtureScenarios.length,
  workflowCount: workflowReferences.length,
  registeredWorkflowCount: workflowReferences.length - localScenarioReferences.length,
  localWorkflowScenarioCount: localScenarioReferences.length,
  families,
  artifactSizeLimitBytes: FIXTURE_ARTIFACT_SIZE_LIMIT_BYTES,
  artifactReports,
  workflowReferences,
  requiredWorkflowBacklog,
  localScenarioReferences,
  issues,
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true })
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')

if (issues.length > 0) {
  console.error('[fixture-matrix] Fixture matrix drift detected:')
  for (const issue of issues) console.error(`- ${issue}`)
  console.error(`[fixture-matrix] Report written to ${path.relative(webRoot, reportPath).replace(/\\/g, '/')}`)
  process.exit(1)
}

console.log(
  `[fixture-matrix] ${fixtureScenarios.length} registered scenario(s), ${workflowReferences.length} workflow reference(s), ${localScenarioReferences.length} local workflow scenario(s).`,
)
console.log(`[fixture-matrix] Report written to ${path.relative(webRoot, reportPath).replace(/\\/g, '/')}`)
