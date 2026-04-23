#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

function argValue(name, fallback = '') {
  const idx = process.argv.findIndex((arg) => arg === name)
  if (idx < 0) return fallback
  return process.argv[idx + 1] ?? fallback
}

function fail(message) {
  console.error(`ERROR: ${message}`)
  process.exit(1)
}

function normalizePath(root, filePath) {
  return path.resolve(root, String(filePath || '').trim())
}

function parseJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function formatPath(base, key) {
  if (typeof key === 'number') return `${base}[${key}]`
  return `${base}.${key}`
}

function compareExactJson(baseline, candidate, atPath, ignored, out) {
  if (ignored.has(atPath)) return

  if (baseline === null || candidate === null) {
    if (baseline !== candidate) out.push(`${atPath}: value mismatch (${JSON.stringify(baseline)} vs ${JSON.stringify(candidate)})`)
    return
  }

  if (Array.isArray(baseline) || Array.isArray(candidate)) {
    if (!Array.isArray(baseline) || !Array.isArray(candidate)) {
      out.push(`${atPath}: type mismatch (${Array.isArray(baseline) ? 'array' : typeof baseline} vs ${Array.isArray(candidate) ? 'array' : typeof candidate})`)
      return
    }
    if (baseline.length !== candidate.length) {
      out.push(`${atPath}: array length mismatch (${baseline.length} vs ${candidate.length})`)
      return
    }
    baseline.forEach((value, idx) => {
      compareExactJson(value, candidate[idx], formatPath(atPath, idx), ignored, out)
    })
    return
  }

  const baselineType = typeof baseline
  const candidateType = typeof candidate
  if (baselineType !== candidateType) {
    out.push(`${atPath}: type mismatch (${baselineType} vs ${candidateType})`)
    return
  }

  if (baselineType === 'object') {
    const baselineKeys = Object.keys(baseline)
    const candidateKeys = Object.keys(candidate)
    const filteredBaselineKeys = baselineKeys.filter((key) => !ignored.has(formatPath(atPath, key)))
    const filteredCandidateKeys = candidateKeys.filter((key) => !ignored.has(formatPath(atPath, key)))
    const missingKeys = filteredBaselineKeys.filter((key) => !candidateKeys.includes(key))
    const extraKeys = filteredCandidateKeys.filter((key) => !baselineKeys.includes(key))
    missingKeys.forEach((key) => out.push(`${formatPath(atPath, key)}: missing key in candidate`))
    extraKeys.forEach((key) => out.push(`${formatPath(atPath, key)}: unexpected key in candidate`))
    filteredBaselineKeys
      .filter((key) => candidateKeys.includes(key))
      .forEach((key) => compareExactJson(baseline[key], candidate[key], formatPath(atPath, key), ignored, out))
    return
  }

  if (baseline !== candidate) {
    out.push(`${atPath}: value mismatch (${JSON.stringify(baseline)} vs ${JSON.stringify(candidate)})`)
  }
}

function comparePair(rootDir, pair) {
  const name = String(pair.name || '').trim() || 'unnamed'
  const baselinePath = normalizePath(rootDir, pair.baseline)
  const candidatePath = normalizePath(rootDir, pair.candidate)
  if (!fs.existsSync(baselinePath)) return [`${name}: missing baseline file ${baselinePath}`]
  if (!fs.existsSync(candidatePath)) return [`${name}: missing candidate file ${candidatePath}`]

  const ignored = new Set(
    (Array.isArray(pair.ignore_paths) ? pair.ignore_paths : [])
      .map((value) => String(value || '').trim())
      .filter(Boolean),
  )
  const failures = []
  compareExactJson(parseJson(baselinePath), parseJson(candidatePath), '$', ignored, failures)
  return failures.map((failure) => `${name}: ${failure}`)
}

function main() {
  const rootDir = process.cwd()
  const pairsArg = argValue('--pairs', 'scripts/output_parity_semantic_pairs.json')
  const pairsPath = normalizePath(rootDir, pairsArg)
  if (!fs.existsSync(pairsPath)) fail(`Pairs file not found: ${pairsPath}`)

  const config = JSON.parse(fs.readFileSync(pairsPath, 'utf8'))
  const pairsBaseDir = normalizePath(path.dirname(pairsPath), String(config.base_dir || '.'))
  const pairs = Array.isArray(config.pairs) ? config.pairs : []
  if (pairs.length === 0) fail(`No semantic parity pairs found in ${pairsPath}`)

  const failures = []
  for (const pair of pairs) {
    failures.push(...comparePair(pairsBaseDir, pair))
  }

  if (failures.length > 0) {
    console.error('Output semantic parity check failed:')
    for (const item of failures) console.error(`  - ${item}`)
    process.exit(1)
  }

  console.log(`Output semantic parity check passed for ${pairs.length} pair(s).`)
}

main()
