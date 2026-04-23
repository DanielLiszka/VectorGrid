# PR-04 Implementation Guide

Source response: `pr04_dynamic_gpu_three_pass_answers.md`

Decision: `Valid`, but only for layered identity, reuse-eligibility, and cache-lifecycle clarification.

## Valid Scope
- [ ] Separate identity and reuse eligibility into layers: IR, compiled plan, executable, prepared-input resident state, and uploaded batch.
- [ ] Require semantic dataset identity or revisioning for exact prepared-CUDA reuse claims.
- [ ] Include `ma_source` series identity in uploaded-batch exactness, not just close series and timestamps.
- [ ] Distinguish alias-free reuse eligibility from actual cache hits because runtime topology is thread-local and resident-state dependent.

## Non-Goals
- [ ] Do not claim current fingerprint strings or pointer identities are exact semantic keys.
- [ ] Do not claim 12-decimal normalized params are exact unless that precision becomes an explicit semantic contract.
- [ ] Do not redesign adjacent routing/search policy surfaces under this guide.

## Additive Guardrails
- [ ] Current fast-path caches must keep working unless they overclaim exactness.
- [ ] Any stronger identity must be additive and opt-in at first where practical.
- [ ] Existing fallback paths must remain supported.

## Primary Touchpoints
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/double_ma.rs`

## Implementation Checklist
- [ ] Enumerate each identity layer and its exact key contents.
- [ ] Define IR identity in terms of normalized structural IR only.
- [ ] Define compiled-plan identity as IR plus route-envelope decisions that affect executable meaning.
- [ ] Define executable identity as compiled plan plus device/runtime-specific execution context.
- [ ] Define prepared-input identity as dataset revision/content identity plus required source-family identity and warmup-relevant shape.
- [ ] Define uploaded-batch identity as prepared input identity plus executable identity plus batch geometry and `ma_source` family.
- [ ] Keep cache-hit telemetry separate from semantic eligibility, since history and residency affect actual hits.
- [ ] Document invalidation rules and resident replacement behavior per layer.

## Verification Checklist
- [ ] Add collision tests for same close series with different `ma_source` inputs.
- [ ] Add tests proving route overrides can change executable identity without changing IR shape.
- [ ] Add resident-state tests proving semantic equality does not imply an actual cache hit.
- [ ] Add key-parity tests for each enumerated identity layer.

## Rollout Checklist
- [ ] Start by hardening docs and telemetry before changing eviction/reuse defaults.
- [ ] Promote stronger identity to correctness-sensitive reuse paths first.
- [ ] Keep sampled fingerprints or lightweight identifiers labeled as heuristic if retained.

## Kill Criteria
- [ ] Reject any patch that still uses endpoint-only or close-only signatures for exact uploaded-batch claims.
- [ ] Reject any observability path that conflates eligibility with actual cache-hit behavior.

