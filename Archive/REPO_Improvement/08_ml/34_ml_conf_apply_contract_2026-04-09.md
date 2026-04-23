# ML Confidence Apply Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current ML-confidence apply contract so in-memory meta augmentation, backend Stage 5 execution, and offline policy-evaluation flows remain clearly separated and do not collapse into one ambiguous “apply ML” concept.

## Scope Boundaries

In scope:

- frontend `Apply to current meta`
- backend `apply_ml_confidence_model`
- Stage 5 application through `run_meta_stage5_ml_confidence`
- policy-evaluation as a distinct offline contract
- current defaults and safeguards that shape the apply path

Out of scope:

- scope identity and health-state computation except where apply depends on them
- detailed Stage 5 selection math beyond the contract boundaries of the apply surface
- generic meta selection behavior outside ML augmentation
- options workflows

## Current User-Facing Behavior

The shipped product currently exposes three related but distinct ML-confidence application paths:

- `Apply to current meta`
  - augments the currently loaded in-memory meta result with ML confidence and may re-sort it locally
- `Run Stage 5`
  - runs the backend blended selection pipeline and writes a saved Stage 5 artifact
- `Evaluate policy`
  - runs offline policy comparison over scoped saved meta artifacts and writes a saved evaluation artifact

Current behavior means:

- “apply” is not one thing in this repo
- the lightweight apply path is in-memory and does not save a Stage 5 artifact
- Stage 5 is an execution/finalization path with saved JSON output
- policy eval is offline historical comparison, not the same contract as current-meta apply

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - user-facing apply, Stage 5, and policy-eval controls
- `web/src/services/metaService.ts`
  - in-memory `mlApply()`, Stage 5 orchestration, policy-eval orchestration, and shared status handling
- `web/src/services/metaStage4RequestBuilder.ts`
  - request shaping for the Stage 5 path
- `src-tauri/src/commands_meta.rs`
  - `apply_ml_confidence_model`
  - Stage 5 command handling
  - policy-eval command handling
- `crates/app_core/src/ml_confidence.rs`
  - feature vector contract and probability prediction
- `src-tauri/src/run_artifacts.rs`
  - Stage 5 and policy-eval artifact families

## Current Verification Surface

- `web/src/services/metaStage4RequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- tests embedded in `src-tauri/src/commands_meta.rs`
- tests embedded in `crates/app_core/src/ml_confidence.rs`
- artifact summary/replay tests in `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- the repo already distinguishes in-memory apply, Stage 5 execution, and policy evaluation instead of hiding them behind one endpoint
- feature extraction for apply is consistent with the backend feature contract
- Stage 5 has a saved-artifact path and policy eval has a separate saved-artifact path
- Stage 5 request defaults are centralized in their builder
- in-memory apply already respects the current health signal enough to avoid hard-blocking unknown-but-not-explicitly-unhealthy cases

## Weaknesses / Failure Modes

- naming drift makes it easy to confuse Stage 4 builder/state names with the user-facing Stage 5 action
- the in-memory apply path is transient and replay-free, which is easy to forget if one assumes every ML application produces an artifact
- `mlTrain()` can immediately trigger `mlApply()` when ML ranking is enabled, which means training can mutate the current meta view indirectly
- current apply behavior depends on the current in-memory meta payload, not a durable artifact boundary
- Stage 5 and policy-eval reuse overlapping thresholds/weights, which can make their contracts look more unified than they really are

## Regressions To Avoid

- collapsing in-memory apply, Stage 5 execution, and policy-eval into one under-documented behavior bucket
- changing Stage 5 defaults or health gates without updating the builder, backend normalization, and UI messaging together
- weakening the feature-vector compatibility contract between frontend apply and backend prediction
- implying that `Apply to current meta` creates durable artifacts when it does not
- breaking the current separation between current-meta mutation and offline historical evaluation
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because all three apply-style paths depend on probability-style confidence outputs having stable semantics
- [scikit-learn Model Selection and Evaluation](https://scikit-learn.org/stable/model_selection.html)
  - relevant because the repo’s Stage 5 and policy-eval paths are evaluation/finalization layers, not just UI actions
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because Stage 5 and policy-eval payloads are persisted JSON contracts while in-memory apply is intentionally not
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the backend apply and Stage 5 paths cross structured command boundaries while the in-memory re-render path does not persist artifacts
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document three distinct apply contracts:
   - current-meta augmentation
   - Stage 5 finalization
   - policy-eval comparison
2. Make the in-memory-only nature of `Apply to current meta` explicit so users and future contributors do not expect replay artifacts from it.
3. Tie Stage 5 request defaults, backend normalization, and saved-artifact behavior together in one contract description.
4. Keep policy evaluation documented as an offline historical comparison path rather than a live apply path.
5. Preserve explicit feature-vector compatibility between frontend current-meta rows and backend `MlConfidenceGroupFeatures`.
6. Document the current health/fallback posture separately for in-memory apply and Stage 5 execution.
7. Keep naming drift visible instead of pretending the Stage 4/Stage 5 aliases do not exist.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current-meta apply flow and its in-memory-only behavior.
- [x] Document the backend `apply_ml_confidence_model` contract and returned confidence payload shape.
- [x] Document the Stage 5 execution path and its saved-artifact behavior.
- [x] Document policy evaluation as a separate offline contract.
- [x] Document the current feature-vector contract shared between frontend rows and backend prediction.
- [x] Document current Stage 5 defaults and health/fallback flags that shape the execution path.
- [x] Document when training can trigger immediate apply behavior in the frontend.
- [x] Preserve tests for in-memory apply, Stage 5 run behavior, and policy-eval orchestration.
- [x] Preserve tests for feature-vector/prediction parity in `ml_confidence.rs`.
- [x] Keep docs explicit that only Stage 5 and policy eval persist durable artifacts today.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaStage4RequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run targeted ML-confidence/apply tests in `src-tauri/src/commands_meta.rs`
- run targeted prediction-parity tests in `crates/app_core/src/ml_confidence.rs`
- manually exercise:
  - `Apply to current meta`
  - `Run Stage 5`
  - `Evaluate policy`
- confirm only Stage 5 and policy eval create durable saved artifacts
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve the current distinction between transient current-meta augmentation and durable Stage 5/policy-eval artifacts
- if a future feature adds saved artifacts for direct apply, document that as a new contract instead of retroactively changing the current one
- keep Stage 4 compatibility aliases explicit until the naming migration is intentional and complete
- prefer additive reporting/state fields over collapsing the three apply paths into one generic status concept
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making the apply-path separation explicit in the ML contract surface instead of relying only on button labels and service naming.
- `MetaWorkspacePanel.vue` now states that:
  - `Apply to current meta` is an in-memory augmentation path
  - `Run Stage 5` creates a saved rerank artifact
  - `Evaluate policy` creates a saved offline comparison artifact
  - only Stage 5 and Policy Eval persist durable artifacts today
- Existing orchestration/tests remain intact:
  - `metaService.test.ts` continues to cover current-meta apply DTO shaping and Stage 5 / policy-eval orchestration
  - targeted Rust suites continue to cover backend feature-vector and ML-confidence prediction behavior
- Stable fixture / replay payload:
  - in-memory `apply_ml_confidence_model` request over current groups
  - `vectorgrid_stage5.json`
  - `vectorgrid_ml_policy_eval.json`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaService.test.ts`
  - `cargo test -p ta_desktop_demo_app commands_meta --lib`
  - `cargo test -p ta_app_core ml_confidence --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/32_ml_conf_model_families_2026-04-09.md`
- `REPO_Improvement/08_ml/33_ml_conf_health_staleness_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/37_stage5_execution_contract_2026-04-09.md`
- `REPO_Improvement/08_ml/38_ml_policy_eval_contract_2026-04-09.md`
