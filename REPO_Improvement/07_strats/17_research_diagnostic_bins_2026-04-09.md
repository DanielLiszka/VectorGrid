# Research Diagnostic Bins Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the repo's strategy-facing diagnostic binaries and headless research runners so they stay useful, discoverable, and non-disruptive to the main product path.

## Scope Boundaries

In scope:

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/bin/ta_cli.rs`
- scripts that directly launch or depend on these bins

Out of scope:

- the desktop product UI itself
- options-related headless tools
- new research programs unrelated to the existing shipped strategy families

## Current User-Facing Behavior

These bins are not exposed as first-class desktop screens, but they are already part of the real repo workflow. They are used to:

- characterize GPU versus CPU behavior
- run bounded real-data fidelity checks
- inspect campaign dataset prefetch and staging behavior
- produce synthetic kernel reports
- support headless CLI-oriented workflows

They therefore act as the repo's internal measurement and diagnostic layer for strategy behavior.

## Current Code Surface

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/bin/ta_cli.rs`
- `src-tauri/src/commands_campaign.rs`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_vectorgrid_double_ma_faceoff.ps1`

## Current Verification Surface

- tests embedded in `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- targeted app-core tests the bins indirectly depend on
- manual CLI runs of the bins themselves
- perf-gate and faceoff scripts that consume the same execution surfaces

## Strengths Already Present

- the repo already uses real binaries under `src/bin` instead of burying all diagnostics inside ad hoc scripts
- several bins emit structured JSON-style artifacts instead of console-only output
- the bins exercise real strategy and optimization paths rather than toy mocks
- the fidelity audit harness already has direct unit coverage
- the bins provide a safer place to characterize behavior without disturbing the desktop UI contract

## Weaknesses / Failure Modes

- the bin set is useful but not yet clearly documented as one cohesive diagnostic layer
- argument surfaces and output conventions are inconsistent across binaries
- some bins are measurement-oriented while others are exploratory, but the maturity difference is not documented
- downstream scripts and docs can drift if a bin's CLI shape changes quietly
- there is little shared compatibility guidance for artifact naming, JSON output, and recommended invocation patterns

## Regressions To Avoid

- breaking existing scripts that invoke these bins by changing flags or output shapes silently
- allowing diagnostic bins to become the only place where important behavior is understandable
- duplicating measurement logic across multiple bins without a clear ownership map
- treating one-off diagnostic output as a stable product contract unless it is explicitly frozen
- weakening the fidelity-audit harness while expanding less important exploratory tools

## External Research Summary

- [Cargo Targets - The Cargo Book](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) is relevant because these tools live under `src/bin` and should be treated as intentional binary targets with clear ownership.
- [Criterion.rs Getting Started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because diagnostic runners and performance checks benefit from a benchmark-informed discipline instead of ad hoc timing only.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because several bins exist specifically to characterize GPU behavior and should keep correctness and measurement discipline explicit.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because several bins already emit structured artifacts that should evolve compatibly.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because these headless tools should stay complementary to, not contradictory with, the command-driven product surface.

## Proposed Improvement Approach

1. Treat the diagnostic bins as one explicit measurement layer with documented ownership.
2. Freeze the bins that already matter to program correctness and evidence generation before expanding the exploratory ones.
3. Document argument conventions, output conventions, and artifact naming conventions.
4. Keep diagnostic bins additive: they should illuminate product behavior, not replace product-facing verification.
5. Keep the fidelity-audit harness as the highest-priority maintained bin in this group.

## Checklist

- [ ] Document every currently supported diagnostic bin and its role.
- [ ] Document which bins are evidence-producing versus exploratory.
- [ ] Document CLI flag conventions and output-file conventions for the maintained bins.
- [ ] Add or tighten tests for any bin whose output is consumed by scripts or docs.
- [ ] Review whether shared JSON artifact helpers should be reused more consistently across bins.
- [ ] Keep the fidelity-audit harness documented as the primary strategy-facing evidence tool.

## Verification Plan

- run `cargo test -p ta_app_core --bin real_data_fidelity_audit`
- manually smoke-test the maintained bins after any CLI or artifact-contract change
- verify scripts that launch the bins still work against the documented flags and outputs
- keep at least one structured output artifact per maintained bin available for inspection

## Rollout / Fallback / Compatibility Notes

- preserve current maintained bin entry points unless a compatibility migration is documented
- prefer additive flags and additive output fields over breaking CLI changes
- if a bin is exploratory-only, label it clearly rather than implying product support
- keep the desktop product contract separate from the headless diagnostic contract

## Related Docs

- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
- `REPO_Improvement/07_strats/15_dynamic_ir_campaign_surface_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/25_headless_report_builders_2026-04-08.md`
