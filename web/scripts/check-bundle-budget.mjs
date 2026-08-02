import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const repoRoot = path.resolve(webRoot, '..')
const distDir = path.resolve(process.cwd(), 'dist')
const assetsDir = path.join(distDir, 'assets')
const indexHtmlPath = path.join(distDir, 'index.html')
const reportPath = path.resolve(repoRoot, 'output', 'qa', 'bundle-budget.json')

const ENTRY_LIMIT_KB = 600
const LAZY_LIMIT_KB = 550
// The optional 3D heatmap is a lazy Three.js surface. Keep entry and per-lazy
// caps tight, while allowing total output to include that opt-in renderer.
const TOTAL_JS_WARN_KB = 3200
const TOTAL_JS_LIMIT_KB = 3450

const budgets = {
  entryChunkKb: ENTRY_LIMIT_KB,
  largestLazyChunkKb: LAZY_LIMIT_KB,
  totalJsWarningKb: TOTAL_JS_WARN_KB,
  totalJsKb: TOTAL_JS_LIMIT_KB,
}

function writeReport({ status, observations = null, warnings = [], issues = [] }) {
  const report = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    status,
    source: path.relative(repoRoot, distDir).replace(/\\/g, '/'),
    budgets,
    observations,
    warnings,
    issues,
  }
  fs.mkdirSync(path.dirname(reportPath), { recursive: true })
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')
  console.log(
    `[bundle-budget] Report written to ${path.relative(webRoot, reportPath).replace(/\\/g, '/')}`,
  )
}

function fail(messages, observations = null, warnings = []) {
  const issues = Array.isArray(messages) ? messages : [messages]
  writeReport({ status: 'failed', observations, warnings, issues })
  for (const issue of issues) console.error(`[bundle-budget] ${issue}`)
  process.exit(1)
}

if (!fs.existsSync(distDir) || !fs.existsSync(indexHtmlPath) || !fs.existsSync(assetsDir)) {
  fail('Missing build output. Run `npm run build` before checking the bundle budget.')
}

const indexHtml = fs.readFileSync(indexHtmlPath, 'utf8')
const entryMatch = indexHtml.match(/<script[^>]+src="\/assets\/([^"]+\.js)"/i)
if (!entryMatch) fail('Could not identify the main entry chunk from dist/index.html.')

const entryFile = entryMatch[1]
const assetFiles = fs.readdirSync(assetsDir).filter((file) => file.endsWith('.js'))
if (!assetFiles.length) fail('No JavaScript assets were found in dist/assets.')

const assetStats = assetFiles.map((file) => {
  const fullPath = path.join(assetsDir, file)
  const sizeBytes = fs.statSync(fullPath).size
  return {
    file,
    sizeBytes,
    sizeKb: sizeBytes / 1024,
  }
})

const entryStat = assetStats.find((item) => item.file === entryFile)
if (!entryStat) fail(`Entry chunk ${entryFile} was not found in dist/assets.`)

const lazyStats = assetStats.filter((item) => item.file !== entryFile)
const largestLazy = lazyStats.reduce((max, item) => (item.sizeBytes > max.sizeBytes ? item : max), {
  file: '',
  sizeBytes: 0,
  sizeKb: 0,
})
const totalJsBytes = assetStats.reduce((sum, item) => sum + item.sizeBytes, 0)
const totalJsKb = totalJsBytes / 1024
const observations = {
  assetCount: assetStats.length,
  entryFile: entryStat.file,
  entryChunkBytes: entryStat.sizeBytes,
  entryChunkKb: entryStat.sizeKb,
  largestLazyFile: largestLazy.file,
  largestLazyChunkBytes: largestLazy.sizeBytes,
  largestLazyChunkKb: largestLazy.sizeKb,
  totalJsBytes,
  totalJsKb,
}

const violations = []
if (entryStat.sizeKb > ENTRY_LIMIT_KB) {
  violations.push(`entry chunk ${entryStat.file} is ${entryStat.sizeKb.toFixed(1)} kB (limit ${ENTRY_LIMIT_KB} kB)`)
}
if (largestLazy.file && largestLazy.sizeKb > LAZY_LIMIT_KB) {
  violations.push(`largest lazy chunk ${largestLazy.file} is ${largestLazy.sizeKb.toFixed(1)} kB (limit ${LAZY_LIMIT_KB} kB)`)
}
if (totalJsKb > TOTAL_JS_LIMIT_KB) {
  violations.push(`total JavaScript output is ${totalJsKb.toFixed(1)} kB (limit ${TOTAL_JS_LIMIT_KB} kB)`)
}

const warnings = []
if (totalJsKb > TOTAL_JS_WARN_KB) {
  warnings.push(`total JavaScript output is ${totalJsKb.toFixed(1)} kB (warning threshold ${TOTAL_JS_WARN_KB} kB)`)
}

console.log(
  [
    `[bundle-budget] entry: ${entryStat.file} ${entryStat.sizeKb.toFixed(1)} kB`,
    `[bundle-budget] largest lazy: ${largestLazy.file || 'n/a'} ${largestLazy.sizeKb.toFixed(1)} kB`,
    `[bundle-budget] total js: ${totalJsKb.toFixed(1)} kB`,
  ].join('\n'),
)

if (violations.length) {
  fail(violations, observations, warnings)
}

if (warnings.length) {
  console.warn(`[bundle-budget] ${warnings.join('\n[bundle-budget] ')}`)
}

writeReport({ status: 'passed', observations, warnings })
