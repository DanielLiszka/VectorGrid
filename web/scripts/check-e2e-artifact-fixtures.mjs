#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const fixtureDir = path.resolve(webRoot, 'e2e', 'browser', 'fixtures', 'artifacts')
const reportPath = path.resolve(webRoot, '..', 'output', 'qa', 'artifact-fixtures.json')

function fail(message, details = []) {
  console.error(`[artifact-fixtures] ${message}`)
  for (const detail of details) console.error(`- ${detail}`)
  process.exit(1)
}

function walkJson(dir) {
  if (!fs.existsSync(dir)) return []
  const files = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...walkJson(fullPath))
    } else if (entry.name.endsWith('.json')) {
      files.push(fullPath)
    }
  }
  return files
}

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'))
  } catch (error) {
    fail(`Invalid JSON in ${path.relative(webRoot, file).replace(/\\/g, '/')}`, [String(error)])
  }
}

function inspectValue(value, pathParts, issues) {
  if (typeof value === 'string') {
    if (/\bmassive\b/i.test(value)) {
      issues.push(`${pathParts.join('.')} uses legacy Massive provider text`)
    }
    return
  }
  if (!value || typeof value !== 'object') return
  if (Array.isArray(value)) {
    value.forEach((item, index) => inspectValue(item, [...pathParts, String(index)], issues))
    return
  }
  for (const [key, nested] of Object.entries(value)) {
    if ((key === 'provider_name' || key === 'providerName') && String(nested).toLowerCase() !== 'polygon.io') {
      issues.push(`${[...pathParts, key].join('.')} must be polygon.io`)
    }
    inspectValue(nested, [...pathParts, key], issues)
  }
}

function requireArray(record, key, issues) {
  if (!Array.isArray(record[key]) || record[key].length === 0) {
    issues.push(`${key} must be a non-empty array`)
  }
}

function validatePack(file, pack) {
  const issues = []
  if (!pack || typeof pack !== 'object' || Array.isArray(pack)) {
    return ['fixture must be a JSON object']
  }
  requireArray(pack, 'savedRunSummaries', issues)
  requireArray(pack, 'repairedRunSummaries', issues)
  requireArray(pack, 'optionsDatasets', issues)
  requireArray(pack, 'optionsArtifactSummaries', issues)
  for (const key of [
    'parentRunDetail',
    'repairDryRun',
    'repairWrite',
    'optionsExpressionArtifact',
    'optionsValidationFeasibility',
    'optionsValidationResult',
  ]) {
    if (!pack[key] || typeof pack[key] !== 'object' || Array.isArray(pack[key])) {
      issues.push(`${key} must be an object`)
    }
  }
  const artifact = pack.optionsExpressionArtifact || {}
  if (artifact.kind !== 'options_expression_artifact') {
    issues.push('optionsExpressionArtifact.kind must be options_expression_artifact')
  }
  if (artifact.price_semantics !== 'minute_aggregate') {
    issues.push('optionsExpressionArtifact.price_semantics must be minute_aggregate')
  }
  const validationKind = pack.optionsValidationResult?.result?.kind
  if (validationKind !== 'options_expression_validation_artifact') {
    issues.push('optionsValidationResult.result.kind must be options_expression_validation_artifact')
  }
  const verdict = pack.optionsValidationResult?.result?.acceptance_summary?.verdict
  if (verdict !== 'accepted') {
    issues.push('optionsValidationResult.result.acceptance_summary.verdict must be accepted')
  }
  inspectValue(pack, [path.basename(file)], issues)
  return issues
}

const files = walkJson(fixtureDir)
if (files.length === 0) {
  fail('No artifact fixture JSON files found.')
}

const reports = []
const failures = []
for (const file of files) {
  const pack = readJson(file)
  const issues = validatePack(file, pack)
  reports.push({
    file: path.relative(webRoot, file).replace(/\\/g, '/'),
    issueCount: issues.length,
  })
  failures.push(...issues.map((issue) => `${path.relative(webRoot, file).replace(/\\/g, '/')}: ${issue}`))
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true })
fs.writeFileSync(reportPath, `${JSON.stringify({ schemaVersion: 1, generatedAt: new Date().toISOString(), reports }, null, 2)}\n`)

if (failures.length > 0) {
  fail('Artifact fixture validation failed.', failures)
}

console.log(`[artifact-fixtures] ${files.length} artifact fixture pack(s) validated.`)
console.log(`[artifact-fixtures] Report written to ${path.relative(webRoot, reportPath).replace(/\\/g, '/')}`)
