# Second-Pass Review: `crates/app_core/src/strategy_plan.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_plan.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_plan.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this planner as policy-heavy because route defaults, override handling, and plan fingerprints all surface through preview and runtime flows.

## Stale Check

The live source still:

- defaults toward `GpuUnified`
- uses `DefaultHasher`-based IR fingerprints
- silently ignores unknown `route_overrides` keys

The first-pass packet remains current.

## Second-Pass Research Log

1. `DefaultHasher`
   - Source: <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
   - Why consulted in second pass:
     - verify fingerprint stability expectations
   - Additional constraint informed:
     - plan fingerprints remain build/runtime-local cache identity, not stable cross-release hashes

2. Serde default field docs
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - ground the serialized default-field behavior in compiled plans
   - Additional constraint informed:
     - defaulted serialized fields remain part of the contract

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess policy-heavy planner behavior
   - Additional constraint informed:
     - route defaults and override handling need clearer documentation more than structural churn

## Delta vs First Pass

### 1. Silent ignore of unknown `route_overrides` keys remains a real contract choice, not a harmless convenience

- Why this matters:
  - callers can believe an override was applied when it was merely ignored
- Practical implication:
  - documentation or explicit validation remains the best bounded improvement

### 2. `GpuUnified` default routing still depends on capability hints, not live runtime probing

- Practical implication:
  - keep that as an explicit planner-policy rule rather than implying hardware certainty

## Verification If Changed

- `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_ir::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current legacy route defaults and descriptor-driven `GpuUnified` routing unless consumers move together
- Preserve current fingerprint semantics as local cache identity
- Do not tighten override validation without checking preview/runtime compatibility

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that unknown override keys are still silently ignored and that `GpuUnified` remains planner policy, not runtime capability proof.
