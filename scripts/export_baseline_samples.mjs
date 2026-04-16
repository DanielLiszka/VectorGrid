#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'

const ROOT = process.cwd()
const OUT_DIR = path.resolve(ROOT, 'scripts', 'baselines', 'samples')

const RUN_ARTIFACT_CANDIDATES = [
  path.resolve(ROOT, '_write_test', 'cli_runs', 'AAPL_opt.json'),
  path.resolve(ROOT, 'scripts', 'baselines', 'samples', 'run_artifact.json'),
]

const REPORT_JSON_CANDIDATES = [
  path.resolve(ROOT, '_write_test', 'headless_reports', 'AAPL_headless_test.json'),
  path.resolve(ROOT, 'scripts', 'baselines', 'samples', 'report.json'),
]

const REPORT_XLSX_CANDIDATES = [
  path.resolve(ROOT, '_write_test', 'headless_reports', 'AAPL_headless_test.xlsx'),
  path.resolve(ROOT, 'scripts', 'baselines', 'samples', 'report.xlsx'),
]

const BATCH_SUMMARY_CANDIDATES = [
  path.resolve(ROOT, 'output', 'benchmarks', 'workflow_regression_summary.json'),
  path.resolve(ROOT, 'scripts', 'baselines', 'workflow_regression_baseline.json'),
]

function mustReadJson(file) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing required source file: ${path.relative(ROOT, file)}`)
  }
  return JSON.parse(fs.readFileSync(file, 'utf8'))
}

function escapeCsvCell(value) {
  const text = String(value ?? '')
  if (text.includes('"') || text.includes(',') || text.includes('\n') || text.includes('\r')) {
    return `"${text.replace(/"/g, '""')}"`
  }
  return text
}

function writeCsv(file, rows, headers) {
  const lines = [headers.join(',')]
  for (const row of rows) {
    const values = headers.map((h) => escapeCsvCell(row[h]))
    lines.push(values.join(','))
  }
  fs.writeFileSync(file, `${lines.join('\n')}\n`, 'utf8')
}

function bestExisting(paths) {
  for (const p of paths) {
    if (fs.existsSync(p)) return p
  }
  return null
}

function copyFile(src, dst) {
  fs.copyFileSync(src, dst)
}

function exportTopRows(runArtifact, outFile) {
  const top = Array.isArray(runArtifact.top) ? runArtifact.top : []
  const rows = top.slice(0, 100).map((entry, idx) => {
    const params = entry?.[0] ?? {}
    const metrics = entry?.[1] ?? {}
    return {
      rank: idx + 1,
      fast_len: params.fast_len ?? '',
      slow_len: params.slow_len ?? '',
      fast_ma_type: params.fast_ma_type ?? '',
      slow_ma_type: params.slow_ma_type ?? '',
      pnl: metrics.pnl ?? '',
      sharpe: metrics.sharpe ?? '',
      max_dd: metrics.max_dd ?? '',
      trades: metrics.trades ?? '',
      exposure: metrics.exposure ?? '',
      net_exposure: metrics.net_exposure ?? '',
    }
  })
  writeCsv(outFile, rows, [
    'rank',
    'fast_len',
    'slow_len',
    'fast_ma_type',
    'slow_ma_type',
    'pnl',
    'sharpe',
    'max_dd',
    'trades',
    'exposure',
    'net_exposure',
  ])
  return rows.length
}

function exportValidationShortlist(reportJson, outFile) {
  const candidates = reportJson?.validation?.res?.candidates
  const rows = Array.isArray(candidates)
    ? candidates.map((candidate, idx) => {
        const params = candidate?.params ?? {}
        const inSample = candidate?.in_sample ?? {}
        const walkForward = candidate?.walk_forward ?? {}
        const cscv = candidate?.cscv_pbo ?? {}
        return {
          rank: idx + 1,
          fast_len: params.fast_len ?? '',
          slow_len: params.slow_len ?? '',
          fast_ma_type: params.fast_ma_type ?? '',
          slow_ma_type: params.slow_ma_type ?? '',
          pnl: inSample.pnl ?? '',
          sharpe: inSample.sharpe ?? '',
          max_dd: inSample.max_dd ?? '',
          trades: inSample.trades ?? '',
          robust_score: candidate?.robust_score ?? '',
          cscv_pbo_is_selected_frac: cscv.is_selected_frac ?? '',
          wf_pnl_median: walkForward.pnl_median ?? '',
          wf_sharpe_median: walkForward.sharpe_median ?? '',
          wf_max_dd_median: walkForward.max_dd_median ?? '',
        }
      })
    : []

  writeCsv(outFile, rows, [
    'rank',
    'fast_len',
    'slow_len',
    'fast_ma_type',
    'slow_ma_type',
    'pnl',
    'sharpe',
    'max_dd',
    'trades',
    'robust_score',
    'cscv_pbo_is_selected_frac',
    'wf_pnl_median',
    'wf_sharpe_median',
    'wf_max_dd_median',
  ])
  return rows.length
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true })
}

function rel(p) {
  return path.relative(ROOT, p).replace(/\\/g, '/')
}

function main() {
  ensureDir(OUT_DIR)

  const runArtifactSrc = bestExisting(RUN_ARTIFACT_CANDIDATES)
  const reportJsonSrc = bestExisting(REPORT_JSON_CANDIDATES)
  const reportXlsxSrc = bestExisting(REPORT_XLSX_CANDIDATES)
  if (!runArtifactSrc) {
    throw new Error(
      `Missing run artifact source. Expected one of: ${RUN_ARTIFACT_CANDIDATES.map(rel).join(', ')}`,
    )
  }
  if (!reportJsonSrc) {
    throw new Error(
      `Missing report JSON source. Expected one of: ${REPORT_JSON_CANDIDATES.map(rel).join(', ')}`,
    )
  }
  if (!reportXlsxSrc) {
    throw new Error(
      `Missing report XLSX source. Expected one of: ${REPORT_XLSX_CANDIDATES.map(rel).join(', ')}`,
    )
  }

  const runArtifact = mustReadJson(runArtifactSrc)
  const reportJson = mustReadJson(reportJsonSrc)
  const batchSummarySrc = bestExisting(BATCH_SUMMARY_CANDIDATES)
  if (!batchSummarySrc) {
    throw new Error(
      `Missing batch summary source. Expected one of: ${BATCH_SUMMARY_CANDIDATES.map(rel).join(', ')}`,
    )
  }
  const outTopRows = path.resolve(OUT_DIR, 'top_rows.csv')
  const outValidationShortlist = path.resolve(OUT_DIR, 'validation_shortlist.csv')
  const outRunArtifact = path.resolve(OUT_DIR, 'run_artifact.json')
  const outBatchSummary = path.resolve(OUT_DIR, 'batch_summary.json')
  const outReportJson = path.resolve(OUT_DIR, 'report.json')
  const outReportXlsx = path.resolve(OUT_DIR, 'report.xlsx')

  const topCount = exportTopRows(runArtifact, outTopRows)
  const shortlistCount = exportValidationShortlist(reportJson, outValidationShortlist)
  copyFile(runArtifactSrc, outRunArtifact)
  copyFile(batchSummarySrc, outBatchSummary)
  copyFile(reportJsonSrc, outReportJson)
  copyFile(reportXlsxSrc, outReportXlsx)

  const manifest = {
    generated_at: new Date().toISOString(),
    outputs: {
      top_rows_csv: rel(outTopRows),
      validation_shortlist_csv: rel(outValidationShortlist),
      run_artifact_json: rel(outRunArtifact),
      batch_summary_json: rel(outBatchSummary),
      report_json: rel(outReportJson),
      report_xlsx: rel(outReportXlsx),
    },
    counts: {
      top_rows: topCount,
      validation_shortlist_rows: shortlistCount,
    },
    sources: {
      run_artifact: rel(runArtifactSrc),
      batch_summary: rel(batchSummarySrc),
      report_json: rel(reportJsonSrc),
      report_xlsx: rel(reportXlsxSrc),
    },
  }

  const manifestPath = path.resolve(OUT_DIR, 'manifest.json')
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8')

  console.log(`Exported baseline samples to ${rel(OUT_DIR)}`)
  console.log(`- top_rows.csv (${topCount} rows)`)
  console.log(`- validation_shortlist.csv (${shortlistCount} rows)`)
  console.log(`- run_artifact.json`)
  console.log(`- batch_summary.json`)
  console.log(`- report.json`)
  console.log(`- report.xlsx`)
  console.log(`- manifest.json`)
}

main()
