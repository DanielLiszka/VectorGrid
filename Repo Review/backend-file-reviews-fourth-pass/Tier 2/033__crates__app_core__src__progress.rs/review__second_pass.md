# Second-Pass Review: `crates/app_core/src/progress.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/progress.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/progress.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a small but shared contract surface because its payload and sink trait bridge backend loops into Tauri-visible progress semantics.

## Stale Check

The live source still provides:

- an object-safe `ProgressSink` trait
- a shared `DoubleMaProgressPayload`
- best-effort sink semantics with no `Result` path

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines `C-OBJECT`
   - Source: <https://rust-lang.github.io/api-guidelines/flexibility.html#traits-are-object-safe-if-they-may-be-useful-as-a-trait-object-c-object>
   - Why consulted in second pass:
     - verify whether the trait’s tiny, non-generic shape is the right contract
   - Additional constraint informed:
     - the current object-safe design is intentional and appropriate

2. Rust `Sync`
   - Source: <https://doc.rust-lang.org/stable/std/marker/trait.Sync.html>
   - Why consulted in second pass:
     - re-check thread-safety expectations for sink implementations
   - Additional constraint informed:
     - implementor-side synchronization is part of the API contract

3. Rust `Send`
   - Source: <https://doc.rust-lang.org/std/marker/trait.Send.html>
   - Why consulted in second pass:
     - confirm cross-thread sink usage expectations
   - Additional constraint informed:
     - sink implementations remain responsible for safe cross-thread use

4. Tauri frontend event guidance
   - Source: <https://v2.tauri.app/develop/calling-frontend/>
   - Why consulted in second pass:
     - ground downstream event-payload implications
   - Additional constraint informed:
     - `phase` is effectively a public string vocabulary

## Delta vs First Pass

### 1. `phase` is not just a debug string; it is effectively part of the public progress contract

- Why this matters:
  - downstream validation and Tauri consumers key on it directly
- Practical implication:
  - preserve its vocabulary unless the full progress surface moves together

### 2. The trait should stay tiny and non-blocking

- Why this matters:
  - hot loops pass sinks as `&dyn ProgressSink`
- Practical implication:
  - do not broaden sink behavior in ways that can block hot paths

## Verification If Changed

- `cargo test -p ta_app_core validation_core::tests:: -- --nocapture`
- representative Tauri progress coverage through `commands_optimize` and `commands_campaign`

## Regression And Feature-Preservation Constraints

- Keep `DoubleMaProgressPayload` field names stable
- Keep best-effort, no-`Result` sink semantics
- Do not broaden sink behavior in ways that can block hot loops

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that `phase` is active compatibility vocabulary and that the trait’s tiny object-safe shape is the right contract to preserve.
