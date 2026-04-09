# Prompt 2 metric extract

Sources:
- spy_warm_phase05_summary.json
- qqq_warm_phase05_summary.json

## SPY warm
- candidate_density: 0.5246913580
- invalid_slot_rate: 0.4753086420
- overlap_window.search_overlap_rate: 1.0
- overlap_window.holdout_overlap_rate: 1.0
- overlap_window.sealed_test_overlap_rate: 1.0
- coverage_adequacy.verdict: holdout_coverage_weak
- coverage_adequacy.non_promotable: false
- event_coverage.search_events_with_candidates: 7
- event_coverage.holdout_events_with_candidates: 0
- event_coverage.sealed_test_events_with_candidates: 4
- event_coverage.search_events_with_baseline_outcomes: 7
- event_coverage.holdout_events_with_baseline_outcomes: 0
- event_coverage.sealed_test_events_with_baseline_outcomes: 4
- invalid_reasons:
  - no_strike_in_tolerance: 36
  - entry_past_cutoff: 272
- split_invalid_reasons.search_invalid_reasons:
  - no_strike_in_tolerance: 36
  - entry_past_cutoff: 118
- split_invalid_reasons.holdout_invalid_reasons:
  - entry_past_cutoff: 108
- split_invalid_reasons.sealed_test_invalid_reasons:
  - entry_past_cutoff: 46
- best_available_backend: gpu_compiled_tensor_hybrid
- auto_backend_decision.recommended_backend: gpu_compiled_tensor_hybrid
- auto_backend_decision.gpu_eligible: true

## QQQ warm
- candidate_density: 0.2513484358
- invalid_slot_rate: 0.7486515642
- overlap_window.search_overlap_rate: 0.2295081967
- overlap_window.holdout_overlap_rate: 1.0
- overlap_window.sealed_test_overlap_rate: 0.8181818182
- coverage_adequacy.verdict: weak_dataset_overlap
- coverage_adequacy.non_promotable: true
- event_coverage.search_events_with_candidates: 10
- event_coverage.holdout_events_with_candidates: 11
- event_coverage.sealed_test_events_with_candidates: 10
- event_coverage.search_events_with_baseline_outcomes: 10
- event_coverage.holdout_events_with_baseline_outcomes: 11
- event_coverage.sealed_test_events_with_baseline_outcomes: 10
- invalid_reasons:
  - no_price_row_after_entry: 145
  - no_strike_in_tolerance: 9
  - entry_past_cutoff: 930
  - contract_not_eligible_as_of_event: 1692
- split_invalid_reasons.search_invalid_reasons:
  - no_strike_in_tolerance: 9
  - entry_past_cutoff: 191
  - contract_not_eligible_as_of_event: 1692
- split_invalid_reasons.holdout_invalid_reasons:
  - entry_past_cutoff: 417
- split_invalid_reasons.sealed_test_invalid_reasons:
  - no_price_row_after_entry: 145
  - entry_past_cutoff: 322
- best_available_backend: gpu_compiled_tensor_hybrid
- auto_backend_decision.recommended_backend: gpu_compiled_tensor_hybrid
- auto_backend_decision.gpu_eligible: true
