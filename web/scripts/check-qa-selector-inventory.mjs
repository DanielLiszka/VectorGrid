#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const sourceRoot = path.resolve(webRoot, 'src')
const reportPath = path.resolve(webRoot, '..', 'output', 'qa', 'qa-selector-inventory.json')

const CONTROL_TAG_PATTERN = /<(button|a|input|select|textarea)\b([^>]*)>/gi
const ACTION_ATTR_PATTERN = /\b:?data-qa-action\s*=/i
const PANEL_ATTR_PATTERN = /\b:?data-qa-panel\s*=/g
const STATE_ATTR_PATTERN = /\b:?data-qa-state\s*=/g
const HIDDEN_INPUT_PATTERN = /\btype\s*=\s*["']hidden["']/i

function walkFiles(dir) {
  const files = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist') continue
      files.push(...walkFiles(fullPath))
      continue
    }
    if (/\.(test|spec)\.(ts|tsx)$/.test(entry.name)) continue
    if (/\.(vue|ts|tsx)$/.test(entry.name)) files.push(fullPath)
  }
  return files
}

function lineNumber(content, index) {
  let line = 1
  for (let i = 0; i < index; i += 1) {
    if (content.charCodeAt(i) === 10) line += 1
  }
  return line
}

function relative(file) {
  return path.relative(webRoot, file).replace(/\\/g, '/')
}

const files = walkFiles(sourceRoot)
const unannotatedControls = []
const perFile = []
let controlCount = 0
let annotatedControlCount = 0
let actionSelectorCount = 0
let panelSelectorCount = 0
let stateSelectorCount = 0

for (const file of files) {
  const content = fs.readFileSync(file, 'utf8')
  const fileStats = {
    file: relative(file),
    controls: 0,
    annotatedControls: 0,
    actionSelectors: 0,
    panelSelectors: 0,
    stateSelectors: 0,
  }

  for (const match of content.matchAll(CONTROL_TAG_PATTERN)) {
    const tag = String(match[1] || '').toLowerCase()
    const attrs = String(match[2] || '')
    if (tag === 'input' && HIDDEN_INPUT_PATTERN.test(attrs)) continue
    fileStats.controls += 1
    controlCount += 1
    if (ACTION_ATTR_PATTERN.test(attrs)) {
      fileStats.annotatedControls += 1
      annotatedControlCount += 1
      continue
    }
    if (unannotatedControls.length < 80) {
      unannotatedControls.push({
        file: fileStats.file,
        line: lineNumber(content, match.index ?? 0),
        tag,
        snippet: match[0].replace(/\s+/g, ' ').slice(0, 160),
      })
    }
  }

  fileStats.actionSelectors = (content.match(/\b:?data-qa-action\s*=/g) || []).length
  fileStats.panelSelectors = (content.match(PANEL_ATTR_PATTERN) || []).length
  fileStats.stateSelectors = (content.match(STATE_ATTR_PATTERN) || []).length
  actionSelectorCount += fileStats.actionSelectors
  panelSelectorCount += fileStats.panelSelectors
  stateSelectorCount += fileStats.stateSelectors
  if (
    fileStats.controls > 0 ||
    fileStats.actionSelectors > 0 ||
    fileStats.panelSelectors > 0 ||
    fileStats.stateSelectors > 0
  ) {
    perFile.push(fileStats)
  }
}

const report = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  controlCount,
  annotatedControlCount,
  unannotatedControlCount: Math.max(0, controlCount - annotatedControlCount),
  actionSelectorCount,
  panelSelectorCount,
  stateSelectorCount,
  annotationRatio: controlCount > 0 ? annotatedControlCount / controlCount : 1,
  unannotatedControls,
  perFile,
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true })
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')

console.log(
  `[qa-selectors] ${annotatedControlCount}/${controlCount} controls carry data-qa-action; ${panelSelectorCount} panels and ${stateSelectorCount} state markers found.`,
)
console.log(`[qa-selectors] Inventory written to ${path.relative(webRoot, reportPath).replace(/\\/g, '/')}`)
