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

function runGit(args) {
  const result = spawnSync('git', args, { encoding: 'utf8', shell: false })
  if (result.error) fail(`git ${args.join(' ')} failed: ${result.error.message}`)
  if ((result.status ?? 1) !== 0) {
    fail(`git ${args.join(' ')} exited with status ${result.status}: ${result.stderr || ''}`)
  }
  return String(result.stdout || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true })
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8')
}

function isAllowedExampleEnv(base) {
  return /^\.(env|npmrc)\.(example|sample|template)$/i.test(base) || /^example\.(env|npmrc)$/i.test(base)
}

function classifyPath(relPath) {
  const normalized = relPath.replace(/\\/g, '/')
  const base = path.basename(normalized)
  const lower = base.toLowerCase()
  if (isAllowedExampleEnv(lower)) return null
  if (lower === '.env' || lower.startsWith('.env.')) return 'environment_file'
  if (lower === '.npmrc') return 'npm_auth_config'
  if (/^(id_rsa|id_dsa|id_ecdsa|id_ed25519)(\.pub)?$/i.test(base)) return 'ssh_key_material'
  if (/\.(pfx|p12|pem|key|cer|crt)$/i.test(base)) return 'certificate_or_key_material'
  return null
}

function scanPrivateKeyMarker(repoRoot, relPath) {
  const filePath = path.join(repoRoot, relPath)
  let stat
  try {
    stat = fs.statSync(filePath)
  } catch {
    return null
  }
  if (!stat.isFile() || stat.size > 2_000_000) return null
  let text
  try {
    text = fs.readFileSync(filePath, 'utf8')
  } catch {
    return null
  }
  const fence = '-'.repeat(5)
  const privateKeyPattern = new RegExp(
    `${fence}BEGIN (RSA |DSA |EC |OPENSSH |ENCRYPTED )?PRIVATE KEY${fence}`,
  )
  if (privateKeyPattern.test(text)) {
    return 'private_key_marker'
  }
  if (text.includes(`${fence}BEGIN PKCS12${fence}`)) {
    return 'pkcs12_marker'
  }
  return null
}

function resolveRepoRoot() {
  const cwd = process.cwd()
  if (fs.existsSync(path.join(cwd, 'src-tauri', 'src', 'lib.rs'))) return cwd
  const parent = path.resolve(cwd, '..')
  if (fs.existsSync(path.join(parent, 'src-tauri', 'src', 'lib.rs'))) return parent
  fail(`could not resolve repo root from ${cwd}`)
}

const repoRoot = resolveRepoRoot()
const outPath = argValue('--out', path.join(repoRoot, 'output', 'qa', 'release-secret-hygiene.json'))
process.chdir(repoRoot)
const tracked = runGit(['ls-files'])
const unignoredUntracked = runGit(['ls-files', '--others', '--exclude-standard'])

const findings = []
for (const [scope, files] of [
  ['tracked', tracked],
  ['unignored_untracked', unignoredUntracked],
]) {
  for (const relPath of files) {
    const pathReason = classifyPath(relPath)
    if (pathReason) findings.push({ scope, path: relPath, reason: pathReason })
    const contentReason = scanPrivateKeyMarker(repoRoot, relPath)
    if (contentReason) findings.push({ scope, path: relPath, reason: contentReason })
  }
}

const report = {
  schema_version: 1,
  generated_at_utc: new Date().toISOString(),
  status: findings.length === 0 ? 'passed' : 'failed',
  scanned: {
    tracked: tracked.length,
    unignored_untracked: unignoredUntracked.length,
  },
  findings,
}
writeJson(outPath, report)

console.log(`Release secret hygiene: ${report.status}`)
console.log(`  tracked files: ${tracked.length}`)
console.log(`  unignored untracked files: ${unignoredUntracked.length}`)
console.log(`  report: ${outPath}`)

if (findings.length > 0) {
  console.error('Potential release-secret files were found. Move them outside the repo or add an explicit non-secret fixture naming pattern.')
  for (const finding of findings) console.error(`  - ${finding.scope}: ${finding.path} (${finding.reason})`)
  process.exit(1)
}
