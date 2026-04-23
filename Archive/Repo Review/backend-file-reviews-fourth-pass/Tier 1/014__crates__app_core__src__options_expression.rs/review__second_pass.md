# Second-Pass Review: `crates/app_core/src/options_expression.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_expression.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_expression.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a strong but contract-heavy engine whose blocking entry points, cache policy, and fallback behavior remain semantically dense.

## Stale Check

The live source still owns:

- process-wide structural and summary caches
- split assignment, ranking, exactification, and inspection
- GPU fallback and feasibility reporting glued to the same blocking engine

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `Mutex`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why consulted in second pass:
     - verify the poisoning contract around the global prepare caches and override state
   - Additional constraint informed:
     - poisoning is advisory and recoverable, but the current code still panics on poisoned locks via `expect(...)`

2. Rust `BTreeMap`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeMap.html>
   - Why consulted in second pass:
     - clarify the actual eviction behavior in `evict_if_needed`
   - Additional constraint informed:
     - removing `cache.keys().next()` evicts the lexicographically smallest key, not the oldest or least-used entry

## Delta vs First Pass

### 1. Prepare-cache eviction is still key-order eviction, not recency-aware eviction

- Evidence:
  - the cache store is a `BTreeMap`, and eviction removes the first key in sorted order
- Why this matters:
  - the current capacity policy is simpler and less intuitive than "oldest" or "LRU"
- Practical implication:
  - preserve it unless there is measured reason to change it, but document what it really is

### 2. Cache and override locks still turn poison into panic even though the standard mutex contract supports recovery

- Evidence:
  - the code still uses `expect(\"... poisoned\")` around those global mutexes
- Why this matters:
  - a single panic during cache build can make future prepares brittle instead of returning a structured error
- Practical implication:
  - this remains a good bounded hardening target without redesigning the whole engine

## Verification If Changed

- `cargo test -p ta_app_core options_expression::tests:: -- --nocapture`
- rerun Tauri-side replay and provenance tests if cache or fallback behavior changes
- rerun CUDA options-expression tests if cache/GPU interaction changes

## Regression And Feature-Preservation Constraints

- Preserve current tuple preparation, ranking, and exactification semantics
- Preserve current artifact and replay compatibility
- Do not alter cache policy or poison handling without explicit coverage for warm-cache and fallback behavior

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that prepare-cache eviction is lexicographic rather than recency-aware, and that poison in the global cache/override locks still escalates to panic instead of recoverable error handling.
