import type { FixtureFamilyModule, MockTauriScenario } from './types'
import { validationDatasetMismatchScenario, validationSuccessScenario } from './scenarios'

function cloneRecord<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function buildValidationScenarioVariant(input: {
  name: string
  description: string
  result: Record<string, unknown>
}): MockTauriScenario {
  return {
    ...validationSuccessScenario,
    name: input.name,
    family: 'validation',
    description: input.description,
    commandResults: {
      ...validationSuccessScenario.commandResults,
      run_double_ma_validation: input.result,
    },
  }
}

const reviewValidationResult = cloneRecord(
  validationSuccessScenario.commandResults.run_double_ma_validation as Record<string, unknown>,
)
reviewValidationResult.candidates = [
  {
    params: {
      fast_len: 8,
      slow_len: 32,
      fast_ma_type: 'sma',
      slow_ma_type: 'sma',
    },
    in_sample: {
      pnl: 0.12,
      sharpe: 1.4,
      max_dd: -0.05,
      trades: 18,
    },
    robust_score: 1.21,
    psr: 0.91,
    dsr: 0.65,
    track_record_bars: 25000,
    min_track_record_length_bars: 20000,
    robust_pareto: true,
    validation_verdict: {
      passed: true,
      approved: true,
      final_recommendation: 'approved_with_warnings',
      warning_flags: ['high_pbo'],
    },
    walk_forward: {
      windows: 4,
      sharpe_worst: 0.35,
      pnl_worst: -0.02,
    },
    trade_bootstrap: {
      simulations: 500,
      trade_count: 35,
      prob_pnl_positive: 0.62,
      pnl_p05: -0.04,
    },
    friction_stress: {
      pass_count: 3,
      scenario_count: 4,
      pass_frac: 0.75,
      worst_scenario_label: 'fee_2x',
    },
  },
]
reviewValidationResult.cscv_pbo = {
  pbo: 0.12,
  combinations: 100,
  combinations_possible: 128,
}
reviewValidationResult.spa = {
  p_value: 0.041,
  passed: true,
  slice_count: 16,
  bootstrap_samples: 500,
}
reviewValidationResult.run_verdict = {
  passed: true,
  approved: true,
  candidates_tested: 4,
  candidates_survived_hard_gates: 2,
  approved_candidate_count: 1,
  final_recommendation: 'approved_with_warnings',
  warning_flags: ['high_pbo'],
  first_approved_rank: 1,
  top_candidate_psr: 0.91,
  top_candidate_dsr: 0.65,
  top_candidate_track_record_bars: 25000,
  top_candidate_min_track_record_length_bars: 20000,
  pbo: 0.12,
  max_pbo: 0.2,
}

const rejectedValidationResult = cloneRecord(reviewValidationResult)
rejectedValidationResult.candidates = [
  {
    ...((reviewValidationResult.candidates as Record<string, unknown>[])[0] ?? {}),
    robust_score: 0.42,
    psr: 0.84,
    dsr: 0.66,
    validation_verdict: {
      passed: true,
      approved: false,
      final_recommendation: 'rejected',
      warning_flags: ['run_level_rejection'],
    },
  },
]
rejectedValidationResult.cscv_pbo = {
  pbo: 0.48,
  combinations: 100,
  combinations_possible: 128,
}
rejectedValidationResult.spa = {
  p_value: 0.31,
  passed: false,
  slice_count: 16,
  bootstrap_samples: 500,
}
rejectedValidationResult.run_verdict = {
  passed: false,
  approved: false,
  candidates_tested: 4,
  candidates_survived_hard_gates: 1,
  approved_candidate_count: 0,
  final_recommendation: 'rejected',
  fail_reasons: ['high_pbo', 'spa_failed'],
  warning_flags: ['high_pbo', 'spa_failed'],
  first_approved_rank: 0,
  top_candidate_psr: 0.84,
  top_candidate_dsr: 0.66,
  pbo: 0.48,
  max_pbo: 0.2,
}

export const validationReviewScenario = buildValidationScenarioVariant({
  name: 'validation-review',
  description:
    'A matching dataset produces an approved-with-warnings validation result that needs evidence review.',
  result: reviewValidationResult,
})

export const validationRejectedScenario = buildValidationScenarioVariant({
  name: 'validation-rejected',
  description:
    'A matching dataset produces a rejected validation result while keeping result evidence inspectable.',
  result: rejectedValidationResult,
})

export {
  validationDatasetMismatchScenario,
  validationSuccessScenario,
}

export const validationFixtureFamily: FixtureFamilyModule = {
  family: 'validation',
  description: 'Validation readiness, mismatch, result, and evidence browser QA scenarios.',
  scenarios: [
    validationDatasetMismatchScenario,
    validationSuccessScenario,
    validationReviewScenario,
    validationRejectedScenario,
  ],
}
