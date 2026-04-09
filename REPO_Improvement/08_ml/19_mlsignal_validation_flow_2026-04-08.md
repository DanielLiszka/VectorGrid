# ML Signal Validation Flow Plan

Last updated: 2026-04-09

## Purpose

Define the current ML Signal validation and policy-sweep flow so dataset/timeframe compatibility, score-cache contract rules, candidate budget limits, artifact persistence, and UI status behavior remain aligned with the repo that actually ships.

## Scope Boundaries

In scope:

- ML Signal validation request and result behavior
- validation-context compatibility checks
- score-cache prerequisites and score-space constraints
- policy sweep generation, budget limits, and result shaping
- artifact attachment and UI status/report behavior

Out of scope:

- model training internals
- Stage 4 / Stage 5 meta/confidence flows
- options or non-ML-signal validation

## Current User-Facing Behavior

Users can export an ML dataset, build a score cache, run direct ML validation over one or more policy candidates, and run a bounded policy sweep. The workflow is stricter than a generic "upload predictions and evaluate" surface:

- validation can require a matching `validation_context` for strategy kind, CSV path, and timeframe
- validation requires a loaded dataset with enough candles
- validation requires a score cache that matches the current `data_id` and feature-schema expectations
- probability-threshold validation requires probability columns, not raw margins alone
- policy sweep can expand multiple axes, but the total candidate budget is capped at `20,000`
- top-k truncation happens after validation and robust-score sorting

The frontend already does more than issue commands. It also manages busy-state messaging, previews, artifact persistence, and reopen/reveal flows around ML outputs.

## Current Code Surface

- `web/src/services/mlSignalService.ts`
  - builds requests, reads current dataset context, attaches artifacts, coordinates busy-state/status lines, and formats ML result summaries and previews
- `src-tauri/src/commands_ml_signal.rs`
  - validates `validation_context` against the loaded dataset and provides the command boundary for dataset export, validation, and sweep
- `crates/app_core/src/ml_signal.rs`
  - owns score-cache loading, score-cache contract checks, direct validation, policy-sweep expansion, top-k truncation, and run-verdict construction
- `src-tauri/src/run_artifacts.rs`
  - threads ML artifact details such as stage, bundle ID, score cache path, score manifest path, feature schema path/version, and policy summaries into saved artifacts and detail responses

Repo-grounded facts that should appear explicitly in the final implementation doc:

- `validate_ml_signal_context()` rejects non-`MlSignal` strategy context and mismatched CSV path/timeframe
- `run_ml_signal_validation_blocking_with_candles()` requires non-empty `data_id`, non-empty `score_cache_path`, at least one candidate, and enough candles
- validation rejects raw-only caches when probability-threshold validation is required
- two-sided validation requires short-side probability scores, not just long-side scores
- policy sweep currently hard-rejects candidate expansions above `20,000`
- sweep results are sorted by `robust_score` before optional `top_k` truncation

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- targeted Rust tests in `crates/app_core/src/ml_signal.rs`
  - raw-only score-cache rejection
  - one-sided versus two-sided probability handling
  - score-cache build/validation smoke
  - policy-sweep smoke
- manual validation / sweep / artifact replay review

## Strengths Already Present

- the command boundary is explicit rather than hidden behind generic validation plumbing
- backend compatibility checks already protect against dataset/timeframe drift
- the app-core test surface already exercises key score-cache failure modes
- artifact persistence already carries ML-specific summaries instead of reducing ML runs to opaque blobs

## Weaknesses / Failure Modes

- prerequisites span dataset export, score cache build, loaded data context, and validation request shape, so the workflow is easy to misunderstand without one contract doc
- raw-margin versus probability-score semantics are subtle and easy to blur in UI copy
- validation and policy sweep share core machinery but expose different user expectations, which makes drift likely if they are documented separately and loosely
- artifact persistence is additive and best-effort enough that missing report/path fields can look like workflow failure unless documented carefully

## Regressions To Avoid

- allowing validation against incompatible dataset/timeframe context
- weakening score-cache contract checks for convenience
- raising or removing the sweep budget cap without documenting the product/runtime consequences
- losing ML artifact details such as schema version, score manifest path, or policy summaries
- making busy/cancel/status behavior less clear than the current flow
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because the docs explicitly frame time-ordered splits as different from ordinary CV and emphasize avoiding future-data leakage
- [scikit-learn calibration guide](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because the repo distinguishes raw margins from calibrated probabilities, and the docs explain why well-calibrated probabilities have a different interpretation than generic scores
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the workflow is command-mediated and long-running enough that busy-state and error behavior matter
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because the request/result and artifact shapes should be treated as stable contracts

## Proposed Improvement Approach

1. Document the ML Signal flow as a staged pipeline:
   - dataset context
   - score cache contract
   - direct validation
   - policy sweep
   - artifact persistence
2. Separate "can this request run?" checks from "did this candidate pass?" checks.
3. Make score-space semantics explicit:
   - raw margin
   - long probability
   - short probability
   - one-sided versus two-sided validation
4. Document how the frontend surfaces artifacts and status for both direct validation and sweep.
5. Tie each major failure mode to a current code path and a current test.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document required dataset and loaded-candle prerequisites.
- [ ] Document current `validation_context` matching rules for strategy kind, CSV path, and timeframe.
- [ ] Document direct-validation request requirements and core result fields.
- [ ] Document score-cache contract rules, including probability requirements.
- [ ] Document one-sided versus two-sided validation behavior.
- [ ] Document policy-sweep axes, candidate-budget cap, and top-k truncation behavior.
- [ ] Document current artifact fields and replay expectations for ML validation runs.
- [ ] Document current status-line, preview, and busy-state behavior in the frontend service.
- [ ] Add or tighten service/backend tests if any of those semantics change.
- [ ] Manually verify one direct validation and one policy sweep end to end.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/mlSignalService.test.ts`
- run targeted tests in `crates/app_core/src/ml_signal.rs`
- manually execute one ML validation and one policy sweep using a valid loaded dataset and current score cache
- inspect the saved artifact detail response to confirm ML-specific fields still round-trip
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve validation-context enforcement and score-cache contract checks
- prefer clearer errors over hidden fallback behavior
- if the workflow starts accepting new score spaces or calibration modes, update the contract doc and tests together
- keep artifact enrichment additive so older ML artifacts remain usable
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/15_mlsignal_score_csv_ingest_2026-04-09.md`
- `REPO_Improvement/08_ml/20_mlsignal_model_bundle_inventory_2026-04-09.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
