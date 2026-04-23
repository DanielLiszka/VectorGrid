# PR-09 Implementation Guide

Source response: `PR-09_options_evaluation_three_pass_answers.md`

Decision: `Valid`, but only for quote-vs-aggregate exactness separation and bounded provisional-pruning contracts.

## Valid Scope
- [ ] Split options exactness semantics into quote-class and aggregate-class behavior.
- [ ] Treat median replay as ranking-irrelevant where the response proves that result.
- [ ] State that aggregate-class exact pruning is invalid without exact extreme-lane state.
- [ ] If provisional GPU scores are used before exactification, require tuple-local interval certificates and a top-k interval-separation rule before claiming exact pruning.

## Non-Goals
- [ ] Do not claim “GPU shortlist first, exactify later” repairs all aggregate drift.
- [ ] Do not claim a universal exact pruning theorem on the current aggregate-class surface.
- [ ] Do not redesign the entire GPU contract unless a later patch explicitly chooses to widen it.

## Additive Guardrails
- [ ] Current exact replay and exactify paths must remain supported.
- [ ] Quote-class improvements must not remove aggregate-class support.
- [ ] Any provisional pruning must remain disabled unless its interval contract is satisfied.

## Primary Touchpoints
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `src-tauri/src/commands_options_expression.rs`

## Implementation Checklist
- [ ] Define the quote-class surfaces and aggregate-class surfaces explicitly in code/docs.
- [ ] Document where median replay is safe to defer and where it is not ranking-critical.
- [ ] For aggregate classes, require exact extreme-lane state before any exact pruning claim.
- [ ] If provisional GPU ranking remains, define the tuple-local interval as the only admissible exact-pruning gate.
- [ ] Keep any mixed-precision or approximation envelope explicit and stored where needed.
- [ ] Do not promote provisional shortlist behavior to “exact” unless interval separation is satisfied.

## Verification Checklist
- [ ] Add quote-class tests proving median replay does not affect ranking.
- [ ] Add aggregate-class tests proving exact pruning is blocked without exact extreme-lane state.
- [ ] Add interval-certificate tests for any future provisional GPU pruning path.
- [ ] Add artifact/report tests that distinguish quote-class and aggregate-class semantics.

## Rollout Checklist
- [ ] Keep current exactify paths as the default exact reference.
- [ ] Treat interval-certified pruning as opt-in until enough evidence exists.
- [ ] Document semantics clearly so ranking users do not confuse quote and aggregate guarantees.

## Kill Criteria
- [ ] Reject any aggregate-class pruning path that ships without exact extreme-lane state or interval certificates.
- [ ] Reject any patch that lets provisional GPU ranking overclaim exactness.

