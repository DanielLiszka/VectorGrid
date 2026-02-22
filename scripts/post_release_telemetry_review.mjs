#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'

const ROOT = process.cwd()

const DEFAULTS = {
  baseline: path.resolve(ROOT, 'scripts', 'baselines', 'workflow_regression_baseline.json'),
  summary: path.resolve(ROOT, 'target', 'workflow_regression_summary.json'),
  uiMetrics: path.resolve(ROOT, 'scripts', 'baselines', 'ui_capture', 'perf_metrics.json'),
  memoryMetrics: path.resolve(ROOT, 'scripts', 'baselines', 'ui_capture', 'optimization_memory_metrics.json'),
  samplesManifest: path.resolve(ROOT, 'scripts', 'baselines', 'samples', 'manifest.json'),
  outDir: path.resolve(ROOT, 'scripts', 'baselines', 'post_release'),
}

function parseArgs(argv) {
  const args = { ...DEFAULTS }
  for (let i = 2; i < argv.length; i += 1) {
    const a = argv[i]
    const next = argv[i + 1]
    if (a === '--baseline' && next) {
      args.baseline = path.resolve(ROOT, next)
      i += 1
      continue
    }
    if (a === '--summary' && next) {
      args.summary = path.resolve(ROOT, next)
      i += 1
      continue
    }
    if (a === '--ui-metrics' && next) {
      args.uiMetrics = path.resolve(ROOT, next)
      i += 1
      continue
    }
    if (a === '--memory-metrics' && next) {
      args.memoryMetrics = path.resolve(ROOT, next)
      i += 1
      continue
    }
    if (a === '--samples-manifest' && next) {
      args.samplesManifest = path.resolve(ROOT, next)
      i += 1
      continue
    }
    if (a === '--out-dir' && next) {
      args.outDir = path.resolve(ROOT, next)
      i += 1
      continue
    }
  }
  return args
}

function rel(p) {
  return path.relative(ROOT, p).replace(/\\/g, '/')
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true })
}

function readJsonOptional(file) {
  if (!fs.existsSync(file)) return null
  const text = fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '')
  return JSON.parse(text)
}

function asNumber(v) {
  const n = Number(v)
  return Number.isFinite(n) ? n : null
}

function addCheck(checks, name, params) {
  const { ok, severity = 'error', detail = '', observed = null, baseline = null, threshold = null } = params
  checks.push({
    name,
    ok: Boolean(ok),
    severity,
    detail,
    observed,
    baseline,
    threshold,
  })
}

function evaluateWorkflowChecks(checks, baseline, summary) {
  if (!baseline) {
    addCheck(checks, 'workflow_baseline_present', {
      ok: false,
      detail: 'Missing workflow regression baseline JSON.',
    })
    return
  }
  if (!summary) {
    addCheck(checks, 'workflow_summary_present', {
      ok: false,
      detail: 'Missing workflow regression summary JSON.',
    })
    return
  }

  const runtimeBaseline = asNumber(baseline.runtime_ms)
  const runtimeObserved = asNumber(summary.runtime_ms)
  const runtimeThreshold = runtimeBaseline !== null ? runtimeBaseline * 1.5 : null
  addCheck(checks, 'workflow_runtime_not_regressed_50pct', {
    ok:
      runtimeObserved !== null &&
      runtimeThreshold !== null &&
      runtimeObserved <= runtimeThreshold,
    detail: `runtime_ms=${runtimeObserved} baseline=${runtimeBaseline}`,
    observed: runtimeObserved,
    baseline: runtimeBaseline,
    threshold: runtimeThreshold,
  })

  const scalarChecks = [
    ['robust_score_p10_non_decreasing', 'robust_score_p10', 'gte'],
    ['holdout_score_p10_non_decreasing', 'holdout_score_p10', 'gte'],
    ['max_dd_p90_non_increasing', 'max_dd_p90', 'lte'],
    ['holdout_max_dd_p90_non_increasing', 'holdout_max_dd_p90', 'lte'],
  ]
  for (const [checkName, key, mode] of scalarChecks) {
    const observed = asNumber(summary[key])
    const ref = asNumber(baseline[key])
    const ok =
      observed !== null &&
      ref !== null &&
      (mode === 'gte' ? observed >= ref : observed <= ref)
    addCheck(checks, checkName, {
      ok,
      detail: `${key} observed=${observed} baseline=${ref}`,
      observed,
      baseline: ref,
      threshold: mode,
    })
  }
}

function evaluateUiChecks(checks, uiMetrics) {
  if (!uiMetrics) {
    addCheck(checks, 'ui_perf_metrics_present', {
      ok: false,
      detail: 'Missing UI perf metrics JSON.',
    })
    return
  }

  const coldStart = asNumber(uiMetrics.app_cold_start_ms)
  const tti = asNumber(uiMetrics.time_to_interactive_ms)
  const avgBarMs = asNumber(uiMetrics.chart_live_feed_responsiveness?.avg_ms_per_event)
  const barEvents = asNumber(uiMetrics.chart_live_feed_responsiveness?.events)

  addCheck(checks, 'ui_cold_start_under_3000ms', {
    ok: coldStart !== null && coldStart <= 3000,
    detail: `app_cold_start_ms=${coldStart}`,
    observed: coldStart,
    threshold: 3000,
  })
  addCheck(checks, 'ui_tti_under_10000ms', {
    ok: tti !== null && tti <= 10000,
    detail: `time_to_interactive_ms=${tti}`,
    observed: tti,
    threshold: 10000,
  })
  addCheck(checks, 'ui_live_bar_avg_under_2ms', {
    ok: avgBarMs !== null && avgBarMs <= 2,
    detail: `avg_ms_per_event=${avgBarMs} events=${barEvents}`,
    observed: avgBarMs,
    threshold: 2,
  })
  addCheck(checks, 'ui_live_bar_sample_size_at_least_100', {
    ok: barEvents !== null && barEvents >= 100,
    detail: `events=${barEvents}`,
    observed: barEvents,
    threshold: 100,
  })
}

function evaluateMemoryChecks(checks, memMetrics) {
  if (!memMetrics) {
    addCheck(checks, 'optimization_memory_metrics_present', {
      ok: false,
      detail: 'Missing optimization memory metrics JSON.',
    })
    return
  }

  const peakWsMb = asNumber(memMetrics.peak_working_set_mb)
  const peakPrivateMb = asNumber(memMetrics.peak_private_mb)
  addCheck(checks, 'optimization_peak_working_set_under_1024mb', {
    ok: peakWsMb !== null && peakWsMb <= 1024,
    detail: `peak_working_set_mb=${peakWsMb}`,
    observed: peakWsMb,
    threshold: 1024,
  })
  addCheck(checks, 'optimization_peak_private_under_1024mb', {
    ok: peakPrivateMb !== null && peakPrivateMb <= 1024,
    detail: `peak_private_mb=${peakPrivateMb}`,
    observed: peakPrivateMb,
    threshold: 1024,
  })
}

function evaluateSampleChecks(checks, manifest) {
  if (!manifest) {
    addCheck(checks, 'baseline_samples_manifest_present', {
      ok: false,
      detail: 'Missing baseline samples manifest JSON.',
    })
    return
  }

  const outputs = manifest.outputs || {}
  const required = [
    'top_rows_csv',
    'validation_shortlist_csv',
    'run_artifact_json',
    'batch_summary_json',
    'report_json',
    'report_xlsx',
  ]
  for (const key of required) {
    const p = outputs[key]
    const abs = typeof p === 'string' ? path.resolve(ROOT, p) : ''
    addCheck(checks, `samples_output_present:${key}`, {
      ok: typeof p === 'string' && p.length > 0 && fs.existsSync(abs),
      detail: `${key}=${p || '<missing>'}`,
      observed: p || null,
    })
  }
}

function writeMarkdown(outFile, payload) {
  const okChecks = payload.checks.filter((c) => c.ok).length
  const total = payload.checks.length
  const failed = payload.checks.filter((c) => !c.ok)
  const lines = []
  lines.push('# Post-Release Telemetry Review')
  lines.push('')
  lines.push(`- Generated: ${payload.generated_at}`)
  lines.push(`- Status: **${payload.status.toUpperCase()}**`)
  lines.push(`- Checks passed: ${okChecks}/${total}`)
  lines.push('')
  lines.push('## Sources')
  lines.push('')
  lines.push(`- Baseline: \`${payload.sources.baseline}\``)
  lines.push(`- Summary: \`${payload.sources.summary}\``)
  lines.push(`- UI metrics: \`${payload.sources.ui_metrics}\``)
  lines.push(`- Memory metrics: \`${payload.sources.memory_metrics}\``)
  lines.push(`- Samples manifest: \`${payload.sources.samples_manifest}\``)
  lines.push('')
  lines.push('## Check Results')
  lines.push('')
  for (const c of payload.checks) {
    lines.push(`- ${c.ok ? '[PASS]' : '[FAIL]'} \`${c.name}\` - ${c.detail}`)
  }
  if (failed.length > 0) {
    lines.push('')
    lines.push('## Failed Checks')
    lines.push('')
    for (const c of failed) {
      lines.push(`- \`${c.name}\`: ${c.detail}`)
    }
  }
  fs.writeFileSync(outFile, `${lines.join('\n')}\n`, 'utf8')
}

function main() {
  const args = parseArgs(process.argv)
  ensureDir(args.outDir)

  const baseline = readJsonOptional(args.baseline)
  const summary = readJsonOptional(args.summary)
  const uiMetrics = readJsonOptional(args.uiMetrics)
  const memoryMetrics = readJsonOptional(args.memoryMetrics)
  const samplesManifest = readJsonOptional(args.samplesManifest)

  const checks = []
  evaluateWorkflowChecks(checks, baseline, summary)
  evaluateUiChecks(checks, uiMetrics)
  evaluateMemoryChecks(checks, memoryMetrics)
  evaluateSampleChecks(checks, samplesManifest)

  const failed = checks.filter((c) => !c.ok)
  const status = failed.length === 0 ? 'pass' : 'fail'
  const payload = {
    generated_at: new Date().toISOString(),
    status,
    summary: {
      total_checks: checks.length,
      passed_checks: checks.length - failed.length,
      failed_checks: failed.length,
    },
    checks,
    sources: {
      baseline: rel(args.baseline),
      summary: rel(args.summary),
      ui_metrics: rel(args.uiMetrics),
      memory_metrics: rel(args.memoryMetrics),
      samples_manifest: rel(args.samplesManifest),
    },
  }

  const jsonOut = path.resolve(args.outDir, 'telemetry_review.json')
  const mdOut = path.resolve(args.outDir, 'telemetry_review.md')
  fs.writeFileSync(jsonOut, `${JSON.stringify(payload, null, 2)}\n`, 'utf8')
  writeMarkdown(mdOut, payload)

  const icon = status === 'pass' ? 'PASS' : 'FAIL'
  console.log(`[${icon}] Post-release telemetry review: ${payload.summary.passed_checks}/${payload.summary.total_checks} checks passed.`)
  console.log(`JSON: ${rel(jsonOut)}`)
  console.log(`MD: ${rel(mdOut)}`)
  if (failed.length > 0) {
    for (const c of failed) {
      console.error(`- ${c.name}: ${c.detail}`)
    }
    process.exit(1)
  }
}

main()
