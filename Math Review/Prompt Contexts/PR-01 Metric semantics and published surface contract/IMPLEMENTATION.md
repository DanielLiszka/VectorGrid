# PR-01 Implementation Guide

Source response: `pr01_metric_surface_all_passes.md`

Decision: `Valid`, but only for additive contract-clarification and publication-hardening work.

## Valid Scope
- [ ] Canonicalize the public metric-surface contract in one normalization boundary.
- [ ] Make `legacy`, `core17`, and `trade22` explicit semantic surface kinds with one authoritative field order each.
- [ ] Treat surface kind and field list as semantic; do not let transport-only widths such as `tile_metrics_stride` redefine artifact meaning.
- [ ] Publish `trade22` only when exact trade-tail support exists; placeholder-zero tails must remain non-`trade22`.

## Non-Goals
- [ ] Do not claim the full published artifact is reducible to a fixed-size scalar-only retained summary.
- [ ] Do not introduce epsilon-style approximation semantics for public `core17` or `trade22`.
- [ ] Do not widen this into optimizer/routing redesign or change product metric meaning outside publication and normalization.

## Additive Guardrails
- [ ] No existing surface kind may disappear.
- [ ] Existing artifact readers must continue to load older artifacts through compatibility normalization.
- [ ] Default behavior must remain backward-compatible unless a path is currently semantically overclaiming exact `trade22`.

## Primary Touchpoints
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/run_artifacts.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/strategies/src/double_ma/mod.rs`

## Implementation Checklist
- [ ] Define one canonical surface enum at the publication boundary with explicit cases for `legacy`, `core17`, and `trade22`.
- [ ] Make the authoritative field order come only from the current `CORE17_FIELDS` and `TRADE22_FIELDS` definitions rather than inferred row width.
- [ ] Normalize artifact backfill on load so older artifacts map into one canonical surface contract without rewriting unrelated data.
- [ ] Add an explicit trade-tail provenance check or metadata bit before emitting `trade22`.
- [ ] Ensure GPU decode paths that currently fill placeholder-zero tails cannot silently satisfy that provenance check.
- [ ] Keep `legacy` semantically empty after normalization rather than partially inferred from transport details.
- [ ] Audit downstream consumers so they branch on semantic surface kind, not row width.

## Verification Checklist
- [ ] Add load-time normalization tests for `legacy`, `core17`, and `trade22`.
- [ ] Add regression coverage proving placeholder-zero tails can never surface as exact `trade22`.
- [ ] Add field-order parity tests between publication, artifact reload, and any downstream reader/renderer.
- [ ] Add compatibility tests for older artifacts that rely on backfill.

## Rollout Checklist
- [ ] Update artifact/report docs to define surface kind as the source of truth.
- [ ] Surface any downgrade from attempted `trade22` to non-`trade22` clearly in diagnostics if it happens.
- [ ] Keep migration load-only first; do not add on-disk rewrite unless separately justified.

## Kill Criteria
- [ ] Reject the patch if any consumer still depends on transport width instead of semantic surface kind.
- [ ] Reject the patch if `trade22` can still be emitted for non-exact-tail paths.

