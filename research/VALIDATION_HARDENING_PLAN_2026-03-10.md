# Validation Hardening Plan

Date: 2026-03-10

Scope: strengthen the validation stage so it does a better job preventing overfitting, selection bias, and false confidence in optimized strategies.

This document is a planning artifact. It is intentionally biased toward low-risk extensions of the current stack rather than rewriting the research workflow from scratch.

## Goal

Turn validation into a much stronger filter between:

- optimization output
- meta-analysis input
- any final strategy decision

The target state is not just "more metrics". The target state is:

- fewer false positives
- better protection against data-snooping
- clearer pass/fail semantics
- more trustworthy ranking of candidate parameter sets

## Research Summary

The most credible anti-overfitting approach for a strategy-search app like this is not a single statistic. It is a layered process:

1. Generate candidates on a train region only.
2. Validate those candidates with multiple out-of-sample and robustness tests.
3. Apply explicit multiple-testing / data-snooping correction.
4. Reserve a final untouched holdout for last-stage confirmation.
5. Prefer elimination rules and confidence sets over winner-take-all ranking whenever the evidence is weak.

The literature points in this direction:

- Bailey et al. argue that ordinary holdout is unreliable for investment backtests and propose CSCV/PBO to estimate the probability of backtest overfitting.
  Source: SSRN / Journal of Computational Finance, "The Probability of Backtest Overfitting"
  https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253

- Lopez de Prado's Deflated Sharpe Ratio is specifically designed to correct for non-normal returns and multiple testing / selection bias.
  Source: SSRN, "Deflating the Sharpe Ratio"
  https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2465675

- Hansen's SPA test and White's Reality Check are core data-snooping corrections when many trading rules or model variants are tried.
  Source: IDEAS / Brown working paper entry for Hansen SPA
  https://ideas.repec.org/p/bro/econwp/2001-06.html

- The Model Confidence Set is useful when the data are not strong enough to identify one clearly superior model; it returns a set of statistically indistinguishable survivors instead.
  Source: IDEAS / Econometrica paper entry
  https://ideas.repec.org/p/aah/create/2010-76.html

- Clark shows that out-of-sample forecast comparisons can help prevent overfitting if selection is confined to the in-sample region.
  Source: IDEAS / Journal of Forecasting paper entry
  https://ideas.repec.org/p/fip/fedkrw/rwp00-05.html

- Giacomini and White push the evaluation question toward forecasting methods rather than just fitted models, which is relevant to full optimize-then-validate workflows.
  Source: IDEAS / Econometrica working paper entry
  https://ideas.repec.org/p/boc/bocoec/572.html

- More recent work still points toward combinatorial / purged cross-validation methods as stronger than plain walk-forward in some controlled settings, especially for model selection.
  Source: SSRN, "Backtest Overfitting in the Machine Learning Era"
  https://papers.ssrn.com/sol3/Delivery.cfm/SSRN_ID4686376_code4361537.pdf?abstractid=4686376&mirid=1

- White's Reality Check remains one of the foundational bootstrap-based corrections for data snooping across a searched rule universe.
  Source: White (2000) PDF
  https://www.ntuzov.com/Nik_Site/Niks_files/Research/papers/mut_funds/White_2000.pdf

- False Discovery Rate control has also been applied directly to technical trading rule universes and can be more decision-friendly than winner-only tests.
  Source: Journal of Financial Economics digest/result trail and related paper coverage
  https://www.sciencedirect.com/science/article/abs/pii/S0304405X1200116X
  Source: spread trading / data-snooping control discussion
  https://www.sciencedirect.com/science/article/pii/S0169207021001655

- The "strategy approval" line of work is relevant because validation should not only rank strategies, but decide whether the evidence is sufficient to approve one at all.
  Source: SSRN, "The Strategy Approval Decision: A Sharpe Ratio Indifference Curve Approach"
  https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2003638

- Minimum Track Record Length is relevant when deciding whether a validation result is too short / too weak to trust even if the estimated Sharpe looks good.
  Source cluster: Bailey / López de Prado Sharpe-ratio work as indexed on QuantResearch
  https://www.quantresearch.org/Publications.htm

## Current State In This Repo

The current validation stack is already stronger than a typical retail quant tool.

Implemented today:

- Walk-forward validation
  - crates/app_core/src/double_ma_validate.rs
  - crates/app_core/src/composite_validate.rs
  - crates/app_core/src/validation_pipeline.rs
- Parameter perturbation
  - crates/app_core/src/double_ma_validate.rs
  - crates/app_core/src/composite_validate.rs
- Trade bootstrap / Monte Carlo
  - crates/app_core/src/validation_pipeline.rs
- Cross-ticker validation
  - src-tauri/src/commands_validation.rs
  - crates/app_core/src/composite_validate.rs
- CSCV / PBO
  - crates/app_core/src/cscv_pbo.rs
- PSR / DSR
  - crates/app_core/src/validation_stats.rs
- Local stability surfaces
  - crates/app_core/src/double_ma.rs
  - crates/app_core/src/composite_cpu.rs
- Meta-stage holdout aggregation
  - crates/app_core/src/meta_select.rs
  - src-tauri/src/commands_meta.rs

That is a good base. The issue is not lack of tools. The issue is that several of them are still softer than they should be.

## Main Weaknesses

### 1. CSCV/PBO is not a hard backend gate

Today CSCV contributes candidate stats and can influence shortlist ranking, but it is not yet treated as a first-class reject condition in the backend.

Current behavior:

- candidate CSCV stats are attached in validation
- CSCV OOS median is folded into robust score
- frontend shortlist uses CSCV thresholds

Problem:

- a candidate can still survive deeper into the workflow without an explicit backend-level CSCV rejection rule
- CSCV/PBO is being used more like a helpful feature than a central anti-overfitting defense

### 2. PSR/DSR exist, but they are optional

This is better than not having them, but it means many runs can still proceed without any probability-based confidence floor.

Problem:

- the default validation experience is still permissive
- users can rank by robust score while ignoring uncertainty in the Sharpe estimate

### 3. Walk-forward is useful, but still a fixed-candidate test

Current walk-forward evaluates an already-selected candidate across rolling windows.

That is good for robustness, but it is not the strongest possible walk-forward setup because it does not model:

- re-optimization over time
- parameter drift
- strategy-selection instability across windows

This is a meaningful limitation for article-grade anti-overfitting claims.

### 4. No explicit multiple-testing correction across the full candidate universe

The app searches many parameter combinations. CSCV/PBO helps, and DSR helps, but there is still no direct implementation of:

- White's Reality Check
- Hansen SPA
- Model Confidence Set

This is the biggest statistical gap if the claim is "the winning strategy survived data-snooping correction".

### 5. Local stability is incomplete across strategy families

Local stability exists, but not uniformly:

- VCC local stability is not implemented
- ROCB local stability is not implemented

That weakens consistency across strategies.

### 6. Meta holdout is helpful but not strict enough to serve as the sole final defense

Meta holdout is valuable, but current group gating is still relatively permissive.

Problem:

- robust_score_p10 > 0 and pass-rate / drawdown bounds are reasonable filters
- but they are not strong enough to substitute for more formal data-snooping correction

## Second-Pass Findings

The first plan was directionally right, but a second literature pass suggests we should explicitly account for a few additional items.

### A. Purging and embargoing were underemphasized

If train and test windows can leak information through overlapping horizons, nearby bars, or event spans, standard rolling validation can still be too optimistic.

Practical implication for this app:

- if validation or future ML layers use forward horizons, delayed outcomes, or overlapping label windows, add purging/embargo logic
- if validation remains purely bar-by-bar rule evaluation with fixed causal signals, this is less urgent than SPA, but still relevant for stronger walk-forward design

Recommendation:

- add purged / embargoed split support as an optional advanced validation mode
- keep it below Phase 1 and Phase 3 in priority, but above cosmetic UI work

### B. FDR-style control may be a better product fit than only RC / SPA

Reality Check and SPA answer:

- "Does at least one rule beat the benchmark after snooping correction?"

False Discovery Rate approaches answer:

- "Among the rules we call significant, how many are likely false positives?"

For product UX, FDR can be easier to operationalize because it naturally supports:

- survivor counts
- acceptance sets
- user-facing confidence bands around a shortlist

Recommendation:

- keep SPA as the first formal data-snooping test to implement
- but treat FDR control as a strong candidate for the next search-level method after SPA

### C. Validation should support "approve / do not approve", not just ranking

The strategy-approval literature is a reminder that the decision is not:

- which candidate is best?

It is:

- is any candidate good enough to approve after considering uncertainty, correlation, and implementation risk?

Recommendation:

- add an approval-oriented run verdict
- include a "no candidate approved" outcome as a normal and expected result

### D. Minimum track record length is missing from the current design

Even when PSR / DSR are present, users still benefit from an explicit answer to:

- how much data would be needed before this Sharpe estimate is actually trustworthy?

Recommendation:

- add MinTRL-style reporting to candidate validation output
- use it as a warning or hard gate in production mode

### E. The bootstrap design itself matters

For RC / SPA / MCS, the choice of bootstrap under serial dependence is not incidental.

Recommendation:

- when implementing RC / SPA / MCS, prefer a time-series-aware bootstrap design rather than naive IID resampling
- document the bootstrap choice in output artifacts

## Recommended Validation Design

The recommended target design is:

### Layer A: Candidate-level robustness

For each optimized candidate:

- in-sample metrics
- iterative execution parity metrics where relevant
- rolling walk-forward stats
- perturbation stats
- trade bootstrap stats
- local stability score
- CSCV candidate stats
- PSR / DSR
- cross-ticker stats

This layer answers:

- is this candidate stable?
- is its Sharpe believable?
- is its edge concentrated in fragile regions?

### Layer B: Search-level overfitting control

For the full candidate set from the optimization run:

- CSCV/PBO summary
- DSR-aware filtering
- SPA or Reality Check against benchmark / null
- optional MCS survivor set instead of single winner

This layer answers:

- did the search likely overfit?
- is the best candidate statistically better than the benchmark after snooping correction?
- are several candidates effectively tied once uncertainty is acknowledged?

### Layer C: Dataset-level external confirmation

Across other assets / related datasets:

- cross-ticker validation
- grouped meta aggregation
- holdout evaluation
- optional heavier holdout validation for finalists

This layer answers:

- does the candidate generalize outside the source dataset?

### Layer D: Final untouched confirmation

Reserve a final chronological holdout or dedicated dataset holdout that is not used for candidate generation or repeated tweaking.

This layer answers:

- after all research choices are locked, does the strategy still hold up?

## Concrete Direction For This App

The best near-term path is not to add ten new statistics at once. It is:

1. Make existing defenses stricter.
2. Add one real multiple-testing correction.
3. Improve the walk-forward design for finalists.

## Proposed Implementation Phases

## Phase 1: Harden Existing Validation Defaults

Objective: make the current validation stack materially stricter without major architecture change.

Changes:

- Add backend-level hard gate options for:
  - minimum PSR
  - minimum DSR
  - minimum CSCV OOS rank fraction
  - minimum CSCV OOS metric
  - maximum allowed PBO
- Make validation output classify each candidate as:
  - pass
  - warning
  - fail
- Add explicit rejection reasons per candidate and per run
- Add stricter recommended defaults in UI for:
  - walk-forward on
  - CSCV on
  - PSR/DSR floors non-empty

Why this phase first:

- highest ROI
- uses existing code paths
- low risk to performance

## Phase 2: Strengthen Walk-Forward

Objective: move from simple rolling robustness toward stronger time-aware validation.

Changes:

- Add a finalist-only re-optimization walk-forward mode:
  - optimize on train window
  - evaluate selected params on next OOS window
  - roll forward
- Keep this limited to top N finalists to control cost
- Track:
  - chosen params by window
  - window-to-window parameter drift
  - OOS hit rate
  - OOS Sharpe / drawdown distribution

Why:

- this gives a much more credible answer to "would this process have held up live?"

## Phase 3: Add A Full Search-Level Data-Snooping Test

Objective: formally correct for trying many rules / parameter combinations.

Recommended first addition:

- Hansen SPA

Why SPA first:

- directly relevant to "best candidate vs benchmark after snooping correction"
- stronger and more modern than relying on raw top-ranked backtest score
- more appropriate for large candidate universes than a cosmetic extra metric

Possible follow-up:

- White Reality Check
- Model Confidence Set
- False Discovery Rate control for large survivor sets
- Minimum Track Record Length reporting

Recommended product behavior:

- if SPA fails, label the whole run as "no statistically superior candidate confirmed"
- if MCS has many survivors, avoid pretending one winner is special

## Phase 4: Tighten Meta-Stage Acceptance

Objective: stop weak validation results from becoming strong-looking meta winners.

Changes:

- raise minimum robustness gates before meta ingestion
- make hard-shortlist mode the true default for production workflows
- include stronger holdout-derived penalties in meta scoring
- add meta warnings when:
  - holdout rows are sparse
  - group pass-rate is weak
  - group uncertainty is high

## Phase 5: Fill Strategy-Coverage Gaps

Objective: consistency across all exposed strategy families.

Changes:

- implement local stability for VCC
- implement local stability for ROCB
- confirm CSCV / probability gates / walk-forward parity across all exposed strategies

## Proposed New Validation Concepts

### Candidate Verdict Model

Each candidate should expose:

- `hard_pass`
- `warning_flags: []`
- `fail_reasons: []`
- `confidence_score`

This is better than forcing the user to interpret ten columns manually.

### Run-Level Validation Verdict

Each validation run should expose:

- `search_overfit_risk`
- `candidate_count_tested`
- `candidates_survived_hard_gates`
- `spa_pvalue` or equivalent
- `mcs_survivor_count` if implemented
- `final_recommendation`

Example recommendation states:

- `no_candidate_survived`
- `candidates_survived_but_search_not_statistically_superior`
- `small_survivor_set_confirmed`
- `single_candidate_supported`
- `evidence_too_short_mintrl_not_met`

### Separate "Research" vs "Production" Validation Modes

Research mode:

- more permissive
- allows exploration
- keeps warnings visible

Production mode:

- hard shortlist only
- stronger defaults
- no fallback when CSCV / cross / probability filters empty out the set

## Recommended Order Of Work

1. Backend hard gates for existing metrics.
2. Frontend validation verdicts and rejection reasons.
3. Re-optimization walk-forward for finalists.
4. SPA implementation with time-series-aware bootstrap.
5. MinTRL reporting and approval-style verdicts.
6. Optional MCS or FDR implementation.
7. Purged / embargoed split support where leakage risk exists.
8. Fill local-stability gaps for missing strategies.

## Risks

- Making validation too strict too quickly may zero out many current runs.
- SPA / Reality Check can be computationally expensive if naively implemented over very large candidate universes.
- Re-optimization walk-forward can become very slow unless restricted to finalists.
- Multiple-testing corrections can confuse users if the UX only shows one top row and hides uncertainty.

## Non-Goals

- Do not rewrite optimization.
- Do not replace the current validation stack wholesale.
- Do not add exotic statistics with unclear decision value.
- Do not turn the UI into a research notebook before hardening the backend rules.

## Implementation Checklist

### Phase 1: Harden existing validation

- [ ] Add backend config fields for hard validation gates:
  - [ ] min CSCV OOS rank fraction
  - [ ] min CSCV OOS metric
  - [ ] max run-level PBO
  - [ ] minimum local-stability score
- [ ] Add candidate `warning_flags` and `fail_reasons` to validation outputs.
- [ ] Add run-level validation verdict summary to:
  - [ ] Double MA validation result
  - [ ] Composite validation result
- [ ] Move CSCV pass/fail logic from frontend-only shortlist code into backend validation semantics.
- [ ] Make recommended frontend defaults stricter for:
  - [ ] walk-forward
  - [ ] CSCV
  - [ ] PSR
  - [ ] DSR

### Phase 2: Stronger walk-forward

- [ ] Design finalist-only re-optimization walk-forward request shape.
- [ ] Implement Double MA re-optimization walk-forward.
- [ ] Implement Composite re-optimization walk-forward.
- [ ] Emit parameter-drift summary across windows.
- [ ] Add UI output for:
  - [ ] OOS window results
  - [ ] parameter drift
  - [ ] pass/fail verdict

### Phase 3: Search-level data-snooping control

- [ ] Implement Hansen SPA for candidate-vs-benchmark evaluation.
- [ ] Decide benchmark policy:
  - [ ] zero / flat
  - [ ] buy-and-hold
  - [ ] strategy-family baseline
- [ ] Choose and document a serial-dependence-aware bootstrap design.
- [ ] Add SPA result fields to validation output.
- [ ] Gate final recommendation on SPA where enabled.
- [ ] Evaluate whether White Reality Check is still needed after SPA.
- [ ] Evaluate whether Model Confidence Set should be added for survivor-set output.
- [ ] Evaluate whether FDR control should be added for large rule universes.

### Phase 3B: Approval-oriented inference

- [ ] Add Minimum Track Record Length reporting to validation output.
- [ ] Add approval / reject semantics at the run level.
- [ ] Add a "no candidate approved" UX path that does not feel like an error.

### Phase 4: Meta tightening

- [ ] Tighten meta ingestion rules for weak validation artifacts.
- [ ] Make hard-shortlist behavior the default for production-facing meta.
- [ ] Raise visibility of holdout quality and sample sufficiency in meta UI.
- [ ] Add explicit warnings when meta evidence is weak or noisy.

### Phase 5: Strategy coverage

- [ ] Implement VCC local stability.
- [ ] Implement ROCB local stability.
- [ ] Decide where purged / embargoed validation applies in the current strategy stack.
- [ ] Verify parity of hard gates across:
  - [ ] DoubleMa family
  - [ ] CompositeCpu
  - [ ] CompositeMo-backed validation path
  - [ ] ASR
  - [ ] VEMR
  - [ ] ROCB
  - [ ] VCC

### Verification

- [ ] Add unit tests for new hard-gate semantics.
- [ ] Add snapshot tests for validation result schema changes.
- [ ] Add one end-to-end workflow regression that fails when overfit candidates survive incorrectly.
- [ ] Run `cargo test --workspace`
- [ ] Run `npm --prefix ./web run typecheck`
- [ ] Run `npm --prefix ./web run test`

## Practical Recommendation

If only one statistical addition is made beyond the current stack, it should be:

- Hansen SPA for full-search data-snooping correction

If only one workflow improvement is made, it should be:

- finalist-only re-optimization walk-forward

If only one default-policy change is made, it should be:

- promote CSCV + PSR/DSR + walk-forward from optional helpers to recommended hard-gate inputs

## Bottom Line

The app already has many of the right pieces.

The next step is not to invent more validation metrics. The next step is to turn the current validation stage from:

- informative

into:

- decisively filtering
- statistically stricter
- harder to misuse
