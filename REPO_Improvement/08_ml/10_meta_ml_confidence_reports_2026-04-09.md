# Meta ML Confidence Reports Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the Meta workspace ML confidence and Stage 5 report surfaces so model status, report reveal/open behavior, and policy-evaluation summaries stay explicit and trustworthy.

## Scope Boundaries

In scope:

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.ts`
- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/ml_confidence.rs`
- ML confidence status, Stage 5 output preview, policy evaluation preview, and related report/reveal actions

Out of scope:

- low-level ML model training internals beyond surfaced status/report contracts
- generic non-meta reporting
- options-related report surfaces

## Current User-Facing Behavior

The Meta workspace already surfaces a distinct ML confidence stage:

- ML confidence enable/autopilot controls
- status text around model health and application
- Stage 5 output preview
- policy evaluation preview
- open/reveal flows for meta report files and saved stage5/policy-eval artifacts

This is a lot of power, but it needs careful presentation. Model health, confidence application, Stage 5 output, and policy evaluation are related but not interchangeable.

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.ts`
- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/ml_confidence.rs`
- relevant tests in `metaService.test.ts` and `MetaWorkspacePanel.test.ts`

## Current Verification Surface

- `MetaWorkspacePanel.test.ts`
- `metaService.test.ts`
- `metaServiceCampaignResume.test.ts`
- `metaServiceCampaignStrategy.test.ts`
- backend ML confidence checks in `commands_meta.rs` and `ml_confidence.rs`

## Strengths Already Present

- ML confidence is already surfaced as its own post-meta phase
- model-health warnings already exist in the backend
- Stage 5 and policy-eval previews already have explicit UI homes
- reveal/delete/load actions for saved stage5 and policy-eval artifacts already exist

## Weaknesses / Failure Modes

- “ML confidence” can refer to status, application, Stage 5 ranking, and policy eval in quick succession
- the preview/report surface is dense enough that status provenance can blur
- it is easy for users to read a preview without understanding whether it reflects live output, saved output, or model status
- model health warnings are only useful if they remain prominent and specific

## Regressions To Avoid

- hiding model-health warnings under generic success messaging
- conflating Stage 5 output with policy-eval results
- breaking reveal/open behavior for saved stage5 or policy-eval reports
- weakening the distinction between model status and artifact output
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because Stage 5 and policy-eval outputs are structured artifacts that need stable semantics.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because much of the ML confidence/report payload is still transmitted as flexible JSON.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the ML confidence UI is fundamentally a typed Rust-command report surface.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because model status, preview text, and saved artifact selections are shared across the meta workspace.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because preview panels, disclosures, and saved-artifact actions need clear, navigable semantics in a dense review surface.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Separate the ML confidence report surface into clear conceptual outputs:
   - model status/health
   - Stage 5 ranking output
   - policy-evaluation output
   - saved artifact/report actions
2. Audit status and preview text so each output states its provenance.
3. Keep model-health warnings prominent and hard to miss.
4. Add tests that verify saved stage5 and policy-eval families remain distinguishable in UI and service behavior.
5. Prefer clearer report semantics over adding more preview density.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the distinction between model status, Stage 5 output, and policy evaluation output.
- [ ] Audit UI copy and preview labeling for provenance clarity.
- [ ] Add or tighten tests for saved stage5 and saved policy-eval reveal/load/delete behavior.
- [ ] Ensure model-health warnings remain visible and specific when ML confidence is enabled or applied.
- [ ] Verify open/reveal report actions still behave correctly across live and saved meta flows.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `MetaWorkspacePanel.test.ts`
- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- run targeted backend tests in `commands_meta.rs` and `ml_confidence.rs` if status/report semantics change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current saved artifact families and report paths
- make report semantics clearer before changing any underlying ML stages
- if a UI cleanup obscures provenance, restore the more explicit preview/status wording
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/09_meta_saved_artifact_families_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
