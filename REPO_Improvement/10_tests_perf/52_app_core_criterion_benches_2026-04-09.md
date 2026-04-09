# App Core Criterion Benches Plan

Last updated: 2026-04-09

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

- `crates/app_core/Cargo.toml` declares the current Criterion benches and therefore the authoritative inventory source.
- `crates/app_core/benches/double_ma_opt.rs`, `dynamic_strategy_cpu.rs`, and `composite_validation_workflow.rs` are representative non-options bench surfaces.
- `crates/app_core/benches/meta_select_workflow.rs`, `double_ma_validation_workflow.rs`, and related files extend the non-options bench inventory.
- `target/criterion/` stores the current local bench outputs that gates and compare flows consume.

## Current Verification Surface

- `cargo bench -p ta_app_core --bench double_ma_opt`
- `cargo bench -p ta_app_core --bench dynamic_strategy_cpu`
- `cargo bench -p ta_app_core --bench composite_validation_workflow`
- inspection of `target/criterion/` outputs for representative non-options benches

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

## External Research Summary

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because these benches are all built on Criterion-style statistical benchmarking.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because the inventory is ultimately surfaced and invoked through Cargo.
- [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html) are relevant because some benches require or assume feature-gated code paths.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because some non-options benches are GPU-adjacent and need careful interpretation.

## Proposed Improvement Approach

1. Catalog the non-options bench inventory explicitly and exclude options benches from this program.
2. Map existing benches to the higher-level gates or investigative docs that actually consume them.
3. Use the inventory to stop duplicate or poorly targeted bench additions.
4. Review representativeness periodically instead of assuming every old bench still tells the right story.

## Checklist

- [ ] Inventory the current non-options `[[bench]]` entries.
- [ ] Mark out-of-scope options benches explicitly so they are not mixed into this program.
- [ ] Map representative benches to the gates or docs that depend on them.
- [ ] Keep the Criterion inventory plan non-options only.
- [ ] Review whether each bench still serves a distinct investigative purpose.
- [ ] Avoid adding new benches without first checking the current inventory.

## Verification Plan

- run representative non-options benches after benchmark-sensitive engine changes
- confirm Cargo inventory changes are reflected in the docs and gates that depend on them
- inspect `target/criterion/` layout after bench renames or additions
- review whether removed benches leave any gate or doc orphaned

## Rollout / Fallback / Compatibility Notes

- preserve the current useful bench inventory and improve cataloging around it
- prefer reusing and clarifying existing benches before adding new ones
- if a bench is obsolete, remove it deliberately and update dependent docs or scripts

## Related Docs

- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
