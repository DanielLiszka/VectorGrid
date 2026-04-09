# Optimizer Grid Stream Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the shared grid-search and streaming-top-k primitives that underpin many strategy-local optimization surfaces.

## Scope Boundaries

In scope:

- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/lib.rs`
- shared scoring, top-k, heatmap, and include-all behavior used by strategy-local optimizers

Out of scope:

- CompositeMo sampler-specific logic
- strategy-local request shaping beyond how it consumes optimizer outputs
- options-related optimizers

## Current User-Facing Behavior

Although users do not interact with `grid.rs` and `stream.rs` directly, these primitives materially affect:

- best-candidate selection
- top-k shortlist composition
- score ordering
- include-all result capture
- heatmap aggregation
- score-function overrides for specialized strategies

These shared behaviors propagate into many optimization results and therefore into visible results/report surfaces.

## Current Code Surface

- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/lib.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`

## Current Verification Surface

- tests in `crates/optimizer/src/grid.rs`
- tests in `crates/optimizer/src/stream.rs`
- tests in `crates/optimizer/src/mo.rs`
- strategy-local optimization tests in `crates/app_core/src/double_ma.rs`
- strategy-local optimization tests in `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/benches/composite_cpu_opt.rs`
- `crates/app_core/benches/composite_mo_opt.rs`

## Strengths Already Present

- grid search is already separated from strategy-specific execution code
- stream aggregation already supports top-k, include-all, heatmap, and custom scoring hooks
- finite-score handling is explicit
- heatmap aggregation already tracks both max and mean-style summaries
- strategy-local code can reuse shared primitives instead of duplicating ranking logic
- `mo.rs` already extends the optimizer layer into richer exact-grid and sampler-backed behavior rather than leaving those concerns outside the crate

## Weaknesses / Failure Modes

- shared optimizer primitives are easy to treat as "obviously correct" and under-test at the integration layer
- custom score hooks can drift from default objective semantics
- top-k and include-all behavior can diverge between direct grid search and stream aggregation
- heatmap aggregation can regress quietly while best/top ordering still looks correct
- changes here have wide blast radius across multiple strategy families
- `grid.rs` and `stream.rs` are intentionally not feature-equivalent, so undocumented assumptions about parity are risky

## Regressions To Avoid

- changing ranking semantics between grid and stream paths
- mishandling non-finite scores
- breaking top-k ordering or best-result selection
- drifting heatmap aggregation from the actual evaluated combos
- changing shared behavior without corresponding strategy-local regression coverage

## External Research Summary

- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because many consumers of these primitives rely on parallel upstream evaluation and then deterministic aggregation.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because shared optimizer primitives are good candidates for benchmark-backed regression checks when behavior is performance-sensitive.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because optimizer outputs are structured contracts consumed by multiple downstream surfaces.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because optimizer results often bridge into JSON-like response/report payloads after aggregation.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because shared aggregation should preserve the benefit of expensive upstream evaluation rather than introducing avoidable host-side inefficiency or inconsistency.

## Proposed Improvement Approach

1. Treat grid and stream aggregation as shared correctness-critical primitives.
2. Freeze shared semantics for:
   - objective scoring
   - custom score hooks
   - best-result selection
   - top-k ordering
   - include-all behavior
   - heatmap aggregation
3. Add integration tests that compare grid and stream behavior on representative inputs.
4. Add performance/regression hooks where shared aggregation is on the hot path.
5. Keep strategy-local overrides explicit rather than partially shadowing shared semantics.

## Checklist

- [ ] Document the shared ranking and aggregation semantics.
- [ ] Document how custom score hooks override default objective scoring.
- [ ] Add or tighten tests comparing grid and stream best/top-k behavior.
- [ ] Add or tighten tests for non-finite score handling.
- [ ] Add or tighten tests for heatmap max/mean/sample-count aggregation.
- [ ] Add or tighten strategy-local integration tests that consume these primitives.
- [ ] Consider benchmark-backed checks for hot-path aggregation behavior.

## Verification Plan

- run targeted tests in `crates/optimizer/src/grid.rs`
- run targeted tests in `crates/optimizer/src/stream.rs`
- run targeted strategy-local optimization tests in `crates/app_core/src/double_ma.rs`
- run targeted strategy-local optimization tests in `crates/app_core/src/composite_cpu.rs`
- if applicable, run targeted Criterion or perf-gate checks for aggregation-heavy paths

## Rollout / Fallback / Compatibility Notes

- preserve shared semantics across grid and stream paths
- treat behavior changes here as wide-scope changes requiring cross-strategy verification
- keep custom scoring explicit and documented
- prefer additive benchmark/test hardening over rewriting shared primitives casually

## Related Docs

- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-08.md`
- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/06_results_heatmap_2026-04-08.md`
- `REPO_Improvement/07_strats/07_strat_composite_mo_2026-04-09.md`
