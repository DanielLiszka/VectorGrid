# Request Builder Bench Plan

Last updated: 2026-04-10

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
- generated benchmark artifact under `target/request-builder-bench.json`
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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vitest benchmark config docs](https://vitest.dev/config/benchmark.html) are relevant because the current frontend benchmark uses Vitest’s benchmark support and output semantics.
- [Vitest main docs](https://main.vitest.dev/) are relevant because the benchmark sits inside the same Vite-native test environment as the rest of the frontend suite.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant as a contrast: request-builder micro-bench results should not be confused with user-visible browser timing.
- [Tauri: Calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because request-building performance is only one part of the broader frontend-to-runtime path.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the request-builder benchmark explicitly classified as a fixture-driven micro-benchmark.
2. Maintain a small representative fixture set tied to the current non-options request-builder surface.
3. Use the compare script for local trend inspection, not for inflated product-level claims.
4. Escalate meaningful regressions into request-builder implementation review and broader frontend verification only when the benchmark points to a real pattern.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current benchmark fixtures and why they are representative.
- [x] Document the benchmark output format and compare workflow.
- [x] Keep the benchmark non-options only.
- [x] Keep correctness assertions in ordinary tests rather than the benchmark file.
- [x] Review fixture coverage whenever request-builder routing or request shape changes.
- [x] Avoid implying browser or end-user timing from micro-benchmark results.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run bench:request-builder` after request-builder performance-sensitive changes
- compare a candidate run against a stored baseline JSON when regression suspicion exists
- re-run relevant correctness tests if the benchmark changes or fails unexpectedly
- spot-check that benchmark fixtures still exercise the intended non-options request routes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the benchmark as a supplemental frontend performance tool before deciding whether it deserves hard thresholds
- prefer improving fixture representativeness over multiplying benchmark cases
- if a threshold policy is added later, keep it clearly separated from user-facing latency claims
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on verification of the existing benchmark harness rather than a source change. The current tree already had the intended request-builder micro-benchmark contract:
  - `web/src/services/optimizeRequestBuilder.bench.test.ts` uses `tinybench` to measure fixture-driven request construction only
  - the harness writes structured JSON to `target/request-builder-bench.json`
  - `web/scripts/compare-request-builder-bench.mjs` compares the candidate JSON against a stored baseline
  - ordinary correctness assertions remain in the normal Vitest suites instead of being overloaded into the benchmark loop
- Fresh acceptance evidence for this closure:
  - `npm.cmd --prefix web run bench:request-builder`
  - `npm.cmd --prefix web run bench:request-builder:compare -- ../target/request-builder-bench-baseline.json ../target/request-builder-bench.json`
  - `npm.cmd --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`
- The fresh benchmark run passed and rewrote `target/request-builder-bench.json`.
- The compare workflow also ran successfully against `target/request-builder-bench-baseline.json` and produced advisory deltas for all six benchmark cases. Those deltas were large in percentage terms on this run, but this item still has no hard threshold gate and the benchmark remains a supplemental micro-benchmark rather than a release authority surface.
- The current fixture set remains clearly non-options and representative of the active optimization request-builder surface:
  - `double_ma/grid_baseline`
  - `double_ma/composite_trend_kernel_route`
  - `double_ma/composite_trend_hybrid_route`
  - `double_ma/composite_cpu_route_with_support_lookup`
  - `composite_cpu/grid_request`
  - `composite_mo/range_builder_request`
- Normal correctness coverage remained green after the benchmark run:
  - `src/services/optimizeRequestBuilder.test.ts`: `92` tests passed
  - `src/services/optimizeService.test.ts`: `63` tests passed
- No `README.md` or `REPO_MAP.md` update was required because the micro-benchmark contract, fixture set, package scripts, and artifact paths were confirmed rather than changed.

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
