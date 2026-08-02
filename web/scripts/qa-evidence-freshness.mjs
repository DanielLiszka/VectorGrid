const RUN_ID_ENV = 'VECTORGRID_FRONTEND_QA_RUN_ID'
const RUN_STARTED_AT_ENV = 'VECTORGRID_FRONTEND_QA_RUN_STARTED_AT'
const CLOCK_SKEW_TOLERANCE_MS = 2_000

function timestampMs(value) {
  const parsed = Date.parse(String(value || ''))
  return Number.isFinite(parsed) ? parsed : null
}

export function resolveFrontendQaReportRun(
  environment = process.env,
  generatedAt = new Date().toISOString(),
) {
  const id = String(environment[RUN_ID_ENV] || '').trim()
  const startedAt = String(environment[RUN_STARTED_AT_ENV] || '').trim()
  const startedAtMs = timestampMs(startedAt)
  const generatedAtMs = timestampMs(generatedAt)
  const available = Boolean(
    id
      && startedAtMs != null
      && generatedAtMs != null
      && startedAtMs <= generatedAtMs + CLOCK_SKEW_TOLERANCE_MS,
  )

  return {
    id: available ? id : '',
    startedAt: available ? new Date(startedAtMs).toISOString() : '',
    generatedAt,
    available,
  }
}

export function wasProducedInFrontendQaReport(timestamp, reportRun) {
  const producedAtMs = timestampMs(timestamp)
  const startedAtMs = timestampMs(reportRun?.startedAt)
  const generatedAtMs = timestampMs(reportRun?.generatedAt)
  return Boolean(
    reportRun?.available
      && producedAtMs != null
      && startedAtMs != null
      && generatedAtMs != null
      && producedAtMs >= startedAtMs
      && producedAtMs <= generatedAtMs + CLOCK_SKEW_TOLERANCE_MS,
  )
}

export function normalizeDesktopSmokeEvidence(summary, reportRun, primarySummary = null) {
  if (!summary) return null

  const generatedAt = String(summary.generatedAt || '')
  const primaryGeneratedAt = String(primarySummary?.generatedAt || '')
  const generatedAtMs = timestampMs(generatedAt)
  const primaryGeneratedAtMs = timestampMs(primaryGeneratedAt)
  const primaryIdentityMatches = Boolean(
    primarySummary
      && String(primarySummary.runId || '')
      && String(primarySummary.runId || '') === String(summary.runId || ''),
  )
  const superseded = Boolean(
    generatedAtMs != null
      && primaryGeneratedAtMs != null
      && generatedAtMs < primaryGeneratedAtMs
      && primaryIdentityMatches,
  )
  const producedInCurrentReport = wasProducedInFrontendQaReport(generatedAt, reportRun)
  const runIdentityMatches = Boolean(
    reportRun?.available
      && String(summary.runId || '')
      && String(summary.runId || '') === reportRun.id,
  )
  const currentRun = producedInCurrentReport && runIdentityMatches && !superseded
  const sourceStatus = String(summary.status || 'unknown')
  let freshnessReason = 'current_report'
  if (!reportRun?.available) freshnessReason = 'current_report_context_unavailable'
  else if (generatedAtMs == null) freshnessReason = 'invalid_generated_at'
  else if (!producedInCurrentReport) freshnessReason = 'not_produced_in_current_report'
  else if (!runIdentityMatches) freshnessReason = 'run_identity_mismatch'
  else if (superseded) freshnessReason = 'superseded_by_primary_desktop_summary'

  return {
    status: currentRun ? sourceStatus : 'reference_only',
    sourceStatus,
    evidenceScope: currentRun ? 'current_run' : 'historical_reference',
    currentRun,
    runIdentityMatches,
    primaryIdentityMatches,
    freshnessReason,
    suite: String(summary.suite || ''),
    specCount: summary.specCount ?? null,
    exitCode: summary.exitCode ?? null,
    clearedBySuite: String(summary.clearedBySuite || ''),
    generatedAt,
    runId: String(summary.runId || ''),
    artifacts: summary.artifacts && typeof summary.artifacts === 'object'
      ? summary.artifacts
      : {},
    stale: !currentRun,
  }
}

export function normalizeUiPerfBudgetEvidence(report, reportRun) {
  if (!report) return null

  const sourceCapture = report.sourceCapture && typeof report.sourceCapture === 'object'
    ? report.sourceCapture
    : {}
  const generatedAt = String(report.generatedAt || '')
  const sourceCapturedAt = String(sourceCapture.generatedAt || '')
  const evaluatedInCurrentReport = wasProducedInFrontendQaReport(generatedAt, reportRun)
  const capturedInCurrentReport = wasProducedInFrontendQaReport(sourceCapturedAt, reportRun)
  const evaluationRunIdentityMatches = Boolean(
    reportRun?.available
      && String(report.runId || '')
      && String(report.runId || '') === reportRun.id,
  )
  const captureRunIdentityMatches = Boolean(
    reportRun?.available
      && String(sourceCapture.runId || '')
      && String(sourceCapture.runId || '') === reportRun.id,
  )
  const currentRun = Boolean(
    evaluatedInCurrentReport
      && capturedInCurrentReport
      && evaluationRunIdentityMatches
      && captureRunIdentityMatches,
  )
  const sourceStatus = String(
    report.status || ((report.issues || []).length > 0 ? 'failed' : 'passed'),
  )
  let freshnessReason = 'current_run_capture'
  if (!reportRun?.available) freshnessReason = 'current_report_context_unavailable'
  else if (!evaluatedInCurrentReport) freshnessReason = 'evaluation_not_produced_in_current_report'
  else if (!evaluationRunIdentityMatches) freshnessReason = 'evaluation_run_identity_mismatch'
  else if (!capturedInCurrentReport) freshnessReason = 'baseline_capture_not_produced_in_current_report'
  else if (!captureRunIdentityMatches) freshnessReason = 'capture_run_identity_mismatch'

  return {
    status: currentRun ? sourceStatus : 'reference_only',
    sourceStatus,
    evidenceScope: currentRun ? 'current_run_capture' : 'baseline_reference',
    currentRun,
    evaluatedInCurrentReport,
    capturedInCurrentReport,
    evaluationRunIdentityMatches,
    captureRunIdentityMatches,
    freshnessReason,
    generatedAt,
    runId: String(report.runId || ''),
    source: report.source || '',
    sourceCapture: {
      generatedAt: sourceCapturedAt,
      runId: String(sourceCapture.runId || ''),
      fileModifiedAt: String(sourceCapture.fileModifiedAt || ''),
      mode: String(sourceCapture.mode || ''),
      note: String(sourceCapture.note || ''),
    },
    budgets: report.budgets || {},
    observations: report.observations || {},
    issues: report.issues || [],
  }
}
