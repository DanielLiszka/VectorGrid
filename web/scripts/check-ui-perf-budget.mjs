#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..', '..')
const metricsPath = path.resolve(repoRoot, 'scripts', 'baselines', 'ui_capture', 'perf_metrics.json')
const reportPath = path.resolve(repoRoot, 'output', 'qa', 'ui-perf-budget.json')

const budgets = {
  app_cold_start_ms: 2500,
  time_to_interactive_ms: 8000,
  js_heap_used_bytes_at_idle: 180 * 1024 * 1024,
  capture_runtime_ms: 60_000,
  chart_live_feed_avg_ms_per_event: 2,
}

function numeric(value) {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

if (!fs.existsSync(metricsPath)) {
  console.error('[ui-perf-budget] Missing baseline UI metrics. Run `npm --prefix web run baseline:ui:built` first.')
  process.exit(1)
}

const metrics = JSON.parse(fs.readFileSync(metricsPath, 'utf8'))
const observations = {
  app_cold_start_ms: numeric(metrics.app_cold_start_ms),
  time_to_interactive_ms: numeric(metrics.time_to_interactive_ms),
  js_heap_used_bytes_at_idle: numeric(metrics.js_heap_used_bytes_at_idle),
  capture_runtime_ms: numeric(metrics.capture_runtime_ms),
  chart_live_feed_avg_ms_per_event: numeric(metrics.chart_live_feed_responsiveness?.avg_ms_per_event),
  main_boot_to_ready_ms: numeric(metrics.frontend_perf?.observations?.main_boot_to_ready_ms),
  main_desktop_initialization_ms: numeric(
    metrics.frontend_perf?.observations?.main_desktop_initialization_ms,
  ),
  workflow_startup_data_hydration_ms: numeric(
    metrics.frontend_perf?.observations?.workflow_startup_data_hydration_ms,
  ),
  workflow_latest_run_sync_ms: numeric(metrics.frontend_perf?.observations?.workflow_latest_run_sync_ms),
  latest_run_fast_path_ms: numeric(metrics.frontend_perf?.observations?.latest_run_fast_path_ms),
  artifact_history_entry_count: numeric(metrics.frontend_perf?.observations?.artifact_history_entry_count),
  validation_report_entry_count: numeric(metrics.frontend_perf?.observations?.validation_report_entry_count),
  mock_tauri_invoke_count: numeric(metrics.frontend_perf?.observations?.mock_tauri_invoke_count),
  mock_tauri_invoke_total_ms: numeric(metrics.frontend_perf?.observations?.mock_tauri_invoke_total_ms),
  mock_tauri_invoke_max_ms: numeric(metrics.frontend_perf?.observations?.mock_tauri_invoke_max_ms),
  mock_tauri_invoke_delay_total_ms: numeric(
    metrics.frontend_perf?.observations?.mock_tauri_invoke_delay_total_ms,
  ),
}

const issues = []
for (const [key, budget] of Object.entries(budgets)) {
  const observed = observations[key]
  if (observed == null) {
    issues.push(`${key} is missing from perf_metrics.json`)
  } else if (observed > budget) {
    issues.push(`${key} ${observed} exceeded budget ${budget}`)
  }
}
if ((observations.artifact_history_entry_count ?? 0) <= 0) {
  issues.push('artifact_history_entry_count is missing or empty in perf_metrics.json')
}
if ((observations.mock_tauri_invoke_count ?? 0) <= 0) {
  issues.push('mock_tauri_invoke_count is missing or empty in perf_metrics.json')
}

const report = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  source: path.relative(repoRoot, metricsPath).replace(/\\/g, '/'),
  budgets,
  observations,
  issues,
}
fs.mkdirSync(path.dirname(reportPath), { recursive: true })
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')

for (const [key, observed] of Object.entries(observations)) {
  const budget = budgets[key]
  const label = observed == null ? 'missing' : observed
  console.log(`[ui-perf-budget] ${key}: ${label} / ${budget ?? 'n/a'}`)
}

if (issues.length > 0) {
  console.error('[ui-perf-budget] UI performance budget failed:')
  for (const issue of issues) console.error(`- ${issue}`)
  process.exit(1)
}

console.log(`[ui-perf-budget] Budget report written to ${path.relative(path.resolve(repoRoot, 'web'), reportPath).replace(/\\/g, '/')}`)
