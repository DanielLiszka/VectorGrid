# Headless Workflow Bins Plan

Last updated: 2026-04-09

## Purpose

Define the non-options headless/bin surface and classify which binaries are authority gates, helpers, or supplemental tools.

## Scope Boundaries

In scope:

- non-options headless bins under `src-tauri/src/bin/` and `crates/app_core/src/bin/`
- classification of those bins by authority gate, helper, snapshot, or investigative role
- the CLI boundary for `ta_cli`, `ta_workflow_regression`, `ta_headless_validate`, and adjacent non-options bins
- bin-level discoverability and role clarity within the repo

Out of scope:

- options-related bins
- desktop-shell entrypoints
- general Cargo bin management beyond current repo surfaces
- new feature design for headless workflows

## Current User-Facing Behavior

The repo already contains a rich set of non-options headless bins, from authority gates like `ta_workflow_regression` to helper tools like `ta_headless_validate`, `ta_profile_route_matrix`, `ta_refactor_snapshots`, and `ta_cli`. This is a strength because it enables targeted investigation, but without explicit classification it is also easy to lose track of which bin is authoritative, which is supplemental, and which is merely convenient.

## Current Code Surface

- `src-tauri/src/bin/ta_workflow_regression.rs` is the main workflow regression gate bin.
- `src-tauri/src/bin/ta_headless_validate.rs`, `ta_profile_route_matrix.rs`, `ta_refactor_snapshots.rs`, `ta_app_path_faceoff.rs`, and `ta_campaign_perf.rs` cover important non-options headless surfaces.
- `crates/app_core/src/bin/ta_cli.rs`, `profile_gpu_dynamic_runner.rs`, `profile_doublema_campaign_prefetch.rs`, `cupti_sass_probe.rs`, and `real_data_fidelity_audit.rs` expand the non-options app-core bin surface.
- `Cargo.toml` and crate-local Cargo manifests determine which bins are feature-gated and how they are built.

## Current Verification Surface

- `cargo run -p ta_desktop_demo_app --bin ta_workflow_regression -- --help`
- `cargo run -p ta_desktop_demo_app --bin ta_headless_validate -- --help`
- `cargo run -p ta_app_core --features cli --bin ta_cli -- --help`
- targeted execution of specific bins when their owned surface changes

## Strengths Already Present

- the repo already exposes many high-leverage surfaces as focused bins rather than hiding everything inside the GUI
- authority and supplemental roles already exist in practice even if not fully cataloged
- bin-level entrypoints make regression investigation and artifact generation easier
- many bins emit structured JSON or artifact outputs that are reviewable

## Weaknesses / Failure Modes

- the sheer number of bins makes role confusion likely
- feature-gated bins are easy to invoke incorrectly if their requirements are not cataloged
- non-options and options bins coexist in some directories, so scoping must stay explicit
- without a classification doc, contributors may reach for the wrong bin or misread its authority

## Regressions To Avoid

- treating all bins as equally authoritative
- forgetting feature requirements or build profiles when using a bin
- accidentally pulling options bins into this non-options documentation program
- adding new bins without clarifying whether they are gates, helpers, or supplemental tools

## External Research Summary

- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because these workflows are bin-driven by design.
- [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html) is relevant because several app-core bins require explicit feature flags.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant as a contrast because some bins are helpers or gates, not statistical benchmarks.
- [Tauri distribution docs](https://v2.tauri.app/distribute/) are relevant because these headless tools intentionally bypass the distributed desktop shell and therefore need separate role clarity.

## Proposed Improvement Approach

1. Catalog the non-options headless bins by role: authority gate, supplemental review, profiling helper, or refactor aid.
2. Document feature requirements, primary output artifacts, and intended consumers for each bin.
3. Use the classification to route contributors toward the right tool for a question.
4. Keep headless bin growth disciplined by requiring new bins to declare their role on arrival.

## Checklist

- [ ] Inventory all current non-options bins under the relevant directories.
- [ ] Classify each bin by authority level and use case.
- [ ] Document feature requirements and expected output artifacts for each bin.
- [ ] Keep this bin-catalog plan non-options only.
- [ ] Cross-link bins to the docs or scripts that consume them.
- [ ] Avoid adding new bins without role clarity and discoverability updates.

## Verification Plan

- run `--help` for representative bins after interface changes
- check feature-gated build commands for bins that are part of active workflows
- verify docs and scripts still point at the correct bin names and roles
- confirm options bins remain excluded from this program’s inventory

## Rollout / Fallback / Compatibility Notes

- preserve the current bin richness while improving classification and discoverability
- prefer better routing and role clarity over removing useful bins
- if a bin becomes obsolete, retire it deliberately and update the catalog

## Related Docs

- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/51_vectorgrid_internal_benchmark_boundaries_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/14_repo_surface_coverage_audit_2026-04-09.md`
