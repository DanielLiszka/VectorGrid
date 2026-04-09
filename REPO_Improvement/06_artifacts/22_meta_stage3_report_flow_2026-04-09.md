# Meta Stage3 Report Flow Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Stage 3 meta-report flow so workbook generation, saved-path persistence, artifact storage, reopen/reveal behavior, and downstream Stage 3 fallback remain fully aligned.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/services/validationService.ts`
- end-to-end Stage 3 report generation and reuse flow

Out of scope:

- Stage 4 / Stage 5 model-scoring semantics
- validation workbook content
- options-related workflows

## Current User-Facing Behavior

Running meta selection already:

- computes Stage 3 results
- builds a Stage 3 XLSX workbook
- saves the workbook into the app `reports/` directory
- persists the workbook path into the saved meta artifact JSON
- exposes saved Stage 3 replay/load/open/reveal actions in Meta Analysis
- provides a validation-side fallback to the latest Stage 3 XLSX when no validation artifact exists

The flow is therefore already an important cross-surface workflow, not only a Meta-only convenience.

## Current Code Surface

- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/services/validationService.ts`
- `web/src/stores/commandActionStore.ts`

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`
- targeted backend tests in `commands_meta.rs` where present

## Strengths Already Present

- Stage 3 workbook generation is already part of the meta-selection success path
- the saved meta artifact already stores `report_xlsx_path`
- meta saved-report reopen and reveal flows already exist
- Stage 3 fallback is already preserved from validation export logic
- retention for meta artifacts is already tag-aware and separate from report storage

## Weaknesses / Failure Modes

- the Stage 3 flow spans report generation, artifact save, pruning, reopen, and fallback reuse across multiple surfaces
- `latestMetaReportPath` behavior is spread across state and service layers
- report save and artifact save happen in sequence, so partial-failure semantics need clearer documentation
- Stage 3 naming and compatibility wording can drift between backend and frontend surfaces
- current tests prove key pieces, but the full flow contract is not yet captured in one plan

## Regressions To Avoid

- breaking automatic Stage 3 workbook save during meta selection
- losing `report_xlsx_path` in saved meta artifacts
- breaking reopen/reveal of saved Stage 3 reports
- making validation export fallback stop honoring the latest Stage 3 workbook
- introducing cleanup/pruning behavior that deletes report files unexpectedly
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because the Stage 3 flow crosses backend generation, command responses, and frontend reopen behavior.
- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because Stage 3 relies on generating a stable workbook artifact with predictable structure.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because latest-report path state and saved-entry actions are coordinated across several stores and services.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the current flow is best protected through behavior-level tests in service and store layers.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because the Stage 3 artifact JSON stores the saved workbook path and result payload used for replay/fallback behavior.
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Freeze the Stage 3 flow as one documented pipeline:
   - generate result
   - build workbook
   - save workbook
   - save meta artifact with `report_xlsx_path`
   - refresh reopen/reveal surfaces
   - preserve validation fallback
2. Document partial-failure expectations and status handling.
3. Add or tighten tests that cover saved-path persistence and Stage 3 fallback reuse.
4. Keep report files and artifact files distinct but linked.
5. Preserve additive behavior only; do not remove any current Stage 3 reopen/reveal path.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the full Stage 3 report generation/save pipeline.
- [ ] Document `report_xlsx_path` persistence rules in the meta artifact.
- [ ] Document how `latestMetaReportPath` is sourced and consumed.
- [ ] Add or tighten tests for saved meta report reopen/reveal behavior.
- [ ] Add or tighten tests for validation-side Stage 3 fallback.
- [ ] Document partial-failure behavior if workbook save or artifact save fails independently.
- [ ] Confirm report retention and artifact retention do not silently diverge.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/metaService.test.ts"` from `web/`
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/validationService.test.ts"` from `web/`
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/stores/analysisStore.actions.test.ts"` from `web/`
- run targeted `commands_meta.rs` tests if present
- manually run one meta selection, then validate reopen/reveal and validation-side fallback behavior
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve the current Stage 3 save-before-artifact-link behavior unless a more transactional design is adopted deliberately
- keep workbook files and artifact JSON files as separate but linked outputs
- if path-tracking logic is centralized, update both Meta and validation fallback surfaces together
- preserve current Stage 3 naming/compatibility behavior until a repo-wide naming change is made
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/05_meta_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
- `REPO_Improvement/06_artifacts/19_report_save_open_reveal_2026-04-09.md`
- `REPO_Improvement/06_artifacts/13_art_retention_tags_compat_2026-04-09.md`
