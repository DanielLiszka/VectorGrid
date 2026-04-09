# Legacy Name Compatibility Plan

Last updated: 2026-04-09

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

- `cargo test -p ta_desktop_demo_app workflow_summary_deserializes_legacy_payload_without_envelope`
- `cargo test -p ta_desktop_demo_app run_artifact_kind_alias_accepts_legacy_and_vectorgrid_names`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- manual review of release-note compatibility wording when names or env vars change

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

- [ ] Inventory current non-options legacy aliases and payload shims.
- [ ] Document the test coverage that proves each shim still works.
- [ ] Keep this compatibility plan non-options only.
- [ ] Review release-note compatibility prose against current code support.
- [ ] Mark shims that are candidates for future retirement instead of letting them stay indefinite by default.
- [ ] Avoid adding new aliases without clear migration value.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the named compatibility tests after artifact or summary contract changes
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

## Related Docs

- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
