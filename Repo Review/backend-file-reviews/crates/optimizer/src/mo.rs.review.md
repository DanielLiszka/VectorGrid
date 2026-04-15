# Review Packet: `crates/optimizer/src/mo.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/mo.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Algorithmically rich and well tested, but not fully review-ready because search-space rules, heuristic defaults, and the file’s many responsibilities are too implicit for such a foundational optimizer hub`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Optimizer-core`, `Algorithmic`, `Semantics-sensitive`, `Heuristic-sensitive`

## What This File Owns

`mo.rs` is the multi-objective optimizer hub for the workspace:

- search-space modeling and validation: `12-205`
- bounded exact grid support and grid iterators: `206-534`
- objective and Pareto helpers: `592-1033`
- NSGA-II implementation: `1034-1145`
- random/QMC samplers: `1199-1519`
- TPE modeling and search: `1521-2365`
- exact/bounded grid MO entry points: `2348-2494`
- extensive local tests: `2495-3311`

## Classification

- Public surface: yes
- Hot path: moderately
- CUDA/runtime-critical: no
- Strategy-local: no
  - shared optimizer infrastructure

## Direct Interactions

### Backend interaction map

- main consumers:
  - `crates/app_core/src/composite_mo.rs`
- sibling support:
  - `crates/optimizer/src/lib.rs`

### Downstream verification impact if changed

- changing search-space validation changes which requests are accepted or discretized
- changing NSGA-II, QMC, or TPE heuristics changes frontier composition and duplicate-avoidance behavior
- changing Pareto or constraint dominance logic changes optimizer outputs directly

## Standards And Research Log

This packet was written after targeted web research focused on the algorithms explicitly implemented in this file.

### Sources consulted

1. `sobol` crate documentation
   - Source: <https://docs.rs/sobol/latest/sobol/>
   - Why it was consulted:
     - the file implements Sobol-based QMC search using the crate directly
   - Standard informed:
     - the Sobol path should remain documented as a low-discrepancy sequence generator, not a generic random sampler

2. Algorithms for Hyper-Parameter Optimization
   - Source: <https://proceedings.neurips.cc/paper/4443-algorithms-for-hyper-parameter-optimization.pdf>
   - Why it was consulted:
     - the file implements a TPE-style search path
   - Standard informed:
     - TPE defaults and expected-improvement-style heuristics are algorithmic choices, not incidental constants

3. A fast and elitist multiobjective genetic algorithm: NSGA-II
   - Source: <https://ieeexplore.ieee.org/document/996017>
   - Alternate searchable abstract: <https://www.researchgate.net/publication/3418687_A_fast_and_elitist_multiobjective_genetic_algorithm_NSGA-II>
   - Why it was consulted:
     - the file implements NSGA-II-style population search and constrained domination helpers
   - Standard informed:
     - non-dominated sorting and crowding behavior are core semantics and should not drift casually

## Findings

## Contract Clarity

### 1. `SearchSpace`, `ParamDomain`, and bounded-grid behavior deserve explicit rustdoc because they encode acceptance and discretization rules

- Evidence:
  - search-space definitions and validation live at `12-205`
  - bounded exact grid logic starts at `206`
- Why this matters:
  - downstream callers need to understand when float ranges are rejected, when grids return `None`, and how duplicate parameter names are normalized
- Smallest justified fix:
  - document the search-space acceptance/discretization rules on the public types and entry points

## Algorithmic Semantics

### 2. NSGA-II, random/QMC, and TPE all carry heuristic defaults that are compatibility-sensitive

- Evidence:
  - NSGA-II config and implementation: `653-1145`
  - QMC config and search: `1146-1519`
  - TPE config and search: `1156-2365`
- Why this matters:
  - values such as crossover probability, TPE gamma, startup trials, and model point limits materially affect frontier shape
- Smallest justified fix:
  - document these defaults as current optimizer policy; defer retuning

## Module Boundaries And Technical Debt

### 3. The file mixes modeling, grid enumeration, Pareto logic, and three distinct search families

- Evidence:
  - search-space modeling: `12-534`
  - Pareto helpers: `592-1033`
  - three search families span the remainder through `2365`
- Why this matters:
  - the code is workable, but it is not a safe target for opportunistic cleanup
- Smallest justified fix:
  - document the internal ownership map now; defer extraction to a dedicated optimizer refactor pass

## Tests And Missing Verification

### 4. The file has strong targeted tests and that is the main reason to keep the current structure stable for now

- Evidence:
  - tests cover validation, grid behavior, duplicate avoidance, NSGA-II smoke, QMC smoke, TPE smoke, and bounded-grid equivalence: `2495-3311`
- Why this matters:
  - algorithmic drift is more dangerous than file size in this case
- Smallest justified fix:
  - preserve the current test coverage and focus on documentation

## Code-Quality Improvement Register

- Safe now:
  - document search-space acceptance rules
  - document heuristic defaults on NSGA-II/QMC/TPE configs
  - add a file-level ownership map comment
- Coordination required:
  - splitting the file by search family
  - retuning heuristics
  - changing dominance or duplicate-avoidance behavior
- Defer:
  - broad algorithm-hub refactor

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc and top-level ownership comments
- Defer:
  - module extraction
  - heuristic tuning
- Do not casually remove:
  - duplicate-avoidance logic
  - bounded-grid `None` behavior
  - constrained domination semantics

## Performance Opportunity Register

- Safe structural work:
  - comments only
- Benchmark-sensitive:
  - changing parallel evaluation strategy
  - retuning population/search heuristics
  - altering grid chunking or duplicate caches

## Recommended Action

`Larger refactor but defer`

The code would benefit from a split by responsibility, but the safe value now is documenting the algorithmic contracts and heuristic defaults that downstream optimizer behavior depends on.

## Exact Local Verification Required If Changed

1. Crate verification:
   - `cargo test -p ta_optimizer -- --nocapture`
2. Representative app-core consumer:
   - `cargo test -p ta_app_core composite_mo -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_optimizer -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve search-space validation and bounded-grid acceptance semantics unless callers are updated deliberately
- Preserve current dominance, duplicate-avoidance, and heuristic-default behavior unless optimizer outputs are intentionally re-baselined
- Defer broad restructuring until a dedicated optimizer pass can verify frontier behavior end to end
