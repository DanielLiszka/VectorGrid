# Meta Batch Registry Modes Plan

Last updated: 2026-04-09

## Purpose

Define the current meta batch registry/mode contract so dataset selection, `execution_mode`, `preload_policy`, tags, holdout settings, and auto-follow-up behavior remain explicit and stable across the frontend, Tauri boundary, and saved campaign summaries.

## Scope Boundaries

In scope:

- selected dataset-path inventory for meta batch runs
- batch request fields:
  - `selected_paths`
  - `holdout_pct`
  - `holdout_tag`
  - `execution_mode`
  - `preload_policy`
  - `auto_meta`
  - `auto_stage5`
  - `skip_done`
  - `tags`
- frontend normalization of batch-mode values
- dataset compatibility rules that affect batch request construction
- summary persistence required to understand which mode a batch actually ran in

Out of scope:

- batch resume/retry mechanics beyond the shape of the persisted summary they depend on
- Stage 3 holdout-mode semantics inside each per-dataset meta run
- policy-eval or saved-artifact library behavior outside batch-summary persistence
- options workflows

## Current User-Facing Behavior

The Meta workspace batch surface lets users select dataset paths and launch a campaign with explicit holdout %, holdout tag, execution mode, preload policy, auto-meta, auto-Stage-5, skip-done, and tag settings. The current shipped behavior is closer to a “selected dataset registry plus execution/preload modes” system than a fully separate registry service. Current behavior includes:

- dataset selection from repo-local CSV discovery and cached dataset listings
- mixed-timeframe selections being blocked before batch execution
- `execution_mode` normalized to `full`, `optimize_only`, or `validate_only`
- `preload_policy` normalized to `host_only`, `disabled`, or `host_plus_doublema_vram`
- `auto_meta` and `auto_stage5` being constrained by execution mode
- `skip_done` being treated as a first-class campaign posture rather than an afterthought
- saved batch summaries capturing the request/mode shape needed for later inspection and resume/retry flows

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - batch controls, mode selectors, dataset checklist, and current batch UX copy
- `web/src/stores/analysisStore.ts`
  - persisted batch defaults, selected-path state, button enablement, and batch actions
- `web/src/services/metaBatchRequestBuilder.ts`
  - normalization of `execution_mode`, `preload_policy`, holdout percentage, tags, and selected paths
- `web/src/services/metaService.ts`
  - dataset discovery/selection, auto-batch request shaping, execution-mode consequences, and summary-path tracking
- `src-tauri/src/commands_campaign.rs`
  - `run_research_campaign` command boundary and per-dataset batch execution ownership
- `src-tauri/src/run_artifacts.rs`
  - suggested/save batch summary paths and summary-shape persistence
- `src-tauri/src/commands_meta.rs`
  - artifact-family recognition for saved batch and campaign summaries
- `src-tauri/src/bin/ta_campaign_perf.rs`
  - manual/perf-oriented surface for campaign-mode behavior

## Current Verification Surface

- `web/src/services/metaBatchRequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- Tauri campaign tests in `src-tauri/src/commands_campaign.rs`
- artifact-summary tests touching `src-tauri/src/run_artifacts.rs`
- manual/perf campaign checks via `src-tauri/src/bin/ta_campaign_perf.rs`

## Strengths Already Present

- the batch request surface is already centralized rather than being built ad hoc from the panel component
- execution mode and preload policy are normalized explicitly instead of leaking arbitrary strings into Tauri
- current defaults are opinionated and conservative:
  - `full`
  - `host_only`
  - `auto_meta=true`
  - `auto_stage5=false`
  - `skip_done=true`
- timeframe compatibility is blocked early in the frontend instead of becoming a confusing mixed-scope campaign
- saved batch summaries already exist, so registry/mode behavior can be reconstructed after the run

## Weaknesses / Failure Modes

- there is no current backend concept literally named “registry mode,” so documentation can drift if it pretends a stronger abstraction already exists than the code actually ships
- mode behavior is split across request builder normalization, service-layer orchestration, and Tauri execution
- selected-path state is partly live UI state and partly persisted summary state, which makes ownership easy to blur
- execution-mode consequences on `auto_meta` and `auto_stage5` are subtle and could be broken by later UI changes
- batch summaries are persisted, but batch/campaign summaries are not yet a first-class artifact-library family like meta or Stage 5

## Regressions To Avoid

- allowing invalid `execution_mode` or `preload_policy` values to slip past normalization
- weakening current mixed-timeframe or unsupported-strategy guards in the batch surface
- changing batch defaults without updating builder tests, service behavior, and UX copy together
- breaking saved summary fields that later resume/retry logic relies on
- documenting a “registry mode” abstraction that does not actually match shipped state ownership
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the batch request and summary envelopes behave like versioned JSON contracts with enumerated modes and defaulted fields
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the normalized batch-mode payload crosses a frontend/backend boundary and needs shape stability
- [AWS Well-Architected: Make All Responses Idempotent](https://docs.aws.amazon.com/wellarchitected/latest/framework/rel_prevent_interaction_failure_idempotent.html)
  - relevant because `skip_done`, saved summaries, and later resume/retry behavior depend on replay-safe, duplicate-tolerant campaign semantics
- [Background Tasks with Hosted Services in ASP.NET Core](https://learn.microsoft.com/en-us/aspnet/core/fundamentals/host/hosted-services)
  - relevant because long-running batch orchestration should keep cancellation, progress, and cleanup semantics explicit rather than implicit in UI state
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the current system honestly as:
   - dataset selection registry-like state
   - execution mode
   - preload policy
   - campaign follow-up toggles
2. Record all current defaults and normalization rules in one place so future refactors cannot quietly reinterpret the batch surface.
3. Separate dataset eligibility rules from mode normalization rules because they fail in different ways and have different owners.
4. Make `skip_done`, summary persistence, and replay-safe mode behavior explicit because those semantics matter more than the UI labels.
5. Document exactly how `optimize_only` and `validate_only` constrain follow-up automation.
6. Tie every persisted batch-summary field to the later flows that consume it so compatibility is treated as a product requirement.
7. Keep the “registry” language grounded in current implementation reality until a stronger backend registry abstraction actually exists.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every current batch request field and its normalization/default behavior.
- [x] Document current defaults for `holdout_pct`, `holdout_tag`, `execution_mode`, `preload_policy`, `auto_meta`, `auto_stage5`, and `skip_done`.
- [x] Document timeframe-compatibility and strategy-eligibility checks that gate batch request construction.
- [x] Document how `execution_mode` affects `auto_meta` and `auto_stage5`.
- [x] Document all allowed `preload_policy` values and their current meaning.
- [x] Document tag parsing and deduplication semantics.
- [x] Document which selected-path state lives in the store versus the saved batch summary.
- [x] Document which fields batch summaries must preserve for later replay/resume understanding.
- [x] Add or preserve tests for invalid mode normalization, holdout clamping, and unsupported campaign inputs.
- [x] Keep batch-surface docs aligned with `metaBatchRequestBuilder`, `metaService`, and `commands_campaign` whenever mode semantics change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaBatchRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts`
- run `npm --prefix web run test -- src/stores/taskStoreCampaignProgress.test.ts`
- run targeted campaign tests in `src-tauri/src/commands_campaign.rs`
- inspect one saved batch summary after a `full` run and one after a constrained execution-mode run
- manually verify mixed-timeframe selection is still blocked in the batch UI
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current enum-like mode strings until there is an explicit migration path for saved summaries
- prefer additive summary fields over reinterpretation of current ones
- if a stronger backend registry abstraction is later introduced, document that transition explicitly rather than silently reusing current terminology
- keep summary files replay-safe so later resume/retry logic can remain conservative and idempotent
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by surfacing the batch registry contract in the Meta batch pane instead of leaving it spread across service and builder code.
- `MetaWorkspacePanel.vue` now states:
  - holdout percentage, holdout tag, and skip-completed posture
  - deduped batch tags as they will be normalized at run time
  - same-timeframe selection requirements unless the Data workspace overrides timeframe
  - current execution-mode consequences, including `optimize_only` and `validate_only` posture
- `metaBatchRequestBuilder.test.ts` now covers conservative fallback for invalid `execution_mode` and `preload_policy` values while preserving current tag deduplication and default holdout-tag behavior.
- `REPO_MAP.md` was synced so contributors can find the batch registry/mode contract quickly from the routing doc.
- Stable fixture / replay payload:
  - `selected_paths = [C:/a.csv, C:/b.csv]`
  - `holdout_pct = 15`
  - `holdout_tag = holdout-a`
  - `execution_mode = optimize_only`
  - `preload_policy = host_plus_doublema_vram`
  - `skip_done = true`
  - `tags = [alpha, beta]`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaBatchRequestBuilder.test.ts src/services/metaService.test.ts`
  - `cargo test -p ta_desktop_demo_app commands_campaign --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/08_meta_batch_generation_2026-04-09.md`
- `REPO_Improvement/08_ml/28_meta_batch_resume_retry_2026-04-09.md`
- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
- `REPO_Improvement/06_artifacts/09_workflow_artifact_store_2026-04-08.md`
