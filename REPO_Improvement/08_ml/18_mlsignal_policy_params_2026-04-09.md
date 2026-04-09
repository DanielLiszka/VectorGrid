# ML Signal Policy Parameters Plan

Last updated: 2026-04-09

## Purpose

Define the current ML Signal single-policy and policy-sweep parameter surface so defaults, request shapes, sweep-budget rules, replay semantics, and frontend/backend drift risks remain explicit and testable.

## Scope Boundaries

In scope:

- single-policy validation parameter contract
- policy-sweep axis contract
- frontend defaults and persisted control state
- backend defaults, candidate expansion, and top-k truncation
- artifact persistence and replay of policy choices

Out of scope:

- score-cache file contract details
- model-bundle inventory health
- Stage 3 / Stage 4 meta-analysis weighting
- generic strategy-store fields unrelated to the ML Signal policy surface

## Current User-Facing Behavior

The ML Signal panel exposes a single-policy form for direct validation and a bounded sweep form for broader exploration. The user can set long and short probability thresholds, delta, max hold bars, optional PSR/DSR gates, and exit behavior. The sweep form adds axis-based exploration across thresholds, delta, hold bars, and retained `top_k`, but still inherits exit toggles from the single-policy surface rather than offering a fully separate exit-axis editor. Current behavior also includes:

- direct validation always submits exactly one labeled candidate
- policy sweep expands multiple axes into candidate policies, then keeps only the best retained subset by robust score
- artifact replay restores current param fields back into the store and panel
- sweep UX is intentionally framed as bounded and predictable rather than an unconstrained optimizer

## Current Code Surface

- `web/src/components/MlSignalWorkspacePanel.vue`
  - single-policy controls, sweep controls, and user-facing explanatory copy
- `web/src/stores/optimizeStore.ts`
  - persisted defaults for validation and sweep fields
- `web/src/services/mlSignalService.ts`
  - request construction for direct validation and policy sweep
- `src-tauri/src/commands_ml_signal.rs`
  - command pass-through and loaded-data context checks
- `crates/app_core/src/ml_signal.rs`
  - `MlSignalPolicyParams`
  - `MlSignalValidationRequest`
  - `MlSignalPolicySweepRequest`
  - candidate-budget expansion and top-k truncation
- `web/src/services/historyService.ts`
  - restoration of saved ML policy fields from artifacts
- `src-tauri/src/run_artifacts.rs`
  - persistence of single-policy and sweep summaries

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`
- embedded ML-signal tests in `crates/app_core/src/ml_signal.rs`
- workflow/history artifact tests where ML replay semantics matter
- ML-signal benches in `crates/app_core/benches/ml_signal_workflow.rs` and `crates/app_core/benches/ml_signal_validation.rs`

## Strengths Already Present

- the user-facing policy contract is already concentrated in one panel and one service
- single-policy and sweep request shapes are distinct instead of overloaded into one vague payload
- backend expansion is bounded by an explicit `20,000`-candidate cap
- retained sweep results are sorted and truncated consistently after validation
- replay/artifact behavior already captures enough policy context to make runs inspectable later

## Weaknesses / Failure Modes

- defaults live in more than one place:
  - backend `Default` impls
  - frontend store defaults
  - request-shaping fallbacks
- single-policy validation and sweep use related but different payload shapes, which is easy to under-document
- sweep still depends on the single-policy exit toggles, which is a product choice that can be mistaken for a missing feature or a bug
- the current default drift is concrete:
  - backend sweep `top_k` default is `25`
  - frontend sweep `top_k` default is `10`
- artifact replay must infer whether the attached ML payload represents:
  - one candidate
  - or a sweep summary with retained candidates

## Regressions To Avoid

- changing current defaults without updating store, service, backend, and tests together
- silently widening the sweep budget or removing the current hard cap
- accidentally giving sweep a more permissive or different validation context than direct validation
- making history replay unable to restore current single-policy versus sweep semantics
- changing exit-toggle behavior without explicit product intent and updated UI copy
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because threshold parameters only make sense if their underlying score semantics stay probability-oriented and explicit
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because policy selection is still part of a time-ordered validation workflow and should not drift into generic shuffled-search assumptions
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because these requests and saved artifacts behave like durable serialized contracts
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the user-facing controls map directly into a Rust request boundary whose shape should remain stable

## Proposed Improvement Approach

1. Document the policy surface in two explicit tracks:
   - single-policy validation
   - sweep-axis exploration
2. Record current defaults in one place and distinguish:
   - backend defaults
   - frontend defaults
   - service-level fallback behavior
3. Make the single-policy versus sweep request-shape difference obvious so artifact replay and history logic stay understandable.
4. Document the deliberate product constraint that sweep exit behavior is inherited from the single-policy toggles.
5. Document the candidate-budget cap, robust-score sorting, and post-validation top-k truncation as first-class constraints.
6. Highlight the current `top_k` drift so future cleanup can be intentional rather than accidental.
7. Keep any future parameter-surface expansion additive and compatible with the current replayable artifacts.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document every current single-policy field and its shipped default.
- [ ] Document every current sweep-axis field and its shipped default.
- [ ] Distinguish backend defaults from frontend store defaults.
- [ ] Call out the current sweep `top_k` mismatch between frontend and backend defaults.
- [ ] Document the single-policy request shape versus the sweep request shape.
- [ ] Document that sweep exit behavior still comes from the single-policy toggles.
- [ ] Document the `20,000`-candidate expansion cap and robust-score ordering behavior.
- [ ] Document PSR/DSR optional gates and where they are forwarded.
- [ ] Document how policy details are persisted into artifacts and restored through replay.
- [ ] Add or maintain tests for defaults, explicit overrides, and replay shape stability.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/mlSignalService.test.ts`
- run `npm --prefix web run test -- src/components/MlSignalWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/optimizeStore.actions.test.ts`
- run targeted ML-signal tests in `crates/app_core/src/ml_signal.rs`
- manually execute one direct ML validation and one bounded sweep, then replay both artifacts
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve the current single-policy request shape and current sweep request shape until a documented migration exists
- if defaults are unified later, do it explicitly and update both store fixtures and backend tests together
- keep the current bounded sweep posture unless product goals change deliberately
- preserve older artifacts that encode the current single-policy or sweep summary fields
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/05_ml_policy_sweep_2026-04-09.md`
- `REPO_Improvement/08_ml/17_mlsignal_score_cache_validation_2026-04-09.md`
- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
- `REPO_Improvement/07_strats/08_strat_ml_signal_2026-04-08.md`
