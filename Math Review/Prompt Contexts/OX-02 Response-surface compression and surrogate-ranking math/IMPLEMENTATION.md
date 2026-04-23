# OX-02 Implementation Guide

Source response: `ox02_all_passes.md`

Decision: `Valid`, but only for guarded proposal-side compression that preserves exact final scoring.

## Valid Scope
- [ ] Keep compression on the proposal side and exact scoring at the end.
- [ ] Add only guarded local-surrogate admission inside the already exact `double_ma` local replay, and only for metric-only score modes.
- [ ] Add shortlist-union telemetry around `build_exact_extra_shortlist_rows`.
- [ ] Add bounded prescan-side recall improvements and telemetry in `CompositeMo`, not a learned scorer between prescan and exact replay.
- [ ] Keep clustered meta output explicitly region-like and carry representatives downstream.

## Non-Goals
- [ ] Do not build a global surrogate optimizer that replaces exact scoring.
- [ ] Do not apply surrogate ranking to pair-context modes such as `EffectiveSampleLcb10`.
- [ ] Do not treat clustered meta groups as exact point winners before holdout.
- [ ] Do not insert a learned model layer between prescan and exact candidate replay.

## Additive Guardrails
- [ ] Exact final scorer selection must remain unchanged.
- [ ] Any local surrogate must stay bounded to already exact local neighborhoods.
- [ ] Existing explicit-candidate search paths must remain preferred when already exact.

## Primary Touchpoints
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/meta_select.rs`

## Implementation Checklist
- [ ] Add a local-surrogate admission helper in `double_ma.rs` that operates only on exact local replay neighborhoods.
- [ ] Gate that helper so it runs only for metric-only score modes and never for pair-observation-context modes.
- [ ] Record residual bounds and elite margins locally rather than promoting a surrogate globally.
- [ ] Add shortlist-union telemetry such as `head_count`, `top_k`, `union_size`, and overlap in exact-extra-shortlist code paths.
- [ ] Add bounded ridge-rescue or recall telemetry in `CompositeMo` around prescan seeds, without changing exact replay ownership.
- [ ] Keep clustered meta outputs explicitly representative/region-like instead of exact point-identifying.

## Verification Checklist
- [ ] Add admission/rejection tests for the local-surrogate gate.
- [ ] Add exactness-preservation tests proving the final scorer outcome is unchanged.
- [ ] Add shortlist-union telemetry tests.
- [ ] Add prescan recall regressions for diagonal/ridge-like candidate families.
- [ ] Add tests proving explicit-candidate search remains exact when available.

## Rollout Checklist
- [ ] Label all surrogate use as local, guarded, and proposal-side only.
- [ ] Keep telemetry additive before any behavior change.
- [ ] Avoid productizing any surrogate path without exact-final-score parity evidence.

## Kill Criteria
- [ ] Reject if a surrogate starts influencing final ranking outside an exact local replay neighborhood.
- [ ] Reject if representative regions are surfaced as exact winners.

