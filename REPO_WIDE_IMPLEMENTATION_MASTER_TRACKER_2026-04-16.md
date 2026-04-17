# Repo-Wide Implementation Master Tracker

Last updated: 2026-04-16

## Purpose

This document is the authoritative execution tracker for the next repo-wide implementation program.

It consolidates four closely related improvement plans into one governed program so the repo can be improved without fragmenting effort across overlapping ledgers:

1. `Workflow Contract And Replay Hardening`
2. `Validation Truth Alignment`
3. `Artifact System Cleanup`
4. `Options Expression Shipped-Slice Hardening`

The primary objective is not feature expansion.

The primary objective is to make current workflows more internally consistent, easier to maintain, safer to replay, and less likely to drift between:

- frontend request builders
- Tauri command boundaries
- persisted artifacts
- reopened/replayed workflows
- report/export surfaces
- operator-visible route and fallback summaries

## Governing Inputs

Use these as the default routing and policy inputs before touching code:

- [REPO_MAP.md](./REPO_MAP.md)
- [AGENTS.md](./AGENTS.md)
- [IMPROVEMENT_MASTER_PLAN_2026-04-08.md](./IMPROVEMENT_MASTER_PLAN_2026-04-08.md)
- [IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md](./IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md)
- [options_expression_optimizer_viable_plan_2026-04-02.md](./options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](./options_research/options_expression_future_scope_roadmap_2026-04-07.md)

When these docs disagree with current code, trust the code-facing entry points listed in `REPO_MAP.md`.

## Program Charter

This program should:

- improve current workflows without creating net-new product lanes
- preserve existing CPU fallback and currently supported safe routes
- reduce frontend/backend/artifact drift
- make replay and restore behavior deterministic and inspectable
- keep report and validation summaries honest relative to actual runtime behavior
- reduce warning noise only when the cleanup is low-risk and does not hide behavior

This program should not:

- expand the options workflow beyond the currently shipped slice
- add new benchmark-driven work before semantic consistency work is stable
- broaden scope into vendor code, generated artifacts, or inactive research notes
- remove capabilities as a disguised cleanup path
- perform large architectural rewrites unless a bounded migration path is written first

## Global Execution Rules

- Semantics first, performance second.
- Request, command, artifact, and replay correctness should be fixed before benchmark-sensitive refactors.
- Use targeted verification after every bounded slice.
- Use broad repo gates only after a logically complete cluster, not after every tiny edit.
- Run benchmark, faceoff, profiling, or workflow timing scripts only after semantic hardening is green.
- Serialize any benchmark or profiling run; never overlap them.
- Prefer updating this file after each completed slice instead of creating overlapping ad hoc trackers unless a sub-area becomes large enough to justify its own narrow implementation plan.

## Program Composition

### Primary epic

- `Epic A`: Workflow Contract And Replay Hardening

### Secondary but in-scope epics

- `Epic B`: Validation Truth Alignment
- `Epic C`: Artifact System Cleanup
- `Epic D`: Options Expression Shipped-Slice Hardening

### Cross-cutting support tracks

- `Track X1`: Warning-noise and dormant-path cleanup
- `Track X2`: Verification and gate discipline
- `Track X3`: Documentation and operator-facing wording consistency

## Recommended Execution Order

1. Epic A: Workflow Contract And Replay Hardening
2. Epic C: Artifact System Cleanup
3. Epic B: Validation Truth Alignment
4. Epic D: Options Expression Shipped-Slice Hardening
5. Track X1: Warning-noise and dormant-path cleanup
6. Track X2: Broad quality gates and, later, performance-sensitive follow-up

Rationale:

- Epic A reduces the highest-risk drift first.
- Epic C depends on replay and handoff contracts being explicit.
- Epic B is safer once request and artifact truth are tighter.
- Epic D should inherit the now-hardened artifact/replay/report patterns instead of inventing its own parallel logic.

## Completion Standard For Any Slice

A slice is only complete when all of the following are true:

- the affected workflow behaves consistently across live use, saved-run load, and rerun
- route and fallback summaries match real backend behavior
- error messages are explicit enough to diagnose the failure class
- artifact payloads carry the metadata needed for reopen and replay
- targeted tests are added or updated
- no obvious compatibility regression is introduced in the same surface

## Epic A: Workflow Contract And Replay Hardening

### Goal

Make workflow requests, saved-run replay, current-run hydration, and rerun behavior agree across the main repo workflows.

### Priority surfaces

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`

### Key risks to eliminate

- replay infers a different mode than the original live run
- artifact restore loses required metadata for reopen/rerun
- route summaries and actual fallback behavior diverge
- reopened runs land in the wrong workflow step or shell mode
- optional-path failures are swallowed and become silent operator confusion

### Checklist

- [x] Build a repo-grounded contract matrix for optimize, validate, ML Signal, artifact reopen, and rerun flows.
- [x] Identify where request-builder output shape differs from saved artifact request shape.
- [x] Identify where rerun logic reconstructs less information than live execution had.
- [x] Normalize frontend defaults so empty or omitted fields do not change replay mode silently.
- [x] Normalize Tauri-side command validation for malformed paths, bundle ids, manifest references, and report targets.
- [x] Ensure artifact payloads carry the minimum metadata needed for deterministic reopen.
- [x] Ensure workflow step restoration is explicit rather than inferred from partial state when the artifact already carries stronger truth.
- [x] Add targeted tests for apply-saved-run, rerun-saved-run, and artifact restore across at least conventional, validation-heavy, and ML Signal paths.

### Progress

- `2026-04-16`: Completed the first implementation cluster for workflow replay and restore hardening.
- Frontend restore/replay work:
  - introduced `web/src/utils/core/workflowRunRestore.ts` so saved-run load, rerun, and latest-artifact sync use one shell-posture helper
  - expanded `historyService.ts` restore coverage for validation-heavy saved runs, including CSCV, walk-forward, SPA, perturbation, trade-bootstrap, and cross-ticker state
  - seeded ML-signal artifact provenance before rerun so score-cache rebuilds use the selected saved run’s bundle/schema/export metadata instead of stale in-memory artifact state
  - clamped invalid persisted live-workspace tabs through `workflowStore.applyLegacySnapshot()` so ML-signal boot posture cannot preserve illegal conventional tabs
  - hardened latest-artifact skip logic in `workflowArtifactsSync.ts` to compare config fingerprint, lineage, route provenance, and salient ML metadata before skipping rehydration
- Backend replay/artifact work:
  - `commands_validation.rs` no longer normalizes unknown `strategy_kind` values to `DoubleMa`
  - `commands_ml_signal.rs` now uses shared validation path normalization instead of a divergent local clone
  - `run_artifacts.rs` now rejects overwrite/delete of non-run artifacts through run-artifact commands and uses deterministic filename-derived `ts` fallback for legacy run files missing `ts`
- Verification completed for this cluster:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test -- --run src/services/historyService.test.ts src/services/workflowArtifactsSync.test.ts src/services/workflowBootstrap.test.ts`
  - `cargo test -p ta_desktop_demo_app validation_context_ -- --nocapture`
  - `cargo test -p ta_desktop_demo_app validate_ml_signal_context -- --nocapture`
  - `cargo test -p ta_desktop_demo_app run_artifact -- --nocapture`
- `2026-04-16`: Closed the remaining request-shape drift between live builders and saved-artifact replay for the shipped optimization and ML-signal flows.
- Implemented:
  - dataset-export saves in `web/src/services/mlSignalService.ts` now persist `strategy_kind: "MlSignal"` so saved artifact rows do not rely on stage inference alone
  - `web/src/services/historyService.ts` now restores conventional composite/CompositeMo replay state more faithfully, including MA params/sweeps, composite profile controls, and advanced sampler/constraint/gpu-prescan fields
  - saved ML-signal dataset-export artifacts now restore/rerun correctly even when the persisted request shape carries only `mlSignalStage="dataset_export"` and not `strategy_kind`
  - saved-run reruns now drop legacy camelCase `dataId` from injected requests and carry only canonical `data_id`
- Verification completed for this slice:
  - `npm --prefix web run test -- --run src/services/historyService.test.ts src/services/mlSignalService.test.ts`
  - `npm --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app existing_path_ -- --nocapture`

### Exit criteria

- replayed workflow mode equals original workflow intent
- restored workflow step equals saved artifact stage when stage truth exists
- rerun requests do not silently narrow or widen execution mode
- all touched flows have targeted regression coverage

## Epic B: Validation Truth Alignment

### Goal

Make validation summaries, warnings, route notes, and exported report semantics match actual backend validation behavior.

### Priority surfaces

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

### Key risks to eliminate

- UI route notes claiming support that the backend actually falls back from
- XLSX or CSV report wording hiding CPU fallback or mixed-route behavior
- summary text implying a stronger verdict than the backend actually computed
- validation warnings reflecting stale route policy rather than current code

### Checklist

- [x] Audit frontend validation route text against current backend route policy in code.
- [x] Audit report builders for fallback/mixed-route wording drift.
- [x] Normalize the distinction between supported, CPU-fallback, mixed-route, and unsupported.
- [x] Ensure shortlist, verdict, and warning summaries reuse the same truth model where practical.
- [x] Add or extend tests where report wording and UI wording are expected to stay aligned.
- [x] Review strategy-family route notes called out in `REPO_MAP.md` and correct any stale product-facing claims.

### Progress

- `2026-04-16`: Completed a bounded VCET validation truth-alignment slice across preview, post-run, and workbook/report surfaces.
- Implemented:
  - updated `web/src/services/validationService.ts` and `src-tauri/src/commands_reports_validation_common.rs` so VCET is described as `GPU-native` for the current fixed-candidate validation module stack on the `GpuKernel` route, including walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap
  - updated `web/src/services/validationRequestBuilder.ts` so preview capability summaries no longer misclassify VCET `perturb` and `trade_bootstrap` as auxiliary CPU tails
  - added/updated targeted tests in `web/src/services/validationService.test.ts` and `src-tauri/tests/validation_xlsx_report_tests.rs` to keep preview wording, post-run wording, and workbook wording aligned for VCET
- Verification completed for this slice:
  - `npm --prefix web run test -- --run src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`
  - `npm --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
- `2026-04-16`: Completed a second validation truth-alignment slice for advanced composite GPU-native route wording.
- Implemented:
  - updated `web/src/services/validationRequestBuilder.ts`, `web/src/services/validationService.ts`, and `src-tauri/src/commands_reports_validation_common.rs` so `Vcc`, `Carm`, and `Cdmc` no longer use stale mixed-route wording and instead describe the current `GpuKernel` fixed-candidate module stack truthfully
  - removed stale `Carm`/`Cdmc` hybrid category treatment in preview, post-run, and report summaries so these lanes now classify as `GPU-native` when they actually run on GPU
  - normalized advanced-composite auxiliary-tail handling so preview and report summaries no longer imply `perturb` / `trade_bootstrap` CPU tails for the current GPU-native advanced composite set
  - updated targeted assertions in `web/src/services/validationRequestBuilder.test.ts`, `web/src/services/validationService.test.ts`, and `src-tauri/tests/validation_xlsx_report_tests.rs`
- Verification completed for this slice:
  - `npm --prefix web run test -- --run src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`
  - `npm --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
- `2026-04-16`: Completed a third validation truth-alignment slice for current composite-profile fixed-candidate validation routing and preserved-finalist wording.
- Implemented:
  - updated `web/src/services/validationRequestBuilder.ts`, `web/src/services/validationService.ts`, and `src-tauri/src/commands_reports_validation_common.rs` so `CompositeTrend`, `BreakoutMomentum`, `RegimeHybrid`, and `CompositeMo` preview/post-run/report summaries now describe the current validation-route truth instead of reusing optimization-route assumptions
  - normalized CPU-preferred vs GPU-hybrid wording for `CompositeMo`, `CompositeTrend`, and `BreakoutMomentum` so `Auto`, explicit `GpuKernel`, and fixed-candidate fallbacks no longer imply unsupported or stronger GPU support than the backend actually provides
  - aligned preserved-finalists wording across preview, report, and workbook surfaces so rerun/retained-candidate summaries consistently describe the current `finalists only (cap=4)` contract
  - updated targeted frontend and workbook assertions so route-category text, route-note text, and preserved-finalist summary wording stay anchored together
- Verification completed for this slice:
  - `npm --prefix web run test -- --run src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`
  - `npm --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app composite_trend_auto_expected_summary_tracks_cpu_and_hybrid_validation_routes -- --nocapture`
  - `cargo test -p ta_desktop_demo_app composite_mo_validation_remains_cpu_preferred -- --nocapture`
  - `cargo test -p ta_desktop_demo_app composite_preserved_finalists_cross_summary_reports_cap_limited_finalists_only -- --nocapture`

### Exit criteria

- frontend route summary, backend behavior, and report wording no longer disagree in the touched lanes
- validation warning text is explicit about fallback or unsupported scope
- workbook tests cover the corrected summary semantics

## Epic C: Artifact System Cleanup

### Goal

Make artifact save, list, reopen, hydrate, compare, and rerun behavior consistent across history-heavy flows.

### Priority surfaces

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/historyService.ts`
- `web/src/components` artifact-heavy panels routed from history and library flows
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports_api.rs`

### Key risks to eliminate

- artifact metadata means different things in different UI surfaces
- open/reveal/report actions accept malformed paths inconsistently
- tagged retention and family-specific retention logic drift from operator expectations
- meta/stage/ML artifact families restore partial state differently for no strong reason

### Checklist

- [x] Inventory artifact family metadata actually persisted today.
- [x] Define the minimum canonical metadata set needed for list, reopen, rerun, compare, and reveal.
- [x] Normalize path sanitation and open/reveal behavior across artifact and report commands.
- [x] Normalize artifact-family naming and UI summaries where semantics are duplicated but rendered differently.
- [x] Add targeted tests for artifact CRUD, hydration, and reopen/reveal failure paths.
- [x] Review prune and retention logic for asymmetric family behavior that is accidental rather than intentional.

### Progress

- `2026-04-16`: Completed the first artifact-cleanup sub-slice while closing Epic A replay gaps.
- Implemented:
  - hardened `save_run_artifact(existing_path=...)` and `delete_run_artifact` so run-artifact commands reject non-run artifact families inside `runs/`
  - deterministic `ts` fallback for legacy run artifacts without embedded `ts`, shared across summary/detail/hydration
  - latest-run fallback selection now uses the same resolved run timestamp contract instead of path order when scanning nonstandard legacy run filenames
  - stronger latest-run rehydration equivalence checks in the frontend artifact sync layer
  - ML-signal path-normalization reuse through shared validation helpers
- Remaining high-value Epic C items:
  - unify open/reveal/report path sanitation across command surfaces
  - normalize cross-surface artifact-family wording and summary rendering
  - review prune/retention asymmetries once command/path contracts are stable
- `2026-04-16`: Completed aggregate-library delete parity for saved meta, Stage 5, and policy-evaluation artifacts.
- Implemented:
  - added a new Tauri `delete_artifact` command backed by `run_artifacts::delete_artifact_blocking` so aggregate-library deletion supports the same artifact families already accepted by tag updates
  - kept `delete_run_artifact` run-only so workflow-history deletion retains the narrower guardrail for saved run artifacts
  - switched `web/src/services/metaService.ts` delete flows for saved meta, saved Stage 5, and saved policy evaluations from `delete_run_artifact` to `delete_artifact`
  - added Rust tests for allowed aggregate-library deletes and rejection of unsupported live-session artifacts, plus web regression tests covering the new delete command usage in the meta-service layer
- Verification completed for this slice:
  - `cargo test -p ta_desktop_demo_app delete_artifact_blocking -- --nocapture`
  - `npm --prefix web run test -- --run src/services/metaService.test.ts`
  - `npm --prefix web run gen:commands`
  - `npm --prefix web run typecheck`
- `2026-04-16`: Completed persisted report-path existence hygiene for saved-run and saved-meta reopen surfaces.
- Implemented:
  - added optional `report_xlsx_exists` / `report_path_exists` metadata in `src-tauri/src/run_artifacts.rs` for run summaries, run detail, workflow hydration payloads, meta summaries, and meta replay payloads while preserving the original stored path text
  - updated `web/src/services/historyService.ts` and `web/src/stores/historyStore.ts` so saved-run report affordances no longer treat a non-empty persisted path as inherently openable when the backend knows the XLSX file is gone
  - updated `web/src/stores/workflowArtifactsStore.ts`, `web/src/services/workflowArtifactsSync.ts`, and `web/src/stores/commandActionStore.ts` so hydrated workflow-artifact report quick actions respect the new existence bit and treat report-existence flips as meaningful artifact updates
  - updated `web/src/services/metaService.ts` so saved-meta and latest-meta report actions stop advertising/opening stale XLSX paths, replayed saved-meta loads clear both the runtime dataset and command-action-store quick-action fallback, and meta-file reveal now falls back to the JSON artifact when the saved XLSX is missing
  - aligned history workspace rendering and saved-run detail text so stale persisted report paths no longer render as if an XLSX is still available and instead surface an explicit missing-on-disk state
  - added targeted Rust and web regression coverage for missing-report and existing-report cases across summary, detail, hydration, saved-run, saved-meta, command-palette, and workflow-artifact store paths
- Verification completed for this slice:
  - `cargo fmt --all -- src-tauri/src/run_artifacts.rs src-tauri/src/commands_campaign.rs`
  - `npm --prefix web run test -- --run src/services/historyService.test.ts src/services/metaService.test.ts src/stores/historyStore.actions.test.ts src/stores/commandActionStore.test.ts src/stores/workflowArtifactsStore.test.ts`
  - `npm --prefix web run test -- --run src/services/metaService.test.ts src/stores/commandActionStore.test.ts src/stores/historyStore.actions.test.ts src/services/validationService.test.ts`
  - `npm --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app run_artifact_surfaces_report_existence_for_summary_detail_and_hydration -- --nocapture`
  - `cargo test -p ta_desktop_demo_app meta_artifact_summary_marks_missing_report_paths_as_unopenable -- --nocapture`
- `2026-04-16`: Completed the remaining artifact cleanup tranche for canonical metadata, family naming, retention parity, and Stage 5 wording.
- Implemented:
  - `src-tauri/src/run_artifacts.rs` now carries canonical `kind` metadata across non-run artifact summaries, detail payloads, and replay payloads so list/reopen/reveal paths share one family identity surface
  - added generic `load_artifact` support and switched non-run artifact consumers to that alias where the payload is family-agnostic
  - canonicalized current batch-summary naming to `vectorgrid_batch_summary_*` while keeping explicit legacy-prefix acceptance for older artifacts, and normalized current UI summaries away from stale Stage 4 path-alias wording
  - unified saved-artifact retention so Stage 4/Stage 5 compatibility prefixes prune as one family, and added dedicated prune hooks for batch summaries, campaign summaries, and options-expression artifacts
  - trimmed/sanitized persisted path resolution in shared report/path helpers instead of preserving whitespace-only distinctions that the command surface does not semantically honor
- Verification completed for this slice:
  - `npm --prefix web run test -- --run src/services/metaService.test.ts src/services/metaServiceCampaignResume.test.ts src/components/MetaWorkspacePanel.test.ts`
  - `cargo test -p ta_desktop_demo_app prune_old_meta_artifacts_preserves_tagged_older_entries -- --nocapture`
  - `cargo test -p ta_desktop_demo_app existing_path_ -- --nocapture`
  - `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit criteria

- artifact metadata presentation is materially more consistent across history/library/replay surfaces
- open/reveal/report path failure behavior is explicit and uniform in the touched commands
- save/load/replay tests cover the corrected contracts

## Epic D: Options Expression Shipped-Slice Hardening

### Goal

Strengthen the current options workflow that already exists without turning this program into an options feature expansion effort.

### Priority surfaces

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`

### Key risks to eliminate

- shipped-slice request legality not matching actual backend support
- feasibility and tuple inspection artifacts carrying insufficient reopen truth
- event-path or report surfaces using wording that implies a larger product scope than exists
- saved options artifacts replaying differently than live requests

### Checklist

- [x] Reconfirm the current shipped slice and explicitly mark out-of-scope future ideas.
- [x] Audit request shaping against current backend support and failure modes.
- [x] Audit options artifact payloads, replay provenance, and CSV/open/reveal surfaces for inspectability gaps.
- [x] Tighten feasibility and realism diagnostics already described in current options docs where they are clearly implementable.
- [x] Add tests for current request-builder legality, artifact replay, and report/open/reveal behavior.
- [x] Keep all work scoped to the existing slice rather than broadening the options product.

### Progress

- `2026-04-16`: Completed a current-slice request-legality and operator-flow hardening pass without broadening the shipped options product.
- Implemented:
  - kept the shipped scope anchored to the current options parent-family adapters and existing structure/fill-model lanes already described in `options_research/options_expression_optimizer_viable_plan_2026-04-02.md` and `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
  - hardened `web/src/stores/optionsExpressionStore.ts` and `crates/app_core/src/options_expression.rs` so premium/volume/open-interest and max-entry-lag filters fail fast on illegal negative, inverted, or non-whole-number combinations instead of drifting into backend-only surprises
  - updated `web/src/services/optionsExpressionService.ts`, `web/src/components/OptionsExpressionPanel.vue`, and `web/src/stores/optionsExpressionStore.ts` so feasibility replay-build timing is surfaced to the UI, reveal actions use true reveal-only behavior, and saved-artifact dataset rebinding no longer silently swaps to an arbitrary same-lane dataset
  - kept direct provider download/product scope unchanged: the current download path remains aggregate-lane only and no new options product families or scenario lanes were introduced
- Verification completed for this slice:
  - `npm --prefix web run test -- --run src/services/optionsExpressionService.test.ts src/stores/optionsExpressionStore.test.ts src/components/OptionsExpressionPanel.test.ts`
  - `cargo test -p ta_app_core run_options_expression_rejects_negative_filter_values --lib -- --nocapture`
  - `cargo test -p ta_app_core feasibility_preview_rejects_inverted_premium_ranges --lib -- --nocapture`
- `2026-04-16`: Completed provenance-guarded saved-artifact replay hardening for the shipped options workflow.
- Implemented:
  - persisted additive replay provenance on `OptionsExpressionArtifactV1` in `crates/app_core/src/options_types.rs` and `src-tauri/src/commands_options_expression.rs`, including `parent_artifact_hash`, `signal_manifest_id`, and dataset provenance fields for provider/source-surface/quote-resolution/aggregate-price-field/contract-reference-as-of
  - replay validation now rejects drift between a saved options artifact and the current replay inputs when parent artifact hash, signal manifest id, dataset id/hash, price semantics, or recorded dataset provenance no longer match
  - kept backward compatibility for older saved artifacts by making the new provenance fields additive and optional rather than changing the existing artifact kind/version contract
  - extended the Rust test fixtures and replay-provenance tests so the happy path remains valid while concrete drift in parent hash, signal manifest id, provider provenance, and contract-reference provenance is now caught explicitly
- Verification completed for this slice:
  - `cargo fmt --all -- crates/app_core/src/options_types.rs src-tauri/src/commands_options_expression.rs`
  - `cargo test -p ta_desktop_demo_app saved_artifact_replay_provenance -- --nocapture`
  - `cargo test -p ta_desktop_demo_app summarize_options_expression_artifact_preserves_price_semantics_provenance -- --nocapture`

### Exit criteria

- options request and artifact behavior are more deterministic inside the already-shipped scope
- no scope creep into new options product lanes occurred
- touched flows have targeted tests

## Track X1: Warning-Noise And Dormant-Path Cleanup

### Goal

Reduce compile-time warning noise that obscures real regressions, but only in bounded, low-risk slices.

### Candidate surfaces

- dormant helper binaries under `crates/app_core/src/bin/`
- narrowly dead `cfg(test)` helpers
- obviously feature-gated or no-op stubs that can be isolated more cleanly

### Rules

- do not hide real warnings with blanket `allow` attributes when a cleaner gate/stub split is available
- do not refactor large runtime modules just to reduce warnings
- prefer narrow feature gating, test gating, or explicit stub entry points

### Checklist

- [ ] Reduce warning clusters in helper bins that are inactive outside feature-enabled paths.
- [x] Gate test-only helpers to test builds where practical.
- [x] Add a short note when a warning is intentionally left because the safer cleanup would be too invasive.

### Progress

- `2026-04-16`: Completed a bounded warning-noise cleanup slice in `ta_app_core` by tightening cfg boundaries around CUDA-only and test-only helpers instead of adding blanket `allow` attributes.
- Implemented:
  - gated `composite_validate.rs` CSCV helper functions behind the same CUDA backtest-kernel feature boundary as their only current callers
  - gated `double_ma.rs` GPU-only score-floor helpers and period-capability test helpers behind the same CUDA backtest-kernel boundary as their consumers
  - gated CUDA-only indicator-id fields/helpers in `vcet.rs` and `vemr.rs` so CPU-only/default test builds no longer carry dead fields/imports
  - gated CUDA-only test helpers in `carm.rs`, `cdmc.rs`, `kqsr.rs`, `kvdf.rs`, `mkcs.rs`, and `options_expression.rs` so the default `ta_app_core` lib-test build no longer emits stale dead-code warnings for helpers whose callers only exist in CUDA-enabled tests
  - gated the `composite_cpu.rs` score-row assertion helper to the same CUDA backtest-kernel test scope as its current callers
- Verification completed for this slice:
  - `cargo fmt --all -- crates/app_core/src/composite_validate.rs crates/app_core/src/double_ma.rs crates/app_core/src/vcet.rs crates/app_core/src/vemr.rs`
  - `cargo fmt --all -- crates/app_core/src/carm.rs crates/app_core/src/cdmc.rs crates/app_core/src/kqsr.rs crates/app_core/src/kvdf.rs crates/app_core/src/mkcs.rs crates/app_core/src/composite_cpu.rs`
  - `cargo test -p ta_app_core --lib --no-run`
- Deferred:
  - broader warning cleanup outside the now-warning-free `ta_app_core` default lib-test compile remains intentionally deferred until a similarly bounded cluster is chosen; the current slice stopped at cfg/test-helper cleanup to avoid mixing behavior changes into a warning pass

## Track X2: Verification And Gate Discipline

### Goal

Keep implementation work evidence-backed without dragging every semantic slice into unnecessary benchmark work.

### Default verification posture

- targeted `cargo test` for touched crates/modules
- targeted `vitest` for touched web services/stores/components
- command-boundary tests in `src-tauri` when command behavior changes

### Use broader gates when a cluster is complete

- `powershell -File scripts/run_repo_quality_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- `cargo test --workspace`

### Defer performance-sensitive verification until semantics are stable

- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- CPU perf gates
- GPU perf gates
- workflow timing or benchmark faceoffs

### Checklist

- [x] Keep a running list of which targeted tests validate each completed slice.
- [x] Run broad non-benchmark quality gates after a logically complete cluster.
- [x] Only re-enter benchmark or perf-gate work once semantic contract hardening is green.

### Progress

- `2026-04-16`: Maintained verification discipline across the bounded hardening slices instead of mixing semantic fixes with benchmark noise.
- Implemented:
  - recorded targeted web and Rust verification commands directly under each completed slice in Epics B, C, and D so the evidence trail stays attached to the change cluster it validates
  - ran a broad non-benchmark repo gate after the current contract-hardening cluster stabilized and kept benchmark/perf gates deferred while semantic work remained in flight
  - kept later options-provenance verification targeted to the touched Rust surfaces rather than re-entering CPU/GPU performance work prematurely
- Verification completed for this slice:
  - `powershell -File scripts/run_repo_quality_gate.ps1`
  - `cargo test -p ta_desktop_demo_app saved_artifact_replay_provenance -- --nocapture`
  - `cargo test -p ta_desktop_demo_app summarize_options_expression_artifact_preserves_price_semantics_provenance -- --nocapture`

## Track X3: Documentation And Operator-Facing Wording Consistency

### Goal

Keep UI copy, report wording, and operational docs aligned to actual behavior after implementation changes.

### Candidate surfaces

- `README.md`
- `RELEASE.md`
- `web/README.md`
- inline help text in workflow panels
- report summary wording where tests anchor expected phrasing

### Checklist

- [x] Update operator-facing wording after route/fallback truth changes.
- [x] Keep `REPO_MAP.md` references honest if an entrypoint or ownership boundary materially changes.
- [x] Avoid silently leaving stale product wording after implementation hardening is finished.

### Progress

- `2026-04-16`: Updated operator-facing repo docs and tracker wording so the hardening work does not leave stale product claims behind.
- Implemented:
  - refreshed `REPO_MAP.md` artifact notes to describe the canonical `report_xlsx_path` / `report_xlsx_exists` contract and the Stage 4 compatibility-alias status honestly
  - aligned repo-wide tracker wording for the options workflow to talk about artifact payloads, replay provenance, and CSV/open/reveal behavior instead of implying a broader report/XLSX product lane than the current shipped slice actually has
  - kept validation route/fallback wording, artifact metadata wording, and options shipped-slice wording synchronized with the code-facing surfaces updated in this pass

## Dependency Map

- Epic A unblocks Epic C.
- Epic A reduces false assumptions inside Epic B.
- Epic B and Epic C should both inform Epic D so the options slice inherits the hardened artifact and validation conventions.
- Track X1 should happen opportunistically during the epics, not as a large standalone rewrite.
- Track X2 should run throughout.
- Track X3 should close behind each finished cluster.

## Immediate First Tranche

The highest-value opening tranche is:

1. conventional workflow request/replay drift
2. ML Signal request/replay/artifact drift
3. artifact open/reveal/reopen/report command consistency
4. validation route summary honesty in the currently touched strategy families

That tranche should produce:

- a stronger contract matrix
- better replay determinism
- fewer silent failure paths
- better confidence that later report and options hardening work will not duplicate the same mistakes

## Explicit Non-Goals For This Tracker

- adding new strategy families
- expanding options beyond the current shipped slice
- redesigning the shell or result workspaces as a visual project
- reopening large historical research topics unless current code demands it
- benchmark hunting before contract correctness work is complete

## Update Protocol

After each completed slice:

- mark the relevant checklist items in this file
- record the exact targeted verification commands run
- note any intentionally deferred work and why it was deferred
- only create a narrower implementation plan if one epic branch becomes too large to track cleanly here

Until that threshold is crossed, this file should remain the source-of-truth tracker for this repo-wide implementation program.
