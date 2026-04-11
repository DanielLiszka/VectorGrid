# Artifact Report Shape Parity Plan

Last updated: 2026-04-10

## Purpose

Define the structural parity expectations for regenerated report artifacts and their checked-in representative samples.

## Scope Boundaries

In scope:

- shape-level parity checks for report JSON, report XLSX, and adjacent representative artifacts
- the checker `scripts/check_output_parity.mjs`
- the sample manifest under `scripts/baselines/samples/manifest.json`
- the relationship between structural parity and richer workbook/report tests

Out of scope:

- options-related report artifacts
- semantic/business correctness beyond structural shape
- authoritative performance metrics
- general artifact-library behavior outside representative parity samples

## Current User-Facing Behavior

The current parity checker compares JSON type/shape, CSV column/header shape, and XLSX zip/container markers for a small set of paired artifacts. Representative sample outputs are also tracked in the sample manifest, and the repo already has richer workbook tests elsewhere. That layered approach is sound, but the parity doc must make the limitations of shape parity explicit so it is used as intended.

## Current Code Surface

- `scripts/check_output_parity.mjs` performs JSON, CSV, and XLSX shape checks.
- `scripts/baselines/samples/manifest.json` records representative sample outputs and their source lineage.
- `src-tauri/tests/validation_xlsx_report_tests.rs` provides deeper workbook-level assertions that complement simple shape parity.
- `scripts/output_parity_pairs.json` and `scripts/headless_output_parity_pack.json` define the inputs to the parity checker.

## Current Verification Surface

- `node scripts/check_output_parity.mjs --pairs scripts/output_parity_pairs.json`
- `node scripts/run_output_parity_check.mjs --pack scripts/headless_output_parity_pack.json --pairs scripts/output_parity_pairs.json`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- manual inspection of representative sample artifacts when shape parity flags something suspicious

## Strengths Already Present

- the checker is explicit about what kinds of shape it compares for each artifact type
- the repo already combines structural parity with richer workbook tests rather than relying on one layer only
- representative sample outputs are checked in and manifest-driven
- shape parity is fast enough to rerun repeatedly during report work

## Weaknesses / Failure Modes

- shape parity can miss semantic drift
- XLSX zip-marker checks are intentionally coarse
- sample representativeness can decay if the report contract evolves
- JSON shape parity can stay green while field meaning changes subtly

## Regressions To Avoid

- using shape parity as proof of semantic report correctness
- removing richer workbook tests because shape parity exists
- adding many sample outputs without preserving their review purpose
- allowing the checker to accept obviously malformed structure just to reduce noise
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because JSON-shape parity is fundamentally a contract-shape concern.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because CSV parity checks rely on stable header and row-shape expectations.
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html) are relevant because workbook output shape is partly driven by the XLSX builder contract.
- [Microsoft Excel chart guidance](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) is relevant because workbook usefulness depends on more than container validity, which is why shape parity must stay layered with richer checks.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep shape parity explicitly structural and explicitly supplemental.
2. Use representative sample outputs to catch broad structural regressions quickly.
3. Layer structural parity with deeper report/workbook tests and occasional manual review.
4. Refresh sample lineage and pair meaning deliberately when the report contract changes.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document which artifact types receive which kind of shape check.
- [x] Document the representative sample set and its limitations.
- [x] Keep structural parity non-options only.
- [x] Cross-link shape parity to richer workbook/report tests.
- [x] Review sample representativeness periodically.
- [x] Avoid conflating container validity with analyst usefulness.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the parity checker after report-structure changes
- run the deeper workbook tests when parity changes or fails
- inspect representative sample outputs if shape changes are expected but need confirmation
- verify the sample manifest still points to real files and accurate lineage
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current layered parity model rather than collapsing everything into one checker
- prefer better explanation over more exhaustive but low-signal shape checks
- if semantic drift becomes common, add targeted deeper tests rather than overstretching the shape checker
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on fresh verification of the existing layered report-shape parity model rather than a checker or sample change.
- The current structural checker responsibilities remain intentionally coarse and were revalidated in place:
  - JSON artifacts: recursive type/key/array-shape comparison
  - CSV artifacts: header and column-count shape comparison
  - XLSX artifacts: zip signature, required container markers, and coarse size-ratio checks
- The representative sample manifest remains the checked-in sample lineage source:
  - `scripts/baselines/samples/manifest.json`
  - current representative outputs:
    - `top_rows.csv`
    - `validation_shortlist.csv`
    - `run_artifact.json`
    - `batch_summary.json`
    - `report.json`
    - `report.xlsx`
- Fresh acceptance evidence for this closure:
  - `node scripts/check_output_parity.mjs --pairs scripts/output_parity_pairs.json`
  - `node scripts/run_output_parity_check.mjs --pack scripts/headless_output_parity_pack.json --pairs scripts/output_parity_pairs.json`
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- Verified results on the current tree:
  - direct structural parity checker: `Output parity check passed for 2 pair(s).`
  - full regeneration plus parity: JSON/XLSX outputs regenerated and structural parity passed for `2` pairs
  - workbook integration suite: `15` tests passed
- This closure intentionally preserved the layered model:
  - fast structural parity on representative JSON/XLSX artifacts
  - deeper workbook/report assertions in `src-tauri/tests/validation_xlsx_report_tests.rs`
  - manual artifact review only as a follow-up when the automated layers flag suspicious shape changes
- No sample-manifest refresh or checker change was required for this closure. The current sample set stayed representative enough for structural parity, while remaining explicitly insufficient as a standalone semantic correctness proof.

## Related Docs

- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/43_output_parity_pack_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/44_parity_pair_registry_2026-04-09.md`
