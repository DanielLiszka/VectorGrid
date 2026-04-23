# PR-08 Implementation Guide

Source response: `pr08_all_passes.md`

Decision: `Valid`, but only for exact coherence/translation semantics plus bounded heuristic economics.

## Valid Scope
- [ ] Treat `PR-08` as a two-layer contract:
  - [ ] exact semantics for cache coherence, descendant invalidation, observable refresh admissibility, quantized live execution, bracket representability, and latest-only runner control
  - [ ] bounded heuristic economics for retention ranking and prefetch admission
- [ ] Add a stronger disk-cache identity only if content-correct reuse is required.
- [ ] Gate Polygon `FullHit` and `AppendOnly` on current clean integrity when exactness matters.
- [ ] Keep retention ranking heuristic and tree-aware.
- [ ] Make prefetch admission measurement-driven.

## Non-Goals
- [ ] Do not claim current eviction is LRU or globally optimal.
- [ ] Do not claim `(size,mtime)` is content-correct cache identity.
- [ ] Do not claim current live execution is endogenous portfolio sizing.
- [ ] Do not redesign route selection under this guide.

## Additive Guardrails
- [ ] Existing cache and execution paths must keep working unless they currently overclaim exactness.
- [ ] Any stronger identity or integrity gate must be additive and correctness-driven.
- [ ] Any heuristic instrumentation must remain observational unless and until separately promoted.

## Primary Touchpoints
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`

## Implementation Checklist
- [ ] Preserve and document the exact semantics already tracked by the repo: base reuse identity, derived reuse by version coherence, descendant invalidation, quantized delta-to-target execution, and latest-only runner behavior.
- [ ] If content-correct cache reuse is needed, add explicit content identity to cache metadata instead of overclaiming the current `(size,mtime)` rule.
- [ ] If exact Polygon hit/append semantics are needed, gate them on integrity-clean state in addition to range coverage.
- [ ] If retention ranking is upgraded, log subtree size, rebuild cost, reuse outcomes, and sharing state, and keep the ranking explicitly heuristic.
- [ ] If prefetch is upgraded, drive admission from measured savings and measured failure risk, not rules of thumb.
- [ ] Keep all observability and docs explicit about exact semantics versus heuristic economics.

## Verification Checklist
- [ ] Add same-size/same-mtime/different-content cache falsifier tests.
- [ ] Add dirty-integrity Polygon tests where outer coverage alone would otherwise admit a hit/append path.
- [ ] Add execution tests around `1e-4` quantity normalization and `1e-6` hold/bracket thresholds.
- [ ] Add three-bar inflight runner tests to prove latest-only coalescing remains explicit.
- [ ] Add instrumentation tests or harness assertions for heuristic retention/prefetch metrics.

## Rollout Checklist
- [ ] Start with telemetry/doc hardening before any default-policy changes.
- [ ] Promote stronger identity/integrity only to correctness-sensitive paths first.
- [ ] Keep heuristic economics opt-in or clearly labeled until thresholds are chosen deliberately.

## Kill Criteria
- [ ] Reject any patch that claims content correctness without a real content identity.
- [ ] Reject any heuristic retention/prefetch patch that is presented as optimal.
- [ ] Reject any doc or behavior change that implies full per-bar replay or budget optimization on the current execution surface.

