# Baseline Samples Manifest Plan

Last updated: 2026-04-09

## Purpose

Define the current baseline-samples manifest contract so representative run/report artifacts remain reproducible and useful for parity, review, and docs-driven verification.

## Scope Boundaries

In scope:

- `scripts/export_baseline_samples.mjs`
- `scripts/baselines/samples/manifest.json`
- sample JSON, CSV, and XLSX outputs exported from representative artifacts
- how these samples support manual and scripted review

Out of scope:

- the full output parity pack
- options sample artifacts
- arbitrary historical benchmark dumps under `target/`

## Current User-Facing Behavior

The repo already provides a baseline-sample export path that collects a representative run artifact, report JSON, report XLSX, batch summary, and CSV extracts into a stable sample directory. This gives the project a compact, human-reviewable evidence pack without forcing reviewers to inspect the full build/test output tree.

## Current Code Surface

- `scripts/export_baseline_samples.mjs`
  - exports sample artifacts and writes the manifest
- `scripts/baselines/samples/manifest.json`
  - records generated outputs, counts, and source paths
- `scripts/baselines/samples/`
  - current sample JSON, CSV, and XLSX artifacts
- `scripts/run_output_parity_check.mjs`
  - related structured-output validation tooling

## Current Verification Surface

- run `node scripts/export_baseline_samples.mjs`
- inspect `scripts/baselines/samples/manifest.json`
- inspect exported `run_artifact.json`, `report.json`, `report.xlsx`, and CSVs

## Strengths Already Present

- the repo already has a compact baseline-evidence export instead of relying only on ephemeral `target/` output
- the manifest records both generated outputs and source paths
- the sample set spans multiple artifact shapes, not only JSON

## Weaknesses / Failure Modes

- the manifest is useful but easy to overlook without a dedicated contract doc
- source candidate selection is still a fallback chain and can drift if not reviewed
- sample artifacts can become stale relative to newer verification expectations
- CSV/XLSX samples can look complete even if the manifest no longer explains their provenance clearly

## Regressions To Avoid

- losing the link from exported sample files back to their source artifacts
- treating the sample set as interchangeable with the full parity pack
- silently changing sample-export behavior without updating the manifest contract
- dropping representative file types from the sample set
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because the manifest itself is a structured contract that should remain stable and reviewable
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because the sample pack includes exported CSV evidence
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html)
  - relevant because the sample pack includes workbook output as part of review evidence
- [GitHub Docs: About READMEs](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because the sample pack helps keep repository-level documentation grounded in real outputs

## Proposed Improvement Approach

1. Document the sample-export manifest as a stable evidence pack, not just a convenience script.
2. Keep the manifest explicit about outputs, counts, and source paths.
3. Preserve multi-format evidence: JSON, CSV, and XLSX.
4. Review the source-candidate order whenever report/artifact generation paths change.
5. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document current sample outputs and why each exists.
- [x] Document current source-candidate lookup behavior.
- [x] Document the manifest fields and their intended stability.
- [x] Keep sample artifacts distinct from the broader parity pack.
- [x] Review sample freshness when report or artifact contracts change.
- [x] Add or tighten checks if the sample manifest shape changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- run `node scripts/export_baseline_samples.mjs`
- inspect `scripts/baselines/samples/manifest.json`
- confirm the exported file set and counts match current script behavior
- spot-check CSV/XLSX outputs for readable, non-empty sample evidence
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current sample-manifest shape unless a coordinated consumer update exists
- if source-candidate order changes, update the manifest documentation in the same change
- keep sample artifacts small and reviewable rather than expanding them into a full archival store
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on a fresh baseline-sample export plus manifest and artifact spot checks.
- The current authority surfaces for this contract remain aligned across:
  - `scripts/export_baseline_samples.mjs`
  - `scripts/baselines/samples/manifest.json`
  - `scripts/run_output_parity_check.mjs`
  - `REPO_MAP.md`
- Fresh acceptance evidence:
  - `node scripts/export_baseline_samples.mjs`
  - inspect `scripts/baselines/samples/manifest.json`
  - spot-check `scripts/baselines/samples/top_rows.csv`
  - spot-check `scripts/baselines/samples/validation_shortlist.csv`
  - verify `scripts/baselines/samples/report.xlsx` exists and is non-empty
- The fresh export confirmed the sample-pack shape this doc is meant to preserve:
  - outputs: `top_rows.csv`, `validation_shortlist.csv`, `run_artifact.json`, `batch_summary.json`, `report.json`, `report.xlsx`, `manifest.json`
  - counts: `top_rows = 80`, `validation_shortlist_rows = 12`
  - sources currently resolve to `_write_test/cli_runs/AAPL_opt.json`, `target/workflow_regression_summary.json`, and `_write_test/headless_reports/AAPL_headless_test.{json,xlsx}` when those preferred sources are present
- The spot check also reconfirmed that this sample pack is intentionally smaller than the broader parity pack:
  - the CSV outputs are human-reviewable slices rather than exhaustive exports
  - the workbook copy remains a compact evidence artifact, not the full parity authority
  - `scripts/run_output_parity_check.mjs` remains the structural-parity surface, while `export_baseline_samples.mjs` remains the review-pack surface
- No additional README or release-doc edits were required for closure because `REPO_MAP.md` already routes contributors to the sample-export helper and distinguishes it from the parity-authority scripts.

## Related Docs

- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/43_output_parity_pack_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/08_docs_archive_policy_2026-04-09.md`
