# Second-Pass Review: `crates/app_core/src/matrix_index.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/matrix_index.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/matrix_index.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a compact but invariant-heavy owner whose public fields and derived deserialization can bypass `new()`.

## Stale Check

The live source still:

- exposes invariant-bearing fields
- derives `Deserialize`
- returns `Option` from lookups rather than loudly failing on impossible states

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde attributes
   - Source: <https://serde.rs/attributes.html>
   - Why consulted in second pass:
     - re-check implications of derived deserialization on invariants
   - Additional constraint informed:
     - invalid states can still bypass `new()` entirely

2. Rust slice docs
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html>
   - Why consulted in second pass:
     - ground indexing and layout assumptions
   - Additional constraint informed:
     - silent bad-state lookups can still become wrong mappings rather than loud corruption failures

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess public invariant-documentation expectations
   - Additional constraint informed:
     - the dominant issue remains invariant leakage, not algorithmic complexity

## Delta vs First Pass

### 1. The combination of public fields, derived deserialization, and `Option` lookups makes bad state especially silent

- Why this matters:
  - invalid deserialized state can degrade to wrong mapping or spurious misses instead of loud corruption
- Practical implication:
  - visibility or deserialization hardening still looks like the best bounded fix

### 2. `MatrixOrder` still looks like public API surface without present extension value

- Practical implication:
  - if visibility is tightened, consider it as part of the same public-surface pass

## Verification If Changed

- `cargo test -p ta_app_core matrix_index::tests:: -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve exact fast/slow matrix ordering semantics
- Avoid serialized-format breaks unless deliberate
- Treat visibility or deserialization tightening as compatibility work

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that invalid state is still too easy to construct and too quiet when consumed.
