# Strategy Family Params Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to strategy-family parameter coverage so exposed family-specific fields, staged family defaults, request-builder legality, and route-family boundaries stay coherent.

## Scope Boundaries

In scope:

- family-local sections in `StrategyWorkspacePanel.vue`
- family-local defaults and storage in `strategyStore.ts`
- family-local request-builder validation and routing
- staged families already present in the store but not fully surfaced

Out of scope:

- shared cost and risk controls
- backend algorithm redesigns
- ML Signal strategy parameters

## Current User-Facing Behavior

The strategy panel already exposes family-local controls for several strategy groups, including ASR, VCC, PIT, VFCT, ROCB, VCET, and VEMR. Those sections are visible only when their corresponding strategy is selected.

The store already carries additional staged family defaults for `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, and `Cdmc`, even though those families are not fully exposed in the panel. Request-building and backend routing are already family-specific, so staged families already affect product truth even when the UI surface is partial.

Route families differ. PIT, VFCT, and composite-profile cases follow the Double-MA route family, while families like VCC, VEMR, ROCB, and VCET ride the Composite CPU product request surface with optional unified-GPU backend. That route split is fundamental to how family parameters should be documented.

## Current Code Surface

- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/stores/strategyStore.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_profile_route.rs`

## Current Verification Surface

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/components/StrategyWorkspacePanel.test.ts`

## Strengths Already Present

- family-local parameter sections already exist in the product
- strategy defaults are already centralized in the strategy store
- request-builder logic already enforces family-specific routing and payloads
- staged families are at least represented in store defaults rather than hidden entirely
- route-family differences are already encoded in the repo

## Weaknesses / Failure Modes

- exposed family sections and staged store-backed families can drift apart
- family-local UI coverage is incomplete relative to the store and backend
- contributors can confuse route-family membership if there is no dedicated family matrix
- adding a family field without synchronized request-builder validation is easy in a large store
- current docs do not clearly separate “supported and surfaced” from “staged and partially surfaced”

## Regressions To Avoid

- mixing route families in the UI or request builder
- dropping staged family defaults accidentally
- adding visible family fields without validation or route support
- letting panel wording imply broader support than the product actually has
- breaking existing family defaults or stored snapshot behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because these family sections are high-density conditional form surfaces.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because family defaults and state ownership should stay centralized in the strategy store.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because family-specific defaults, visibility, and request shaping need repeatable store-aware tests.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because family-local fields become backend command payloads and must remain route-consistent.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Produce one family matrix that distinguishes surfaced families, staged families, and route-family membership.
2. Freeze the ownership of defaults in `strategyStore.ts`.
3. Document which family fields are already productized and which are intentionally staged.
4. Add explicit UI-to-request parity checks for each surfaced family section.
5. Keep family route buckets visible in the implementation plan so contributors do not accidentally mix them.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every currently surfaced family section and its fields.
- [x] Document every staged family already present in the store.
- [x] Document route-family membership for each strategy family.
- [x] Add or tighten UI-to-request parity tests for surfaced family fields.
- [x] Record a policy for adding future family fields without regressions.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/StrategyWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/stores/optimizeStore.actions.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current defaults for surfaced and staged families unless intentionally changed
- prefer explicit “staged” labeling over ambiguous partial exposure
- keep route-family truth authoritative if UI coverage lags behind store support
- document missing panel coverage rather than pretending it does not exist
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Tightened the family-surface evidence at the panel layer rather than widening family support. `StrategyWorkspacePanel.test.ts` now proves the strategy panel renders the selected surfaced family section and hides unrelated family-local controls, using representative `Vcc` and `Vcet` selections as guardrails against family-section bleed.
- Revalidated the existing request and store coverage instead of duplicating family-specific builder logic in the panel tests. `optimizeRequestBuilder.test.ts` continues to cover family-local request shaping across surfaced families and staged route families, while `optimizeStore.actions.test.ts` already freezes the staged family option inventory (`Kqsr`, `Carm`, `Kvdf`, `Cdmc`, `DynamicDoubleMa`) so store-backed partial support cannot silently disappear.
- Kept the plan honest about current scope: surfaced family sections remain productized in the strategy panel, staged families remain present in store and request-builder routing, and route-family truth continues to live in the request builder plus backend route modules until broader UI surfacing is intentionally added later.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/StrategyWorkspacePanel.test.ts src/stores/optimizeStore.actions.test.ts src/services/optimizeRequestBuilder.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/07_strategy_costs_risk_2026-04-09.md`
- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/07_strats/04_strat_breakout_momentum_2026-04-08.md`
