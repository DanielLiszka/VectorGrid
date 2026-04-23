# Second-Pass Review: `crates/app_core/src/double_ma_strategy.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_strategy.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_strategy.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this strategy-config layer as semantics-sensitive because accessor normalization can hide invalid raw values before validation sees them.

## Stale Check

The live source still:

- normalizes through accessors before some validation sees the raw values
- carries profile and family compatibility rules
- acts as a shared config surface across optimization and validation routes

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground argument-validation expectations
   - Additional constraint informed:
     - raw rejection still belongs ahead of accessor-based sanitization if the contract is tightened

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - verify how much of the current normalization should be treated as public behavior
   - Additional constraint informed:
     - silent normalization remains real runtime behavior that multiple callers rely on

3. Rust Book test organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why consulted in second pass:
     - confirm where raw-invalid coverage should live
   - Additional constraint informed:
     - invariant tests should stay colocated with this owner

## Delta vs First Pass

### 1. The accessor-normalization issue is broader than period counts

- Why this matters:
  - float and threshold accessors also clamp invalid raw values into safe defaults before validation sees them
- Practical implication:
  - any fix remains semantics-sensitive across multiple call sites

### 2. `strategy_cfg_for_optimization` still looks like the right seam to keep

- Practical implication:
  - preserve the seam, but move raw invalid-value rejection earlier if the contract is ever tightened

## Verification If Changed

- `cargo test -p ta_app_core validate_strategy_cfg -- --nocapture`
- add raw-zero/raw-invalid tests across multiple strategy families

## Regression And Feature-Preservation Constraints

- Preserve same-bar execution rejection and the current profile incompatibility matrix
- Preserve current runtime accessor defaults unless the boundary contract is deliberately tightened
- Update all reused callers and tests together if raw validation is fixed

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly broadens the concern: accessor-driven normalization still hides more invalid raw input than period counts alone.
