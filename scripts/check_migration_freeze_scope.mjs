#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import path from 'node:path'

const ALLOWED_EXACT = new Set([
  'README.md',
  'RELEASE.md',
  'VUE_VITE_MIGRATION_MASTER_CHECKLIST.md',
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

function main() {
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

  console.error('Migration freeze scope check failed. Disallowed changed paths:')
  for (const p of blocked) console.error(`- ${p}`)
  console.error('')
  console.error('Allowed exact files:')
  for (const p of Array.from(ALLOWED_EXACT).sort()) console.error(`- ${p}`)
  console.error('Allowed prefixes:')
  for (const p of ALLOWED_PREFIXES) console.error(`- ${p}`)
  process.exit(1)
}

main()
