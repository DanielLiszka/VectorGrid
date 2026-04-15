# Third-Pass Review: `crates/app_core/src/options_expression.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_expression.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_expression.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_expression.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

The second pass found cache-policy clarity issues, but this file remained high ROI because it sits under every run/analyze/inspect path in the options-expression engine and still uses process-wide caches with panic-on-poison behavior.

## Third-Pass Research Log

1. Rust `Mutex` poisoning
   - Source: <https://doc.rust-lang.org/std/sync/poison/struct.Mutex.html>
   - Why consulted in third pass:
     - re-check whether panic-on-poison is still merely cosmetic or materially operational
   - Third-pass constraint informed:
     - poisoning is advisory, so `expect("... poisoned")` is a deliberate choice to escalate a recoverable state into repeated panics

2. Rust `BTreeMap`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeMap.html>
   - Why consulted in third pass:
     - verify whether lexicographic eviction remains the most important cache finding
   - Third-pass constraint informed:
     - key-order eviction is still real, but operational recovery matters more than eviction policy for the third pass

## What Third Pass Sharpened

### 1. The bigger issue is now cache operability, not eviction semantics

- Evidence:
  - prepare caches still lock with `expect("... poisoned")`
  - these caches are touched by run, analyze, and inspection entrypoints
- Why this matters:
  - one panic during cache population can turn the entire options-expression surface into repeat panics until process restart
- Practical implication:
  - the highest-ROI hardening is poison recovery or cache reset, not redesigning eviction order

### 2. Inspection-by-index remains too coupled to current ranking behavior

- Evidence:
  - the engine re-prepares and re-ranks tuples before inspection
  - selection still hinges on current tuple index rather than stable tuple identity
- Why this matters:
  - historical artifact inspection can fail after benign ranking or exactification changes even when provenance still matches
- Practical implication:
  - the best bounded improvement is inspect-by-spec or inspect-by-stable-hash rather than current-rank lookup

### 3. Lexicographic eviction still deserves documentation, but it is no longer the most valuable third-pass message

- Practical implication:
  - keep the finding documented
  - prioritize poison recovery and stable inspection identity first

## Implementation Priority

- Highest-ROI hardening:
  - recover from cache poison by clearing/rebuilding instead of panicking
- Next bounded improvement:
  - decouple inspection from current tuple rank

## Verification If Changed

- `cargo test -p ta_app_core options_expression::tests:: -- --nocapture`
- rerun Tauri-side options-expression replay and inspection tests if stable tuple identity changes
- rerun CUDA options-expression tests if cache interaction changes

## Regression And Feature-Preservation Constraints

- Preserve tuple preparation, ranking, and exactification semantics
- Keep caches as accelerators only, never sources of truth
- Do not change artifact or replay compatibility while improving inspection identity

## Third-Pass Conclusion

`Bucket unchanged, but the better target is now operability`

The file still fits `Small surgical refactor`, but the third pass shifts emphasis. The most valuable bounded work is not changing lexicographic cache eviction; it is stopping process-wide cache poison from escalating into persistent user-facing failure and making inspection less dependent on current rank order.
