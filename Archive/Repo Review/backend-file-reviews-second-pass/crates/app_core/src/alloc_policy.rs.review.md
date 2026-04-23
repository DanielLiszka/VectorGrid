# Second-Pass Review: `crates/app_core/src/alloc_policy.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/alloc_policy.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/alloc_policy.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as an unsafe, performance-sensitive allocation helper whose soundness depends on strict write-before-read discipline and therefore should not be "cleaned up" casually.

## Stale Check

The live source still uses:

- `MaybeUninit`-backed growth helpers
- caller-obligated full initialization before exposure
- performance-oriented behavior that downstream CPU/GPU callers rely on

The first-pass packet remains current.

## Second-Pass Research Log

1. `MaybeUninit`
   - Source: <https://doc.rust-lang.org/std/mem/union.MaybeUninit.html>
   - Why consulted in second pass:
     - verify the actual UB boundary for partially initialized tails
   - Additional constraint informed:
     - exposing a `Vec<T>` with an uninitialized tail is UB even for plain numeric-looking types

2. `Vec::spare_capacity_mut`
   - Source: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.spare_capacity_mut>
   - Why consulted in second pass:
     - identify the standard-library migration target if this module is ever hardened
   - Additional constraint informed:
     - the stdlib fill-then-`set_len` pattern is the clearest future direction

3. `Vec::set_len`
   - Source: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.set_len>
   - Why consulted in second pass:
     - re-check the exact caller obligations behind the current helpers
   - Additional constraint informed:
     - the current contract remains sharp and easy to misuse

## Delta vs First Pass

### 1. The soundness bar is stricter than the first pass implied

- Why this matters:
  - the problem is not "uninitialized but numeric"; it is fully UB once an uninitialized tail becomes visible through a real `Vec<T>`
- Practical implication:
  - any future hardening should use spare-capacity fill discipline rather than broad zero-fill

## Verification If Changed

- `cargo test -p ta_app_core alloc_policy -- --nocapture`
- `cargo test -p ta_app_core --release alloc_policy -- --nocapture`
- `cargo check -p ta_app_core`
- `cargo test -p ta_app_core double_ma -- --nocapture` if signatures change

## Regression And Feature-Preservation Constraints

- Preserve overwrite-heavy hot-path behavior for current callers
- Do not replace these helpers with blanket zero-fill unless perf gates justify it
- Treat any soundness fix as performance-sensitive shared infrastructure work

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly tightens the soundness reading: the current helpers are not merely "sharp"; exposing an uninitialized tail as a real `Vec<T>` is UB, so any future cleanup needs a deliberate sound allocation pattern rather than cosmetic churn.
