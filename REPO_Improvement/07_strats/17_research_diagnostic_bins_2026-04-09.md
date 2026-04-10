# Research Diagnostic Bins Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Cargo Targets - The Cargo Book](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) is relevant because these tools live under `src/bin` and should be treated as intentional binary targets with clear ownership.
- [Criterion.rs Getting Started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because diagnostic runners and performance checks benefit from a benchmark-informed discipline instead of ad hoc timing only.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because several bins exist specifically to characterize GPU behavior and should keep correctness and measurement discipline explicit.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because several bins already emit structured artifacts that should evolve compatibly.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because these headless tools should stay complementary to, not contradictory with, the command-driven product surface.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat the diagnostic bins as one explicit measurement layer with documented ownership.
2. Freeze the bins that already matter to program correctness and evidence generation before expanding the exploratory ones.
3. Document argument conventions, output conventions, and artifact naming conventions.
4. Keep diagnostic bins additive: they should illuminate product behavior, not replace product-facing verification.
5. Keep the fidelity-audit harness as the highest-priority maintained bin in this group.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every currently supported diagnostic bin and its role.
- [x] Document which bins are evidence-producing versus exploratory.
- [x] Document CLI flag conventions and output-file conventions for the maintained bins.
- [x] Add or tighten tests for any bin whose output is consumed by scripts or docs.
- [x] Review whether shared JSON artifact helpers should be reused more consistently across bins.
- [x] Keep the fidelity-audit harness documented as the primary strategy-facing evidence tool.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `cargo test -p ta_app_core --bin real_data_fidelity_audit`
- manually smoke-test the maintained bins after any CLI or artifact-contract change
- verify scripts that launch the bins still work against the documented flags and outputs
- keep at least one structured output artifact per maintained bin available for inspection
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change

## Rollout / Fallback / Compatibility Notes

- preserve current maintained bin entry points unless a compatibility migration is documented
- prefer additive flags and additive output fields over breaking CLI changes
- if a bin is exploratory-only, label it clearly rather than implying product support
- keep the desktop product contract separate from the headless diagnostic contract
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by expanding the diagnostic-bin map in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md):
  - added `profile_doublema_campaign_prefetch.rs`
  - added `real_data_fidelity_audit.rs`
  - documented maintained evidence bins versus supplemental/exploratory helpers
  - documented the current role of `ta_cli.rs`, `profile_gpu_dynamic_runner.rs`, `real_data_optimizer_eval.rs`, `profile_doublema_campaign_prefetch.rs`, `real_data_fidelity_audit.rs`, and `stage6b_kernel_report.rs`
- Kept the fidelity-audit harness as the primary maintained strategy-facing evidence bin and reused its newly testable CLI contract as the main tightened test surface for this tranche.
- Review result:
  - `real_data_fidelity_audit.rs` is the primary maintained evidence tool
  - `profile_gpu_dynamic_runner.rs` remains gate-backed via `scripts/run_gpu_perf_gate.ps1`
  - `profile_doublema_campaign_prefetch.rs`, `real_data_optimizer_eval.rs`, and `stage6b_kernel_report.rs` remain supplemental/exploratory and are now labeled that way in the routing doc
- Acceptance evidence:
  - `cargo test -p ta_app_core --bin real_data_fidelity_audit`
  - `cargo test -p ta_desktop_demo_app campaign_ --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
- `REPO_Improvement/07_strats/15_dynamic_ir_campaign_surface_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/25_headless_report_builders_2026-04-08.md`
