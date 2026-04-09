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

## External Research Summary

- [clap crate docs](https://docs.rs/clap/latest/clap/) are relevant because all current headless binaries rely on derived CLI contracts that should remain intentional and consistent.
- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) are relevant because these binaries are not incidental; they are explicit maintenance and verification targets in the repo.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because headless outputs should be treated as stable artifact contracts, not just ad hoc printouts.
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html) are relevant because one of the main headless surfaces produces XLSX workbooks as a first-class output.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because CSV input/output compatibility remains part of the operational headless surface.

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

## Checklist

- [ ] Document the role of each headless binary and why it exists separately.
- [ ] Audit CLI argument names and defaults for avoidable inconsistency.
- [ ] Document the artifact output matrix across `ta_cli`, `ta_headless_validate`, and `ta_workflow_regression`.
- [ ] Add or tighten tests for output-path generation and argument validation helpers.
- [ ] Add or tighten wrapper-script checks so output-shape drift is caught early.
- [ ] Decide whether any shared CLI helper extraction would improve consistency without making the tools less clear.
- [ ] Keep the app-core-only versus Tauri-linked boundary explicit in docs and naming.

## Verification Plan

- run targeted tests in `ta_headless_validate.rs`
- run targeted tests in `ta_workflow_regression.rs`
- run representative `ta_cli` invocations on bounded inputs
- run `scripts/run_output_parity_check.mjs`
- run `scripts/run_workflow_gate.ps1`
- run `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` when output compatibility or benchmark wiring changes

## Rollout / Fallback / Compatibility Notes

- keep current binary names and core arguments stable unless there is a strong compatibility plan
- prefer shared documentation and wrapper hardening before argument renames
- if unification creates ambiguity, keep the simpler separated tools
- treat artifact output drift as a compatibility event, not a casual cleanup

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/25_headless_report_builders_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/48_generated_command_inventory_2026-04-08.md`
