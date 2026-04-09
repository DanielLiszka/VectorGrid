## Purpose

Define the improvement plan for the shared multi-objective optimizer core that currently lives in `crates/optimizer/src/mo.rs`, with emphasis on contract clarity, regression containment, and reusable verification rather than new algorithm scope.

## Scope Boundaries

In scope:

- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/app_core/src/composite_mo.rs`
- shared search-space, Pareto, sampler, cache, and evaluation-accounting behavior used by non-options strategy paths

Out of scope:

- frontend optimization UX
- options-related optimizer code
- replacing the current algorithm set with different families
- changing product-level route policy outside how `composite_mo` consumes the optimizer core

## Current User-Facing Behavior

Users do not interact with `mo.rs` directly, but this shared core affects visible behavior in the composite multi-objective optimization lane:

- which candidates are actually evaluated
- how Pareto fronts are formed and surfaced
- how duplicate parameter sets are avoided or collapsed
- how exact-grid bounded search differs from sampler-backed search
- whether search progress and result quality stay stable across repeated runs

Because these behaviors feed `composite_mo`, any change in `mo.rs` can alter optimization outputs without any frontend code changing.

## Current Code Surface

- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/benches/composite_mo_opt.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`

Key current ownership inside `mo.rs`:

- search-space/domain definitions via `ParamDomain`, `ParamDef`, `SearchSpace`, and `ParamValue`
- bounded exact-grid helpers and chunked iteration
- constrained Pareto sorting and elite selection
- duplicate-aware evaluation caching
- `NSGA-II`, random, QMC, and TPE-based multi-objective searches
- shared result/stat structs such as `MoSearchStats` and `MoSearchResult`

## Current Verification Surface

- unit tests embedded in `crates/optimizer/src/mo.rs`
- crate-level optimizer tests through `crates/optimizer/src/lib.rs`
- consumer tests in `crates/app_core/src/composite_mo.rs`
- performance coverage in `crates/app_core/benches/composite_mo_opt.rs`
- broader dynamic-strategy CPU benchmark coverage in `crates/app_core/benches/dynamic_strategy_cpu.rs`

## Strengths Already Present

- the optimizer crate already centralizes the multi-objective search mechanics instead of scattering them across strategy code
- bounded exact-grid and sampler-backed search live in the same module, which makes parity checks possible
- evaluation caching and duplicate suppression are already treated as first-class concerns
- result structs already carry explicit stats rather than forcing callers to reconstruct search behavior indirectly
- `composite_mo.rs` is a clear consumer boundary, which makes it possible to harden the core without redesigning the whole product

## Weaknesses / Failure Modes

- `mo.rs` currently carries too many responsibilities in one file, which makes boundary drift more likely
- exact-grid and sampler paths can silently diverge on evaluation accounting, dedup, or repair semantics
- it is easy to change one algorithm path while leaving the shared result/stat contract underspecified
- Pareto/frontier correctness bugs may not be obvious from superficial result inspection
- duplicate suppression and caching are correctness-critical, but their guarantees are not documented as an explicit contract
- benchmark coverage exists, but it is not yet clearly tied to stable behavioral invariants

## Regressions To Avoid

- changing Pareto dominance or frontier semantics unintentionally
- changing exact-grid or candidate-set accounting without updating stats expectations
- letting sampler paths produce duplicate-heavy or repair-heavy behavior that obscures search efficiency
- changing search-space validation rules in a way that silently breaks existing composite requests
- improving one algorithm family while degrading result-contract consistency across the others

## External Research Summary

- [NSGA-II original reference](https://doi.org/10.1109/4235.996017) is relevant because the current implementation explicitly uses non-dominated sorting, crowding distance, and elitist selection; improvements should preserve those semantics where the code claims to implement NSGA-II.
- [Multiobjective Tree-Structured Parzen Estimator](https://doi.org/10.1613/jair.1.13188) is relevant because the current optimizer core includes TPE-style multi-objective search, so the implementation plan should treat startup-trial behavior, candidate generation, and Pareto-aware weighting as contract-sensitive rather than ad hoc.
- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because batch evaluation and deterministic aggregation often interact with parallel iteration choices in optimizer code.
- [Criterion.rs getting started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because sampler overhead, cache behavior, and bounded-grid iteration are all good candidates for benchmark-backed regression checks.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because optimizer outputs eventually become structured artifacts and report payloads, which should stay stable even if the search internals change.

## Proposed Improvement Approach

1. Treat `mo.rs` as a shared optimizer contract, not just a bag of algorithms.
2. Split the improvement work into four explicit concerns without changing external behavior first:
   - search-space validation and bounded-grid rules
   - Pareto/frontier correctness rules
   - evaluation-accounting, dedup, and cache behavior
   - per-algorithm parameter/default governance
3. Add written invariants for every search entry point:
   - how many evaluations may occur
   - when repair hooks run
   - when duplicates are counted versus skipped
   - what stats fields must mean across exact and sampled paths
4. Add cross-algorithm consistency tests using a small deterministic problem family that can be exercised by:
   - exact-grid
   - bounded exact-grid
   - candidate-set search
   - random/QMC/TPE/NSGA-II
5. Keep the first implementation wave additive:
   - documentation
   - stronger tests
   - narrower helper extraction only where it clarifies invariants
6. Defer structural file splits until the semantic contract is frozen by tests, so cleanup does not accidentally redefine behavior.

## Checklist

- [ ] Document search-space validation invariants for `ParamDomain`, `ParamDef`, and `SearchSpace`.
- [ ] Document the exact meaning of `MoSearchStats` fields across all search entry points.
- [ ] Add a deterministic fixture that exercises bounded exact-grid, candidate-set, and sampler-backed search on the same objective surface.
- [ ] Add or tighten tests for constrained Pareto dominance and crowding-distance behavior.
- [ ] Add or tighten tests for duplicate suppression and cache-hit accounting.
- [ ] Add or tighten tests for repair-hook behavior and invalid-candidate handling.
- [ ] Add or tighten tests ensuring exact-grid and bounded exact-grid agree where their domains overlap.
- [ ] Add benchmark-backed checks for cache-heavy and duplicate-heavy workloads.
- [ ] Decide whether helper extraction inside `mo.rs` improves auditability without creating API churn.

## Verification Plan

- run targeted tests in `crates/optimizer/src/mo.rs`
- run broader optimizer crate tests via `cargo test -p ta_optimizer`
- run consumer tests in `crates/app_core/src/composite_mo.rs`
- run `crates/app_core/benches/composite_mo_opt.rs` if behavior-preserving refactors touch hot paths
- compare representative `composite_mo` outputs before and after changes when stats or ranking logic is touched

## Rollout / Fallback / Compatibility Notes

- preserve the existing public optimizer-core API unless a stronger contract forces a change
- keep default algorithm configs compatible with current consumers until dedicated follow-up docs say otherwise
- prefer additive docs/tests first, helper extraction second, and behavioral tuning last
- if any refactor makes the result contract harder to explain, stop and simplify rather than continuing the split

## Related Docs

- `REPO_Improvement/07_strats/07_strat_composite_mo_2026-04-09.md`
- `REPO_Improvement/07_strats/13_eng_composite_mo_prescan_search_2026-04-09.md`
- `REPO_Improvement/07_strats/21_optimizer_grid_stream_2026-04-09.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-08.md`
