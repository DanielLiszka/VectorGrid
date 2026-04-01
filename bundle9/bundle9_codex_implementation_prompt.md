# Codex implementation prompt — Bundle 9 frontend validation orchestration and artifact-heavy flows

Use this document as the **exact implementation prompt** for an OpenAI Codex agent working inside the repository.

This prompt is intentionally detailed so Codex can implement the bundle-9 plan end to end without drifting into unrelated product work.

---

## Copy/paste prompt for Codex

You are Codex working in my Rust/Tauri + Vue/Pinia repository.

Your task is to **implement only the repo-valid portions of the bundle-9 validation-orchestration plan** described in these markdown documents:

1. `gpt54_feature_review_map.md` — use **only section `## 9. Frontend validation orchestration and artifact-heavy validation flows`** as the hard scope guard.
2. `bundle9/bundle9_frontend_validation_orchestration_plan (1).md`
3. `bundle9/bundle9_frontend_validation_orchestration_plan_addendum.md`

Treat the plan and addendum as **background input only**. This prompt's repo-validation findings and checklist are authoritative and override any stale instructions in the older docs.
Use `bundle9/bundle9_frontend_validation_orchestration_validated_tracking_checklist.md` as the authoritative progress tracker for this bundle.

Read the docs first, then inspect the repo files listed below, then implement the validated changes. If any detail in the older plan docs conflicts with repo reality, follow this prompt and the current codebase. Do not stop at review or advice; actually make the code changes.

---

## Repo-state validation findings

This prompt is already validated against the current repo state. Use the checklist below as the only implementation scope.

### Validation completed

- [x] bundle-9 prompt, plan, and addendum were audited against the live validation/export/history code
- [x] current scope was cross-checked against `gpt54_feature_review_map.md` section 9 and the newer frontend review docs

### Already present or preserve-only

- [x] `ValidationWorkspacePanel.vue` is still the canonical validation surface; docks/palette actions remain shortcuts, not competing primary surfaces
- [x] `workflowArtifactsStore.ts` already normalizes both `dataset_meta` and `datasetMeta`; do not redesign that store contract
- [x] `historyService.test.ts` already exists; extend it instead of creating a new test file from scratch
- [x] `commands_reports_validation_common.rs` already contains substantial expected/actual route-summary helpers; update the existing helpers rather than adding a second Rust route-policy layer
- [x] `save_report_xlsx` already honors `max_rows` on the normal saved-file path
- [x] Stage 3 meta-report fallback in `exportReportXlsx()` already exists and should remain intact
- [x] strict dataset/timeframe/strategy matching in `validationService.ts` and backend validation-context strictness in `commands_validation.rs` already exist and must remain intact
- [x] recent frontend layout/shell cleanup is out of scope for this bundle

### Authoritative validated checklist

#### Required implementation scope

- [ ] make `runValidation()` timing truthful: stop labeling command-only time as `frontend`, and distinguish command timing from end-to-end timing in visible status text
- [ ] preserve `dataset_meta` / `datasetMeta` through validation artifact persistence, Vue apply, saved-run reload, and report payload building
- [ ] make validation artifact persistence prefer the existing artifact-backed `datasetMeta` when present, falling back to rebuilt metadata only when it is missing
- [ ] make report payload building prefer artifact-backed dataset context over silently recomputing fresh runtime dataset context when artifact context exists
- [ ] remove duplicate run-artifact persistence on the auto-export success path while preserving the existing non-fatal auto-export failure/download fallback behavior
- [ ] make the fallback `build_report_xlsx` path honor `max_rows`, including the Tauri command wrapper path in `commands_reports_api.rs`
- [ ] align the still-stale preview-side route/warning/category policy for `Vcc`, `Carm`, `Cdmc`, `Vcet`, and `CompositeMo` with the current post-run/export policy
- [ ] remove the stale `Vcc` Auto CPU-preferred preview messaging so it matches the current post-run/runtime policy
- [ ] preserve current export-common route-policy behavior and patch it only where a real residual mismatch remains after the preview-side correction
- [ ] add or update focused frontend tests in `validationService.test.ts`, `validationRequestBuilder.test.ts`, and `historyService.test.ts`
- [ ] add focused Rust tests only for the report/export path you actually change

#### Recommended low-risk items after the required scope is green

- [ ] avoid duplicate `reveal_path` calls when `openPathWithRevealFallback()` already revealed the file
- [ ] add regression coverage that proves auto-export success persists exactly once and that save-fail/download-fallback paths stay non-fatal
- [ ] add shortlist-parity regression coverage only if it is low-risk after the required fixes land
- [ ] reduce route-policy duplication in TypeScript only if a very small shared helper clearly lowers drift without widening scope

#### Optional follow-up only

- [ ] add hidden phase-level timing buckets for validation follow-up work
- [ ] memoize preview-summary recomputation only if it is still clearly low-risk after the correctness fixes land
- [ ] persist a richer dataset-context snapshot only if the lighter artifact-backed dataset-meta fix proves insufficient

### Scope rule

If any later phase/file-by-file section conflicts with the validated checklist above, or asks you to re-implement something already present, treat the later section as background only and follow the validated checklist.

---

## Hard scope guard

Stay strictly inside the bundle-9 surface:

- validation execution cost
- route-summary fidelity
- shortlist shaping
- artifact persistence
- export-adjacent validation follow-up work
- saved-run replay drift
- frontend/backend validation drift

Do **not** drift into:

- new validation UI surfaces or panel redesign
- generic architecture rewrites
- new strategies or new product knobs
- broad export UX redesign
- loosening dataset/timeframe/strategy validation guards
- unrelated accessibility, visual, shell, or navigation work
- broad backend route-contract redesign unless a very small change is needed for parity

Keep `ValidationWorkspacePanel.vue` as the canonical validation surface. Keep recent frontend-only cleanup out of scope. Preserve features and behavior unless a change is explicitly required for correctness, performance, or parity.

---

## Primary repo files to inspect and likely modify

Use repo-relative paths. If the local checkout uses a slightly different directory prefix than the uploaded copies, adapt accordingly.

### Main in-scope files

- `web/src/services/validationService.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/historyService.ts`
- `web/src/services/historyService.test.ts`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`

### Adjacent files you may need to locate and patch

- `web/src/components/ValidationWorkspacePanel.vue` *(read-only compatibility surface unless a tiny text/assertion adjustment is truly required)*
- `src-tauri/src/commands_validation.rs` *(read-only compatibility surface unless a tiny verification-oriented adjustment is truly required)*
- `web/src/services/validationRoutePolicy.ts` *(new shared helper; recommended if it reduces TS drift cleanly)*
- any store-test file that already covers `workflowArtifactsStore` normalization *(optional only if the repo already has a store test pattern)*

### Files that should usually remain unchanged except for verification or tiny adjustments

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_validation.rs`

Do not make broad changes to those three unless they are truly required to complete the bundle safely.

---

## Non-negotiable constraints

1. **Preserve correctness and feature parity.**
2. **Prefer smaller, parity-safe changes before larger refactors.**
3. **Keep validation completion non-fatal even when export follow-up fails.**
4. **Do not remove `assertValidationRunMatchesCurrentDataset()` behavior or the backend-side `validate_validation_context()` strictness.**
5. **Do not remove download fallback behavior for report export.**
6. **Do not redesign the validation UI.**
7. **Do not add broad new persisted-contract fields unless explicitly called out below as optional.**
8. **Do not introduce backend-owned route metadata in this pass unless you discover a tiny, clearly necessary change.**
9. **Run targeted tests after changes and report exactly what passed.**
10. **Use the repo’s existing test/build tooling rather than inventing new scripts.**

---

## High-level implementation goals

Implement the following in this order:

1. Make validation timing truthful.
2. Propagate and preserve `dataset_meta` / `datasetMeta` end to end.
3. Refactor auto-export so the success path persists the validation artifact only once.
4. Make the XLSX fallback path honor `max_rows` too.
5. Remove route-summary drift across preview, post-run, and export surfaces.
6. Strengthen regression coverage for auto-export, dataset metadata, and route-policy parity.
7. Optionally add the small, low-risk follow-up items after the core path is stable.

Treat the validated checklist above as mandatory. Treat later phase text below as implementation background only.

---

## Phase 1 — timing truthfulness and dataset metadata propagation (mandatory)

### A. Fix validation timing boundaries in `validationService.ts`

The current code labels command-only timing as “frontend”. Change that.

#### Required behavior

- distinguish **command roundtrip time** from **frontend end-to-end time**
- update user-visible status text so it no longer says “frontend” for command-only timing
- keep the status copy concise and user-facing, but accurate
- do not make the panel more complex just to show timings

#### Search anchors

- `runValidation(`
- current timing variables around the `invokeCommand(...)` call
- status-line construction after validation returns

#### Expected implementation shape

Add a small helper like:

- `nowMs()`
- `formatValidationStatus(commandMs, endToEndMs)`

Measure at least:

- `commandMs`
- `validationEndToEndMs`

Optionally, later in phase 4, add hidden/debug phase timings.

#### Acceptance criteria

- the visible status line no longer claims command-only time is “frontend”
- the status line clearly expresses end-to-end timing and command timing
- tests assert wording shape, not exact duration values

---

### B. Preserve `dataset_meta` / `datasetMeta` through the full validation artifact lifecycle

The repo already persists dataset metadata, but hot apply/load paths drop it. Fix that.

#### Required behavior

1. `persistValidationArtifact()` must return dataset metadata in its result object.
2. `applyVueValidationArtifacts()` must apply that metadata back into the workflow artifact store’s `currentRun`.
3. `historyService.loadSavedRun()` must preserve `dataset_meta` when loading a saved run artifact.
4. `buildCurrentReportPayloadFromStore()` must prefer artifact dataset context instead of silently rebuilding from the current runtime data store whenever richer artifact metadata is available.
5. When persisting a validation artifact, prefer the **existing** `workflowArtifactsStore.currentRun.datasetMeta` if present; only rebuild fresh metadata when it is missing.

#### Search anchors

- `persistValidationArtifact(`
- `applyVueValidationArtifacts(`
- `buildCurrentReportPayloadFromStore(`
- `readWorkflowRunArtifactFromStore(`
- `loadSavedRun(`
- `normalizeRunArtifact(` in `workflowArtifactsStore.ts`

#### Required implementation details

- extend the persistence result type to include `datasetMeta`
- when persisting, prefer:
  1. existing `currentRun.datasetMeta`
  2. fallback to `buildDatasetMetaForArtifact(...)`
- when applying workflow payloads after validation, pass dataset metadata into `currentRun`
- when loading saved runs from disk, map `record.dataset_meta` into the `currentRun` payload
- add a helper if needed to build report `data.meta` / `data.ctx` from artifact-backed data instead of always recomputing from the active data store

#### Constraints

- do not break the existing normalization contract in `workflowArtifactsStore.ts`
- avoid changing `workflowArtifactsStore.ts` production logic unless necessary; it already supports `dataset_meta` / `datasetMeta`
- do not remove current fallback behavior to runtime store data when artifact data is absent

#### Acceptance criteria

- after validation, `workflowArtifactsStore.currentRun.datasetMeta` survives
- after `loadSavedRun()`, `workflowArtifactsStore.currentRun.datasetMeta` survives
- report export prefers persisted artifact dataset context when available
- existing dataset/timeframe mismatch enforcement still works

---

### C. Add targeted tests for Phase 1

#### `validationService.test.ts`

Add or update tests so they cover:

- truthful timing wording (`end-to-end` plus `command`, no stale `frontend` wording)
- dataset metadata retention after a successful validation run
- reuse of existing `workflowArtifactsStore.currentRun.datasetMeta` during validation artifact persistence

#### `historyService.test.ts`

Add tests that cover:

- `loadSavedRun()` preserving `dataset_meta`
- re-applied run artifacts still carrying `datasetMeta` after load

Do not make these tests brittle; assert shape and key fields, not exact timestamp text.

---

## Phase 2 — auto-export path, export fallback parity, and follow-up orchestration (mandatory)

### A. Eliminate duplicate run-artifact persistence on the auto-export success path

The current success path persists a validation artifact in `runValidation()`, then can persist again immediately after `exportReportXlsx()` when the XLSX path is known. Remove that duplication.

#### Required behavior

- validation with `autoExport=false` still persists once
- validation with `autoExport=true` persists **exactly once** on the success path
- export failure must remain non-fatal
- download-fallback success must not prevent artifact persistence
- saved artifact should still carry the final `reportXlsxPath` when a real saved report path exists

#### Search anchors

- `runValidation(`
- `exportReportXlsx(`
- `persistValidationArtifact(`
- `save_run_artifact`

#### Recommended implementation direction

- let `exportReportXlsx()` support a mode where it **does not** persist the run artifact itself
- let `runValidation()` own the final single persistence step when auto-export is enabled
- keep the final validation state update non-fatal if export/open/reveal fails

A safe pattern is:

1. run validation command
2. shape shortlist/table/store state
3. if `autoExport=false`, persist artifact once
4. if `autoExport=true`, export first, gather `reportXlsxPath`, then persist artifact once with that path

#### Acceptance criteria

- only one `save_run_artifact` on the success path with auto-export enabled
- auto-export failure still leaves a saved validation artifact behind
- final workflow artifact state contains the correct `reportXlsxPath` when the save path exists

---

### B. Make fallback XLSX export honor `max_rows`

The success path already threads `max_rows`, but the fallback builder path does not. Fix that end to end.

#### Required behavior

- `save_report_xlsx` path honors `max_rows`
- `build_report_xlsx` fallback path also honors `max_rows`
- the degraded/base64 fallback path should not produce a larger workbook or do more work than the normal save path

#### Search anchors

- `exportReportXlsx(` in `validationService.ts`
- `build_report_xlsx` command invocation
- `build_report_xlsx` Tauri command in `commands_reports_api.rs`
- `build_report_xlsx_blocking(` in `commands_reports.rs`
- the actual Tauri command wrapper exposing `build_report_xlsx`

#### Required implementation details

- add `max_rows` to the fallback command invocation arguments
- update `commands_reports.rs` so `build_report_xlsx_blocking(report, max_rows)` passes the cap into `build_validation_report_xlsx(...)`
- patch any Tauri command wrapper signature needed so frontend and Rust stay in sync

#### Constraints

- do not redesign workbook generation
- do not change workbook semantics beyond honoring the existing row cap consistently

#### Acceptance criteria

- fallback tests prove `max_rows` is passed to `build_report_xlsx`
- Rust compilation passes after the signature change

---

### C. Avoid duplicate reveal calls on fallback-open paths (recommended low-risk addendum item)

Implement this after the single-save refactor if it is still low-risk.

#### Required behavior

- keep current behavior where opening and/or revealing can happen
- but if `openPathWithRevealFallback()` already revealed the file because `open_path` failed, do **not** call `reveal_path` again from the later `autoReveal` block

#### Recommended implementation

Make `openPathWithRevealFallback()` return something like:

```ts
interface OpenPathOutcome {
  mode: 'opened' | 'revealed'
}
```

Then skip the later reveal if the helper already revealed.

#### Acceptance criteria

- when `open_path` fails and fallback reveal succeeds, `reveal_path` is called once, not twice

---

### D. Strengthen auto-export regression coverage

Update `validationService.test.ts` to cover these scenarios explicitly:

1. **Successful validation with `autoExport=false`**
   - one `save_run_artifact`
   - zero report-save calls
   - no open/reveal calls

2. **Successful validation with `autoExport=true`**
   - one artifact save only
   - one report save
   - correct final `reportXlsxPath`

3. **Report save fails, download fallback succeeds**
   - artifact still saved once
   - status reflects fallback/download result
   - no stale `reportXlsxPath`

4. **Artifact persistence fails but validation succeeds**
   - validation rows and summary state still populate
   - persistence warning is visible/non-fatal

5. **No duplicate reveal when fallback already revealed**

Keep existing Stage 3 fallback behavior coverage unless the repo has already renamed that path.

---

## Phase 3 — preview/post-run/export route-policy parity (mandatory)

### A. Fix route-note drift in `validationRequestBuilder.ts`

The preview strings are currently the stale side for part of this bundle. Fix them without re-implementing the already-landed Rust export-common helper layer.

#### Required route-text updates

Update preview-side route detail strings and CPU-preference rules so they match current post-run/export behavior.

Use the following intended wording:

- **VCC GPU route note**
  - `VCC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. This removes the old fp32 momentum roundtrip fallback while keeping strict validation parity.`
- **VCC Auto CPU preference**
  - **empty / none**
- **CARM GPU route note**
  - `CARM GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.`
- **CDMC GPU route note**
  - `CDMC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Fresh advanced-stack benches show GPU faster than CPU on the measured 50k/8 and 100k/32 workloads.`
- **VCET GPU route note**
  - `VCET GPU validation now supports the fixed-candidate GPU route for the main profile. Walk-forward currently uses the generic validation path, and perturb/trade bootstrap remain auxiliary CPU work.`

Keep `CompositeMo` CPU-route semantics explicit, keep `Carm` Auto CPU-preferred, keep `Cdmc` Auto GPU-routed, and make preview/post-run/export agree.

#### Required category-parity behavior

Keep category classification aligned with the current intended live policy:

- `Vcc` is no longer Auto CPU-preferred
- `Carm` remains Auto CPU-preferred
- `Cdmc` remains Auto GPU-routed
- `Vcet` remains GPU-routed with explicit auxiliary-CPU notes where relevant
- `CompositeMo` remains CPU-route / CPU-preferred in the current canonical user-visible summaries

#### Recommended implementation

If low-risk, create a shared TS helper such as:

- `web/src/services/validationRoutePolicy.ts`

and centralize:

- strategy-kind normalization
- double-like classification
- route-detail strings
- auto-CPU preference strings
- preview category rules
- actual/post-run category rules where possible
- bootstrap summary helpers if that reduces duplication cleanly

At minimum, stop maintaining stale preview-side string tables that drift from the current post-run/export policy.

#### Search anchors

- `normalizeStrategyKindRaw(`
- `isDoubleLikeKind(`
- `compositeProfileGpuRouteNote(`
- `compositeProfileAutoCpuPreferenceNote(`
- `buildValidationCategorySummaryForMode(`
- `buildValidationCapabilitySummary(`

#### Acceptance criteria

- preview no longer says VCC is Auto CPU-preferred
- preview wording for VCC/CARM/CDMC/VCET matches the current post-run/export intent
- preview category summaries no longer drift from the current intended policy

---

### B. Fix export-side route drift in `commands_reports_validation_common.rs` only if a residual mismatch remains

The export layer already has local helper structure. Patch it only if a real residual mismatch remains after correcting the stale preview-side behavior.

#### Required behavior

- preserve the existing expected/actual export helper layer
- keep VCC not Auto CPU-preferred in expected export summaries
- keep actual route-note summaries for VCC/CARM/CDMC/VCET/CompositeMo aligned with post-run summaries
- patch export strings/categories only where a real mismatch still exists after the preview-side correction

#### Recommended implementation

Do **not** add a new helper layer here unless you discover the existing one is missing a tiny required function. Prefer small edits to the current helper functions and model population.

#### Search anchors

- `build_expected_execution_summary` or equivalent expected-summary builder
- `actual_route_note_summary`
- VCC/CARM/CDMC-specific string tables
- category-summary builders

#### Acceptance criteria

- export-side expected summaries do not drift from the corrected preview policy
- export-side actual route-note summaries do not drift from post-run behavior
- add/update Rust tests only if you actually change export-common route-policy code

---

### C. Update tests for preview/export parity

#### `validationRequestBuilder.test.ts`

Update/add tests so they cover:

- VCC preview is no longer CPU-preferred
- VCC warning/route summary matches the new route-note intent
- CDMC wording uses `exact CPU-series validation metrics`
- CARM wording is updated if needed
- any affected VCET or CompositeMo expectations remain correct

#### `commands_reports_validation_common.rs` tests *(only if you touch Rust export-common route-policy code)*

Add focused Rust unit tests that prove the specific edited behavior:

- VCC expected summary does **not** advertise Auto CPU preference
- any edited CARM/CDMC/VCET summaries align with the current preview/post-run intent
- CompositeMo CPU-route note remains explicit if you touch that path

Keep the tests narrow and deterministic.

---

## Secondary implementation item — shortlist duplication only (recommended only after the required remainder)

This item is real, but should not derail the core patch.

### A. Shortlist parity

There is known shortlist duplication between frontend TS and Rust export code.

#### Required minimum action

Add representative regression coverage so shortlist behavior does not silently drift further.

#### Preferred action if low-risk

Add small fixture-like tests or paired cases that keep frontend shortlist membership and Rust export shortlist flags aligned for representative inputs.

#### Avoid in this pass

- a large cross-language contract rewrite
- a broad persisted shortlist schema change

---

### B. Preserve existing `topRowsFiltered` behavior

Inspect:

- `resolveValidationBuildContextFromWorkflowArtifacts()` in `validationService.ts`
- `buildValidationRequest()` in `validationRequestBuilder.ts`

Current repo state: `topRowsFiltered` is supported by the request builder and should be treated as compatibility/live behavior, not as a dead branch to remove in this bundle.

#### Required decision

Do not turn this into a wiring/removal project unless you discover a concrete bug while implementing the required scope. If you touch shortlist behavior here at all, keep the change limited to parity coverage and preserve the existing filtered-row behavior.

#### Acceptance criteria

- no prompt/task text asks Codex to remove or rewire `topRowsFiltered` as if it were dead
- any shortlist tests added in this pass preserve the current filtered-row behavior

---

## Phase 4 — optional low-risk addendum items (only after Phases 1–3 pass)

Implement these only if they remain low-risk after the mandatory work is done.

### A. Add hidden phase-level timing buckets

Recommended hidden timing fields:

- `requestBuildMs`
- `commandMs`
- `shortlistMs`
- `tableMs`
- `persistMs`
- `applyArtifactsMs`
- optional `exportMs`
- `endToEndMs`

Keep them out of the main UI unless there is already a suitable debug surface. A hidden store field or dev-only state is fine.

Minimum test assertion:

- values exist and are finite
- `endToEndMs >= commandMs`

### B. Memoize preview-summary recomputation

Only do this after route-policy helper extraction is stable.

Memoize `previewValidationExecutionSummaryFromStores()` by a deterministic signature based on:

- current run artifact ts/path
- strategy kind
- validation form inputs that affect preview
- selected cross datasets

Do not overcomplicate this. Skip it if it risks making the preview path harder to reason about.

### C. Persist richer dataset context snapshot

This is optional and future-oriented. Only do it if it is clearly low-risk and version-safe.

Possible extension:

- persist richer `data.dataset_ctx` / `data_meta` / `base_dataset_ctx` / `base_data_meta`

Do **not** take on a large artifact versioning project in this pass.

### D. Avoid broader DOM-write cleanup

The original plan mentions duplicate DOM writes and possible cleanup. Do **not** remove direct DOM writes in this pass unless you can prove the legacy shell no longer depends on them and the change is tiny.

---

## File-by-file expectations

Use this section as background only. If any item here conflicts with the validated checklist or preserve-only findings above, the validated checklist wins.

### `web/src/components/ValidationWorkspacePanel.vue`

- Prefer **no production logic change**.
- Verify the existing preview block correctly reflects the improved service-layer output after the other changes.
- Touch only if a tiny adjustment is required for a renamed status/meta field.

### `web/src/services/validationService.ts`

Mandatory:

- truthful timing boundaries
- persistence result expansion
- datasetMeta propagation
- reuse existing artifact datasetMeta when saving
- prefer artifact-backed dataset context for report payloads
- single-save auto-export refactor
- fallback `max_rows` pass-through
- keep export failure non-fatal

Recommended low-risk addendum:

- avoid duplicate reveal after fallback reveal
- add hidden phase timings if still low-risk

### `web/src/services/validationService.test.ts`

Mandatory:

- updated timing expectations
- datasetMeta retention
- reuse existing datasetMeta during persistence
- single-save auto-export success path
- fallback builder `max_rows` coverage
- export failure/download fallback coverage
- non-fatal persistence warning coverage if you surface one
- duplicate reveal avoidance coverage if you implement that helper result

### `web/src/services/validationRequestBuilder.ts`

Mandatory:

- update VCC/CARM/CDMC preview route notes
- remove VCC Auto CPU-preference preview logic
- keep the preview category policy aligned with the current post-run/export policy

Recommended:

- create `validationRoutePolicy.ts` and centralize shared policy helpers

### `web/src/services/validationRequestBuilder.test.ts`

Mandatory:

- update VCC expectations
- update CARM/CDMC wording expectations
- add any missing VCET/CompositeMo parity assertions that the route helper extraction makes easy

### `web/src/services/historyService.ts`

Mandatory:

- preserve `dataset_meta` in `loadSavedRun()` payloads

Recommended:

- expand the existing `historyService.test.ts` coverage

### `web/src/stores/workflowArtifactsStore.ts`

- inspect normalization logic and use it as the contract target
- avoid production changes unless necessary
- optional store-contract test only if the repo already has a store test pattern

### `src-tauri/src/commands_validation.rs`

- inspect only
- keep strict validation-context enforcement as-is
- do not weaken dataset/timeframe/strategy guardrails
- no production change required unless a tiny compile-time adjustment is needed elsewhere

### `src-tauri/src/commands_reports.rs`

Mandatory:

- keep the existing saved-file `max_rows` behavior intact
- thread `max_rows` into the fallback base64 workbook-builder path

### `src-tauri/src/commands_reports_api.rs`

Mandatory:

- update the `build_report_xlsx` Tauri command signature so the frontend can pass `max_rows` on the fallback path
- keep the `save_report_xlsx` signature aligned with the existing saved-file flow

### `src-tauri/src/commands_reports_validation_common.rs`

Mandatory:

- preserve the existing expected/actual route-summary helper layer
- patch only the residual export wording/category drift that still remains after the preview-side correction
- keep VCC not Auto CPU-preferred in export expectations
- preserve the existing actual route-note summary plumbing; only touch it if a tiny parity fix is truly required
- add Rust regression tests only if you edit this file

Optional only if very easy:

- expose `actual_category_summary` more broadly

---

## Output expectations from you (Codex)

At the end, provide:

1. a concise summary of what you changed
2. the list of files you modified
3. the list of new files you created
4. the tests/build commands you ran
5. the exact pass/fail results
6. any optional items you intentionally deferred
7. any repo-specific blockers you encountered

Do not just describe intended changes. Make the edits and report what landed.

---

## Acceptance checklist (must satisfy before stopping)

### Required

- [ ] `runValidation()` no longer labels command-only time as “frontend”
- [ ] visible timing text distinguishes end-to-end and command timing
- [ ] `dataset_meta` / `datasetMeta` survives validation artifact persistence, apply, and saved-run reload
- [ ] validation artifact persistence prefers existing artifact `datasetMeta` when present
- [ ] report payload building prefers artifact-backed dataset context when available
- [ ] auto-export success path persists the run artifact only once
- [ ] export failure/download fallback remains non-fatal and still leaves a saved validation artifact behind
- [ ] fallback XLSX export honors `max_rows` on both the save path and the base64 fallback path
- [ ] `commands_reports_api.rs` and the Rust workbook builders stay signature-compatible after the fallback `max_rows` fix
- [ ] VCC preview no longer claims Auto CPU-preferred behavior, and export remains aligned with the current policy
- [ ] preview wording for VCC/CARM/CDMC/VCET aligns with the current post-run/export policy
- [ ] export/common route-policy behavior is preserved or patched only where a real residual mismatch remains
- [ ] targeted frontend tests pass
- [ ] targeted Rust compile/tests for the touched report/export path pass

### Strongly recommended

- [ ] `historyService.test.ts` covers `datasetMeta` replay on `loadSavedRun()`
- [ ] auto-export tests cover success, save failure, download fallback, persistence warning, and duplicate-reveal avoidance
- [ ] shortlist-parity tests are added only if they stay low-risk and preserve current filtered-row behavior
- [ ] route-policy duplication in TS is reduced only if a very small helper lowers drift without widening scope

### Optional / phase 4

- [ ] hidden validation phase timings added
- [ ] preview summary memoized safely
- [ ] richer dataset context snapshot persisted

---

## Test/build guidance

Use the repo’s native tooling. Detect whether the frontend uses `npm`, `pnpm`, or another package manager and use the existing scripts.

### Minimum required verification

#### Frontend

Run the targeted validation/history tests, for example the repo-native equivalent of:

- `validationService.test.ts`
- `validationRequestBuilder.test.ts`
- `historyService.test.ts`

Also run typecheck/build if the repo already has fast scripts for them.

#### Rust

Run targeted tests/build for:

- the `commands_reports_api.rs` / `commands_reports.rs` path affected by the `build_report_xlsx` signature change
- `commands_reports_validation_common.rs` only if you edit its route-policy helpers or tests

If a broader repo build is reasonably fast, run it too. If not, at least ensure the touched crates/modules compile.

Do **not** create new benchmark harnesses for this task. Reuse existing tests/builds. Heavy perf harnesses are optional and only worth running if they are already cheap and scripted.

---

## Search anchors to speed implementation

Use these exact names to jump into the hot paths:

### TypeScript

- `runValidation(`
- `previewValidationExecutionSummaryFromStores(`
- `persistValidationArtifact(`
- `applyVueValidationArtifacts(`
- `buildCurrentReportPayloadFromStore(`
- `exportReportXlsx(`
- `openPathWithRevealFallback(`
- `resolveValidationBuildContextFromWorkflowArtifacts(`
- `buildValidationCapabilitySummary(`
- `buildValidationRequest(`
- `compositeProfileGpuRouteNote(`
- `compositeProfileAutoCpuPreferenceNote(`
- `normalizeStrategyKindRaw(`
- `isDoubleLikeKind(`
- `loadSavedRun(`
- `normalizeRunArtifact(`

### Rust

- `build_report_xlsx_blocking(`
- `build_report_xlsx(` in `commands_reports_api.rs`
- `build_validation_report_xlsx(`
- any `#[tauri::command]` exposing `build_report_xlsx`
- expected/actual route summary builders in `commands_reports_validation_common.rs`
- VCC/CARM/CDMC route-note helper functions in export common

---

## Final instruction

Implement the validated required checklist completely, keep the patch parity-safe, and avoid scope drift. Only do the recommended or optional items if they are still clearly low-risk after the required changes and test passes.

When in doubt, favor:

- correctness
- truthful timing
- single-save artifact flow
- metadata preservation
- preview/post-run/export parity
- targeted regression tests

Do not stop after analysis. Make the changes.
