# CompositeMo Strategy Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for `CompositeMo` as the multi-objective composite search surface built on explicit search-space, objective, and sampler contracts.

## Scope Boundaries

In scope:

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `src-tauri/src/commands_optimize.rs`
- CompositeMo request, sampler, objective, and search-space behavior

Out of scope:

- single-objective CompositeCpu behavior outside shared prescan helpers
- validation workbook/report behavior
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

## External Research Summary

- [NSGA-II overview in pymoo](https://pymoo.org/algorithms/moo/nsga2.html) is relevant because the repo explicitly exposes NSGA-II-style multi-objective search behavior and should keep its terminology and expectations coherent.
- [Optuna `TPESampler` docs](https://optuna.readthedocs.io/en/v2.10.1/reference/generated/optuna.samplers.TPESampler.html) are relevant because the repo exposes a TPE-style sampler and should keep sampler-configuration expectations explicit.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because stage-0 prescan and heterogeneous execution only help if they preserve clear compute/memory boundaries.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because CompositeMo request payloads cross the frontend/backend boundary as structured JSON-like values.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because CompositeMo is effectively defined by a structured request schema with sampler and search-space invariants.

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

## Checklist

- [ ] Document the CompositeMo request contract and sampler/objective invariants.
- [ ] Document how constraints and optional prescan interact with the search space.
- [ ] Add or tighten tests for each exposed sampler family.
- [ ] Add or tighten tests for objective ordering and constraint carry-through.
- [ ] Add or tighten request-builder tests for CompositeMo payload normalization.
- [ ] Add or tighten capability-summary tests so product text stays aligned with CPU-evaluated plus optional-GPU-prescan semantics.
- [ ] Document which behaviors are strategy-local vs optimizer-core shared.
- [ ] Keep CompositeMo-specific terminology explicit in user-visible status/reporting surfaces where applicable.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_mo.rs`
- run targeted tests in `crates/optimizer/src/mo.rs`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run targeted optimize-command tests in `src-tauri`
- manually inspect one normalized CompositeMo request and one representative result payload

## Rollout / Fallback / Compatibility Notes

- preserve current sampler names and objective semantics unless a versioned request change is made
- keep CompositeMo distinct from CompositeCpu in both docs and request shaping
- if sampler options change, update both frontend normalization and backend validation together
- keep optional prescan explicit rather than silently mandatory

## Related Docs

- `REPO_Improvement/03_opt/16_opt_composite_mo_command_2026-04-08.md`
- `REPO_Improvement/07_strats/13_eng_composite_mo_prescan_search_2026-04-09.md`
- `REPO_Improvement/07_strats/21_optimizer_grid_stream_2026-04-09.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-08.md`
