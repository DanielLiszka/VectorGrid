import { describe, expect, it } from 'vitest'
import { buildMetaSelectionRequest } from './metaRequestBuilder'

describe('metaRequestBuilder', () => {
  it('builds baseline meta request with holdout disabled', () => {
    const req = buildMetaSelectionRequest({
      strategyKind: 'CompositeMo',
      timeframe: '',
      requiredTag: '',
      minSamples: '30',
      topK: '25',
      evalHoldout: false,
      groupingMode: 'clustered',
      clusterIntBucket: '5',
      clusterFloatBucket: '0.25',
    })

    expect(req).toMatchObject({
      strategy_kind: 'CompositeMo',
      timeframe: null,
      required_tag: null,
      eval_holdout: false,
      holdout_mode: 'light',
      holdout_validation: null,
      min_samples: 30,
      top_k: 25,
      strict_shortlist: true,
      allow_shortlist_fallback: false,
    })
  })

  it('builds heavy holdout validation payload when enabled', () => {
    const req = buildMetaSelectionRequest({
      strategyKind: 'CompositeMo',
      evalHoldout: true,
      holdoutMode: 'heavy',
      holdoutTag: 'holdout-a',
      holdoutEvalTopN: '10',
      holdoutExploreN: '3',
      holdoutRankBy: 'auto',
      saveHoldoutRows: true,
      valSeed: '3',
      valWfWindow: '4000',
      valWfStep: '500',
      valPerturbEnable: true,
      valPerturbIters: '12',
      valPerturbMin: '0.01',
      valPerturbMax: '0.08',
      valMcEnable: true,
      valMcSims: '800',
    })

    expect(req).toMatchObject({
      eval_holdout: true,
      holdout_mode: 'heavy',
      holdout_tag: 'holdout-a',
      holdout_eval_top_n: 10,
      holdout_eval_explore_n: 3,
      save_holdout_rows: true,
      holdout_validation: {
        seed: 3,
        walk_forward: { window_bars: 4000, step_bars: 500 },
        perturb: { iterations: 12, pct_min: 0.01, pct_max: 0.08, flip_bool_prob: 0.02 },
        trade_bootstrap: { simulations: 800 },
        validation_engine: 'iterative',
      },
    })
  })

  it('disables holdout for unsupported strategy kinds', () => {
    const req = buildMetaSelectionRequest({
      strategyKind: 'UnknownStrategy',
      evalHoldout: true,
      holdoutMode: 'heavy',
    })

    expect(req).toMatchObject({
      strategy_kind: 'UnknownStrategy',
      eval_holdout: false,
      holdout_mode: 'light',
      holdout_validation: null,
    })
  })
})
