# Bundle 9 Validated Tracking Checklist

This is the authoritative execution tracker for Bundle 9:

- `gpt54_feature_review_map.md` Section `## 9. Frontend validation orchestration and artifact-heavy flows`
- `bundle9/bundle9_frontend_validation_orchestration_plan (1).md`
- `bundle9/bundle9_frontend_validation_orchestration_plan_addendum.md`
- `bundle9/bundle9_codex_implementation_prompt.md`

This checklist has already been validated against the current repo state.
It tracks only the repo-valid portions of the older Bundle 9 plan/addendum documents.

Rules:

- Only implement items in the unchecked required sections below.
- Treat the older Bundle 9 plan/addendum prose as background only.
- Do not re-implement items marked `[x]` in the preserve sections.
- Use this file, not the older plan prose, as the progress tracker.

## Validation Completed

- [x] Re-read `gpt54_feature_review_map.md` Section 9.
- [x] Re-read `bundle9/bundle9_frontend_validation_orchestration_plan (1).md`.
- [x] Re-read `bundle9/bundle9_frontend_validation_orchestration_plan_addendum.md`.
- [x] Compare the Bundle 9 docs against the current validation service, history service, workflow artifact store, preview/request-builder path, and Rust report/export path.
- [x] Identify already-landed surfaces that should be preserved instead of re-implemented.
- [x] Update `bundle9/bundle9_codex_implementation_prompt.md` so it only directs valid current-repo work.

## Already Present: Preserve, Extend, Do Not Re-implement

- [x] `ValidationWorkspacePanel.vue` remains the canonical validation surface; shortcut surfaces are not competing primary flows.
- [x] `workflowArtifactsStore.ts` already normalizes both `dataset_meta` and `datasetMeta`.
- [x] `historyService.test.ts` already exists and should be extended rather than recreated.
- [x] `commands_reports_validation_common.rs` already has an expected/actual export route-summary helper layer; patch it only where real residual drift remains.
- [x] `save_report_xlsx` already honors `max_rows` on the normal saved-file path.
- [x] Stage 3 meta-report fallback in `exportReportXlsx()` already exists and must stay intact.
- [x] Strict dataset/timeframe/strategy enforcement in `validationService.ts` and backend validation-context strictness in `commands_validation.rs` must stay intact.
- [x] `topRowsFiltered` is supported by the request builder and should not be treated as a dead branch in this bundle.
- [x] Recent frontend layout/shell cleanup is out of scope for Bundle 9.

## Required Implementation Scope

### Timing Truthfulness

- [x] Add separate command-timing and end-to-end timing measurement to `runValidation()`.
- [x] Replace the stale `Validation done in Xms (frontend)` wording with truthful visible status text.
- [x] Keep the visible timing text concise and user-facing while accurately distinguishing command timing from end-to-end timing.

### Dataset Metadata Retention and Reuse

- [x] Extend validation artifact persistence/apply flow so `dataset_meta` / `datasetMeta` survives through persist, Vue apply, saved-run reload, and report payload building.
- [x] Make validation artifact persistence prefer the existing artifact-backed `datasetMeta` before rebuilding fresh metadata.
- [x] Preserve `dataset_meta` when `historyService.loadSavedRun()` hydrates workflow artifacts.
- [x] Make report payload building prefer artifact-backed dataset context when it exists instead of silently rebuilding from current runtime dataset state.
- [x] Preserve existing dataset/timeframe mismatch enforcement while fixing metadata retention.

### Auto-export Orchestration and Fallback `max_rows` Parity

- [x] Remove duplicate run-artifact persistence on the auto-export success path so validation plus auto-export persists exactly once.
- [x] Keep export failure and download fallback non-fatal while still leaving a saved validation artifact behind.
- [x] Pass `max_rows` to the fallback `build_report_xlsx` frontend invocation in `validationService.ts`.
- [x] Update `src-tauri/src/commands_reports_api.rs` so the `build_report_xlsx` command wrapper accepts and forwards `max_rows`.
- [x] Update `src-tauri/src/commands_reports.rs` so `build_report_xlsx_blocking(...)` passes `max_rows` into the validation workbook builder.
- [x] Keep the existing `save_report_xlsx(max_rows)` path and Stage 3 fallback behavior unchanged apart from fallback-cap parity.

### Preview/Post-run/Export Route-policy Parity

- [x] Remove the stale `Vcc` Auto CPU-preferred preview messaging in `validationRequestBuilder.ts`.
- [x] Align the remaining preview-side `Vcc` wording with the current post-run/export route note.
- [x] Align the remaining preview-side `Carm` wording/policy with the current post-run/export route note.
- [x] Align the remaining preview-side `Cdmc` wording/policy with the current post-run/export route note.
- [x] Align the remaining preview-side `Vcet` wording/policy with the current post-run/export route note.
- [x] Keep `CompositeMo` CPU-route semantics explicit across preview/post-run/export.
- [x] Patch `commands_reports_validation_common.rs` only where a real residual export mismatch remains after preview-side correction.
- [x] Preserve the existing Rust export helper layer instead of adding a second route-policy abstraction.

### Focused Regression Coverage

- [x] Add or update `validationService.test.ts` coverage for truthful timing wording.
- [x] Add or update `validationService.test.ts` coverage for `datasetMeta` retention and reuse during validation artifact persistence.
- [x] Add or update `validationService.test.ts` coverage proving auto-export success persists exactly once.
- [x] Add or update `validationService.test.ts` coverage for non-fatal save-fail / download-fallback behavior.
- [x] Add or update `validationService.test.ts` coverage proving fallback `build_report_xlsx` receives `max_rows`.
- [x] Add or update `validationRequestBuilder.test.ts` only for the preview policy cases that still changed.
- [x] Expand `historyService.test.ts` so `loadSavedRun()` replay proves `datasetMeta` survives.
- [x] Add focused Rust tests only for the report/export path that actually changed.

### Verification and Signoff

- [x] Run the targeted frontend tests for `validationService.test.ts`, `validationRequestBuilder.test.ts`, and `historyService.test.ts`.
- [x] Run the targeted Rust tests for the touched report/export path.
- [x] Run the closest practical frontend typecheck or build-equivalent for the touched frontend files.
- [x] Record exact commands, pass/fail results, and any repo-specific blockers.

## Recommended Low-risk Follow-up Only

- [ ] Avoid duplicate `reveal_path` calls when `openPathWithRevealFallback()` already revealed the file.
- [ ] Add shortlist-parity regression coverage only if it stays low-risk after the required fixes land.
- [ ] Reduce route-policy duplication in TypeScript only if a very small shared helper clearly lowers drift without widening scope.

## Optional Follow-up Only

- [ ] Add hidden phase-level timing buckets for validation follow-up work.
- [ ] Memoize preview-summary recomputation only if it stays clearly low-risk after the correctness fixes land.
- [ ] Persist a richer dataset-context snapshot only if the lighter artifact-backed dataset-meta fix proves insufficient.

## Final Acceptance

- [x] `runValidation()` no longer labels command-only time as `frontend`.
- [x] Visible timing text distinguishes end-to-end timing from command timing.
- [x] `dataset_meta` / `datasetMeta` survives validation artifact persistence, Vue apply, saved-run reload, and report payload building.
- [x] Validation artifact persistence prefers existing artifact-backed `datasetMeta` when present.
- [x] Auto-export success persists the run artifact exactly once.
- [x] Export failure/download fallback remains non-fatal and still leaves a saved validation artifact behind.
- [x] Fallback `build_report_xlsx` honors `max_rows`.
- [x] Preview/export no longer claim stale `Vcc` Auto CPU-preferred behavior.
- [x] Preview, post-run, and export wording/categories align for the still-drifting `Vcc`, `Carm`, `Cdmc`, `Vcet`, and `CompositeMo` cases.
- [x] Targeted frontend tests pass.
- [x] Targeted Rust tests pass.

## Verification Log

- [x] `npm --prefix web test -- src/services/validationService.test.ts src/services/validationRequestBuilder.test.ts src/services/historyService.test.ts`
- [x] `npm --prefix web run typecheck`
- [x] `$env:RUSTC_WRAPPER=''; cargo check -p ta_desktop_demo_app --lib`
- [x] `$env:RUSTC_WRAPPER=''; cargo test -p ta_desktop_demo_app --lib build_report_xlsx_blocking_matches_headless_builder_for_max_rows -- --nocapture`
- [x] `$env:RUSTC_WRAPPER=''; cargo test -p ta_desktop_demo_app --lib commands_reports_validation_common::tests:: -- --nocapture`
- [x] Rust verification initially hit a local `sccache` startup timeout and a stale `commands_validation.rs` test initializer; both were corrected before the touched Bundle 9 report/export tests were rerun successfully.
