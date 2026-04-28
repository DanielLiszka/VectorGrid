import { describe, expect, it, vi } from 'vitest'
import {
  datasetProvenanceEvidence,
  liveReadinessEvidence,
  routeFallbackEvidence,
  scoreScopeEvidence,
  validationFreshnessEvidence,
} from './workflowEvidenceModel'

describe('workflowEvidenceModel', () => {
  it('is pure and does not read localStorage', () => {
    const input = Object.freeze({
      routeProvenance: Object.freeze({
        requestedRoute: 'GpuKernel',
        realizedRoute: 'CpuOnly',
        fallbackReason: 'kernel unavailable',
      }),
    })
    const getItem = vi.fn(() => {
      throw new Error('localStorage must not be read')
    })
    vi.stubGlobal('localStorage', { getItem })

    const chips = routeFallbackEvidence(input)

    expect(chips).toEqual([
      {
        id: 'route-fallback',
        label: 'Route fallback',
        detail: 'requested=GpuKernel | actual=CpuOnly | fallback=kernel unavailable',
        tone: 'warning',
        actionLocal: true,
      },
    ])
    expect(input.routeProvenance.realizedRoute).toBe('CpuOnly')
    expect(getItem).not.toHaveBeenCalled()
    vi.unstubAllGlobals()
  })

  it('does not warn on CPU-only route evidence unless fallback is recorded', () => {
    expect(routeFallbackEvidence({ backendUsed: 'CpuOnly' })).toEqual([
      {
        id: 'route-used',
        label: 'CPU-only route',
        detail: 'No fallback recorded.',
        tone: 'neutral',
        actionLocal: true,
      },
    ])
  })

  it('warns for non-holdout ML score scopes and native backend fallback', () => {
    expect(
      scoreScopeEvidence({
        scoreSpace: 'validation_scores',
        scoreScope: 'CalibrationSlice',
        nativeBackendUsed: 'native_cpu',
        nativeFallbackReason: 'native_cuda unavailable',
      }),
    ).toEqual([
      {
        id: 'score-scope',
        label: 'Non-holdout score scope',
        detail: 'CalibrationSlice | space=validation_scores',
        tone: 'warning',
        actionLocal: true,
      },
      {
        id: 'native-backend-fallback',
        label: 'Native backend fallback',
        detail: 'native_cpu | native_cuda unavailable',
        tone: 'warning',
        actionLocal: true,
      },
    ])
  })

  it('treats walk-forward test score scope as complete evidence', () => {
    expect(scoreScopeEvidence({ scoreScope: 'WalkforwardTest' })).toEqual([
      {
        id: 'score-scope',
        label: 'Walk-forward score scope',
        tone: 'success',
        actionLocal: true,
      },
    ])
  })

  it('separates missing report path from missing known report file', () => {
    expect(validationFreshnessEvidence({ currentRunSignature: 'run-a' })).toContainEqual({
      id: 'report-missing',
      label: 'Report not generated',
      tone: 'neutral',
      actionLocal: true,
    })

    expect(
      validationFreshnessEvidence({
        currentRunSignature: 'run-a',
        currentValidationSignature: 'validation-a',
        expectedRunSignature: 'run-a',
        reportPath: 'C:/runs/report.xlsx',
        reportExists: false,
      }),
    ).toContainEqual({
      id: 'report-file-missing',
      label: 'Report file missing',
      detail: 'C:/runs/report.xlsx',
      tone: 'warning',
      actionLocal: true,
    })
  })

  it('warns when validation was produced for a different run signature', () => {
    expect(
      validationFreshnessEvidence({
        currentRunSignature: 'current-run',
        currentValidationSignature: 'validation-a',
        expectedRunSignature: 'previous-run',
      }),
    ).toContainEqual({
      id: 'validation-stale',
      label: 'Validation may be stale',
      detail: 'Current run signature differs from the validation signature.',
      tone: 'warning',
      actionLocal: true,
    })
  })

  it('summarizes dataset provenance and lineage from existing artifact fields', () => {
    expect(
      datasetProvenanceEvidence({
        datasetMeta: {
          csv_path: 'C:/data/spy.csv',
          timeframe: '15m',
          source_content_fingerprint: 'sha256:abc',
        },
        lineage: {
          lineage_relation: 'derived_from',
          parent_artifact_path: 'C:/runs/parent.json',
        },
      }),
    ).toEqual([
      {
        id: 'dataset-provenance',
        label: 'Dataset provenance',
        detail: 'timeframe=15m | fingerprint=sha256:abc | source=C:/data/spy.csv',
        tone: 'info',
        actionLocal: true,
      },
      {
        id: 'artifact-lineage',
        label: 'Artifact lineage',
        detail: 'relation=derived_from | parent=C:/runs/parent.json',
        tone: 'info',
        actionLocal: true,
      },
    ])
  })

  it('returns bounded empty or neutral output for unsupported evidence contexts', () => {
    expect(routeFallbackEvidence({})).toEqual([])
    expect(scoreScopeEvidence({})).toEqual([])
    expect(validationFreshnessEvidence({})).toEqual([])
    expect(datasetProvenanceEvidence({})).toEqual([])
    expect(liveReadinessEvidence({})).toEqual([])
    expect(liveReadinessEvidence({ liveWorkspaceAvailable: true })).toEqual([
      {
        id: 'live-readiness-placeholder',
        label: 'Live readiness not evaluated',
        detail: 'Execution-store gating is intentionally not summarized by this helper yet.',
        tone: 'neutral',
        actionLocal: true,
      },
    ])
  })
})
