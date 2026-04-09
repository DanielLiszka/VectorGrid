# Request Builder Bench Plan

Last updated: 2026-04-09

## Purpose

Define the benchmark contract for the frontend optimization request-builder micro-benchmark and keep it distinct from real browser or product UX timing.

## Scope Boundaries

In scope:

- the benchmark file `web/src/services/optimizeRequestBuilder.bench.test.ts`
- the compare helper `web/scripts/compare-request-builder-bench.mjs`
- the package scripts `bench:request-builder` and `bench:request-builder:compare`
- fixture-based request-construction cost for current non-options optimization request builders

Out of scope:

- options-related request builders
- real browser interaction timing
- service or store correctness tests outside benchmark fixtures
- hard release gating unless a separate threshold policy is introduced intentionally

## Current User-Facing Behavior

The repo already has a dedicated benchmark file that uses benchmark-style execution to measure request-construction cost across a curated fixture set for the optimization request builders. A companion compare script summarizes candidate vs baseline JSON. This is useful as a narrow regression tool for frontend request shaping, but it is fundamentally a micro-benchmark on fixtures and should not be interpreted as end-user UI responsiveness.

## Current Code Surface

- `web/src/services/optimizeRequestBuilder.bench.test.ts` contains the current benchmark fixtures and measurement setup.
- `web/scripts/compare-request-builder-bench.mjs` summarizes candidate-vs-baseline benchmark deltas.
- `web/package.json` exposes `bench:request-builder` and `bench:request-builder:compare`.
- `web/src/services/optimizeRequestBuilder.ts` is the actual request-construction surface being benchmarked.

## Current Verification Surface

- `npm --prefix web run bench:request-builder`
- `node web/scripts/compare-request-builder-bench.mjs <baseline.json> <candidate.json>`
- normal Vitest request-builder correctness tests for functional regressions

## Strengths Already Present

- the benchmark is tightly scoped to a real high-frequency frontend computation surface
- fixture curation already anchors the benchmark in repo-specific request shapes
- the compare script makes repeated local runs easier to interpret
- the benchmark stays outside the main correctness test suite, which keeps its purpose clearer

## Weaknesses / Failure Modes

- micro-benchmark noise can be significant in a JS runtime
- fixture-driven request-construction cost does not capture browser layout, store propagation, or native bridge overhead
- there is no hard threshold gate attached yet, so interpretation is still mostly manual
- bench fixture representativeness can drift if request-builder logic changes faster than the benchmark set

## Regressions To Avoid

- treating request-builder micro-bench numbers as full optimize-panel UX latency
- letting fixtures go stale until the benchmark no longer resembles current request paths
- using the compare script without recording the environment or benchmark mode
- mixing correctness assertions into the benchmark and making it harder to reason about either purpose

## External Research Summary

- [Vitest benchmark config docs](https://vitest.dev/config/benchmark.html) are relevant because the current frontend benchmark uses Vitest’s benchmark support and output semantics.
- [Vitest main docs](https://main.vitest.dev/) are relevant because the benchmark sits inside the same Vite-native test environment as the rest of the frontend suite.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant as a contrast: request-builder micro-bench results should not be confused with user-visible browser timing.
- [Tauri: Calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because request-building performance is only one part of the broader frontend-to-runtime path.

## Proposed Improvement Approach

1. Keep the request-builder benchmark explicitly classified as a fixture-driven micro-benchmark.
2. Maintain a small representative fixture set tied to the current non-options request-builder surface.
3. Use the compare script for local trend inspection, not for inflated product-level claims.
4. Escalate meaningful regressions into request-builder implementation review and broader frontend verification only when the benchmark points to a real pattern.

## Checklist

- [ ] Document the current benchmark fixtures and why they are representative.
- [ ] Document the benchmark output format and compare workflow.
- [ ] Keep the benchmark non-options only.
- [ ] Keep correctness assertions in ordinary tests rather than the benchmark file.
- [ ] Review fixture coverage whenever request-builder routing or request shape changes.
- [ ] Avoid implying browser or end-user timing from micro-benchmark results.

## Verification Plan

- run `npm --prefix web run bench:request-builder` after request-builder performance-sensitive changes
- compare a candidate run against a stored baseline JSON when regression suspicion exists
- re-run relevant correctness tests if the benchmark changes or fails unexpectedly
- spot-check that benchmark fixtures still exercise the intended non-options request routes

## Rollout / Fallback / Compatibility Notes

- preserve the benchmark as a supplemental frontend performance tool before deciding whether it deserves hard thresholds
- prefer improving fixture representativeness over multiplying benchmark cases
- if a threshold policy is added later, keep it clearly separated from user-facing latency claims

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
