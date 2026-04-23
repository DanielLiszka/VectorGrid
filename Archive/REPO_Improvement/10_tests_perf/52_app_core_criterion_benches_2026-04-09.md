# App Core Criterion Benches Plan

Last updated: 2026-04-10

## Purpose

Define the non-options Criterion bench inventory in `ta_app_core` and how it should be used without conflating it with full-app or options coverage.

## Scope Boundaries

In scope:

- non-options `[[bench]]` entries in `crates/app_core/Cargo.toml`
- representative non-options bench files in `crates/app_core/benches/`
- bench classification by optimization, validation, meta, or dynamic-strategy surface
- the relation between Criterion benches and higher-level gates

Out of scope:

- options-expression bench entries and planning
- desktop-shell or frontend performance
- helper bins that are not Criterion benches
- general benchmark theory unrelated to the current bench inventory

## Current User-Facing Behavior

The app-core crate already has a substantial Criterion bench inventory. Most of it is non-options and relevant to this improvement program, while a few bench targets are explicitly options-related and therefore out of scope. This mixed inventory is exactly why a non-options inventory doc matters: contributors need to know what already exists before adding more.

## Current Code Surface

- `crates/app_core/Cargo.toml` declares most of the current Criterion benches, but Cargo bench discovery on the current tree also includes runnable bench-source files under `crates/app_core/benches/` that are not listed as explicit `[[bench]]` entries.
- `crates/app_core/benches/double_ma_opt.rs`, `dynamic_strategy_cpu.rs`, and `composite_validation_workflow.rs` are representative non-options bench surfaces.
- `crates/app_core/benches/meta_select_workflow.rs`, `double_ma_validation_workflow.rs`, and related files extend the non-options bench inventory.
- `target/criterion/` stores the current local bench outputs that gates and compare flows consume.

## Current Verification Surface

- `cargo bench -p ta_app_core --bench double_ma_opt`
- `cargo bench -p ta_app_core --bench dynamic_strategy_cpu`
- `cargo bench -p ta_app_core --bench composite_validation_workflow`
- inspection of `target/criterion/` outputs for representative non-options benches

## Active Bench Inventory

- explicit non-options `[[bench]]` entries in `crates/app_core/Cargo.toml`:
  - `double_ma_opt`
  - `double_ma_metrics`
  - `composite_mo_opt`
  - `composite_cpu_opt`
  - `asr_opt`
  - `vcc_opt`
  - `vemr_opt`
  - `vcet_opt`
  - `rocb_opt`
  - `mkcs_opt`
  - `gpu_native_expansion_opt`
  - `dynamic_strategy_cpu`
  - `composite_validation_workflow`
  - `double_ma_validation_workflow`
  - `double_ma_followup_workflow`
  - `meta_select_workflow`
  - `e2e_workflow_ml`
- additional runnable non-options bench targets discovered from `crates/app_core/benches/` on the current tree:
  - `ml_signal_workflow`
  - `ml_signal_validation`
- declared but out-of-scope options benches:
  - `options_expression_candidate_compile`
  - `options_expression_summary_compile`
  - `options_expression_cpu_score`
  - `options_expression_gpu_score`
- out-of-scope options/support bench-source targets present under `crates/app_core/benches/`:
  - `options_expression_bench_support.rs`

## Representative Mapping

- `double_ma_opt`
  - primary engine optimization inventory and the main Criterion source behind CPU/GPU perf-gate case paths
- `dynamic_strategy_cpu`
  - supports the CPU perf-gate dynamic-strategy ratio lane
- `composite_validation_workflow`
  - representative validation-workflow Criterion surface for composite strategy review
- `double_ma_validation_workflow`
  - double-MA validation workflow Criterion lane
- `meta_select_workflow`
  - meta-selection workflow Criterion lane
- `e2e_workflow_ml`
  - ML-oriented end-to-end workflow Criterion lane with a distinct investigative purpose rather than direct gate authority

## Strengths Already Present

- the repo already has wide non-options Criterion coverage across several backend surfaces
- the inventory is declared centrally in Cargo, which makes it diffable and reviewable
- multiple higher-level gates already depend on a small subset of these benches
- bench files are granular enough that contributors can target a surface without rebuilding the whole app

## Weaknesses / Failure Modes

- the mixed non-options/options inventory can create confusion if not filtered deliberately
- Criterion benches are easy to overinterpret as product-level performance
- bench sprawl can happen if new files are added without inventory discipline
- a declared bench inventory does not guarantee every bench is still equally representative

## Regressions To Avoid

- accidentally pulling options benches into the non-options planning program
- using Criterion bench results as a substitute for route, workflow, or desktop measurements
- adding more benches without first checking whether a relevant bench already exists
- forgetting to document bench inventory changes in the surrounding docs and gates
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because these benches are all built on Criterion-style statistical benchmarking.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because the inventory is ultimately surfaced and invoked through Cargo.
- [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html) are relevant because some benches require or assume feature-gated code paths.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because some non-options benches are GPU-adjacent and need careful interpretation.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Catalog the non-options bench inventory explicitly and exclude options benches from this program.
2. Map existing benches to the higher-level gates or investigative docs that actually consume them.
3. Use the inventory to stop duplicate or poorly targeted bench additions.
4. Review representativeness periodically instead of assuming every old bench still tells the right story.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory the current non-options `[[bench]]` entries.
- [x] Mark out-of-scope options benches explicitly so they are not mixed into this program.
- [x] Map representative benches to the gates or docs that depend on them.
- [x] Keep the Criterion inventory plan non-options only.
- [x] Review whether each bench still serves a distinct investigative purpose.
- [x] Avoid adding new benches without first checking the current inventory.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run representative non-options benches after benchmark-sensitive engine changes
- confirm Cargo inventory changes are reflected in the docs and gates that depend on them
- inspect `target/criterion/` layout after bench renames or additions
- review whether removed benches leave any gate or doc orphaned
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current useful bench inventory and improve cataloging around it
- prefer reusing and clarifying existing benches before adding new ones
- if a bench is obsolete, remove it deliberately and update dependent docs or scripts
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closure date: `2026-04-10`
- Code changes were not required; this item closed on inventory clarification plus fresh representative bench-target verification.
- Verified the authoritative inventory by inspecting:
  - `crates/app_core/Cargo.toml`
  - `crates/app_core/benches/`
  - `target/criterion/`
- Fresh representative compile-only bench verification:
  - `cargo bench -p ta_app_core --bench double_ma_opt --no-run`
  - `cargo bench -p ta_app_core --bench dynamic_strategy_cpu --no-run`
  - `cargo bench -p ta_app_core --bench composite_validation_workflow --no-run`
  - `cargo bench -p ta_app_core --bench ml_signal_workflow --no-run`
  - `cargo bench -p ta_app_core --bench ml_signal_validation --no-run`
- Representative local `target/criterion/` outputs present during closeout included:
  - `double_ma_cpu_sma_sma_200k`
  - `double_ma_gpu_kernel_alma_alma_200k`
  - `dynamic_strategy_cpu_full_run_2k`
  - `composite_validate_workflow_50k_32candidates`
- Inventory notes:
  - the four `options_expression_*` benches remain explicitly out of scope for this non-options plan
  - `ml_signal_workflow` and `ml_signal_validation` are currently runnable non-options bench targets even though they are not declared as explicit `[[bench]]` entries in `Cargo.toml`
- `REPO_MAP.md` required a small sync update so its app-core bench list includes `vcet_opt`.
- Representative bench compilation still emits existing workspace warnings in `ta_app_core`, but those warnings are broader cleanup debt rather than blockers for this inventory closeout.

## Related Docs

- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
