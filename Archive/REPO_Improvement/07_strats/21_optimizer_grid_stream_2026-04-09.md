# Optimizer Grid Stream Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because many consumers of these primitives rely on parallel upstream evaluation and then deterministic aggregation.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because shared optimizer primitives are good candidates for benchmark-backed regression checks when behavior is performance-sensitive.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because optimizer outputs are structured contracts consumed by multiple downstream surfaces.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because optimizer results often bridge into JSON-like response/report payloads after aggregation.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because shared aggregation should preserve the benefit of expensive upstream evaluation rather than introducing avoidable host-side inefficiency or inconsistency.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

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
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the shared ranking and aggregation semantics.
- [x] Document how custom score hooks override default objective scoring.
- [x] Add or tighten tests comparing grid and stream best/top-k behavior.
- [x] Add or tighten tests for non-finite score handling.
- [x] Add or tighten tests for heatmap max/mean/sample-count aggregation.
- [x] Add or tighten strategy-local integration tests that consume these primitives.
- [x] Consider benchmark-backed checks for hot-path aggregation behavior.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `crates/optimizer/src/grid.rs`
- run targeted tests in `crates/optimizer/src/stream.rs`
- run targeted strategy-local optimization tests in `crates/app_core/src/double_ma.rs`
- run targeted strategy-local optimization tests in `crates/app_core/src/composite_cpu.rs`
- if applicable, run targeted Criterion or perf-gate checks for aggregation-heavy paths
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve shared semantics across grid and stream paths
- treat behavior changes here as wide-scope changes requiring cross-strategy verification
- keep custom scoring explicit and documented
- prefer additive benchmark/test hardening over rewriting shared primitives casually
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by tightening the shared optimizer proofs instead of widening the runtime surface:
  - [grid.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/optimizer/src/grid.rs) now clamps non-finite scores to `NEG_INFINITY`, matching the stream aggregator
  - `grid.rs` also now has direct parity tests for best/top-k behavior against `StreamAggregator`
  - [stream.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/optimizer/src/stream.rs) now has direct tests for:
    - custom `with_score_fn` ranking overriding the default objective
    - param-aware `with_pair_score_fn` ranking with non-finite clamping
    - include-all push-order preservation while Top-K remains score-sorted
- Expanded the optimizer truth anchors in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md) so contributors can find the shared grid/stream contract faster.
- Kept scope intentionally additive:
  - no strategy-local optimizer requests or outputs were rewritten
  - no benchmark contract was widened
  - `all_scores` remains strategy-local where it already was
- Acceptance evidence:
  - `cargo test -p ta_optimizer --lib`
  - `cargo test -p ta_optimizer grid_search_ --lib`
  - `cargo test -p ta_desktop_demo_app campaign_ --lib`
- Static-analysis note:
  - the touched Rust files were formatted with `rustfmt --edition 2021`
  - repo-wide `cargo fmt --all --check` still reports unrelated pre-existing formatting drift outside this tranche, so no broad workspace formatting sweep was applied here

## Related Docs

- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-08.md`
- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/06_results_heatmap_2026-04-08.md`
- `REPO_Improvement/07_strats/07_strat_composite_mo_2026-04-09.md`
