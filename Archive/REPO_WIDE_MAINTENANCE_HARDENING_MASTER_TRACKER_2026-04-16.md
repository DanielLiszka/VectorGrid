# Repo-Wide Maintenance And Hardening Master Tracker

Last updated: 2026-04-16

## Purpose

This document is the authoritative tracker for the next repo-wide maintenance and hardening program.

It covers the next practical improvement tranche after the recent replay, artifact, and Stage 5 backend hardening work:

1. `Clippy And Warning-Debt Reduction`
2. `Stage 5 Canonicalization`
3. `Persisted Artifact Retention Settings`
4. `Artifact Schema Migration And Repair`
5. `Artifact-Family Parsing Unification`
6. `Request-Normalization Consolidation`
7. `Backend Regression Coverage Expansion`
8. `Benchmark Harness And Fixture Hygiene`

The objective is not feature expansion.

The objective is to reduce maintenance cost, make contracts more explicit, keep compatibility behavior intentional, and make future repo-wide changes safer to implement.

## Governing Inputs

Use these before touching code:

- [REPO_MAP.md](./REPO_MAP.md)
- [AGENTS.md](./AGENTS.md)
- [REPO_WIDE_IMPLEMENTATION_MASTER_TRACKER_2026-04-16.md](./REPO_WIDE_IMPLEMENTATION_MASTER_TRACKER_2026-04-16.md)
- [IMPROVEMENT_MASTER_PLAN_2026-04-08.md](./IMPROVEMENT_MASTER_PLAN_2026-04-08.md)
- [IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md](./IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md)

When this document disagrees with current code, trust the code-facing entry points listed in `REPO_MAP.md`.

## Program Charter

This program should:

- preserve all current shipped workflows and compatibility aliases unless a migration phase explicitly says otherwise
- reduce duplicated parsing, routing, and normalization logic
- move compatibility debt into explicit wrappers instead of leaving it scattered across the repo
- make artifact retention and legacy artifact handling operationally explicit
- improve regression protection around artifact replay, migration, and reopen behavior
- keep benchmark harnesses compiling and semantically aligned without making performance claims from noisy runs
- reduce warning noise only when the cleanup is low-risk and does not blur behavior

This program should not:

- remove Stage 4 compatibility fields or commands in the same slice that introduces Stage 5 internal cleanup
- rewrite artifact schemas in-place without a repair or migration boundary
- turn the settings workspace into a broad configuration redesign
- treat clippy cleanup as a license for large mechanical rewrites
- run broad benchmark campaigns before semantic and compatibility work is green
- remove artifact-family aliases or legacy `vectorbt_*` reads without a deliberate compatibility decision

## Current Repo Reality

The following facts are already true and should anchor the implementation order:

- `run_artifacts.rs` is now the strongest source of truth for artifact-family interpretation and retention defaults, but some secondary modules still carry local parsing or compatibility assumptions.
- Stage 5 is now more canonical in backend execution and service-level request building, but the frontend store and panel layers still retain extensive `stage4*` compatibility naming.
- persisted app settings are currently very small and only carry market-data-root state in [src-tauri/src/settings.rs](./src-tauri/src/settings.rs)
- benchmark harnesses now compile again on the touched surfaces, but the bench layer still carries stale fixture duplication and warning noise
- repo-quality gates pass, but clippy warning volume remains high enough to hide more meaningful follow-up regressions

## Recommended Execution Order

1. `Epic A`: Artifact-Family Parsing Unification Follow-Through
2. `Epic B`: Persisted Artifact Retention Settings
3. `Epic C`: Artifact Schema Migration And Repair
4. `Epic D`: Stage 5 Canonicalization
5. `Epic E`: Request-Normalization Consolidation
6. `Epic F`: Backend Regression Coverage Expansion
7. `Track X1`: Clippy And Warning-Debt Reduction
8. `Track X2`: Benchmark Harness And Fixture Hygiene

Rationale:

- artifact ownership and retention policy are now close to a clean boundary, so finishing that work first reduces future drift
- migration and repair helpers should land before any compatibility alias cleanup broadens
- Stage 5 naming cleanup is safer once artifact identity and migration posture are explicit
- request normalization consolidation should inherit the now-clearer artifact and workflow boundaries
- warning cleanup and bench hygiene should follow semantic hardening, not lead it

## Global Execution Rules

- semantics first, warning count second
- do not remove Stage 4 compatibility wrappers, saved-field aliases, or legacy artifact-kind reads in the same slice that changes internal naming
- every migration helper must be reversible or no-op on already-canonical files
- every persistence change must define default behavior when the new settings fields are absent
- every request-normalization cleanup must preserve current UI defaults and current Tauri command contracts
- benchmark harness cleanup must not be presented as a performance improvement unless separate timing evidence is gathered later
- use targeted verification for each slice and the repo quality gate after each logically complete cluster

## Completion Standard For Any Slice

A slice is only complete when all of the following are true:

- current user-facing workflows still function
- compatibility aliases still work where the slice says they should
- new ownership boundaries are explicit in code, not implied by comments only
- targeted tests cover the new boundary or migration behavior
- replay, load, and reopen behavior stay deterministic
- repo-quality gates still pass

## Epic A: Artifact-Family Parsing Unification Follow-Through

### Goal

Finish consolidating artifact-family parsing so one backend-owned source of truth is used consistently across command modules and frontend compatibility helpers.

### Why This Matters

- the repo recently centralized the main table in `run_artifacts.rs`
- local copies still exist in secondary modules and frontend helper code
- future artifact additions will drift again if the remaining copies are left in place

### Priority Surfaces

- [src-tauri/src/run_artifacts.rs](./src-tauri/src/run_artifacts.rs)
- [src-tauri/src/commands_meta.rs](./src-tauri/src/commands_meta.rs)
- [src-tauri/src/commands_options_expression.rs](./src-tauri/src/commands_options_expression.rs)
- [src-tauri/src/commands_artifacts.rs](./src-tauri/src/commands_artifacts.rs)
- [web/src/services/metaService.ts](./web/src/services/metaService.ts)
- [web/src/utils/core/runArtifactReview.ts](./web/src/utils/core/runArtifactReview.ts)

### Required Invariants

- continue accepting legacy `vectorbt_*` kinds where current replay/load behavior already does
- keep canonical write kinds unchanged
- do not change artifact JSON schema in this epic

### Implementation Plan

- [ ] inventory every remaining artifact-kind and filename-prefix parser outside `run_artifacts.rs`
- [ ] decide which helpers should move into `run_artifacts.rs` and which frontend helpers should remain local but derive from one constant table
- [ ] remove duplicate backend kind matchers in secondary command modules where the source of truth already exists
- [ ] normalize batch/campaign/options-expression allowlists so legacy compatibility is intentional and test-covered
- [ ] add targeted tests for every moved or consolidated parser boundary

### Verification

- targeted Rust tests in `commands_meta`, `commands_options_expression`, and `run_artifacts`
- targeted frontend tests for meta/library artifact loading helpers
- `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit Criteria

- no backend command module maintains an unreviewed handwritten artifact-family table when `run_artifacts.rs` can own it
- frontend compatibility helpers are limited, explicit, and covered by tests

## Epic B: Persisted Artifact Retention Settings

### Goal

Move artifact retention policy from env-only overrides to persisted application settings with safe defaults and no behavior change for users who never touch the setting.

### Why This Matters

- retention policy is now centralized internally
- it is not yet operator-visible or persisted
- users cannot intentionally tune retention without env plumbing

### Priority Surfaces

- [src-tauri/src/settings.rs](./src-tauri/src/settings.rs)
- [src-tauri/src/run_artifacts.rs](./src-tauri/src/run_artifacts.rs)
- [src-tauri/src/commands_data.rs](./src-tauri/src/commands_data.rs)
- [src-tauri/src/commands_meta.rs](./src-tauri/src/commands_meta.rs)
- [src-tauri/src/commands_campaign.rs](./src-tauri/src/commands_campaign.rs)
- [src-tauri/src/commands_options_expression.rs](./src-tauri/src/commands_options_expression.rs)
- [web/src/stores/settingsStore.ts](./web/src/stores/settingsStore.ts)
- [web/src/components/SettingsWorkspacePanel.vue](./web/src/components/SettingsWorkspacePanel.vue)
- any existing frontend settings service helpers used for persisted backend settings

### Required Invariants

- absent new settings fields must preserve current defaults
- env overrides may remain as the highest-precedence emergency/operator mechanism, but that precedence must be explicit
- no retention setting may allow invalid zero or negative keep counts

### Implementation Plan

- [ ] extend `AppSettings` with an explicit artifact-retention section for run/meta/stage5/policy-eval/batch/campaign/options families
- [ ] define precedence clearly:
  - persisted setting when present
  - env override when present and intentionally allowed to win
  - current hardcoded default otherwise
- [ ] add validation and normalization for retention values in backend settings load/save paths
- [ ] expose the new settings in the frontend settings workspace with conservative labels and hints
- [ ] add tests for default load, partial settings files, invalid values, and precedence behavior
- [ ] update the repo map once the settings become a stable supported operator control

### Verification

- Rust unit tests for `settings.rs` and retention resolution
- frontend settings-store and settings-panel tests
- `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit Criteria

- retention policy is persisted, validated, documented, and still default-safe when unset

## Epic C: Artifact Schema Migration And Repair

### Goal

Introduce explicit legacy-artifact normalization, migration helpers, and a repair command so future artifact contract changes do not rely on ad hoc reopen logic.

### Why This Matters

- the repo still reopens legacy artifacts through permissive compatibility logic
- compatibility is currently spread across load paths rather than expressed as a migration boundary
- that makes future schema evolution harder to reason about and harder to test

### Priority Surfaces

- [src-tauri/src/run_artifacts.rs](./src-tauri/src/run_artifacts.rs)
- [src-tauri/src/commands_artifacts.rs](./src-tauri/src/commands_artifacts.rs)
- [src-tauri/src/commands_meta.rs](./src-tauri/src/commands_meta.rs)
- [src-tauri/src/commands_campaign.rs](./src-tauri/src/commands_campaign.rs)
- [web/src/services/historyService.ts](./web/src/services/historyService.ts)
- [web/src/services/metaService.ts](./web/src/services/metaService.ts)

### Required Invariants

- no migration should destroy the source artifact without explicit opt-in
- repair helpers must preserve original file content unless the command is explicitly write-back
- canonicalization should be idempotent
- Stage 4 compatibility fields remain readable after repair

### Implementation Plan

- [ ] define the supported legacy artifact shapes worth repairing instead of trying to “fix everything”
- [ ] add pure normalization helpers for run/meta/stage5/policy-eval summary and replay payloads
- [ ] add a backend repair/dry-run command that reports what would change before writing anything
- [ ] support optional write-back into a sibling or explicit target path rather than in-place overwrite by default
- [ ] add tests for:
  - already-canonical artifact is no-op
  - legacy aliases normalize correctly
  - malformed payloads fail with explicit reasons
  - repaired payloads remain loadable through current replay paths

### Verification

- targeted Rust tests in `run_artifacts` and artifact command modules
- targeted frontend tests only if repair responses surface in library tooling
- `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit Criteria

- legacy compatibility behavior is expressed as code-owned normalization and repair logic, not just scattered reopen fallbacks

## Epic D: Stage 5 Canonicalization

### Goal

Complete the safe Stage 5 naming cleanup in the frontend/store/UI layers while preserving boundary compatibility aliases.

### Why This Matters

- backend execution and service-level request building are already more Stage 5 canonical
- `analysisStore.ts` and `MetaWorkspacePanel.vue` still carry extensive `stage4*` compatibility naming
- that naming increases maintenance cost and makes Stage 5 changes harder to reason about

### Priority Surfaces

- [web/src/stores/analysisStore.ts](./web/src/stores/analysisStore.ts)
- [web/src/components/MetaWorkspacePanel.vue](./web/src/components/MetaWorkspacePanel.vue)
- [web/src/services/metaService.ts](./web/src/services/metaService.ts)
- [web/src/services/metaStage4RequestBuilder.ts](./web/src/services/metaStage4RequestBuilder.ts)
- [src-tauri/src/commands_meta.rs](./src-tauri/src/commands_meta.rs)
- [src-tauri/src/run_artifacts.rs](./src-tauri/src/run_artifacts.rs)

### Required Invariants

- keep Tauri command names and response alias fields stable unless a dedicated migration slice says otherwise
- preserve `runStage4()` and related compatibility wrappers until the store and panel migration is complete
- preserve saved-artifact reopen fallback from `stage5_*` to `stage4_*` fields

### Implementation Plan

- [ ] define the precise canonical names to adopt in `analysisStore.ts`, `MetaWorkspacePanel.vue`, and `metaService.ts`
- [ ] separate internal renames from persisted snapshot/storage-key changes; do not mix them casually
- [ ] keep store snapshot hydration tolerant of both old and new keys during the migration window
- [ ] update panel local state and queue-binding names after store internals are stable
- [ ] keep public wrapper exports and command invocations compatible while internal names migrate
- [ ] add tests for:
  - restore from legacy store snapshot
  - current Stage 5 run/load/reveal flows
  - saved artifact replay with legacy and canonical field names

### Verification

- targeted frontend tests for `metaService`, `analysisStore`, and `MetaWorkspacePanel`
- targeted Rust tests only where internal backend alias contracts are adjusted
- `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit Criteria

- Stage 5 is the canonical internal concept in service/store/UI layers
- Stage 4 names are clearly compatibility-only and limited to boundary surfaces

## Epic E: Request-Normalization Consolidation

### Goal

Reduce duplicated request-builder and request-normalization logic across optimize, validate, meta, campaign, execution, and options-expression flows.

### Why This Matters

- the repo has many request builders and normalizers that independently trim, clamp, and canonicalize similar fields
- drift between builders and backend normalizers is a recurring source of replay and contract issues

### Priority Surfaces

- [web/src/services/optimizeRequestBuilder.ts](./web/src/services/optimizeRequestBuilder.ts)
- [web/src/services/validationRequestBuilder.ts](./web/src/services/validationRequestBuilder.ts)
- [web/src/services/metaRequestBuilder.ts](./web/src/services/metaRequestBuilder.ts)
- [web/src/services/metaStage4RequestBuilder.ts](./web/src/services/metaStage4RequestBuilder.ts)
- [web/src/services/metaBatchRequestBuilder.ts](./web/src/services/metaBatchRequestBuilder.ts)
- [web/src/services/optionsExpressionService.ts](./web/src/services/optionsExpressionService.ts)
- [web/src/services/optimizeService.ts](./web/src/services/optimizeService.ts)
- [src-tauri/src/commands_meta.rs](./src-tauri/src/commands_meta.rs)
- [src-tauri/src/commands_execution.rs](./src-tauri/src/commands_execution.rs)
- [src-tauri/src/commands_options_expression.rs](./src-tauri/src/commands_options_expression.rs)

### Required Invariants

- do not collapse strategy-specific rules into generic helpers that hide product constraints
- shared helpers should own only truly common normalization behavior
- frontend builders and backend normalizers must remain explicit enough to debug

### Implementation Plan

- [ ] inventory repeated normalization patterns:
  - path trimming and empty-to-null behavior
  - numeric clamp/default behavior
  - boolean normalization
  - artifact-path and scope payload normalization
- [ ] identify which helpers should become shared utilities versus which should remain strategy- or workflow-local
- [ ] consolidate the smallest safe set first, starting with meta and artifact-related request paths
- [ ] add or update tests to lock current defaults before helper extraction
- [ ] only then consider broader request-builder refactors in optimize/validate/services

### Verification

- targeted frontend unit tests for each touched request builder
- targeted Rust tests for backend normalizers
- `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit Criteria

- duplicated normalization logic is materially reduced without hiding workflow-specific policy

## Epic F: Backend Regression Coverage Expansion

### Goal

Increase automated coverage around artifact replay, migration boundaries, retention behavior, and other non-run workflow surfaces.

### Why This Matters

- artifact and replay logic is safety-critical but easy to regress
- future cleanup work will be safer if the non-run surfaces are protected first

### Priority Surfaces

- [src-tauri/src/run_artifacts.rs](./src-tauri/src/run_artifacts.rs)
- [src-tauri/src/commands_meta.rs](./src-tauri/src/commands_meta.rs)
- [src-tauri/src/commands_campaign.rs](./src-tauri/src/commands_campaign.rs)
- [src-tauri/src/commands_options_expression.rs](./src-tauri/src/commands_options_expression.rs)
- [src-tauri/src/commands_reports.rs](./src-tauri/src/commands_reports.rs)

### Implementation Plan

- [ ] add focused tests for artifact retention resolution and precedence
- [ ] add tests for schema-repair or dry-run behavior once Epic C lands
- [ ] add tests for Stage 5 legacy/canonical snapshot compatibility once Epic D lands
- [ ] add tests for command rejection paths around malformed or cross-family artifact reuse
- [ ] add tests for any shared request-normalization utilities introduced by Epic E

### Verification

- `cargo test -p ta_desktop_demo_app --lib`
- targeted frontend tests when store/service replay behavior changes
- `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit Criteria

- the new ownership and migration boundaries are guarded by tests, not just comments

## Track X1: Clippy And Warning-Debt Reduction

### Goal

Reduce warning volume in the repo-quality gate so meaningful follow-up regressions are easier to see.

### Why This Matters

- the current clippy output is noisy across `src-tauri`, `app_core`, `strategies`, and `optimizer`
- the gate passes, but warning density is high enough to hide actionable new warnings

### Required Invariants

- do not do large mechanical rewrites just to silence stylistic warnings
- prioritize low-risk fixes with no semantic impact
- avoid hot-path behavior changes unless separately justified and verified

### Recommended Scope Tiers

- `Tier 1`: obvious no-behavior-change fixes
  - `manual_strip`
  - `get_first`
  - `unnecessary_map_or`
  - `manual_unwrap_or_default`
  - `reserve_after_initialization`
  - simple `single_match` and `needless_bool`
- `Tier 2`: moderate cleanup where the rewritten code is still obviously equivalent
  - `manual_clamp`
  - `manual_div_ceil`
  - `manual_slice_fill`
  - `field_reassign_with_default`
  - trivial `filter_map_bool_then`
- `Tier 3`: postpone unless directly useful
  - `too_many_arguments`
  - large `needless_range_loop` rewrites in hot loops
  - public API reshaping like `len_without_is_empty`

### Implementation Plan

- [ ] capture a fresh grouped warning inventory by crate and warning class
- [ ] execute Tier 1 low-risk cleanup first
- [ ] rerun the quality gate and record the reduced warning set
- [ ] only then decide whether any Tier 2 cleanup is worth the churn
- [ ] leave Tier 3 warnings alone unless they naturally fall out of other work

### Verification

- `powershell -File scripts/run_repo_quality_gate.ps1`

### Exit Criteria

- gate warning volume is materially lower and the remaining warnings are mostly high-churn or intentionally deferred classes

## Track X2: Benchmark Harness And Fixture Hygiene

### Goal

Keep benchmark harnesses compiling, semantically current, and easier to maintain without making unsupported performance claims.

### Why This Matters

- request struct drift breaks benches repeatedly
- fixture setup is duplicated across many benchmark files
- warning noise in benches makes future benchmark-authority work harder

### Priority Surfaces

- [crates/app_core/benches](./crates/app_core/benches)
- any benchmark support files already serving multiple benches

### Required Invariants

- no benchmark timing claims should be made from this cleanup alone
- benchmark inputs must preserve the current semantic intent of each harness

### Implementation Plan

- [ ] inventory repeated benchmark fixture builders and request-default stubs
- [ ] move only the most obviously shared fixture pieces into support helpers
- [ ] standardize current request-default population for drifting structs
- [ ] remove low-value bench warnings only when the cleanup is trivial
- [ ] defer benchmark execution campaigns until semantic work across the repo is stable

### Verification

- `cargo check -p ta_app_core --benches --bins`
- any targeted bench-specific tests already present

### Exit Criteria

- benchmark harnesses compile reliably after request-struct evolution
- shared fixture patterns are explicit enough to update once instead of many times

## Suggested First Implementation Cluster

The safest next execution cluster is:

1. `Epic B`: Persisted Artifact Retention Settings
2. `Epic C`: Artifact Schema Migration And Repair groundwork
3. `Epic F`: regression tests for those two boundaries

Why this cluster first:

- it builds directly on the artifact work already landed
- it is mostly backend-owned and low-UI-risk
- it reduces future migration risk before any broader naming cleanup

## Suggested Second Implementation Cluster

1. `Epic D`: Stage 5 Canonicalization
2. `Epic E`: meta/request-normalization consolidation on the touched surfaces
3. `Epic F`: frontend and backend regression tests for the new canonical boundary

Why this cluster second:

- it benefits from the clearer artifact and migration posture
- it is easier to execute safely once snapshot and replay contracts are explicit

## Suggested Final Cleanup Cluster

1. `Track X1`: Tier 1 clippy cleanup
2. `Track X2`: bench fixture hygiene

Why this cluster last:

- it should follow semantic hardening, not compete with it
- it is easier to review and verify once the more important compatibility work is done

## Exit Criteria For This Whole Program

This program is complete when:

- artifact-family ownership is centralized and secondary drift is materially reduced
- artifact retention policy is a persisted, validated operator-facing setting
- legacy artifact handling has an explicit migration or repair boundary
- Stage 5 is canonical internally across backend and frontend layers while boundary compatibility remains intact
- duplicated request-normalization logic is reduced in the touched workflows
- non-run artifact and replay paths have stronger regression coverage
- the quality gate still passes and warning noise is lower than today

## Status

- `2026-04-16`: tracker created
- no implementation slices under this tracker have been started yet
