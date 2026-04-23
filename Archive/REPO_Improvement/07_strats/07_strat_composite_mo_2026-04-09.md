# CompositeMo Strategy Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for `CompositeMo` as the multi-objective composite search surface built on explicit search-space, objective, and sampler contracts.

## Scope Boundaries

In scope:

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `src-tauri/src/commands_optimize.rs`
- CompositeMo request, sampler, objective, search-space, and persisted-artifact validation/report behavior

Out of scope:

- single-objective CompositeCpu behavior outside shared prescan helpers
- validation workbook rendering internals outside the CompositeMo artifact contract
- options-related search

## Current User-Facing Behavior

`CompositeMo` already exists as a distinct optimization surface. The current request contract exposes:

- explicit search spaces
- maximize/minimize objective pairing
- sampler choice across `Nsga2`, `Random`, `Tpe`, and `Qmc`
- optional constraints
- optional stage-0 prescan narrowing

This means CompositeMo is already more than “advanced mode”; it is a separate search contract with its own optimization semantics.

## Current Code Surface

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `src-tauri/src/commands_optimize.rs`

## Current Verification Surface

- tests in `crates/app_core/src/composite_mo.rs`
- tests in `crates/optimizer/src/mo.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- Tauri optimize-command tests where CompositeMo requests are normalized and dispatched
- `crates/app_core/benches/composite_mo_opt.rs`

## Strengths Already Present

- CompositeMo already uses explicit search-space and objective types
- sampler choice is explicit rather than hidden
- optional prescan is already modeled in the request
- constraints are part of the request contract
- the surface is strategy-local rather than buried inside generic request plumbing
- small discrete spaces can already collapse into bounded exact-grid evaluation instead of always forcing the full sampler path

## Weaknesses / Failure Modes

- sampler/objective/search-space complexity is easy to expose without enough user-facing explanation
- request-shaping drift can break backend assumptions about domains and sampler config
- prescan narrowing and sampler semantics can interact in hard-to-see ways
- multi-objective behavior has fewer user-facing review surfaces than simpler optimization modes
- the surface spans optimizer-core code and strategy-local code
- users can confuse CompositeMo with normal Top-K flows even though it produces a different artifact/result shape

## Regressions To Avoid

- changing sampler semantics without updating request docs/tests
- weakening search-space validation
- breaking constraint handling or objective ordering
- letting frontend request shaping produce impossible or ambiguous CompositeMo payloads
- treating CompositeMo as interchangeable with CompositeCpu despite different optimization semantics
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [NSGA-II overview in pymoo](https://pymoo.org/algorithms/moo/nsga2.html) is relevant because the repo explicitly exposes NSGA-II-style multi-objective search behavior and should keep its terminology and expectations coherent.
- [Optuna `TPESampler` docs](https://optuna.readthedocs.io/en/v2.10.1/reference/generated/optuna.samplers.TPESampler.html) are relevant because the repo exposes a TPE-style sampler and should keep sampler-configuration expectations explicit.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because stage-0 prescan and heterogeneous execution only help if they preserve clear compute/memory boundaries.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because CompositeMo request payloads cross the frontend/backend boundary as structured JSON-like values.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because CompositeMo is effectively defined by a structured request schema with sampler and search-space invariants.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat CompositeMo as a distinct optimization product surface with its own vocabulary and invariants.
2. Freeze the request contract around:
   - search space
   - objectives
   - constraints
   - sampler selection
   - optional prescan
3. Expand tests where frontend shaping and backend sampler assumptions can drift.
4. Keep sampler semantics explicit in docs/status surfaces rather than relying on internal type names.
5. Preserve strategy-local ownership even when optimizer-core helpers are shared.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the CompositeMo request contract and sampler/objective invariants.
- [x] Document how constraints and optional prescan interact with the search space.
- [x] Add or tighten tests for each exposed sampler family.
- [x] Add or tighten tests for objective ordering and constraint carry-through.
- [x] Add or tighten request-builder tests for CompositeMo payload normalization.
- [x] Add or tighten capability-summary tests so product text stays aligned with CPU-evaluated plus optional-GPU-prescan semantics.
- [x] Document which behaviors are strategy-local vs optimizer-core shared.
- [x] Keep CompositeMo-specific terminology explicit in user-visible status/reporting surfaces where applicable.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_mo.rs`
- run targeted tests in `crates/optimizer/src/mo.rs`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run targeted optimize-command tests in `src-tauri`
- manually inspect one normalized CompositeMo request and one representative result payload
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current sampler names and objective semantics unless a versioned request change is made
- keep CompositeMo distinct from CompositeCpu in both docs and request shaping
- if sampler options change, update both frontend normalization and backend validation together
- keep optional prescan explicit rather than silently mandatory
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Kept this CompositeMo tranche additive and vocabulary-first. No sampler/runtime rewrite landed here; the work instead tightened the exposed contract and the follow-up-analysis verification around the existing Pareto search path.
- Expanded `REPO_MAP.md` so CompositeMo is explicitly documented as a distinct search-space/sampler/prescan product surface with `Nsga2`, `Random`, `Tpe`, and `Qmc`, bounded exact-grid collapse for tiny discrete spaces, and CPU-preferred validation semantics.
- Tightened the user-facing capability summary in `web/src/utils/strategyOptimizationCapabilities.ts` so CompositeMo now explicitly advertises its sampler families, prescan boundary, and bounded exact-grid collapse instead of presenting as a vague multi-objective path.
- Added a request-builder regression in `web/src/services/optimizeRequestBuilder.test.ts` for the `Qmc` sampler contract, covering the last exposed sampler family that was not explicitly asserted in frontend normalization tests.
- Added frontend analysis regressions in `web/src/services/analysisService.test.ts` that prove CompositeMo follow-up drilldown/local-stability requests route through the composite command family and use row-scoped Pareto params, overlays, signal mode, and strategy payloads instead of incorrectly falling back to the base request.
- Added validation/report parity coverage for persisted CompositeMo artifacts: `validationService.test.ts` now proves `run.res.pareto` drives `pareto_frontier` admission during validation and stays on the artifact-backed `save_report_xlsx_from_artifact` path for report export.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts src/services/optimizeRequestBuilder.test.ts src/services/analysisService.test.ts src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`, `cargo test -p ta_app_core composite_mo --quiet`, `cargo test -p ta_desktop_demo_app commands_reports_validation_common --lib`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/16_opt_composite_mo_command_2026-04-08.md`
- `REPO_Improvement/07_strats/13_eng_composite_mo_prescan_search_2026-04-09.md`
- `REPO_Improvement/07_strats/21_optimizer_grid_stream_2026-04-09.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-08.md`
