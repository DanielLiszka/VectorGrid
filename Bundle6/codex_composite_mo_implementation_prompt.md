# Codex implementation prompt — CompositeMo Bundle 6 validated state

Copy/paste the prompt below into your OpenAI Codex agent.

---

You are working on the repo-grounded Bundle 6 surface from `gpt54_feature_review_map.md`:

- `CompositeMo` search efficiency
- optional GPU prescan behavior
- request shaping
- search telemetry / benchmark fidelity

Use the current repo code as the source of truth.

The historical Bundle 6 plan docs are not authoritative on their own. The repo already contains the main Bundle 6 implementation work, and this prompt is intentionally constrained so Codex does not re-implement already-landed changes.

## Read these first

1. `Bundle6/composite_mo_bundle6_validated_tracking_checklist.md`
   - This is the authoritative Bundle 6 tracker.
   - If the required sections are fully checked, there is no default implementation work to do.

2. `gpt54_feature_review_map.md`
   - Focus only on **Bundle 6: `CompositeMo` search efficiency and GPU prescan**.
   - Respect the repo posture described there: optimization-first, benchmark-first, parity-preserving, no broad product expansion, no speculative redesigns.

3. `Bundle6/composite_mo_bundle6_implementation_plan.md`
   - Historical context only.

4. `Bundle6/composite_mo_bundle6_addendum.md`
   - Historical corrections/context only.

## Current validated repo state

These points are already true in the current repo and must not be re-implemented as if they were missing:

- `CompositeMo` is product-wired and command-wired.
- Sampler support exists for `Nsga2`, `Random`, `Tpe`, and `Qmc`.
- GPU prescan exists.
- Predictive rescue in prescan exists.
- Multiple prescan MA-pair support exists.
- Phase-based throttled progress reporting exists.
- Generic optimizer grid support exists.
- Duplicate param rejection already exists in `SearchSpace::validate()`.
- Public `param_values_hash(...)` already exists.
- `CompositeMo` telemetry already exists for prescan/search details.
- Actual-vs-requested evaluation accounting already exists.
- Exact-grid auto-switch support already exists for bounded eligible spaces.
- Frontend request shaping already rejects unsupported/duplicate custom `CompositeMo` params.
- Frontend categorical numeric/bool search-space choices already serialize as strings.
- Frontend no longer forces prescan `device_id: 0` by default.
- Capability text already describes `CompositeMo` as CPU-side with optional GPU prescan and current validation limits.

## Decision gate

Before making any code change:

1. Read `Bundle6/composite_mo_bundle6_validated_tracking_checklist.md`.
2. Confirm whether any required implementation item is still unchecked.
3. If no required item is unchecked, do not perform Bundle 6 implementation work.
4. Only implement Bundle 6 changes when at least one of these is true:
   - a required tracker item is unchecked, or
   - the user explicitly asks for one of the optional follow-up items.

If the checklist says work is done but the live code clearly contradicts it, treat the live code as authoritative, explain the mismatch, and update the tracker/prompt before implementing anything else.

## Default objective

By default, this prompt is now a validation/scope-guard prompt, not an implementation prompt.

If the required Bundle 6 tracker is fully checked:

- do not re-implement historical plan items
- do not “clean up” Bundle 6 just because the old plan suggested it
- do not reopen already-landed work
- return a concise validation summary instead of a code patch

## Optional follow-up only if explicitly requested

These are valid but out of scope unless the user explicitly asks for them:

- backward-compatible sampler stop/cancel support that avoids a broad optimizer API redesign
- `PeriodIndex` or similar bank row-lookup micro-optimization, but only if profiling shows it is still worthwhile

If one of these follow-ups is explicitly requested, keep the patch narrow, update the validated tracker first, and add only the directly relevant tests/benchmarks.

## Primary files to inspect if validation or follow-up work is needed

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/benches/composite_mo_opt.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`

## Scope guard

Do not do any of the following unless the user explicitly widens scope:

- re-add existing sampler support
- re-add existing prescan features
- re-add predictive rescue or multi-MA-pair support
- re-add progress phases/throttling
- redesign the optimizer API
- add broad new UI surfaces or knobs
- introduce a new typed optimizer-domain redesign
- add a prescanned pair-index redesign
- add a broad new perf-gate system

## Authoritative checklist for this prompt

### Required Bundle 6 implementation state

- [x] `Bundle6/composite_mo_bundle6_validated_tracking_checklist.md` is the authoritative tracker.
- [x] The required Bundle 6 implementation items are already marked complete in that tracker.
- [x] The current repo spot-checks match that completed state.
- [x] There is no default Bundle 6 implementation scope remaining in this prompt.

### Allowed work in this prompt

- [x] Validate tracker state against current code when needed.
- [x] Report mismatches if current code and tracker diverge.
- [ ] Implement backward-compatible sampler stop/cancel support only if explicitly requested.
- [ ] Implement `PeriodIndex` or similar bank row-lookup micro-optimization only if explicitly requested and profiling-backed.

## Expected behavior from Codex

If the user invokes this prompt without explicitly requesting an optional follow-up:

- read the validated tracker
- verify the relevant code surface
- confirm that no required Bundle 6 implementation work remains
- stop without making code changes

If the user explicitly requests an optional follow-up:

- restate the bounded scope
- update the validated tracker to move that follow-up into active scope
- implement only that follow-up
- add only the tests/benchmarks needed for that follow-up

## Historical docs policy

`Bundle6/composite_mo_bundle6_implementation_plan.md` and `Bundle6/composite_mo_bundle6_addendum.md` remain useful as rationale/reference documents, but they are not implementation checklists anymore.

If a section in those docs conflicts with:

- the current repo code, or
- `Bundle6/composite_mo_bundle6_validated_tracking_checklist.md`

then the tracker and current code win.
