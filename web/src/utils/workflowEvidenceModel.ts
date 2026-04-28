export type EvidenceTone = 'neutral' | 'info' | 'success' | 'warning' | 'danger'

export interface EvidenceChip {
  id: string
  label: string
  detail?: string
  tone: EvidenceTone
  actionLocal: true
}

export interface RouteFallbackEvidenceInput {
  routeProvenance?: unknown
  backendUsed?: unknown
  backendRoute?: unknown
  backendFallbackReason?: unknown
}

export interface ScoreScopeEvidenceInput {
  scoreSpace?: unknown
  scoreScope?: unknown
  nativeBackendUsed?: unknown
  nativeFallbackReason?: unknown
}

export interface ValidationFreshnessEvidenceInput {
  currentRunSignature?: unknown
  currentValidationSignature?: unknown
  expectedRunSignature?: unknown
  reportPath?: unknown
  reportExists?: unknown
}

export interface DatasetProvenanceEvidenceInput {
  datasetMeta?: unknown
  lineage?: unknown
  sourceContentFingerprint?: unknown
  timeframe?: unknown
}

export interface LiveReadinessEvidenceInput {
  liveWorkspaceAvailable?: unknown
}

function chip(id: string, label: string, tone: EvidenceTone, detail?: string): EvidenceChip {
  const next: EvidenceChip = {
    id,
    label,
    tone,
    actionLocal: true,
  }
  if (detail) next.detail = detail
  return next
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function lowerText(value: unknown): string {
  return text(value).toLowerCase()
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {}
}

function bool(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const normalized = lowerText(value)
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on'
}

function routeSegments(value: string): string[] {
  return value
    .split('->')
    .map((segment) => text(segment))
    .filter((segment) => segment.length > 0)
}

function routeDetail(parts: string[]): string | undefined {
  return parts.filter((part) => part.length > 0).join(' | ') || undefined
}

function canonicalScope(value: unknown): string {
  return lowerText(value).replace(/[\s_-]+/g, '')
}

function firstText(...values: unknown[]): string {
  for (const value of values) {
    const next = text(value)
    if (next) return next
  }
  return ''
}

export function routeFallbackEvidence(input: RouteFallbackEvidenceInput): EvidenceChip[] {
  const provenance = asRecord(input.routeProvenance)
  const backendRoute = text(input.backendRoute)
  const routeParts = routeSegments(backendRoute)
  const requestedRoute = firstText(provenance.requestedRoute, provenance.requested_route)
  const resolvedRoute = firstText(
    provenance.resolvedRoute,
    provenance.resolved_route,
    routeParts[0],
  )
  const realizedRoute = firstText(
    provenance.realizedRoute,
    provenance.realized_route,
    routeParts[routeParts.length - 1],
    input.backendUsed,
  )
  const fallbackReason = firstText(
    provenance.fallbackReason,
    provenance.fallback_reason,
    input.backendFallbackReason,
  )
  const routeChanged = bool(provenance.routeChanged ?? provenance.route_changed)
    || routeParts.length > 1
    || (!!requestedRoute && !!realizedRoute && requestedRoute !== realizedRoute)
    || !!fallbackReason

  if (!requestedRoute && !resolvedRoute && !realizedRoute && !fallbackReason && !routeChanged) return []

  if (routeChanged) {
    return [
      chip(
        'route-fallback',
        'Route fallback',
        'warning',
        routeDetail([
          requestedRoute ? `requested=${requestedRoute}` : '',
          resolvedRoute ? `resolved=${resolvedRoute}` : '',
          realizedRoute ? `actual=${realizedRoute}` : '',
          fallbackReason ? `fallback=${fallbackReason}` : '',
        ]),
      ),
    ]
  }

  if (realizedRoute) {
    return [
      chip(
        'route-used',
        realizedRoute.toLowerCase() === 'cpuonly' ? 'CPU-only route' : 'Route used',
        realizedRoute.toLowerCase() === 'cpuonly' ? 'neutral' : 'info',
        realizedRoute.toLowerCase() === 'cpuonly'
          ? 'No fallback recorded.'
          : routeDetail([`actual=${realizedRoute}`]),
      ),
    ]
  }

  return []
}

export function scoreScopeEvidence(input: ScoreScopeEvidenceInput): EvidenceChip[] {
  const out: EvidenceChip[] = []
  const scope = text(input.scoreScope)
  const canonical = canonicalScope(scope)
  const scoreSpace = text(input.scoreSpace)
  const nativeBackendUsed = text(input.nativeBackendUsed)
  const nativeFallbackReason = text(input.nativeFallbackReason)

  if (scope) {
    if (canonical === 'walkforwardtest') {
      out.push(chip('score-scope', 'Walk-forward score scope', 'success', scoreSpace ? `space=${scoreSpace}` : undefined))
    } else if (canonical === 'calibrationslice' || canonical === 'fulldataset' || canonical === 'custompartial') {
      out.push(chip('score-scope', 'Non-holdout score scope', 'warning', scoreSpace ? `${scope} | space=${scoreSpace}` : scope))
    } else {
      out.push(chip('score-scope', 'Score scope', 'neutral', scoreSpace ? `${scope} | space=${scoreSpace}` : scope))
    }
  } else if (scoreSpace) {
    out.push(chip('score-space', 'Score space', 'neutral', scoreSpace))
  }

  if (nativeFallbackReason) {
    out.push(chip('native-backend-fallback', 'Native backend fallback', 'warning', nativeBackendUsed ? `${nativeBackendUsed} | ${nativeFallbackReason}` : nativeFallbackReason))
  } else if (nativeBackendUsed) {
    out.push(chip('native-backend-used', 'Native backend used', 'info', nativeBackendUsed))
  }

  return out
}

export function validationFreshnessEvidence(input: ValidationFreshnessEvidenceInput): EvidenceChip[] {
  const out: EvidenceChip[] = []
  const currentRunSignature = text(input.currentRunSignature)
  const currentValidationSignature = text(input.currentValidationSignature)
  const expectedRunSignature = text(input.expectedRunSignature)
  const reportPath = text(input.reportPath)
  const reportExists = input.reportExists

  if (!currentRunSignature && !currentValidationSignature && !expectedRunSignature && !reportPath && reportExists == null) {
    return []
  }

  if (currentRunSignature && expectedRunSignature && currentRunSignature !== expectedRunSignature) {
    out.push(
      chip(
        'validation-stale',
        'Validation may be stale',
        'warning',
        'Current run signature differs from the validation signature.',
      ),
    )
  } else if (currentValidationSignature) {
    out.push(chip('validation-current', 'Validation loaded', 'success'))
  } else if (currentRunSignature) {
    out.push(chip('validation-missing', 'Validation not loaded', 'neutral'))
  }

  if (!reportPath) {
    out.push(chip('report-missing', 'Report not generated', 'neutral'))
  } else if (reportExists === false) {
    out.push(chip('report-file-missing', 'Report file missing', 'warning', reportPath))
  } else if (reportExists === true) {
    out.push(chip('report-available', 'Report available', 'success', reportPath))
  } else {
    out.push(chip('report-unknown', 'Report file not checked', 'neutral', reportPath))
  }

  return out
}

export function datasetProvenanceEvidence(input: DatasetProvenanceEvidenceInput): EvidenceChip[] {
  const out: EvidenceChip[] = []
  const datasetMeta = asRecord(input.datasetMeta)
  const lineage = asRecord(input.lineage)
  const timeframe = firstText(
    input.timeframe,
    datasetMeta.timeframe,
    datasetMeta.derived_timeframe,
    datasetMeta.derivedTimeframe,
    datasetMeta.base_timeframe,
    datasetMeta.baseTimeframe,
  )
  const fingerprint = firstText(
    input.sourceContentFingerprint,
    datasetMeta.source_content_fingerprint,
    datasetMeta.sourceContentFingerprint,
  )
  const csvPath = firstText(datasetMeta.csv_path, datasetMeta.csvPath, datasetMeta.path)
  const parentArtifactPath = firstText(lineage.parentArtifactPath, lineage.parent_artifact_path)
  const lineageRelation = firstText(lineage.lineageRelation, lineage.lineage_relation)

  if (timeframe || fingerprint || csvPath) {
    out.push(
      chip(
        'dataset-provenance',
        'Dataset provenance',
        fingerprint ? 'info' : 'neutral',
        routeDetail([
          timeframe ? `timeframe=${timeframe}` : '',
          fingerprint ? `fingerprint=${fingerprint}` : '',
          csvPath ? `source=${csvPath}` : '',
        ]),
      ),
    )
  }

  if (parentArtifactPath || lineageRelation) {
    out.push(
      chip(
        'artifact-lineage',
        'Artifact lineage',
        'info',
        routeDetail([
          lineageRelation ? `relation=${lineageRelation}` : '',
          parentArtifactPath ? `parent=${parentArtifactPath}` : '',
        ]),
      ),
    )
  }

  return out
}

export function liveReadinessEvidence(input: LiveReadinessEvidenceInput): EvidenceChip[] {
  if (input.liveWorkspaceAvailable == null) return []
  return [
    chip(
      'live-readiness-placeholder',
      'Live readiness not evaluated',
      'neutral',
      'Execution-store gating is intentionally not summarized by this helper yet.',
    ),
  ]
}
