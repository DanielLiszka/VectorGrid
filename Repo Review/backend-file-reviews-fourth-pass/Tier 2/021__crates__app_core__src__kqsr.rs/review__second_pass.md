# Second-Pass Review: `crates/app_core/src/kqsr.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kqsr.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kqsr.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this strategy file as mostly sound but permissive in a few places where invalid inputs and output selection can fail open.

## Stale Check

The live source still:

- has entrypoints that bypass `validate_kqsr_params`
- builds desired positions with a fail-open flat-output fallback on malformed input
- resolves output IDs permissively

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground predictable helper behavior and fail-open review
   - Additional constraint informed:
     - silent fallback remains the sharpest boundary issue

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess config/default/output-selection docs expectations
   - Additional constraint informed:
     - output-selection permissiveness still deserves explicit documentation or tightening

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - re-check the public request/config surface
   - Additional constraint informed:
     - config permissiveness remains real serialized behavior

## Delta vs First Pass

### 1. `validate_kqsr_params` enforcement remains inconsistent across entrypoints

- Why this matters:
  - the live trade metrics path still bypasses the full local contract gate
- Practical implication:
  - still a bounded cleanup target if the strategy is touched

### 2. `resolve_output_id()` failing open to the first declared output remains too permissive for a semantically specific multi-output strategy

- Practical implication:
  - keep this issue on the same small-refactor list as desired-position fail-open behavior

## Verification If Changed

- `cargo test -p ta_app_core kqsr::tests:: -- --nocapture`
- KQSR optimization smoke
- CUDA parity suites if GPU paths are touched

## Regression And Feature-Preservation Constraints

- Preserve KQSR entry and exit semantics
- Preserve route diagnostics
- Do not silently narrow accepted output-selection behavior without coordinated tests

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly reinforces that inconsistent validation enforcement and permissive output selection are still the sharpest contract edges.
