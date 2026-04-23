#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(SCRIPT_DIR, '..')

function argValue(name, fallback = '') {
  const idx = process.argv.findIndex((arg) => arg === name)
  if (idx < 0) return fallback
  return process.argv[idx + 1] ?? fallback
}

function fail(message, exitCode = 1) {
  console.error(`ERROR: ${message}`)
  process.exit(exitCode)
}

function rootPath(relPath) {
  return path.resolve(ROOT, String(relPath || '').trim())
}

function inputPath(argPath, fallbackRel) {
  const raw = String(argPath || fallbackRel || '').trim()
  if (!raw) return rootPath(fallbackRel)
  if (path.isAbsolute(raw)) return raw
  return path.resolve(process.cwd(), raw)
}

function ensureParent(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true })
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function runOrFail(command, args, options = {}) {
  const pretty = [command, ...args].join(' ')
  console.log(`> ${pretty}`)
  const result = spawnSync(command, args, {
    cwd: ROOT,
    stdio: 'inherit',
    shell: false,
    ...options,
  })
  if (result.error) fail(`${pretty} failed: ${result.error.message}`)
  if ((result.status ?? 1) !== 0) fail(`${pretty} exited with status ${result.status}`, result.status ?? 1)
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/')
}

function buildCargoArgs(pack, reportJsonPath, reportXlsxPath) {
  const buildProfile = String(pack.build_profile || 'dev').trim().toLowerCase()
  const cargoArgs = ['run']
  if (buildProfile === 'release') cargoArgs.push('--release')
  cargoArgs.push('-p', String(pack.package || 'ta_desktop_demo_app'))
  cargoArgs.push('--bin', String(pack.bin || 'ta_headless_validate'))
  cargoArgs.push('--')
  const cliArgs = Array.isArray(pack.args) ? pack.args.map((value) => String(value)) : []
  cargoArgs.push(...cliArgs)
  cargoArgs.push('--out-json', rel(reportJsonPath), '--out-xlsx', rel(reportXlsxPath))
  return cargoArgs
}

function main() {
  const packArg = argValue('--pack', 'scripts/headless_output_parity_pack.json')
  const pairsArg = argValue('--pairs', 'scripts/output_parity_pairs.json')
  const packPath = inputPath(packArg, 'scripts/headless_output_parity_pack.json')
  const pairsPath = inputPath(pairsArg, 'scripts/output_parity_pairs.json')

  if (!fs.existsSync(packPath)) fail(`Headless parity pack not found: ${packPath}`)
  if (!fs.existsSync(pairsPath)) fail(`Parity pairs file not found: ${pairsPath}`)

  const pack = readJson(packPath)
  const pairsConfig = readJson(pairsPath)
  const outputs = pack.outputs || {}
  const reportJsonPath = rootPath(outputs.report_json)
  const reportXlsxPath = rootPath(outputs.report_xlsx)
  ensureParent(reportJsonPath)
  ensureParent(reportXlsxPath)

  const cargoArgs = buildCargoArgs(pack, reportJsonPath, reportXlsxPath)
  runOrFail('cargo', cargoArgs)

  if (!fs.existsSync(reportJsonPath)) fail(`Expected current JSON output was not produced: ${reportJsonPath}`)
  if (!fs.existsSync(reportXlsxPath)) fail(`Expected current XLSX output was not produced: ${reportXlsxPath}`)

  const baseDir = path.resolve(path.dirname(pairsPath), String(pairsConfig.base_dir || '.'))
  const pairs = Array.isArray(pairsConfig.pairs) ? pairsConfig.pairs : []
  if (pairs.length === 0) fail(`No output parity pairs found in ${pairsPath}`)

  const refreshedPairs = []
  for (const pair of pairs) {
    const name = String(pair.name || '').trim() || 'unnamed'
    const baselinePath = path.resolve(baseDir, String(pair.baseline || '').trim())
    const candidatePath = path.resolve(baseDir, String(pair.candidate || '').trim())
    if (!fs.existsSync(candidatePath)) {
      fail(`Candidate file for ${name} does not exist: ${candidatePath}`)
    }
    ensureParent(baselinePath)
    fs.copyFileSync(candidatePath, baselinePath)
    refreshedPairs.push({
      name,
      baseline: rel(baselinePath),
      candidate: rel(candidatePath),
      size_bytes: fs.statSync(baselinePath).size,
    })
  }

  const manifestPath = rootPath('scripts/baselines/samples/output_parity_refresh_manifest.json')
  const manifest = {
    schema_version: 1,
    generated_at_utc: new Date().toISOString(),
    pack_path: rel(packPath),
    pairs_path: rel(pairsPath),
    command: {
      executable: 'cargo',
      args: cargoArgs,
    },
    refreshed_pairs: refreshedPairs,
  }
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8')
  console.log(`Wrote refresh manifest: ${rel(manifestPath)}`)

  runOrFail('node', [rel(rootPath('scripts/check_output_parity.mjs')), '--pairs', rel(pairsPath)])
  runOrFail('node', [rel(rootPath('scripts/export_baseline_samples.mjs'))])
  console.log(`Refreshed ${refreshedPairs.length} output parity baseline file(s).`)
}

main()
