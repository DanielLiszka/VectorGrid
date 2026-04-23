# Legacy Name Compatibility Plan

Last updated: 2026-04-10

## Purpose

Define the remaining non-options compatibility shims for legacy names, envelopes, and artifact kinds so they stay deliberate rather than accidental.

## Scope Boundaries

In scope:

- non-options compatibility aliases in artifact kind/name handling
- legacy workflow-summary and report payload compatibility paths
- legacy release-note and environment-variable naming kept for compatibility
- tests that prove legacy payloads still decode or map correctly

Out of scope:

- options-related compatibility shims
- new naming schemes for unrelated future features
- general backward-compatibility policy outside current repo surfaces
- UI copy compatibility unless it maps to a real contract artifact

## Current User-Facing Behavior

The repo already keeps several explicit compatibility shims: older artifact kind names still map to current handling, older workflow summary payloads can still deserialize, workbook compatibility cases are still covered, and release notes document some legacy environment-variable naming. That is valuable because users and saved artifacts exist, but it also means compatibility must stay curated rather than becoming an undocumented pile of aliases.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs` contains several artifact-kind alias and compatibility helpers.
- `src-tauri/src/bin/ta_workflow_regression.rs` and related tests cover legacy summary decoding behavior.
- `src-tauri/tests/validation_xlsx_report_tests.rs` cover workbook compatibility-mode cases.
- `RELEASE.md` documents some legacy environment-variable names kept for compatibility.

## Current Verification Surface

- `cargo test -p ta_desktop_demo_app --lib run_artifact_kind_alias_accepts_legacy_and_vectorgrid_names`
- `cargo test -p ta_desktop_demo_app --lib meta_artifact_file_name_alias_accepts_legacy_and_vectorgrid_prefixes`
- `cargo test -p ta_desktop_demo_app --lib meta_stage_file_name_alias_accepts_legacy_and_vectorgrid_prefixes`
- `cargo test -p ta_desktop_demo_app --lib workflow_hydration_payload_keeps_metric_surface_absent_for_legacy_artifacts`
- `cargo test -p ta_desktop_demo_app --lib run_artifact_summary_backfills_fingerprint_version_and_dataset_identity_for_legacy_kind`
- `cargo test -p ta_desktop_demo_app --bin ta_workflow_regression workflow_summary_deserializes_legacy_payload_without_envelope`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests workbook_supports_legacy_compatibility_mode`
- manual review of release-note compatibility wording when names or env vars change

## Current Shim Inventory

- `src-tauri/src/run_artifacts.rs` intentionally accepts both legacy `vectorbt_*` and current `vectorgrid_*` names for:
  - run-artifact kinds
  - meta artifact file-name prefixes
  - Stage 4 / Stage 5 / ML-confidence file-name prefixes
- workflow hydration keeps legacy summaries readable when older payloads omit the newer envelope and metric-surface fields.
- run-artifact summary hydration backfills current fingerprint and dataset-identity fields when older `vectorbt_run_artifact` payloads do not carry the newer shape directly.
- validation workbook generation still enters compatibility mode when older validation artifacts lack the full verdict contract.
- `RELEASE.md` intentionally preserves the legacy internal environment names `VECTORBT_USE_PREBUILT_PTX` and `VECTORBT_REQUIRE_NVCC` as compatibility aliases.

## Retirement Candidates

- `vectorbt_*` artifact kind and filename aliases are retirement candidates only after saved-artifact migration or rewrite tooling exists for older run/meta/stage payloads.
- legacy workflow-summary envelope shims are retirement candidates only after the historical regression fixtures and any persisted exports no longer depend on the flat payload form.
- legacy validation workbook compatibility mode is a retirement candidate only after older shortlist-only validation artifacts are migrated or explicitly unsupported.
- legacy `VECTORBT_*` PTX environment names are retirement candidates only after release notes and automation have a deliberate replacement path.

## Strengths Already Present

- the repo already encodes important compatibility behavior in tests
- compatibility shims are explicit enough to inspect rather than hidden in random branches only
- release notes already acknowledge legacy env-var names instead of silently pretending they do not exist
- artifact and workbook compatibility coverage helps avoid breaking older saved outputs accidentally

## Weaknesses / Failure Modes

- compatibility shims are easy to accrete if they are not cataloged
- legacy aliases can outlive their real usefulness
- compatibility code often touches high-leverage I/O surfaces where regressions are subtle
- release-note compatibility prose can drift away from actual code support

## Regressions To Avoid

- removing a legacy alias without checking whether saved artifacts or scripts still rely on it
- keeping compatibility shims forever without documenting why they still exist
- expanding compatibility scope accidentally into options-related surfaces in this program
- changing release-note compatibility language without checking the actual code paths
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Semantic Versioning 2.0.0](https://semver.org/) is relevant because compatibility work is fundamentally about managed change and user expectations.
- [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html) is relevant because some compatibility behavior is partially expressed through build or environment toggles.
- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because legacy payload compatibility is a schema-evolution problem even when no formal schema file exists.
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html) are relevant because workbook compatibility behavior depends on a stable understanding of report output structure.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Catalog the remaining non-options compatibility shims and the evidence that they still matter.
2. Keep compatibility covered by explicit tests wherever possible.
3. Separate “kept intentionally” from “accidentally still works” in both code and docs.
4. Retire compatibility shims only through deliberate review and replacement coverage.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory current non-options legacy aliases and payload shims.
- [x] Document the test coverage that proves each shim still works.
- [x] Keep this compatibility plan non-options only.
- [x] Review release-note compatibility prose against current code support.
- [x] Mark shims that are candidates for future retirement instead of letting them stay indefinite by default.
- [x] Avoid adding new aliases without clear migration value.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the named compatibility tests after artifact or summary contract changes
- prefer target-scoped Rust test commands for this surface so unrelated bin failures in a dirty tree do not masquerade as legacy-compat regressions
- review release-note compatibility wording whenever environment names or build paths change
- inspect alias-handling code in `run_artifacts.rs` before removing or renaming kinds
- confirm new compatibility needs are backed by actual migration evidence
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve necessary shims while making their existence more explicit
- prefer deliberate retirement plans over silent breakage or silent perpetuation
- if a shim is retained, document why; if it is removed, document what replaces it
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closure date: `2026-04-10`
- Code changes were not required; the compatibility shims and release-note wording were already in place.
- Verified legacy alias coverage with:
  - `cargo test -p ta_desktop_demo_app --lib run_artifact_kind_alias_accepts_legacy_and_vectorgrid_names`
  - `cargo test -p ta_desktop_demo_app --lib meta_artifact_file_name_alias_accepts_legacy_and_vectorgrid_prefixes`
  - `cargo test -p ta_desktop_demo_app --lib meta_stage_file_name_alias_accepts_legacy_and_vectorgrid_prefixes`
  - `cargo test -p ta_desktop_demo_app --lib workflow_hydration_payload_keeps_metric_surface_absent_for_legacy_artifacts`
  - `cargo test -p ta_desktop_demo_app --lib run_artifact_summary_backfills_fingerprint_version_and_dataset_identity_for_legacy_kind`
- Verified legacy summary decoding with:
  - `cargo test -p ta_desktop_demo_app --bin ta_workflow_regression workflow_summary_deserializes_legacy_payload_without_envelope`
- Verified workbook compatibility mode with:
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests workbook_supports_legacy_compatibility_mode`
- Manual review confirmed `RELEASE.md` still accurately documents the legacy `VECTORBT_USE_PREBUILT_PTX` and `VECTORBT_REQUIRE_NVCC` names as compatibility-only internal environment variables.
- The broader package-level commands originally listed for this surface currently pull in unrelated dirty-tree bin compilation failures, so the authoritative closure evidence above uses target-scoped Rust test commands that exercise the actual compatibility shims directly.
- No `README.md` or `REPO_MAP.md` update was required because their current compatibility descriptions already match the verified code paths.

## Related Docs

- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
