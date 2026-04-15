# Review Packet: `crates/app_core/src/validation_shared.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_shared.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Important config/result contract module, but not fully review-ready because many defaults and helper semantics are only implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Serialized-contract`, `Config-defaults`, `Validation-support`, `Semantics-sensitive`

## What This File Owns

`validation_shared.rs` owns shared validation DTOs, config structs, and deterministic-key helpers:

- artifact/admission/cross-ticker configuration and provenance structs:
  - `5-58`, `326-410`
- walk-forward, reopt, SPA, perturb, and bootstrap config/result types:
  - `61-325`
- auxiliary-work mask:
  - `412-446`
- canonical JSON key helpers:
  - `448-476`
- file-local tests:
  - `480-521`

## Classification

- Public surface: yes
  - exported from `crates/app_core/src/lib.rs`
  - used by Tauri validation/campaign command surfaces and backend validation modules
- Hot path: no
- Contract-critical: yes
- Support code: yes
  - mostly DTOs and helpers, but heavily reused

## Direct Interactions

### Backend interaction map

- used widely by:
  - `double_ma_validate.rs`
  - `composite_validate.rs`
  - `src-tauri/src/commands_campaign.rs`
  - `src-tauri/src/commands_validation.rs`
  - `src-tauri/src/validation_common.rs`
- `canonical_json_key` is used to build deterministic candidate/config keys in validation flows

### Downstream verification impact if changed

- changing defaults alters request behavior even when callers omit fields
- changing canonical-key behavior alters candidate identity and dedupe/grouping behavior
- changing `ValidationAuxWorkMask` semantics alters cross-ticker preserved-finalist workloads

## Standards And Research Log

This packet was written after targeted web research focused on defaulted serde fields and documentation expectations for behavior-heavy config DTOs.

### Sources consulted

1. Serde field defaults
   - Source: <https://serde.rs/attr-default.html>
   - Why it was consulted:
     - this file is dense with defaulted config fields
   - Standard informed:
     - omitted-field behavior should be explicitly documented when it changes runtime validation work

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - these structs define an internal/public contract across backend and Tauri surfaces
   - Standard informed:
     - default values, modes, and special helper behavior should be described locally

## Findings

## Config And DTO Contracts

### 1. Many important defaults are encoded only in helper functions, not in the surrounding type docs

- Evidence:
  - walk-forward defaults: `66-76`, `131-146`
  - SPA defaults: `78-92`, `156-164`
  - cross-ticker defaults: `94-126`, `340-381`
- Why this matters:
  - callers crossing the Tauri/backend boundary can omit fields and still get substantial validation behavior
  - today, readers must inspect helper functions to know what “default” means
- Smallest justified fix:
  - add rustdoc to the major config structs describing their effective defaults and intended use

## Helper Semantics

### 2. `canonical_json_key` is a deterministic-key helper, not a full canonical-JSON spec implementation, and that distinction is undocumented

- Evidence:
  - `canonicalize_json_value` sorts object keys recursively: `448-464`
  - tests only lock down map insertion-order stability: `490-509`
- Why this matters:
  - the helper is suitable for stable internal keys, but maintainers should not over-read it as a standards-complete canonicalization layer
  - changing it casually would affect validation candidate identity
- Smallest justified fix:
  - document the helper as an internal deterministic-key utility for serde values, not a general canonical-JSON promise

## Auxiliary Work Controls

### 3. `ValidationAuxWorkMask::preserved_finalists` disables every auxiliary task, but the rationale is hidden

- Evidence:
  - all flags become `false` in `preserved_finalists()`: `429-446`
  - the test `preserved_finalists_mask_disables_auxiliary_work` locks this in: `510-521`
- Why this matters:
  - this is a policy decision about preserved-finalist execution cost and scope
  - without documentation, a maintainer could re-enable pieces opportunistically and change validation cost unexpectedly
- Smallest justified fix:
  - add a short comment explaining why preserved-finalist mode disables all auxiliary work today

## Tests And Missing Verification

### 4. The current tests are small but useful

- Evidence:
  - insertion-order-insensitive canonical keys are tested
  - preserved-finalist aux-work disabling is tested
- Residual gap:
  - no tests or docs enumerate the high-impact defaults on the public config structs
- Smallest justified fix:
  - document the defaults before expanding tests further

## Code-Quality Improvement Register

- Safe now:
  - document effective defaults
  - document canonical-key scope
  - document preserved-finalist aux-work policy
- Defer:
  - broader config consolidation
  - replacing the current deterministic-key helper

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc/comments
- Defer:
  - DTO reshaping
  - cross-module config regrouping
- Do not casually remove:
  - current serde defaults
  - current canonical-key behavior
  - current preserved-finalist aux-work mask

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - re-enabling auxiliary work under preserved-finalist flows

## Recommended Action

`Small surgical refactor`

The main value here is contract clarity. These shared DTOs are already widely reused, so the safe improvement is to document defaults and helper scope rather than redesigning the types.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core validation_shared::tests:: -- --nocapture`
2. Representative validation caller surfaces:
   - `cargo test -p ta_app_core double_ma_validate -- --nocapture`
   - `cargo test -p ta_app_core composite_validate -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core validation_shared::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change config defaults casually; many callers rely on omission-based defaults
- Do not change canonical-key behavior without auditing candidate grouping and dedupe flows
- Do not re-enable preserved-finalist auxiliary work by default without measuring validation-cost impact
