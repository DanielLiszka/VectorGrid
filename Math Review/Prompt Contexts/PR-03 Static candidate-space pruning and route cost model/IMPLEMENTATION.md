# PR-03 Implementation Guide

Source response: `pr03_static_pruning_route_model_answers.md`

Decision: `Valid`, but only for exact-boundary preservation plus explicit heuristic-contract hardening.

## Valid Scope
- [ ] Keep exhaustive-output requests pinned to exact `Grid`.
- [ ] Treat route-cost optimization as admissible only inside a verified exact-route equivalence class.
- [ ] Keep current prescan and VDPS paths explicitly heuristic unless they emit an auditable empirical contract on a named workload family.

## Non-Goals
- [ ] Do not claim current prescan or current VDPS is sound certificate pruning.
- [ ] Do not treat route cost as a universal per-candidate additive law.
- [ ] Do not broaden this into route-selection redesign outside the owned mode and route-policy boundaries.

## Additive Guardrails
- [ ] Any existing exact-output request must remain exact.
- [ ] Heuristic modes must remain optional and visibly labeled.
- [ ] No route or mode may silently change meaning for existing exhaustive-output requests.

## Primary Touchpoints
- `src-tauri/src/commands_optimize.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/double_ma_vdps.rs`

## Implementation Checklist
- [ ] Preserve command-boundary coercion to exact `Grid` whenever outputs require exhaustive semantics.
- [ ] Define the exact-route equivalence class explicitly as: same exact candidate scores, same tie order, same retained-output semantics on the retained candidate set.
- [ ] Document which request flags and blocker states determine whether a reduced-output heuristic mode is even admissible.
- [ ] Re-label current prescan and VDPS code paths, artifacts, and UI/status text as heuristic unless and until exact certificates exist.
- [ ] If heuristic paths remain enabled, define one audit artifact bundle with score regret, top-k overlap, exact winner rank, holdout regret, and speedup.
- [ ] Keep route-cost decisions measured at batch-level timing boundaries rather than implicit per-candidate math.

## Verification Checklist
- [ ] Add mode-normalization tests for exhaustive-output requests.
- [ ] Add exact-route equivalence tests for route/blocker combinations that should be semantically identical.
- [ ] Add fidelity-audit coverage for every promoted heuristic reduced-output mode.
- [ ] Add regression tests proving heuristic labels and metrics appear in artifacts or diagnostics when non-exact modes run.

## Rollout Checklist
- [ ] Keep exhaustive-output mode as the default whenever exactness is required by outputs.
- [ ] Require a named workload-family audit before promoting any heuristic path in product-facing defaults.
- [ ] Publish heuristic metrics without overstating proof or certificate semantics.

## Kill Criteria
- [ ] Reject any patch that presents current prescan or VDPS as exact pruning without certificates.
- [ ] Reject any patch that hides heuristic mode selection behind exact-looking labels.

