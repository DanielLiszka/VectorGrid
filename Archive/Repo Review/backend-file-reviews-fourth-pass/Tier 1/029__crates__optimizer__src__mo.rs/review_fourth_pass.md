# Fourth-Pass Review: crates/optimizer/src/mo.rs

- Review date: `2026-04-15`
- Source file: `crates/optimizer/src/mo.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/mo.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/mo.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/mo.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Targeted hardening now; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet correctly reopened this file as a semantics-sensitive optimizer hub, but it narrowed too quickly onto `SearchSpace::sample_param()` as the main bounded defect. Re-reading the live source, the current consumer, and the external algorithm/reference material changes the priority order.

My conclusion is that the **large structural refactor should still be deferred**, but the **overall bucket should change**. There are now several bounded hardening fixes that are more justified than another documentation-only pass:

- the Sobol QMC path can panic even though the public API returns `Result`
- non-finite objective/constraint handling is weaker than the current packet acknowledged
- one public exact-grid contract is currently overstated
- the `sample_param()` issue is real, but it is not the highest-leverage safe-now fix and it is semver-sensitive if handled incautiously

So the prior “larger refactor but defer” framing is now too coarse. The right framing is: **do the bounded hardening now, still defer the larger redesign**.

## Fourth-Pass Research Log

I re-read the live `mo.rs` source with special attention to five surfaces: public sampling/search-space helpers, conversion of `EvalOutcome<T>` into `MoPoint<T>`, Pareto/frontier selection, the Sobol QMC path, and test coverage near the bottom of the file. I also re-read the provided `composite_mo.rs` consumer to see whether the optimizer core can actually receive raw non-finite objectives from live callers.

Externally, I re-checked four references:

1. **Rust API Guidelines — Dependability / C-VALIDATE**  
   This reinforced that bounded hardening should prefer explicit validation/erroring over panic or fabricated fallback behavior.

2. **`sobol` crate docs and source**  
   This was the key external confirmation that `JoeKuoD6::minimal()` is intentionally a **100-dimension** parameter set, and that `Sobol::new()` enforces that dimension ceiling with an assertion rather than a `Result`.

3. **Bergstra et al., “Algorithms for Hyper-Parameter Optimization”**  
   This confirmed the earlier packets were right to treat TPE heuristics as compatibility-sensitive policy, not opportunistic cleanup.

4. **Deb et al., NSGA-II**  
   This again supported preserving dominance/frontier semantics for normal finite inputs and avoiding casual heuristic retuning during a hardening pass.

Those external references did not weaken the existing caution around heuristic tuning. They did, however, make the panic-avoidance and invalid-input story look more urgent than the third-pass packet suggested.

## What Fourth Pass Sharpened

### 1. The third-pass packet got the right *direction*, but not the right *priority order*

The third pass was right to flag `SearchSpace::sample_param()` at lines `157-161` as a public contract smell. Returning a fabricated random `Bool` on an out-of-range index is not an honest failure mode.

But that packet overstates it as the first hardening target. The more urgent bounded defects are the ones that can silently corrupt search semantics or panic through a `Result`-returning API:

- `Objectives2::normalize()` at lines `603-625`
- `overall_violation()` at lines `628-635`
- `pareto_front_constrained_indices()` at lines `759-820`
- the Sobol path at lines `1479-1482`

So the third-pass packet was directionally correct, but too narrow.

### 2. Non-finite handling is the most important missed hardening lane

This is the clearest thing the third-pass packet missed.

`Objectives2::normalize()` first maps any non-finite raw objective to `NEG_INFINITY` at lines `605-614`, and only then applies direction normalization at lines `615-623`. That means a **non-finite minimize objective becomes `+INFINITY`** after negation. In other words, broken minimize-side input can be transformed into the best possible normalized score.

`overall_violation()` at lines `628-635` is similarly permissive in the wrong direction: every non-finite constraint is rewritten to `0.0`, then clipped with `max(0.0)`. So `NaN` or `+INF` constraints collapse to **no violation at all**.

That would already be concerning in isolation, but the provided `CompositeMo` consumer does in fact feed raw metric values straight into objective slots at `composite_mo.rs` lines `2571-2573` without a final optimizer-core sanitization layer. That means the optimizer core is not only theoretically exposed to non-finite inputs; its main known consumer can generate them if an upstream metric goes non-finite.

The downstream consequences are not cosmetic:

- `constrained_dominates()` at lines `696-716` uses direct float comparisons, so `NaN` objectives can disable dominance in both directions.
- `pareto_front_constrained_indices()` skips non-finite second-objective values entirely at lines `813-815`, which means a feasible population with only non-finite second objectives can collapse to an **empty front**.
- NSGA-II parent selection and elite selection then operate on these broken points as if they were ordinary non-dominated members.

This is not a “retune later” issue. It is a bounded correctness hardening issue.

### 3. The Sobol path currently has an avoidable panic boundary

The current `QmcSequence::Sobol` branch unconditionally loads `JoeKuoD6::minimal()` and calls `Sobol::<f64>::new(dim, &params)` at lines `1479-1482`.

That matters because the external `sobol` crate documents `minimal()` as the compact parameter set for up to 100 dimensions, and its constructor asserts the dimension ceiling instead of returning an error. In this file, that means a sufficiently large `SearchSpace` can turn `qmc_mo_search_with_stats()` into a panic path even though the API itself returns `Result<MoSearchResult<T>, String>`.

That is a stronger bounded defect than `sample_param()` because it is a direct panic risk on a real search entry point.

This is also where the current packet was too light on compatibility nuance. There are two possible fixes, and they are not equally safe:

- **Safer now:** preflight Sobol dimensionality and return a normal `Err` for unsupported dimensions.
- **More ambitious:** switch between `minimal()`, `standard()`, and `extended()` automatically.

I would not recommend auto-switching blindly in a hardening pass. The `sobol` docs make clear that those parameter sets have materially different embedded-data sizes. That creates a binary-size / packaging consideration that is outside the scope of a small optimizer-only cleanup. An explicit `Err` is the conservative fix.

### 4. The exact-grid axis-limit contract is currently slightly overstated

The public `grid_iter()` docs at lines `164-170` say enumeration refuses “any axis with more than `max_axis_values` discrete values.”

That is true for `IntRange` and `Categorical`, but not quite true for `Bool`.

- `bounded_exact_grid()` treats `ParamDomain::Bool` as axis length `2` at line `260`, without comparing against `max_axis_values`.
- `domain_grid_values()` always returns two Bool values at line `344`, again without checking `max_axis_values`.

This is not a major live consumer risk, because the current `CompositeMo` caller uses very large axis limits. Still, the contract is overstated today. Either the docs should carve Bool out explicitly, or Bool should honor the same axis-limit rule as the other discrete domains.

This is a safe-now hardening/documentation alignment issue, not a refactor.

### 5. `sample_param()` is still worth addressing, but the third-pass recommendation was too strong as written

The third-pass packet said the first hardening step should be to “reject or explicitly surface out-of-range parameter indexes.”

That is directionally correct, but it underweights API-compatibility risk. `sample_param()` is public. Changing it from `ParamValue` to `Result<ParamValue, String>` would be a public API break. Doing that in a supposedly conservative hardening pass would be a poor fit unless the crate is intentionally taking a breaking change.

The conservative near-term options are narrower:

- add a checked sibling helper and migrate internal callers
- add an internal `debug_assert!` for invariant-bearing call sites
- document that `sample()` / `sample_param()` assume a validated `SearchSpace`

That last point also matters because the problem is broader than just the invalid index. The public sampling helpers at lines `150-161` do not validate the space first, so malformed domains can still panic or misbehave inside `sample_one()`. Fixing only the out-of-range index path would improve one edge but leave the wider precondition implicit.

So the third-pass packet was right to keep this in view, but wrong to make it the first bounded change.

### 6. One real technical-debt item still belongs in the deferred bucket

The file still has a real design inconsistency around duplicate handling, but I would keep it deferred.

Most search paths use the collision-aware cache pipeline at lines `928-1008`, where `param_values_hash()` is only a bucket key and full `ParamValue` equality is checked before treating candidates as duplicates.

The TPE path does not do that. At lines `2307-2313`, it tracks only `HashSet<u64>` keys. That means:

- collision handling is weaker than in the rest of the file
- uniqueness statistics are not conceptually aligned with the other search families
- any change here can shift trial-budget consumption and duplicate-avoidance telemetry

That is real technical debt, but it is not the safest immediate change because it is close to search-budget semantics, not just validation hardening. I would keep this deferred behind the bounded hardening items above.

## Implementation Priority

### Safe-now hardening

**1. Make Sobol oversize dimensions fail with `Err`, not panic.**  
Add an explicit dimensionality preflight in the Sobol branch before calling the external constructor. Keep the current compact parameter-set choice unless the project deliberately accepts a larger embedded-data footprint.

**2. Centralize non-finite `EvalOutcome` hardening before constructing `MoPoint<T>`.**  
This is the highest-value correctness hardening in the file. The fix should be centralized where `EvalOutcome<T>` is converted into `MoPoint<T>` so that random search, QMC, NSGA-II cache evaluation, and TPE evaluation all share the same rules. The goal is not to retune the optimizer. The goal is to ensure broken metrics do not become feasible or “best possible” points.

A conservative policy would be:

- non-finite constraints should become an explicit violation, not `0.0`
- non-finite objectives should be pessimized consistently
- frontier helpers should never be able to return an empty feasible front solely because the second objective is non-finite

**3. Add missing targeted tests for the above behavior.**  
The file already has strong breadth coverage, but it is missing narrow regression fixtures for exactly the fragile cases above. Add focused tests for:
- `Objectives2::normalize()` with non-finite minimize inputs
- `overall_violation()` with `NaN` and `INF`
- `pareto_front_constrained_indices()` when feasible points carry non-finite second objectives
- Sobol search with more than 100 parameters returning `Err`
- Bool-axis `max_axis_values` behavior, or update the docs to match the current implementation

**4. Keep `sample_param()` hardening additive, not signature-breaking.**  
Do not turn a bounded hardening pass into a semver break unless that is an explicit project choice. If touched now, prefer an additive checked helper plus internal migration.

### Larger redesign still deferred

The following still belong in the deferred bucket:

- splitting the file by responsibility (`SearchSpace`, grid, Pareto/dominance, NSGA-II, QMC, TPE)
- unifying TPE duplicate handling with the richer cache/equality model used elsewhere
- changing heuristic defaults or ranking heuristics
- changing `param_values_hash()` semantics
- redesigning the public sampling helpers around checked return types

That work is still real and still worth doing later, but it is no longer the only meaningful action available.

## Verification If Changed

At minimum, keep the existing local verification lane and add targeted coverage for the new hardening cases:

1. `cargo test -p ta_optimizer -- --nocapture`
2. `cargo test -p ta_app_core composite_mo -- --nocapture`

In addition, add focused tests in `mo.rs` for the new bounded cases rather than relying on broad smoke coverage:

- a non-finite minimize-objective normalization fixture
- a non-finite constraint aggregation fixture
- a Pareto-front fixture proving feasible non-finite objective inputs do not collapse to an empty front
- a Sobol dimension-limit fixture
- a Bool-axis `max_axis_values` fixture, unless the public docs are narrowed instead

No GPU ABI validation or persistence migration is required for these fixes. The main downstream risk surface is **optimizer-output semantics for bad inputs**, plus the `CompositeMo` consumer that currently passes raw objective values through.

## Regression And Feature-Preservation Constraints

Preserve the current behavior for **ordinary finite inputs**. This file is still a shared optimizer-core surface, and the external papers/docs reinforce that heuristic and frontier behavior are policy-sensitive.

That means:

- do **not** retune NSGA-II, TPE, or QMC defaults in the same change
- do **not** alter exact-bit duplicate semantics in the shared cache path
- do **not** change finite-input Pareto ordering just to “clean up” the code
- do **not** silently switch Sobol parameter sets without an explicit binary-size / packaging decision
- do **not** break the public `sample_param()` signature unless a deliberate API change is acceptable

The bounded fixes above should be framed as **panic prevention and invalid-input pessimization**, not as algorithm redesign.

One more downstream caution: if non-finite objective handling is hardened centrally, some previously accepted but broken points may stop appearing in Pareto outputs. That is the correct direction, but it means regression checks should focus on preserving finite-input outputs while intentionally rebasing the new invalid-input fixtures.

## Fourth-Pass Conclusion

The third-pass packet was right to keep the large refactor deferred, and it was right to notice that `sample_param()` is not an honest public failure mode. But it underweighted two more important bounded problems:

1. the Sobol QMC path can panic through a `Result`-returning API
2. non-finite objectives/constraints can be transformed into feasible or even favorable optimizer state

Because those problems are both **bounded and fixable without retuning the search families**, the current third-pass bucket should **change**.

**Recommended fourth-pass bucket:** `Targeted hardening now; larger redesign still deferred`.

That keeps the earlier caution where it belongs — around heuristic tuning, module extraction, and cross-family semantic cleanup — while acknowledging that the live file now has enough concrete hardening work to justify action before any broader redesign pass.
