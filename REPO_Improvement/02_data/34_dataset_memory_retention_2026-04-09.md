# Dataset Memory Retention Plan

Last updated: 2026-04-09

## Purpose

Plan improvements around bounded in-memory dataset residency so retention, eviction, invalidation, and dataset reuse remain predictable for optimization, validation, and ML workflows.

## Scope Boundaries

In scope:

- `src-tauri/src/state.rs` in-memory dataset map and loaded-data context
- retention limits for base and derived datasets
- invalidation and derived-child removal behavior
- workflow consequences for validation and ML-signal commands

Out of scope:

- redesigning disk parse cache
- durable artifact storage
- options-related data residency

## Current User-Facing Behavior

The application keeps loaded datasets in memory to speed repeated work, but that residency is explicitly bounded. Base datasets and derived datasets have separate limits, both defaulting to one active dataset each unless environment overrides are set. Loading a new base can evict an older base and its derived descendants.

Current behavior is tree-aware rather than naive LRU only. Derived datasets are tied to their parent base id and version, so live updates or explicit invalidation remove stale descendants. Re-loading an identical base dataset with the same source identity can reuse the current in-memory base id instead of creating a duplicate.

This matters because downstream workflows depend on live in-memory dataset state, not just path references. Validation and ML-signal command paths call into `get_loaded_data_context()` and `get_candles()`, so eviction or invalidation affects real workflow availability.

## Current Code Surface

- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- retention is already bounded instead of unbounded growth
- base and derived datasets already use separate residency budgets
- eviction already respects parent-child relationships rather than treating all ids as flat peers
- in-memory reuse already consults source identity for identical base loads
- live updates already invalidate derived descendants when the base changes

## Weaknesses / Failure Modes

- the current policy is easy to miss because it lives inside backend state code rather than dedicated documentation
- contributors can confuse bounded in-memory residency with durable cache guarantees
- validation and ML flows depend on retained datasets, so silent policy changes can break higher-level workflows
- eviction-ordering and protect-set behavior are subtle and deserve first-class contract documentation
- the default `1/1` posture is operationally important but easy to change casually without realizing the product impact

## Regressions To Avoid

- evicting protected parents while derived children are still in play
- leaving stale `loaded_data_context` entries behind after eviction
- silently changing the default base/derived retention posture
- preferring eviction of actively shared `Arc`-held datasets before colder entries
- allowing stale derived datasets to survive a base-version change
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Rust `Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) is relevant because eviction heuristics already distinguish entries with extra shared owners from colder entries.
- [Rust `Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html) is relevant because the dataset map and loaded-data context are guarded backend state and must preserve internal consistency during eviction and invalidation.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because residency policy belongs to authoritative backend state rather than ad hoc frontend assumptions.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant indirectly because downstream state and artifact contracts often assume a valid loaded dataset id and should make those dependencies explicit.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document bounded in-memory residency as its own contract, separate from disk caching.
2. Freeze the current default `1/1` base/derived policy in the implementation plan unless deliberately changed with explicit justification.
3. Document protect-set expansion, LRU ordering, and tree-removal behavior together so the policy is understandable.
4. Make the coupling to validation and ML-signal workflows explicit.
5. Add more direct tests for protect-set behavior and eviction ordering if those cases are still only implicitly covered.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current default residency budgets and environment overrides.
- [ ] Document protect-set expansion and derived-tree removal behavior.
- [ ] Document how identical base reloads reuse live dataset ids.
- [ ] Document how live updates invalidate derived descendants.
- [ ] Add or strengthen unit tests for eviction ordering and protected-parent behavior.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app state`
- run `cargo test -p ta_desktop_demo_app commands_data`
- run targeted validation and ML-signal command tests if residency behavior changes
- manually verify that loading, resampling, invalidating, and reloading datasets still behave consistently after retention changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- treat this as backend residency policy, not a UI preference
- prefer additive diagnostics before changing actual eviction behavior
- if retention defaults change, update validation and ML-signal expectations in the same change
- keep in-memory reuse truthfulness aligned with source-identity checks
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/33_data_cache_state_backend_2026-04-09.md`
- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-09.md`
- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/05_validation/05_validation_context_compatibility_2026-04-08.md`
