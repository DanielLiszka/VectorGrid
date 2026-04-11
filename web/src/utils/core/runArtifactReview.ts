export interface RouteProvenanceSummary {
  requestedRoute: string
  resolvedRoute: string
  realizedRoute: string
  routeChanged: boolean
  fallbackReason: string
}

export interface ArtifactLineageSummary {
  parentArtifactPath: string
  parentArtifactKind: string
  lineageRelation: string
}

export interface RunComparisonTarget {
  source: string
  ts: string
  strategyKind: string
  csvPath: string
  timeframe: string
  reportPath: string
  hasValidation: boolean
  configFingerprint: string
  datasetMeta: Record<string, unknown> | null
  optimizationMetricSurface: Record<string, unknown> | null
  routeProvenance: RouteProvenanceSummary | null
  lineage: ArtifactLineageSummary | null
  req: Record<string, unknown>
  res: Record<string, unknown>
}

function text(value: unknown): string {
  return String(value ?? "").trim()
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {}
}

function num(value: unknown, fallback = 0): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

function fileBasename(path: string): string {
  const next = text(path)
  if (!next) return ""
  const parts = next.split(/[\\/]+/g)
  return text(parts[parts.length - 1])
}

function fmt(value: unknown, digits = 6): string {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "NaN"
}

function fmtCompact(value: unknown, digits = 4): string {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "-"
}

function routeSegments(value: string): string[] {
  return value
    .split("->")
    .map((segment) => text(segment))
    .filter((segment) => segment.length > 0)
}

function normalizeRequestedRoute(value: unknown): string {
  const row = asRecord(value)
  if (Object.keys(row).length > 0) {
    return text(row.backend ?? row.kind ?? row.label)
  }
  return text(value)
}

function normalizeBackendUsedLabel(requested: string, value: unknown): string {
  const raw = text(value)
  if (!raw) return ""
  if (raw === "Cpu") return requested.startsWith("Cpu") ? requested : "CpuOnly"
  if (raw === "Gpu") return requested.startsWith("Gpu") ? requested : "Gpu"
  return raw
}

export function parseRouteProvenance(value: unknown): RouteProvenanceSummary | null {
  const row = asRecord(value)
  const requestedRoute = text(row.requested_route ?? row.requestedRoute)
  const resolvedRoute = text(row.resolved_route ?? row.resolvedRoute)
  const realizedRoute = text(row.realized_route ?? row.realizedRoute)
  const fallbackReason = text(row.fallback_reason ?? row.fallbackReason)
  const routeChangedValue = row.route_changed ?? row.routeChanged
  const routeChanged =
    typeof routeChangedValue === "boolean"
      ? routeChangedValue
      : text(routeChangedValue).toLowerCase() === "true"
  if (
    !requestedRoute &&
    !resolvedRoute &&
    !realizedRoute &&
    !fallbackReason &&
    !routeChanged
  ) {
    return null
  }
  return {
    requestedRoute,
    resolvedRoute,
    realizedRoute,
    routeChanged,
    fallbackReason,
  }
}

export function parseArtifactLineage(value: unknown): ArtifactLineageSummary | null {
  const row = asRecord(value)
  const parentArtifactPath = text(
    row.parent_artifact_path ?? row.parentArtifactPath,
  )
  const parentArtifactKind = text(
    row.parent_artifact_kind ?? row.parentArtifactKind,
  )
  const lineageRelation = text(row.lineage_relation ?? row.lineageRelation)
  if (!parentArtifactPath && !parentArtifactKind && !lineageRelation) return null
  return {
    parentArtifactPath,
    parentArtifactKind,
    lineageRelation,
  }
}

export function buildRouteProvenanceFromRun(
  reqRaw: unknown,
  resRaw: unknown,
): RouteProvenanceSummary | null {
  const req = asRecord(reqRaw)
  const res = asRecord(resRaw)
  const requestedRoute = normalizeRequestedRoute(req.backend)
  const route = text(res.backend_route ?? res.backendRoute)
  const fallbackReason = text(
    res.backend_fallback_reason ?? res.backendFallbackReason,
  )
  const segments = routeSegments(route)
  const resolvedRoute = segments[0] ?? requestedRoute
  const realizedRoute =
    segments[segments.length - 1] ??
    normalizeBackendUsedLabel(requestedRoute, res.backend_used ?? res.backendUsed)
  const routeChanged =
    !!fallbackReason ||
    segments.length > 1 ||
    (!!requestedRoute && !!realizedRoute && requestedRoute !== realizedRoute)
  if (
    !requestedRoute &&
    !resolvedRoute &&
    !realizedRoute &&
    !fallbackReason &&
    !routeChanged
  ) {
    return null
  }
  return {
    requestedRoute,
    resolvedRoute,
    realizedRoute,
    routeChanged,
    fallbackReason,
  }
}

export function resolveRouteProvenance(
  run: Pick<RunComparisonTarget, "routeProvenance" | "req" | "res">,
): RouteProvenanceSummary | null {
  return run.routeProvenance ?? buildRouteProvenanceFromRun(run.req, run.res)
}

export function formatRouteProvenanceCompact(
  provenance: RouteProvenanceSummary | null,
): string {
  if (!provenance) return "-"
  const parts: string[] = []
  if (provenance.requestedRoute) parts.push(`requested=${provenance.requestedRoute}`)
  if (provenance.resolvedRoute) parts.push(`resolved=${provenance.resolvedRoute}`)
  if (provenance.realizedRoute) parts.push(`actual=${provenance.realizedRoute}`)
  if (provenance.fallbackReason) parts.push(`fallback=${provenance.fallbackReason}`)
  if (provenance.routeChanged && !provenance.fallbackReason) {
    parts.push("route_changed=yes")
  }
  return parts.length > 0 ? parts.join(" | ") : "-"
}

export function formatArtifactLineageCompact(
  lineage: ArtifactLineageSummary | null,
): string {
  if (!lineage) return "-"
  const relation = text(lineage.lineageRelation) || "derived_from"
  const kind = text(lineage.parentArtifactKind)
  const path = text(lineage.parentArtifactPath)
  const base = fileBasename(path) || path || "-"
  return kind ? `${relation} ${kind}:${base}` : `${relation} ${base}`
}

function metricSurfaceLabel(value: Record<string, unknown> | null): string {
  if (!value) return ""
  const kind = text(
    value.kind ??
      value.source_kind ??
      value.sourceKind ??
      value.surface,
  )
  const label = text(value.label ?? value.metric_surface ?? value.metricSurface)
  const rule = text(value.rule ?? value.mode)
  const normalizedKind =
    kind.toLowerCase() === "core17"
      ? "Core17"
      : kind.toLowerCase() === "trade22"
        ? "Trade22"
        : kind.toLowerCase() === "legacy"
          ? "Legacy"
          : kind
  return [normalizedKind, label, rule].filter((part) => part.length > 0).join("/")
}

function readBestCandidate(target: RunComparisonTarget): {
  params: Record<string, unknown>
  metrics: Record<string, unknown>
} {
  const res = target.res
  const bestParams = asRecord(res.best_params ?? res.bestParams)
  const bestMetrics = asRecord(res.best_metrics ?? res.bestMetrics)
  if (Object.keys(bestParams).length || Object.keys(bestMetrics).length) {
    return { params: bestParams, metrics: bestMetrics }
  }

  const top = Array.isArray(res.top) ? res.top : []
  const first = Array.isArray(top[0]) ? (top[0] as unknown[]) : []
  const firstParams = asRecord(first[0])
  const firstMetrics = asRecord(first[1])
  if (Object.keys(firstParams).length || Object.keys(firstMetrics).length) {
    return { params: firstParams, metrics: firstMetrics }
  }

  const pareto = Array.isArray(res.pareto) ? res.pareto : []
  const paretoFirst = asRecord(pareto[0])
  return {
    params: asRecord(paretoFirst.params),
    metrics: asRecord(paretoFirst.metrics),
  }
}

function readRuntimeMs(res: Record<string, unknown>): number {
  return Math.max(
    0,
    Math.floor(
      num(
        res.runtime_ms ??
          res.runtimeMs ??
          res.total_runtime_ms ??
          res.totalRuntimeMs,
        0,
      ),
    ),
  )
}

function readComboCount(res: Record<string, unknown>): number {
  const direct = Math.floor(
    Math.max(
      0,
      num(
        res.pairs ??
          res.pair_count ??
          res.pairCount ??
          res.candidates_considered ??
          res.candidatesConsidered,
        0,
      ),
    ),
  )
  if (direct > 0) return direct
  const top = Array.isArray(res.top) ? res.top.length : 0
  return Math.max(0, top)
}

function buildDatasetIdentity(target: RunComparisonTarget): string {
  const parts: string[] = []
  const csv = fileBasename(target.csvPath) || text(target.csvPath)
  if (csv) parts.push(csv)
  const timeframe = text(target.timeframe)
  if (timeframe) parts.push(`tf=${timeframe}`)
  const datasetMeta = target.datasetMeta ?? {}
  const baseTimeframe = text(
    datasetMeta.base_timeframe ?? datasetMeta.baseTimeframe,
  )
  const derivedTimeframe = text(
    datasetMeta.derived_timeframe ?? datasetMeta.derivedTimeframe,
  )
  const resampleApplied = datasetMeta.resample_applied ?? datasetMeta.resampleApplied
  if (baseTimeframe && derivedTimeframe && baseTimeframe !== derivedTimeframe) {
    parts.push(`base=${baseTimeframe}`)
    parts.push(`derived=${derivedTimeframe}`)
  } else if (derivedTimeframe && derivedTimeframe !== timeframe) {
    parts.push(`derived=${derivedTimeframe}`)
  }
  if (resampleApplied === true) parts.push("resample=yes")
  const fingerprint = text(
    datasetMeta.source_content_fingerprint ??
      datasetMeta.sourceContentFingerprint,
  )
  if (fingerprint) parts.push(`fp=${fingerprint}`)
  return parts.length > 0 ? parts.join(" | ") : "-"
}

function diffParams(
  baseline: Record<string, unknown>,
  current: Record<string, unknown>,
  limit = 12,
): Array<{ key: string; from: string; to: string }> {
  const keys = [...new Set([...Object.keys(baseline), ...Object.keys(current)])].sort()
  const out: Array<{ key: string; from: string; to: string }> = []
  for (const key of keys) {
    const from = JSON.stringify(baseline[key] ?? null)
    const to = JSON.stringify(current[key] ?? null)
    if (from === to) continue
    out.push({ key, from: from ?? "null", to: to ?? "null" })
    if (out.length >= limit) break
  }
  return out
}

function truncate(value: string, limit = 60): string {
  return value.length > limit ? `${value.slice(0, limit - 1)}...` : value
}

function changeLabel(left: string, right: string): string {
  if (!left && !right) return "same (-)"
  return left === right ? `same (${left || "-"})` : `changed (${left || "-"} -> ${right || "-"})`
}

function strategyKindFor(target: RunComparisonTarget): string {
  return text(target.strategyKind) || text(target.req.strategy_kind ?? target.req.strategyKind)
}

export function buildRunCompareSummary(
  current: RunComparisonTarget,
  baseline: RunComparisonTarget,
): string {
  const currentBest = readBestCandidate(current)
  const baselineBest = readBestCandidate(baseline)
  const currentMetrics = currentBest.metrics
  const baselineMetrics = baselineBest.metrics
  const currentRoute = resolveRouteProvenance(current)
  const baselineRoute = resolveRouteProvenance(baseline)
  const currentMetricSurface = metricSurfaceLabel(current.optimizationMetricSurface)
  const baselineMetricSurface = metricSurfaceLabel(baseline.optimizationMetricSurface)

  const lines: string[] = []
  lines.push(
    `baseline (${baseline.source || "saved"}): ${baseline.ts || "-"} | strategy=${strategyKindFor(baseline)} | combos=${readComboCount(baseline.res)} | runtime_ms=${readRuntimeMs(baseline.res)}`,
  )
  lines.push(
    `current (${current.source || "current"}): ${current.ts || "-"} | strategy=${strategyKindFor(current)} | combos=${readComboCount(current.res)} | runtime_ms=${readRuntimeMs(current.res)}`,
  )
  lines.push(
    `config: ${changeLabel(baseline.configFingerprint, current.configFingerprint)}`,
  )
  lines.push(
    `dataset: ${changeLabel(buildDatasetIdentity(baseline), buildDatasetIdentity(current))}`,
  )
  lines.push(
    `route: ${changeLabel(
      formatRouteProvenanceCompact(baselineRoute),
      formatRouteProvenanceCompact(currentRoute),
    )}`,
  )
  lines.push(
    `validation/report: ${changeLabel(
      `${baseline.hasValidation ? "validation" : "no-validation"} | ${baseline.reportPath ? "xlsx" : "artifact-only"}`,
      `${current.hasValidation ? "validation" : "no-validation"} | ${current.reportPath ? "xlsx" : "artifact-only"}`,
    )}`,
  )
  if (baselineMetricSurface || currentMetricSurface) {
    lines.push(
      `metric surface: ${changeLabel(baselineMetricSurface, currentMetricSurface)}`,
    )
  }
  const baselineLineage = formatArtifactLineageCompact(baseline.lineage)
  const currentLineage = formatArtifactLineageCompact(current.lineage)
  if (baselineLineage !== "-" || currentLineage !== "-") {
    lines.push(`lineage: ${changeLabel(baselineLineage, currentLineage)}`)
  }

  lines.push("")
  lines.push("Optimization best:")
  lines.push(
    `  baseline: pnl=${fmtCompact(baselineMetrics.pnl)} sharpe=${fmtCompact(baselineMetrics.sharpe)} dd=${fmtCompact(baselineMetrics.max_dd)} trades=${Math.floor(num(baselineMetrics.trades, 0))} expo=${fmtCompact(baselineMetrics.exposure)}`,
  )
  lines.push(
    `  current:  pnl=${fmtCompact(currentMetrics.pnl)} sharpe=${fmtCompact(currentMetrics.sharpe)} dd=${fmtCompact(currentMetrics.max_dd)} trades=${Math.floor(num(currentMetrics.trades, 0))} expo=${fmtCompact(currentMetrics.exposure)}`,
  )
  lines.push(
    `  deltas:   pnl=${fmt(num(currentMetrics.pnl, 0) - num(baselineMetrics.pnl, 0))} sharpe=${fmt(num(currentMetrics.sharpe, 0) - num(baselineMetrics.sharpe, 0))} dd=${fmt(num(currentMetrics.max_dd, 0) - num(baselineMetrics.max_dd, 0))} trades=${Math.floor(num(currentMetrics.trades, 0) - num(baselineMetrics.trades, 0))} expo=${fmt(num(currentMetrics.exposure, 0) - num(baselineMetrics.exposure, 0), 4)}`,
  )

  const diffs = diffParams(baselineBest.params, currentBest.params)
  if (diffs.length > 0) {
    lines.push("")
    lines.push("Params diff (current - baseline):")
    for (const row of diffs) {
      lines.push(
        `  ${row.key}: ${truncate(row.from, 60)} -> ${truncate(row.to, 60)}`,
      )
    }
  }

  return lines.join("\n")
}
