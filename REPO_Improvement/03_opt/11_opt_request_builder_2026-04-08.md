# Optimize Request Builder Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the non-options optimize request builder layer.

## Scope Boundaries

In scope:

- frontend request normalization
- legality/defaulting
- route-sensitive control shaping
- export/path normalization
- strategy-specific input handling

Out of scope:

- backend optimization execution itself
- options workflows

## Current User-Facing Behavior

The optimize UI feeds a large request-normalization layer that converts store-backed form state into strategy-specific optimize requests before the payload crosses the Tauri boundary. It already handles route-sensitive control shaping for the Double-MA family, composite-profile product routes, CompositeCpu and CompositeMo variants, and the constrained `DynamicDoubleMa` strategy-IR preview path. It also decides when `top_k`, `include_all`, `heatmap_bins`, export paths, explicit metric-surface selectors, advanced shortlist controls, and `vdps` knobs are legal together and when they must be rejected early.

## Current Code Surface

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeRequestBuilder.bench.test.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `src-tauri/src/commands_optimize.rs`

## Current Verification Surface

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeRequestBuilder.bench.test.ts`
- `web/src/services/optimizeService.test.ts`

## Strengths Already Present

- the builder already centralizes a large amount of legality and normalization logic
- composite-profile route preview is already wired into the builder layer
- there is already dedicated test coverage and a request-builder benchmark
- the builder already rejects unsupported extra-shortlist objective combinations instead of relying on late backend failures
- the current path distinguishes exact-route-only controls from broader product-facing controls
- `DynamicDoubleMa` already uses an explicit constrained strategy-IR preview rather than silently pretending to be a normal static strategy request

## Weaknesses / Failure Modes

- the builder is large enough that silent drift is a real risk
- input normalization, route shaping, and product-policy overrides are intertwined
- export/path behavior and shortlist controls are easy to break without obvious type errors
- legality rules for `extra_shortlist_objectives`, `shortlist_confidence_mode`, `shortlist_union_mode`, and slice controls are subtle and distributed across several request branches
- metric-surface posture and route-preview posture can drift if capability messaging and actual payload shaping stop matching
- `DynamicDoubleMa` uses a different previewed-builder path, so it can regress independently from the standard optimize request flow
- `vdps` defaults and guardrails are easy to loosen accidentally because the knobs are numerous and mostly numeric

## Regressions To Avoid

- changing request defaults without updating tests and capability messaging
- desynchronizing UI route preview from backend route behavior
- breaking advanced shortlist or export-all behavior
- widening accepted inputs in the UI without backend support
- accepting illegal exact-only shortlist modes on CPU or hybrid routes
- silently changing `heatmap_bins`, `top_k`, or `include_all` semantics for composite or standalone GPU routes
- weakening current early-rejection behavior for malformed export paths or unsupported `DynamicDoubleMa` shapes
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for treating requests as explicit contracts with stable fields and validation semantics.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because this builder ultimately feeds command payloads.
- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html) is relevant because builder inputs are assembled from store-backed UI state.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the builder consumes shared store state and should remain the single normalization boundary instead of duplicating request logic in components.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant for later UX verification when request legality must remain aligned with what the user can actually select in the optimize workspace.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Split the doc into authoritative request sections:
   - base execution context
   - strategy-family parameters
   - backend-route preview and route-sensitive clamps
   - output controls (`top_k`, `include_all`, `heatmap_bins`, export path)
   - metric-surface selectors
   - advanced shortlist controls
   - `vdps` controls
   - `DynamicDoubleMa` strategy-IR preview and handoff
2. For each section, separate three things explicitly:
   - raw user/store inputs
   - product-policy normalization performed in TypeScript
   - backend-required invariants that must remain true when `invoke` happens
3. Make the request-builder plan reference current route-preview authorities so `predictCompositeProfileGpuRoute`, capability summaries, and emitted payloads stay aligned.
4. Lock the exact-route-only shortlist contract in the doc, including the current conditions around `top_k`, `include_all`, score mode, export disablement, and standalone unified-GPU previews.
5. Extend fixture coverage around the most failure-prone combinations instead of relying on generic smoke tests.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the canonical request sections and field ownership.
- [ ] Document which fields are product policy vs backend contract.
- [ ] Document route-preview shaping and capability gating.
- [ ] Document export/all/heatmap/top-k normalization rules.
- [ ] Document DynamicDoubleMa IR handoff rules.
- [ ] Document strategy-family-specific parameter clamps and defaults.
- [ ] Document exact-route-only shortlist objective, confidence, union, and slice rules.
- [ ] Document `vdps` enablement, defaulting, and rejection rules.
- [ ] Document metric-surface selector behavior and current audited-route expectations.
- [ ] Add or tighten edge-case fixtures for invalid or borderline UI inputs.
- [ ] Keep benchmark coverage for hot request-builder paths.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run `npm run bench:request-builder`
- manually inspect route preview for representative DoubleMa and composite cases
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current accepted request fields where possible
- treat builder refactoring as contract-preserving unless explicitly approved otherwise
- if the builder must split into smaller modules later, keep the external request shape unchanged first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/19_eng_dm_backend_routing_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/03_opt/33_advanced_shortlist_diagnostics_2026-04-08.md`
