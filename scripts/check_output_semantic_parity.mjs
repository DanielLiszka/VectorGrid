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

function escapeRegexChar(char) {
  return /[\\^$.*+?()[\]{}|]/.test(char) ? `\\${char}` : char
}

function tolerancePathToRegex(pattern) {
  const text = String(pattern || '').trim()
  let out = ''
  for (let i = 0; i < text.length; i += 1) {
    if (text.slice(i, i + 3) === '[*]') {
      out += '\\[\\d+\\]'
      i += 2
    } else {
      out += escapeRegexChar(text[i])
    }
  }
  return new RegExp(`^${out}$`)
}

function parseToleranceRules(pair) {
  const rawRules = Array.isArray(pair.numeric_tolerances) ? pair.numeric_tolerances : []
  return rawRules
    .filter((rule) => rule && typeof rule === 'object' && rule.enabled !== false)
    .map((rule) => {
      const pathExpr = String(rule.path || '').trim()
      const abs = Number(rule.abs ?? 0)
      const rel = Number(rule.rel ?? 0)
      if (!pathExpr || !Number.isFinite(abs) || !Number.isFinite(rel) || abs < 0 || rel < 0) {
        return null
      }
      return {
        path: pathExpr,
        regex: tolerancePathToRegex(pathExpr),
        abs,
        rel,
      }
    })
    .filter(Boolean)
}

function numericToleranceFailure(baseline, candidate, atPath, toleranceRules) {
  if (typeof baseline !== 'number' || typeof candidate !== 'number') return null
  const rule = toleranceRules.find((candidateRule) => candidateRule.regex.test(atPath))
  if (!rule) return null
  const diff = Math.abs(baseline - candidate)
  const allowed = Math.max(rule.abs, rule.rel * Math.abs(baseline))
  if (diff <= allowed) return ''
  return `${atPath}: numeric tolerance mismatch (${baseline} vs ${candidate}; diff ${diff} > ${allowed} for ${rule.path})`
}

function compareExactJson(baseline, candidate, atPath, ignored, toleranceRules, out) {
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
      compareExactJson(value, candidate[idx], formatPath(atPath, idx), ignored, toleranceRules, out)
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
      .forEach((key) => compareExactJson(baseline[key], candidate[key], formatPath(atPath, key), ignored, toleranceRules, out))
    return
  }

  if (baseline !== candidate) {
    const toleranceFailure = numericToleranceFailure(baseline, candidate, atPath, toleranceRules)
    if (toleranceFailure === '') return
    if (toleranceFailure) {
      out.push(toleranceFailure)
      return
    }
    out.push(`${atPath}: value mismatch (${JSON.stringify(baseline)} vs ${JSON.stringify(candidate)})`)
  }
}

function parseCsvRecords(text) {
  const records = []
  let record = []
  let field = ''
  let inQuotes = false

  for (let idx = 0; idx < text.length; idx += 1) {
    const ch = text[idx]
    if (ch === '"') {
      if (inQuotes && text[idx + 1] === '"') {
        field += '"'
        idx += 1
      } else {
        inQuotes = !inQuotes
      }
      continue
    }
    if (ch === ',' && !inQuotes) {
      record.push(field)
      field = ''
      continue
    }
    if ((ch === '\n' || ch === '\r') && !inQuotes) {
      if (ch === '\r' && text[idx + 1] === '\n') idx += 1
      record.push(field)
      if (record.some((value) => value.length > 0)) records.push(record)
      record = []
      field = ''
      continue
    }
    field += ch
  }

  if (inQuotes) throw new Error('unterminated quoted CSV field')
  if (field.length > 0 || record.length > 0) {
    record.push(field)
    if (record.some((value) => value.length > 0)) records.push(record)
  }
  return records
}

function parseCsvObjects(filePath) {
  const records = parseCsvRecords(fs.readFileSync(filePath, 'utf8'))
  if (records.length === 0) throw new Error('CSV output is empty')
  const headers = records[0]
  const duplicateHeaders = headers.filter((header, idx) => headers.indexOf(header) !== idx)
  if (duplicateHeaders.length > 0) {
    throw new Error(`duplicate CSV headers: ${[...new Set(duplicateHeaders)].join(', ')}`)
  }
  return records.slice(1).map((record, idx) => {
    if (record.length !== headers.length) {
      throw new Error(
        `CSV row ${idx + 2} has ${record.length} columns; expected ${headers.length}`,
      )
    }
    return Object.fromEntries(headers.map((header, colIdx) => [header, record[colIdx]]))
  })
}

function jsonPathValue(value, pathParts) {
  let current = value
  for (const part of pathParts) {
    if (current === null || current === undefined || typeof current !== 'object') return undefined
    current = current[part]
  }
  return current
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  if (value && typeof value === 'object') {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(',')}}`
  }
  return JSON.stringify(value)
}

function numericValuesMatch(expected, actual, tolerance) {
  const diff = Math.abs(expected - actual)
  const allowed = Math.max(tolerance.abs, tolerance.rel * Math.abs(expected))
  return { matches: diff <= allowed, diff, allowed }
}

const VALIDATION_CSV_FIELD_MAPPINGS = [
  ['robust_score', ['robust_score']],
  ['psr', ['psr']],
  ['dsr', ['dsr']],
  ['track_record_bars', ['track_record_bars']],
  ['min_track_record_length_bars', ['min_track_record_length_bars']],
  ['in_sample_pnl', ['in_sample', 'pnl']],
  ['in_sample_sharpe', ['in_sample', 'sharpe']],
  ['in_sample_max_dd', ['in_sample', 'max_dd']],
  ['in_sample_trades', ['in_sample', 'trades']],
  ['wf_sharpe_median', ['walk_forward', 'sharpe_median']],
  ['wf_dd_worst', ['walk_forward', 'max_dd_worst']],
  ['cscv_selected_frac', ['cscv_pbo', 'is_selected_frac']],
  ['cscv_oos_rank_frac_p50', ['cscv_pbo', 'oos_rank_frac_p50']],
  ['cscv_oos_metric_p50', ['cscv_pbo', 'oos_metric_p50']],
  ['cross_score', ['cross_ticker', 'score']],
  ['cross_pareto', ['cross_ticker_pareto']],
  ['perturb_pnl_mean', ['perturb', 'pnl_mean']],
]

function compareMappedCsvValue(row, candidate, csvField, jsonPath, tolerance, out, rowNumber) {
  const expected = jsonPathValue(candidate, jsonPath)
  const actualText = String(row[csvField] ?? '')
  const location = `CSV row ${rowNumber} ${csvField}`

  if (expected === null || expected === undefined) {
    if (actualText !== '') {
      out.push(`${location}: expected an empty cell because JSON ${jsonPath.join('.')} is absent`)
    }
    return
  }

  if (typeof expected === 'number') {
    if (actualText === '') {
      out.push(`${location}: missing numeric value ${expected}`)
      return
    }
    const actual = Number(actualText)
    if (!Number.isFinite(actual)) {
      out.push(`${location}: ${JSON.stringify(actualText)} is not a finite number`)
      return
    }
    const { matches, diff, allowed } = numericValuesMatch(expected, actual, tolerance)
    if (!matches) {
      out.push(
        `${location}: numeric mismatch (${expected} vs ${actual}; diff ${diff} > ${allowed})`,
      )
    }
    return
  }

  if (typeof expected === 'boolean') {
    const actual = actualText === '1' ? true : actualText === '0' ? false : null
    if (actual !== expected) {
      out.push(`${location}: boolean mismatch (${expected} vs ${JSON.stringify(actualText)})`)
    }
    return
  }

  if (actualText !== String(expected)) {
    out.push(`${location}: value mismatch (${JSON.stringify(expected)} vs ${JSON.stringify(actualText)})`)
  }
}

function compareValidationCsvToJson(rootDir, pair) {
  const jsonPath = normalizePath(rootDir, pair.json)
  const csvPath = normalizePath(rootDir, pair.csv)
  if (!fs.existsSync(jsonPath)) return [`missing JSON file ${jsonPath}`]
  if (!fs.existsSync(csvPath)) return [`missing CSV file ${csvPath}`]

  try {
    const report = parseJson(jsonPath)
    const candidatePath = Array.isArray(pair.json_candidates_path)
      ? pair.json_candidates_path.map((part) => String(part))
      : ['validation', 'res', 'candidates']
    const candidates = jsonPathValue(report, candidatePath)
    if (!Array.isArray(candidates)) {
      return [`JSON ${candidatePath.join('.')} is not an array`]
    }

    const rows = parseCsvObjects(csvPath)
    const failures = []
    if (rows.length !== candidates.length) {
      failures.push(`candidate count mismatch (JSON ${candidates.length} vs CSV ${rows.length})`)
    }

    const candidatesByParams = new Map()
    for (const candidate of candidates) {
      const key = canonicalJson(candidate?.params ?? {})
      const matching = candidatesByParams.get(key) ?? []
      matching.push(candidate)
      candidatesByParams.set(key, matching)
    }

    const rawTolerance = pair.numeric_tolerance ?? {}
    const tolerance = {
      abs: Number(rawTolerance.abs ?? 0),
      rel: Number(rawTolerance.rel ?? 0),
    }
    if (
      !Number.isFinite(tolerance.abs) ||
      !Number.isFinite(tolerance.rel) ||
      tolerance.abs < 0 ||
      tolerance.rel < 0
    ) {
      return ['numeric_tolerance must contain non-negative finite abs and rel values']
    }

    rows.forEach((row, idx) => {
      const rowNumber = idx + 2
      if (row.rank !== String(idx + 1)) {
        failures.push(`CSV row ${rowNumber} rank: expected ${idx + 1}, found ${JSON.stringify(row.rank)}`)
      }

      let params
      try {
        params = JSON.parse(String(row.params_json ?? ''))
      } catch (error) {
        failures.push(`CSV row ${rowNumber} params_json is invalid JSON: ${error.message}`)
        return
      }

      const key = canonicalJson(params)
      const matches = candidatesByParams.get(key) ?? []
      const candidate = matches.shift()
      if (!candidate) {
        failures.push(`CSV row ${rowNumber} params_json has no matching JSON candidate`)
        return
      }

      for (const [csvField, candidatePathParts] of VALIDATION_CSV_FIELD_MAPPINGS) {
        compareMappedCsvValue(
          row,
          candidate,
          csvField,
          candidatePathParts,
          tolerance,
          failures,
          rowNumber,
        )
      }
    })

    const unmatched = [...candidatesByParams.values()].reduce((count, values) => count + values.length, 0)
    if (unmatched > 0) failures.push(`${unmatched} JSON candidate(s) were not represented in CSV`)
    return failures
  } catch (error) {
    return [`could not compare validation CSV to JSON: ${error.message}`]
  }
}

function compareExactJsonPair(rootDir, pair) {
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
  const toleranceRules = parseToleranceRules(pair)
  const failures = []
  compareExactJson(
    parseJson(baselinePath),
    parseJson(candidatePath),
    '$',
    ignored,
    toleranceRules,
    failures,
  )
  return failures.map((failure) => `${name}: ${failure}`)
}

function comparePair(rootDir, pair) {
  const name = String(pair.name || '').trim() || 'unnamed'
  const kind = String(pair.kind || 'exact_json').trim()
  if (kind === 'exact_json') return compareExactJsonPair(rootDir, pair)
  if (kind === 'validation_shortlist_csv_vs_json') {
    return compareValidationCsvToJson(rootDir, pair).map((failure) => `${name}: ${failure}`)
  }
  return [`${name}: unsupported semantic parity kind ${JSON.stringify(kind)}`]
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
