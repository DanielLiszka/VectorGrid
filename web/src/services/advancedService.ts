type BoolDomain = 'Bool'

interface IntRangeDomain {
  IntRange: { low: number; high: number; step: number; log: boolean }
}

interface FloatRangeDomain {
  FloatRange: { low: number; high: number; log: boolean }
}

interface CategoricalDomain {
  Categorical: { choices: Array<string | number | boolean> }
}

type SearchParamDomain = BoolDomain | IntRangeDomain | FloatRangeDomain | CategoricalDomain

interface SearchParam {
  name: string
  domain: SearchParamDomain
}

export interface CompositeMoSearchSpace {
  params: SearchParam[]
}

export interface AdvancedResetMoSearchSpaceSummary {
  searchSpace: CompositeMoSearchSpace
  json: string
}

function defaultCompositeMoSearchSpace(): CompositeMoSearchSpace {
  return {
    params: [
      { name: 'halftrend_amplitude', domain: { IntRange: { low: 2, high: 20, step: 1, log: false } } },
      { name: 'halftrend_channel_deviation', domain: { FloatRange: { low: 0.5, high: 5.0, log: false } } },
      { name: 'halftrend_atr_period', domain: { IntRange: { low: 20, high: 200, step: 1, log: false } } },
      { name: 'macz_fast_length', domain: { IntRange: { low: 5, high: 30, step: 1, log: false } } },
      { name: 'macz_slow_length', domain: { IntRange: { low: 10, high: 60, step: 1, log: false } } },
      { name: 'macz_signal_length', domain: { IntRange: { low: 3, high: 30, step: 1, log: false } } },
      { name: 'macz_lengthz', domain: { IntRange: { low: 5, high: 50, step: 1, log: false } } },
      { name: 'macz_length_stdev', domain: { IntRange: { low: 5, high: 80, step: 1, log: false } } },
      { name: 'macz_a', domain: { FloatRange: { low: -2.0, high: 2.0, log: false } } },
      { name: 'macz_b', domain: { FloatRange: { low: -2.0, high: 2.0, log: false } } },
      { name: 'macz_gamma', domain: { FloatRange: { low: 0.0, high: 0.2, log: false } } },
      { name: 'macz_use_lag', domain: 'Bool' },
      { name: 'macz_use_volume', domain: 'Bool' },
      { name: 'ott_period', domain: { IntRange: { low: 1, high: 50, step: 1, log: false } } },
      { name: 'ott_percent', domain: { FloatRange: { low: 0.1, high: 5.0, log: false } } },
      {
        name: 'ott_ma_type',
        domain: { Categorical: { choices: ['VAR', 'EMA', 'SMA', 'WMA', 'TMA', 'WWMA', 'ZLEMA', 'TSF'] } },
      },
      { name: 'strategy_commission', domain: { FloatRange: { low: 0.0, high: 0.001, log: false } } },
      { name: 'strategy_eps_rel', domain: { FloatRange: { low: 0.0, high: 0.005, log: false } } },
      { name: 'strategy_long_only', domain: 'Bool' },
      { name: 'strategy_allow_flip', domain: 'Bool' },
    ],
  }
}

export async function resetMoSearchSpace(): Promise<AdvancedResetMoSearchSpaceSummary> {
  const searchSpace = defaultCompositeMoSearchSpace()
  return {
    searchSpace,
    json: JSON.stringify(searchSpace, null, 2),
  }
}
