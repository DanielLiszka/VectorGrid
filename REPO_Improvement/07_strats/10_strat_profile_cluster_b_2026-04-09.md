# Profile Cluster B Strategy Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for the second standalone profile cluster inside the non-options strategy surface: `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, and `Cdmc`.

## Scope Boundaries

In scope:

- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/cdmc.rs`
- the corresponding `*_gpu_kernel.rs` files
- shared orchestration in `crates/app_core/src/composite_cpu.rs` and `crates/app_core/src/composite_validate.rs`
- frontend/backend capability and request surfaces that expose these profile families

Out of scope:

- Cluster A standalone profiles (`Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`)
- `CompositeMo`
- options-related strategies

## Current User-Facing Behavior

This cluster is also already user-visible through the standard optimize workflow. `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, and `Cdmc` can be requested through the normal product path, produce standard optimization outputs, and participate in validation.

Current behavior already includes:

- strategy-local parameter validation and signal-generation logic
- mixed CPU/GPU-kernel execution in the families that already support it
- route diagnostics for fallback-sensitive families such as `Kqsr` and `Kvdf`
- reuse of the existing result and validation surfaces rather than a sidecar research-only lane

## Current Code Surface

- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/cdmc.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

## Current Verification Surface

- tests embedded in `crates/app_core/src/composite_cpu.rs`
- tests embedded in `crates/app_core/src/composite_validate.rs`
- module-local tests inside the profile files listed above
- `crates/app_core/benches/mkcs_opt.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`

## Strengths Already Present

- each family already owns real strategy-local computation code instead of being implied by config flags alone
- the cluster already participates in the same optimize/validate/artifact workflow as the rest of the product
- route diagnostics exist for some members and are already asserted in integration tests
- GPU-kernel execution is already implemented for multiple families in the cluster
- the cluster is large enough to justify a shared improvement doc instead of scattering one-off notes

## Weaknesses / Failure Modes

- benchmark coverage is visibly thinner and less even than Cluster A
- some of the cluster's quality signal comes from diagnostic-heavy integration tests rather than cleaner dedicated bench surfaces
- orchestration still runs through the same very large shared files, so ownership can blur quickly
- route diagnostic behavior is partially text-driven and can drift from the runtime path
- capability-summary copy can lag actual route behavior for the less frequently exercised profile kinds

## Regressions To Avoid

- changing fallback or route-diagnostic behavior without synchronized tests
- weakening validation parity while tuning optimization paths
- breaking less frequently exercised strategy kinds because shared orchestration changes were only tested against the more popular profiles
- silently changing benchmark posture or removing useful diagnostic coverage without replacement
- merging cluster-specific logic into generic helpers that obscure family ownership
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because this cluster depends on mixed CPU/GPU execution and route fallback clarity.
- [Criterion.rs Getting Started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because this cluster needs more deliberate benchmark coverage and clearer regression baselines.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because these strategies are still part of the standard command-driven product surface, not isolated binaries.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because request/result/report payloads for these families are shared structured contracts.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because frontend capability and visibility regressions around lower-traffic strategy kinds benefit from focused, user-visible checks.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat Cluster B as the second standalone-profile tier with explicit maturity gaps relative to Cluster A.
2. Document which members already have strong route and validation coverage and which still lean on shared integration tests.
3. Keep strategy-local ownership explicit while narrowing the amount of hidden policy inside the giant composite orchestrators.
4. Raise coverage and diagnostics quality before expanding the cluster's public surface further.
5. Keep the cluster fully compatible with the shared optimize/validate workflow while improving the weakest verification gaps first.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the exact membership of Cluster B and why these families are grouped together.
- [x] Document route, fallback, and validation expectations for each member of the cluster.
- [x] Document current benchmark gaps and diagnostic-only coverage gaps.
- [x] Add or tighten route-diagnostic tests where runtime fallback is important.
- [x] Add or tighten capability-summary and request-builder tests for these profile kinds.
- [x] Review whether additional dedicated benches are needed beyond `mkcs_opt.rs`.
- [x] Keep result, validation, and artifact behavior aligned with the standard product workflow.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_cpu.rs`
- run targeted tests in `crates/app_core/src/composite_validate.rs`
- run the existing `mkcs_opt.rs` bench and any relevant strategy-local benches added later
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current strategy ids and frontend labels
- prefer additive hardening and ownership cleanup before broadening user-facing promises for the cluster
- if benchmark coverage is expanded, do it without altering current route semantics
- keep the shared product workflow intact even if more strategy-local code is extracted over time
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making Cluster B explicit in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md): `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, and `Cdmc` are now documented as the staged unified-GPU standalone-profile tier with family-specific validation posture and shared truth anchors.
- Tightened capability evidence by expanding the dedicated standalone capability test in [strategyOptimizationCapabilities.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/utils/strategyOptimizationCapabilities.test.ts) so Cluster B coverage now explicitly includes `Mkcs` instead of leaving it implied by broader loops only.
- Request-builder evidence remains anchored in [optimizeRequestBuilder.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optimizeRequestBuilder.test.ts), where all five Cluster B families already have positive request-shaping coverage through the shared composite CPU builder path and the staged unified-GPU expansion cases.
- Benchmark review result:
  - dedicated optimize bench exists for `Mkcs` in `crates/app_core/benches/mkcs_opt.rs`
  - shared benchmark coverage for `Kqsr`, `Carm`, `Kvdf`, and `Cdmc` already exists in:
    - `crates/app_core/benches/gpu_native_expansion_opt.rs`
    - `crates/app_core/benches/composite_validation_workflow.rs`
  - current shared benches are sufficient for this tranche; no new per-profile bench files were required.
- Stable truth anchors:
  - strategy-local runtime owners in `crates/app_core/src/{mkcs,kqsr,carm,kvdf,cdmc}.rs`
  - shared optimize orchestration in `crates/app_core/src/composite_cpu.rs`
  - shared validation orchestration in `crates/app_core/src/composite_validate.rs`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts src/services/optimizeRequestBuilder.test.ts`
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/07_strats/06_strat_composite_cpu_2026-04-09.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_2026-04-08.md`
- `REPO_Improvement/07_strats/09_strat_profile_cluster_a_2026-04-09.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
