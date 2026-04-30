#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { spawnSync } from 'node:child_process'

function fail(message) {
  console.error(`ERROR: ${message}`)
  process.exit(1)
}

function argValue(name, fallback = '') {
  const idx = process.argv.findIndex((arg) => arg === name)
  if (idx < 0) return fallback
  return process.argv[idx + 1] ?? fallback
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function parseVersion(raw) {
  const m = String(raw || '').trim().replace(/^v/, '').match(/^(\d+)\.(\d+)\.(\d+)/)
  if (!m) return null
  return { major: Number(m[1]), minor: Number(m[2]), patch: Number(m[3]), raw: String(raw).trim() }
}

function compareVersion(left, right) {
  for (const key of ['major', 'minor', 'patch']) {
    if (left[key] !== right[key]) return left[key] < right[key] ? -1 : 1
  }
  return 0
}

function satisfiesComparator(version, comparator) {
  const clean = comparator.trim()
  if (!clean) return false
  if (clean.startsWith('>=')) {
    const minimum = parseVersion(clean.slice(2))
    return minimum != null && compareVersion(version, minimum) >= 0
  }
  if (clean.startsWith('^')) {
    const base = parseVersion(clean.slice(1))
    if (base == null) return false
    return version.major === base.major && compareVersion(version, base) >= 0
  }
  const exact = parseVersion(clean)
  return exact != null && compareVersion(version, exact) === 0
}

function satisfiesEngine(version, expression) {
  return String(expression || '')
    .split('||')
    .some((part) => satisfiesComparator(version, part))
}

function runVersion(command, args = ['--version']) {
  const result =
    process.platform === 'win32'
      ? spawnSync([command, ...args].join(' '), { encoding: 'utf8', shell: true })
      : spawnSync(command, args, { encoding: 'utf8', shell: false })
  if (result.error) fail(`failed to run ${command} ${args.join(' ')}: ${result.error.message}`)
  if ((result.status ?? 1) !== 0) {
    fail(`${command} ${args.join(' ')} exited with status ${result.status}: ${result.stderr || ''}`)
  }
  return String(result.stdout || '').trim()
}

function resolveNpmVersionText() {
  const userAgent = String(process.env.npm_config_user_agent || '').trim()
  const m = userAgent.match(/\bnpm\/([^\s]+)/)
  if (m?.[1]) return m[1]
  return runVersion(process.platform === 'win32' ? 'npm.cmd' : 'npm')
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true })
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8')
}

function resolveRepoRoot() {
  const cwd = process.cwd()
  if (fs.existsSync(path.join(cwd, 'src-tauri', 'src', 'lib.rs'))) return cwd
  const parent = path.resolve(cwd, '..')
  if (fs.existsSync(path.join(parent, 'src-tauri', 'src', 'lib.rs'))) return parent
  fail(`could not resolve repo root from ${cwd}`)
}

const repoRoot = resolveRepoRoot()
const webRoot = path.join(repoRoot, 'web')
const packageJsonPath = path.join(webRoot, 'package.json')
const packageLockPath = path.join(webRoot, 'package-lock.json')
const outPath = argValue('--out', path.join(repoRoot, 'output', 'qa', 'node-reproducibility.json'))

if (!fs.existsSync(packageJsonPath)) fail(`missing web package.json: ${packageJsonPath}`)
if (!fs.existsSync(packageLockPath)) fail(`missing web package-lock.json: ${packageLockPath}`)

const pkg = readJson(packageJsonPath)
const lock = readJson(packageLockPath)
const nodeVersion = parseVersion(process.version)
if (nodeVersion == null) fail(`could not parse Node version ${process.version}`)

const npmVersionText = resolveNpmVersionText()
const npmVersion = parseVersion(npmVersionText)
if (npmVersion == null) fail(`could not parse npm version ${npmVersionText}`)

const packageManager = String(pkg.packageManager || '').trim()
const pmMatch = packageManager.match(/^npm@(.+)$/)
if (!pmMatch) fail(`web/package.json packageManager must be pinned as npm@<version>, got ${JSON.stringify(packageManager)}`)
const expectedNpm = parseVersion(pmMatch[1])
if (expectedNpm == null) fail(`could not parse packageManager npm version ${packageManager}`)

const errors = []
const nodeEngine = String(pkg.engines?.node || '').trim()
if (!nodeEngine) {
  errors.push('web/package.json must declare engines.node.')
} else if (!satisfiesEngine(nodeVersion, nodeEngine)) {
  errors.push(`Node ${nodeVersion.raw} does not satisfy engines.node ${nodeEngine}.`)
}

if (compareVersion(npmVersion, expectedNpm) !== 0) {
  errors.push(`npm ${npmVersion.raw} does not match packageManager ${packageManager}.`)
}
if (String(lock.name || '') !== String(pkg.name || '')) {
  errors.push(`package-lock name ${JSON.stringify(lock.name)} does not match package.json name ${JSON.stringify(pkg.name)}.`)
}
if (String(lock.version || '') !== String(pkg.version || '')) {
  errors.push(
    `package-lock version ${JSON.stringify(lock.version)} does not match package.json version ${JSON.stringify(pkg.version)}.`,
  )
}
if (Number(lock.lockfileVersion) < 3) {
  errors.push(`package-lock lockfileVersion must be >= 3, got ${lock.lockfileVersion}.`)
}

const report = {
  schema_version: 1,
  generated_at_utc: new Date().toISOString(),
  status: errors.length === 0 ? 'passed' : 'failed',
  package_json: path.relative(repoRoot, packageJsonPath).replace(/\\/g, '/'),
  package_lock: path.relative(repoRoot, packageLockPath).replace(/\\/g, '/'),
  node: {
    version: nodeVersion.raw,
    engines_node: nodeEngine,
  },
  npm: {
    version: npmVersion.raw,
    package_manager: packageManager,
  },
  errors,
}
writeJson(outPath, report)

console.log(`Node/npm reproducibility: ${report.status}`)
console.log(`  node: ${nodeVersion.raw} (${nodeEngine})`)
console.log(`  npm: ${npmVersion.raw} (${packageManager})`)
console.log(`  report: ${outPath}`)

if (errors.length > 0) {
  for (const error of errors) console.error(`  - ${error}`)
  process.exit(1)
}
