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
})
