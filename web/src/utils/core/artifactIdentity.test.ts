import { describe, expect, it } from 'vitest'
import { computeConfigFingerprint } from './artifactIdentity'

describe('artifactIdentity', () => {
  it('ignores derived admission provenance when hashing validation config', () => {
    const runReq = {
      strategy_kind: 'DoubleMa',
      data_id: 'run-a',
      objective: 'Sharpe',
    }
    const baseValidationReq = {
      data_id: 'val-a',
      candidates: [{ fast_len: 5, slow_len: 20 }],
      admission_policy: 'scalar_union',
      admission_provenance: {
        requested_policy: 'scalar_union',
        applied_policy: 'scalar_union',
        source_kind: 'top_rows',
        source_candidate_count: 8,
        pre_dedupe_count: 16,
        post_dedupe_count: 8,
        post_cap_count: 4,
        heads: [
          { head: 'primary', objective: 'Sharpe', candidate_count: 8 },
        ],
      },
      cross_ticker: {
        datasets: [
          { csv_path: 'b.csv', label: 'B' },
          { csv_path: 'a.csv', label: 'A' },
        ],
      },
    }

    const variantValidationReq = {
      ...baseValidationReq,
      data_id: 'val-b',
      candidates: [{ fast_len: 12, slow_len: 48 }],
      admission_provenance: {
        requested_policy: 'scalar_union',
        applied_policy: 'scalar_union',
        source_kind: 'best_params_fallback',
        source_candidate_count: 2,
        pre_dedupe_count: 2,
        post_dedupe_count: 2,
        post_cap_count: 2,
        heads: [
          { head: 'primary', objective: 'Sharpe', candidate_count: 2 },
        ],
      },
      cross_ticker: {
        datasets: [
          { csv_path: 'a.csv', label: 'A2' },
          { csv_path: 'b.csv', label: 'B2' },
        ],
      },
    }

    expect(
      computeConfigFingerprint(runReq, baseValidationReq),
    ).toBe(computeConfigFingerprint({ ...runReq, data_id: 'run-b' }, variantValidationReq))
  })

  it('stays stable under key ordering noise but changes for real request changes', () => {
    const runReqA = {
      strategy_kind: 'DoubleMa',
      objective: 'Sharpe',
      backend: { backend: 'CpuOnly', device_id: 0 },
      data_id: 'data-a',
    }
    const runReqB = {
      data_id: 'data-b',
      backend: { device_id: 0, backend: 'CpuOnly' },
      objective: 'Sharpe',
      strategy_kind: 'DoubleMa',
    }
    const validationReq = {
      max_candidates: 12,
      seed: 7,
      admission_policy: 'scalar_union',
    }

    const stableA = computeConfigFingerprint(runReqA, validationReq)
    const stableB = computeConfigFingerprint(runReqB, {
      seed: 7,
      admission_policy: 'scalar_union',
      max_candidates: 12,
    })
    const changed = computeConfigFingerprint(
      { ...runReqA, objective: 'NetProfit' },
      validationReq,
    )

    expect(stableA).toBe(stableB)
    expect(changed).not.toBe(stableA)
  })

  it('excludes reproducibility telemetry from semantic config identity', () => {
    const runReq = {
      strategy_kind: 'DoubleMa',
      objective: 'Sharpe',
      reproducibility: {
        environment: {
          generated_at_utc: '2026-04-28T12:00:00Z',
          free_memory_bytes: 123,
        },
      },
    }
    const validationReq = {
      seed: 7,
      reproducibility: {
        kernel: {
          status: 'blocked_pending_build_release_file_review',
        },
      },
    }

    expect(computeConfigFingerprint(runReq, validationReq)).toBe(
      computeConfigFingerprint(
        {
          ...runReq,
          reproducibility: {
            environment: {
              generated_at_utc: '2026-04-28T12:30:00Z',
              free_memory_bytes: 456,
            },
          },
        },
        {
          ...validationReq,
          reproducibility: {
            kernel: {
              status: 'unavailable',
            },
          },
        },
      ),
    )
  })
})
