# Second-Pass Review: `crates/app_core/src/strategy_ir.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_ir.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_ir.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as schema-sensitive because omission defaults, execution policy validation depth, and constant-bit transport all affect the dynamic strategy boundary.

## Stale Check

The live source still:

- uses serde defaults on several schema and execution fields
- accepts both numeric and string JSON forms for `ConstantBits`
- defers some profile/execution invariants to later runtime preparation

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde default field docs
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - verify omitted-field semantics
   - Additional constraint informed:
     - omission remains part of the live schema contract, not just builder convenience

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground cross-boundary schema documentation
   - Additional constraint informed:
     - the validator remains intentionally shallower than its name can suggest

3. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in second pass:
     - reassess early-validation expectations
   - Additional constraint informed:
     - some invariants are still intentionally deferred

## Delta vs First Pass

### 1. `ExecutionPolicy` remains more shape-validated than meaning-validated at IR time

- Why this matters:
  - some profile and execution invariants are still left for later stages
- Practical implication:
  - preserve this consciously unless runtime prep and preview layers move together

### 2. `ConstantBits` still trades stricter schema simplicity for precision-preserving compatibility

- Practical implication:
  - keep that compatibility surface explicit rather than trying to "simplify" it into raw JS numbers

## Verification If Changed

- `cargo test -p ta_app_core strategy_ir::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`
- Tauri dynamic preview command tests

## Regression And Feature-Preservation Constraints

- Preserve schema v1 compatibility and bit-precise constant encoding
- Preserve current omitted-field defaults unless the frontend/Tauri builders move together
- Do not tighten IR-time validation casually if runtime prep still owns the deeper invariants

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that this validator intentionally preserves a broader compatibility surface than its name suggests, especially around omitted defaults and `ConstantBits`.
