# PR-02 Implementation Guide

Source response: `pr02_all_passes.md`

Decision: `Valid`, but only for route-aware, stage-specific exactness boundaries.

## Valid Scope
- [ ] Make post-ranking exactness explicitly route-aware and stage-specific.
- [ ] Allow replay-free exact rerank only for shortlist union and confidence stages, using retained summaries that preserve current comparators.
- [ ] Require slice-local or neighborhood-local retained state, or replay, for slice, omission, benchmark-relative slice, and neighborhood outputs.

## Non-Goals
- [ ] Do not invent one universal retained summary for all PR-02 outputs.
- [ ] Do not claim winner-row-only exact neighborhood robustness.
- [ ] Do not redesign unrelated ranking families outside the current PR-02 stages.

## Additive Guardrails
- [ ] Current comparator semantics must stay unchanged.
- [ ] Current routes that already replay for exactness must remain supported.
- [ ] Any replay avoidance must be opt-in through sufficient retained state, not by weakening outputs.

## Primary Touchpoints
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`

## Implementation Checklist
- [ ] Write down the owned stages explicitly: union rerank, confidence rerank, slice diagnostics, omission diagnostics, benchmark-relative slice diagnostics, neighborhood robustness.
- [ ] For union rerank, retain the exact inputs needed by the current comparator: shortlisted objective scores and `max_dd` tie-break data.
- [ ] For confidence rerank, retain the exact inputs needed by the current confidence comparator: current probability/confidence scalar, tie-break scalar, and `max_dd`.
- [ ] For slice order only, define the minimal retained tuple needed to reproduce current ordering.
- [ ] For slice and omission diagnostic parity, require final-partition slice-local metrics rows and slice-local scores.
- [ ] For benchmark-relative slice parity, require benchmark-relative slice-local state rather than full-period aggregates alone.
- [ ] For neighborhood robustness, require the local stability result itself or replay-equivalent state.
- [ ] Label any route-specific behavior where union or confidence does not materialize the same retained shortlist object.

## Verification Checklist
- [ ] Add route-aware parity tests for union rerank.
- [ ] Add route-aware parity tests for confidence rerank.
- [ ] Add regression coverage proving slice parity fails without slice-local state.
- [ ] Add regression coverage proving winner-only retained rows are insufficient for neighborhood exactness.
- [ ] Add tests for benchmark-relative slice parity under the current arithmetic-proxy semantics only.

## Rollout Checklist
- [ ] Update diagnostics/docs so each PR-02 output advertises whether it is replay-free exact, replay-backed exact, or heuristic.
- [ ] Keep current replay paths available as the reference implementation.
- [ ] Avoid changing UI labels unless they currently overclaim exactness.

## Kill Criteria
- [ ] Reject any patch that claims exactness across all PR-02 outputs with one retained summary.
- [ ] Reject any patch that weakens current comparator semantics to make replay avoidance easier.

