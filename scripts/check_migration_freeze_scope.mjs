#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import path from 'node:path'

// Narrow helper for explicitly documented migration/stabilization windows only.
// Keep the allowlist aligned with the matching repo notes when this guard is used.

const ALLOWED_EXACT = new Set([
  'README.md',
  'RELEASE.md',
  'src-tauri/tauri.conf.json',
  'tauri.conf.json',
  'scripts/check_output_parity.mjs',
  'scripts/output_parity_pairs.json',
  'scripts/check_migration_freeze_scope.mjs',
  'scripts/export_baseline_samples.mjs',
  'scripts/capture_optimization_memory.ps1',
  'scripts/post_release_telemetry_review.mjs',
])

const ALLOWED_PREFIXES = ['web/', 'scripts/baselines/']
const USAGE_NOTE = [
  'Migration freeze scope helper:',
  '- use this only during an explicitly declared migration or stabilization window',
  '- keep allowlist changes paired with the matching human-readable note in RELEASE.md/REPO_MAP.md',
  '- this is not a general repo-wide gate',
].join('\n')

function asRepoPath(p) {
  return p.replace(/\\/g, '/')
}

function changedPathsFromStatus() {
  const out = execFileSync('git', ['status', '--porcelain'], { encoding: 'utf8' })
  const lines = out.split(/\r?\n/).map((s) => s.trimEnd()).filter(Boolean)
  const paths = []
  for (const line of lines) {
    const raw = line.slice(3).trim()
    if (!raw) continue
    // Handle rename entries: "old -> new"
    const parts = raw.split(' -> ')
    const last = parts[parts.length - 1]
    if (!last) continue
    paths.push(asRepoPath(path.normalize(last)))
  }
  return Array.from(new Set(paths)).sort()
}

function isAllowed(p) {
  if (ALLOWED_EXACT.has(p)) return true
  return ALLOWED_PREFIXES.some((prefix) => p.startsWith(prefix))
}

function printScope() {
  console.log(USAGE_NOTE)
  console.log('')
  console.log('Allowed exact files:')
  for (const p of Array.from(ALLOWED_EXACT).sort()) console.log(`- ${p}`)
  console.log('Allowed prefixes:')
  for (const p of ALLOWED_PREFIXES) console.log(`- ${p}`)
}

function main() {
  if (process.argv.includes('--help') || process.argv.includes('-h') || process.argv.includes('--print-scope')) {
    printScope()
    return
  }

  const changed = changedPathsFromStatus()
  if (changed.length === 0) {
    console.log('Migration freeze scope check: no local changes detected.')
    return
  }

  const blocked = changed.filter((p) => !isAllowed(p))
  if (blocked.length === 0) {
    console.log(`Migration freeze scope check passed (${changed.length} changed path(s) in approved scope).`)
    return
  }

  console.error(USAGE_NOTE)
  console.error('')
  console.error('Migration freeze scope check failed. Disallowed changed paths:')
  for (const p of blocked) console.error(`- ${p}`)
  console.error('')
  printScope()
  process.exit(1)
}

main()
