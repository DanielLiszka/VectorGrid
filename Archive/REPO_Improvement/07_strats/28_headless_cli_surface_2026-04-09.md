# Headless Cli Surface Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for the non-options headless binary surface so the repo’s CLI, report-builder, regression, and benchmarking entry points behave like one coherent tooling family instead of a loose collection of useful bins.

## Scope Boundaries

In scope:

- `crates/app_core/src/bin/ta_cli.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- scripts and helpers that wrap those binaries, including:
  - `scripts/run_output_parity_check.mjs`
  - `scripts/run_workflow_gate.ps1`
  - `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`

Out of scope:

- new user-facing desktop features
- options-related headless tools
- replacing these binaries with a single monolithic CLI immediately

## Current User-Facing Behavior

For power users and maintainers, the headless surface is already important:

- `ta_cli` runs bounded Double-MA optimization on CSV or synthetic data and prints JSON
- `ta_headless_validate` performs optimize -> validate -> cross-ticker -> JSON/XLSX report generation
- `ta_workflow_regression` acts as a deterministic workflow-regression gate
- scripts wrap those binaries for parity checks, workflow gates, and benchmark/faceoff runs

The problem is not lack of utility. The problem is fragmentation:

- argument naming is not clearly harmonized
- artifact expectations differ across bins
- scope boundaries are implicit rather than documented

## Current Code Surface

- `crates/app_core/src/bin/ta_cli.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `scripts/run_output_parity_check.mjs`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`

## Current Verification Surface

- helper tests embedded in `ta_headless_validate.rs`
- helper tests embedded in `ta_workflow_regression.rs`
- structural parity checks in `run_output_parity_check.mjs`
- workflow regression execution via `run_workflow_gate.ps1`
- faceoff execution via `run_vectorbt_vs_vectorgrid_faceoff.ps1`

## Strengths Already Present

- the repo already has meaningful headless entry points for optimization, validation, regression, and benchmarking
- `clap`-based argument parsing makes the binaries relatively auditable
- `ta_headless_validate` already generates real report artifacts instead of being a toy smoke-test path
- `ta_workflow_regression` already captures timing and verdict summaries instead of relying on human eyeballing
- wrapper scripts already exist, which means there is a natural place to enforce consistency

## Weaknesses / Failure Modes

- the binaries feel like related tools created at different times rather than one documented family
- argument names and defaults are similar in spirit but not clearly standardized
- artifact outputs are useful but not described as a shared contract matrix
- the line between app-core-only headless tools and Tauri-linked headless tools is easy to misunderstand
- a future change to one binary can quietly strand scripts or documentation that assume older output shapes

## Regressions To Avoid

- breaking existing script wrappers while “cleaning up” CLI arguments
- changing JSON or XLSX artifact expectations without a documented compatibility review
- merging scopes so aggressively that simple bounded tools become harder to use
- making regression tooling less deterministic in the name of convenience
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [clap crate docs](https://docs.rs/clap/latest/clap/) are relevant because all current headless binaries rely on derived CLI contracts that should remain intentional and consistent.
- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) are relevant because these binaries are not incidental; they are explicit maintenance and verification targets in the repo.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because headless outputs should be treated as stable artifact contracts, not just ad hoc printouts.
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html) are relevant because one of the main headless surfaces produces XLSX workbooks as a first-class output.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because CSV input/output compatibility remains part of the operational headless surface.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat the headless binaries as one operational family with distinct roles:
   - direct optimization utility
   - full validation/report builder
   - deterministic regression gate
2. Document a shared argument vocabulary where possible:
   - dataset path conventions
   - backend names
   - timeframe naming
   - output path conventions
3. Create a small artifact-schema matrix documenting what each binary emits:
   - stdout JSON
   - saved JSON
   - CSV
   - XLSX
   - timing summaries
4. Tighten wrapper-script expectations so drift is caught quickly.
5. Keep scope boundaries explicit rather than trying to collapse everything into one binary immediately.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the role of each headless binary and why it exists separately.
- [x] Audit CLI argument names and defaults for avoidable inconsistency.
- [x] Document the artifact output matrix across `ta_cli`, `ta_headless_validate`, and `ta_workflow_regression`.
- [x] Add or tighten tests for output-path generation and argument validation helpers.
- [x] Add or tighten wrapper-script checks so output-shape drift is caught early.
- [x] Decide whether any shared CLI helper extraction would improve consistency without making the tools less clear.
- [x] Keep the app-core-only versus Tauri-linked boundary explicit in docs and naming.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `ta_headless_validate.rs`
- run targeted tests in `ta_workflow_regression.rs`
- run representative `ta_cli` invocations on bounded inputs
- run `scripts/run_output_parity_check.mjs`
- run `scripts/run_workflow_gate.ps1`
- run `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` when output compatibility or benchmark wiring changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep current binary names and core arguments stable unless there is a strong compatibility plan
- prefer shared documentation and wrapper hardening before argument renames
- if unification creates ambiguity, keep the simpler separated tools
- treat artifact output drift as a compatibility event, not a casual cleanup
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by extracting a small validation helper in [ta_cli.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/ta_cli.rs):
  - added `validate_cli_configuration`
  - added parser/config tests for `U32Range` and the current `export_all_csv_path` backend/`include_all` constraints
- Expanded [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md) with a compact headless-family role split:
  - `ta_cli` as the bounded optimize-and-print utility
  - `ta_headless_validate` as the report builder
  - `ta_workflow_regression` as the deterministic regression gate
- Kept scope intentionally additive:
  - no CLI flags changed
  - no wrapper script behavior changed
  - no JSON/XLSX output behavior changed
- Acceptance evidence:
  - `cargo test -p ta_app_core --features cli --bin ta_cli`

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/25_headless_report_builders_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/48_generated_command_inventory_2026-04-08.md`
