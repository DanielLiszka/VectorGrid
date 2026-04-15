# Review Packet: `src-tauri/src/data_cache.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/data_cache.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Efficient and broadly useful cache helper, but not yet fully hardened cache infrastructure because freshness validation is metadata-only, timestamp-unit expectations are split across consumers, unsafe invariants are undocumented, and a guarded big-endian recovery branch is currently broken`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Runtime-critical`, `Disk-format-sensitive`, `Unsafe-sensitive`, `Performance-sensitive`

## What This File Owns

`data_cache.rs` owns the backend's reusable binary candle-cache layer for CSV-backed datasets.

This file owns:

- source-file identity metadata:
  - `SourceIdentity`: `src-tauri/src/data_cache.rs:18-22`
  - `identity_for_source`: `36-50`
- cache load result contract:
  - `LoadOutcome`: `24-30`
- app-local cache-root resolution:
  - `app_data_cache_dir`: `32-34`
- reproducibility-oriented content fingerprinting:
  - `content_fingerprint_for_source`: `80-109`
- cache path derivation:
  - sidecar path: `112-114`
  - fallback path: `122-135`
- binary header encoding/decoding and field-mask handling:
  - `138-193`
- low-level slab serialization/deserialization helpers:
  - `195-269`
- cache load/build orchestration:
  - `load_or_build`: `271-274`
  - `load_or_build_with_identity`: `276-322`
- cache reader/writer implementation:
  - `try_load_cache`: `324-364`
  - `write_cache`: `366-423`
- direct unit tests for core cache behavior:
  - `425-617`

This file does not own:

- in-memory dataset retention and reuse policy
  - `src-tauri/src/state.rs`
- timestamp normalization to milliseconds for base datasets
  - `src-tauri/src/state.rs:1523-1532`
- frontend-visible load-source reporting
  - assembled in `src-tauri/src/state.rs:209-279`
- artifact provenance decisions
  - those are higher-level consumers in `commands_campaign.rs`, `commands_options_expression.rs`, and `run_artifacts.rs`

## Classification

- Public surface: yes
  - shared backend support module used across runtime and headless flows
- Hot path: moderate
  - sits on dataset-load boundaries, not on indicator/backtest inner loops
- Runtime-critical: high
  - validation, campaign, options, and base dataset loading all depend on it
- CUDA/runtime-critical: indirect only
  - it feeds CPU/GPU workflows but does not own CUDA logic
- Strategy-local: no
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- base dataset loading and identity reuse:
  - `src-tauri/src/state.rs:51-57`
  - `src-tauri/src/state.rs:209-279`
  - normalization happens only in `state.rs`, not in this cache module:
    - `src-tauri/src/state.rs:221-224`
    - `src-tauri/src/state.rs:1523-1532`
- Tauri load-price-data command:
  - `src-tauri/src/commands_data.rs:20-29`
- cross-ticker validation dataset loads:
  - `src-tauri/src/commands_validation.rs:217-324`
  - cross-ticker alignment compares timestamps numerically:
    - `src-tauri/src/commands_validation.rs:62-91`
- campaign dataset preparation:
  - `src-tauri/src/commands_campaign.rs:961-1025`
- meta holdout evaluation reuses cache layer:
  - `src-tauri/src/commands_meta.rs:2719-2778`
- options-expression replay/workflow uses both identity and content fingerprint:
  - `src-tauri/src/commands_options_expression.rs:1037-1042`
- artifact enrichment backfills size/mtime/fingerprint:
  - `src-tauri/src/run_artifacts.rs:1081-1105`

### Downstream contract anchors

- `LoadPriceDataResponse` load-source selection depends on `LoadOutcome.cache_hit` and `cache_path`:
  - `src-tauri/src/state.rs:268-279`
- campaign dataset metadata threads `SourceIdentity`, but content fingerprint is optional caller input:
  - `src-tauri/src/commands_campaign.rs:1003-1016`
- options-expression dataset replay uses both `SourceIdentity` and `content_fingerprint_for_source` for stronger replay identity:
  - `src-tauri/src/commands_options_expression.rs:1038-1042`
- run-artifacts backfills content fingerprints for persisted metadata:
  - `src-tauri/src/run_artifacts.rs:1086-1105`

## Standards And Research Log

This packet was written after targeted external review focused on Rust unsafe initialization rules, filesystem existence/error behavior, and rename semantics for temp-file cache replacement.

### Sources consulted

1. `MaybeUninit` in `core::mem`
   - Source: <https://doc.rust-lang.org/core/mem/union.MaybeUninit.html>
   - Why it was consulted:
     - this file uses `Vec<MaybeUninit<i64>>` and `Vec<MaybeUninit<f64>>` plus raw-pointer reconstruction
   - Standard informed:
     - uninitialized memory cannot be observed as initialized `T`
     - soundness depends on fully writing bytes before converting to initialized vectors

2. Rustonomicon: working with unsafe
   - Source: <https://doc.rust-lang.org/stable/nomicon/working-with-unsafe.html>
   - Why it was consulted:
     - this module contains several `unsafe` blocks that form a safe abstraction
   - Standard informed:
     - unsafe code should keep its invariants explicit and narrow

3. `Path::exists` / `Path::try_exists`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html#method.try_exists>
   - Why it was consulted:
     - this file uses `exists()` in cache probe and replacement logic
   - Standard informed:
     - `exists()` coerces errors to `false`
     - `try_exists()` is preferable when callers should distinguish unreadable from absent paths

4. `std::fs::rename`
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why it was consulted:
     - cache writes use temp-file + rename replacement
   - Standard informed:
     - rename replacement behavior is platform-sensitive
     - same-filesystem assumptions matter

5. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this is shared runtime infrastructure with public helper contracts
   - Standard informed:
     - support modules still need explicit contracts, clear invariants, and representative tests

## Findings

## Correctness And Safety

### 1. Cache freshness is validated only by source size and source mtime, even though stronger content fingerprinting exists elsewhere in the repo

- Evidence:
  - `identity_for_source` captures only `size` and `mtime_utc_ms`: `36-50`
  - the cache header stores only size and mtime as source-identity fields used by the reader: `160-177`, `180-192`
  - `try_load_cache` accepts a cache hit when only size and mtime match: `342-344`
  - stronger content fingerprinting exists in this same module: `80-109`
  - higher-level consumers already use content fingerprints for provenance/replay identity:
    - `src-tauri/src/commands_options_expression.rs:1038-1042`
    - `src-tauri/src/run_artifacts.rs:1101-1105`
    - `src-tauri/src/commands_campaign.rs:1007-1016`
- Why it matters:
  - stale-cache false positives are possible if file content changes while size and visible mtime do not
  - this is an inference from the current code contract, not a claim that the bug is happening today
- Review judgment:
  - this is the highest-value correctness concern in the file
- Smallest justified fix:
  - safe-now: document clearly that cache freshness is metadata-based only
  - defer-later: if stronger freshness guarantees are needed, version the cache format and evaluate whether to add fingerprint-backed validation

### 2. The `unsafe` bulk I/O helpers rely on real soundness invariants, but those invariants are not documented next to the code

- Evidence:
  - raw byte-slice casts for writes: `195-223`
  - `Vec<MaybeUninit<T>>` + `set_len` + raw-part reconstruction for reads: `226-269`
- Why it matters:
  - this code is only sound because:
    - `i64` and `f64` can be reconstructed from fully initialized raw bytes
    - `read_exact` fills the entire buffer before `Vec::from_raw_parts`
    - no partially initialized values are ever exposed as initialized `T`
  - future edits can easily invalidate one of these assumptions if the reasoning is not written down
- Smallest justified fix:
  - add explicit safety comments explaining the invariants for each unsafe block

### 3. Timestamp normalization is caller-side and the contract is currently inconsistent across consumers

- Evidence:
  - `data_cache` returns raw `Candles` from cache/CSV without timestamp normalization: `271-322`, `324-364`
  - base dataset loading normalizes timestamps to milliseconds after cache load:
    - `src-tauri/src/state.rs:221-224`
    - `src-tauri/src/state.rs:1523-1532`
  - cross-ticker validation loads directly from `data_cache` and aligns timestamps numerically without local normalization:
    - `src-tauri/src/commands_validation.rs:284-285`
    - `src-tauri/src/commands_validation.rs:62-91`
- Why it matters:
  - a seconds-based cross dataset against a milliseconds-based base dataset can be aligned incorrectly
  - this is a cross-file contract risk at the cache boundary
- Smallest justified fix:
  - document the raw-timestamp contract explicitly and add an integration test
  - longer-term, decide whether timestamp normalization belongs in `data_cache` or in every caller

### 4. The big-endian recovery branch in the read helpers is currently broken

- Evidence:
  - `read_le_vec_i64` constructs `out` immutably and then tries to iterate `&mut out` under `#[cfg(target_endian = "big")]`: `237-244`
  - `read_le_vec_f64` does the same: `259-266`
- Why it matters:
  - this code path will not compile on big-endian targets
  - even if big-endian support is not a current product target, this is still an objective portability defect in the file
- Smallest justified fix:
  - fix the local mutability issue and keep the big-endian branches covered by at least compile-level verification

## API And Naming

### 1. `LoadOutcome.cache_path` carries two meanings depending on hit vs miss

- Evidence:
  - on hit, it is the cache file read from: `284-300`
  - on miss, it is the cache file successfully written for future reuse: `307-313`
  - `state.rs` has to combine `cache_hit`, `cache_path`, and a derived `LoadSource` enum to interpret the result: `src-tauri/src/state.rs:268-279`
- Why it matters:
  - the field is useful, but its semantics are overloaded
- Review judgment:
  - low severity; a doc comment is probably enough

## Error Handling

### 1. Cache read/write failures are intentionally downgraded to fallback behavior without diagnostics

- Evidence:
  - read path ignores all `Err` values and just keeps searching/parsing CSV: `284-301`
  - write path ignores `write_cache` failures and only sets `cache_path` when successful: `307-313`
  - direct tests intentionally validate the "corrupt cache is ignored and source is reparsed" policy: `601-616`
- Why it matters:
  - this keeps dataset loading resilient
  - it also hides permission errors, short reads, temp-file rename failures, or repeated write failures
- Smallest justified fix:
  - preserve fallback behavior, but log or otherwise expose cache read/write failures at debug/warn level

### 2. `exists()` is used in a place where the standard library explicitly recommends more explicit handling

- Evidence:
  - `try_load_cache` checks `if !cache_path.exists()`: `324-327`
  - `write_cache` checks `if cache_path.exists()` before removal: `418-420`
  - standard docs note that `exists()` coerces errors to `false` and suggest `try_exists()` when error distinction matters
- Why it matters:
  - unreadable or permission-denied paths are treated the same as absent paths
  - this can mask real filesystem problems as cache misses
- Smallest justified fix:
  - use `try_exists()` where the code cares about distinguishing "missing" from "unreadable"

## Module Boundaries And Technical Debt

### 1. The on-disk header writes more fields than the loader currently consumes

- Evidence:
  - `write_header` writes `min_ts` and `max_ts`: `166-177`
  - `read_header` reads only `fields_mask`, `len`, `csv_size`, and `csv_mtime_ms`: `180-192`
- Why it matters:
  - the binary format surface is larger than the active validation logic
  - future reviewers must infer whether the extra header fields are reserved, legacy, or accidentally unused
- Smallest justified fix:
  - document that the extra bytes are reserved/not yet validated, or start consuming them if they are supposed to matter

### 2. This module is part of multiple higher-level provenance contracts, but those contracts are split across files

- Evidence:
  - metadata-only freshness here
  - content-fingerprint provenance in `commands_options_expression.rs`, `commands_campaign.rs`, and `run_artifacts.rs`
- Why it matters:
  - the repo already distinguishes between "fast source identity" and "stronger provenance identity", but the distinction is implicit
- Review judgment:
  - acceptable today, but worth documenting before further cache/provenance changes

## Docs And Examples

### 1. The cache validity model needs explicit documentation

- Missing documentation topics:
  - sidecar-first, fallback-second lookup order
  - metadata-only freshness check
  - corrupt/unreadable cache fallback policy
  - meaning of `cache_path` on hit vs miss

### 2. The unsafe helpers need local safety comments

- Missing documentation topics:
  - why the raw casts are sound
  - why `MaybeUninit` is used
  - why this optimization should stay constrained to POD-like numeric slabs

## Tests And Missing Verification

### 1. Direct cache coverage is decent but still narrow for a disk-format module

- Direct Rust coverage currently passing:
  - `cargo test -p ta_desktop_demo_app data_cache -- --nocapture`
    - passed `7` tests
- Covered directly:
  - same-content fingerprint stability: `477-490`
  - changed-content fingerprint mismatch: `492-507`
  - source-identity equality: `509-526`
  - default loader vs explicit-identity parity: `528-548`
  - fallback-cache load: `550-568`
  - sidecar precedence over fallback: `570-599`
  - corrupt-cache reparsing: `601-616`

### 2. Downstream consumer coverage is strong and currently green

- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
  - passed `19` tests
- `cargo test -p ta_desktop_demo_app state:: -- --nocapture`
  - passed `14` tests

### 3. Important gaps remain

- no test for the sampled-fingerprint branch above `CONTENT_FP_FULL_MAX_BYTES`
- no test for version-mismatch or header-field mismatch behavior beyond a generic corrupt-cache smoke case
- no test for `write_cache` inconsistent-length rejection: `367-375`
- no test for `exists()` / unreadable-path ambiguity
- no test for temp-file rename failure path
- no test for big-endian conversion branches or big-endian compile viability
- no direct test pinning the semantics of metadata-only freshness vs content-based freshness
- no integration test covering mixed timestamp units between `state.rs`-normalized base datasets and direct `data_cache` cross datasets

## Performance, Allocation, And CUDA Concerns

### 1. The `MaybeUninit` read path is a deliberate performance optimization and should not be "cleaned up" casually

- Evidence:
  - bulk numeric reads avoid pre-filling large vectors: `226-269`
- Why it matters:
  - this module sits on every CSV-backed dataset load
  - replacing it with full zero-initialization or per-element pushes may materially change load-time behavior
- Review judgment:
  - keep it unless measurement justifies a change

### 2. Stronger freshness checks would trade correctness confidence for extra I/O cost

- Evidence:
  - content fingerprinting exists but is not part of cache validation: `80-109`, `342-344`
- Why it matters:
  - validating every cache hit with full or sampled file hashing would change load-time cost across the repo
- Review judgment:
  - benchmark-sensitive; do not change by default without measurement

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/run_artifacts.rs`

### Downstream consumers needing verification

- base dataset loading and reuse
- cross-ticker validation dataset loads
- campaign dataset preload/prepare flows
- options-expression dataset replay
- artifact metadata backfill and provenance displays

## Code-Quality Improvement Register

### Safe now

- add safety comments to the unsafe helpers
- document the cache lookup/freshness policy
- replace selected `exists()` uses with `try_exists()` where error distinction matters
- add lightweight diagnostics for silent cache read/write fallback

### Coordination required

- strengthening cache freshness beyond size+mtime
- changing `LoadOutcome` field semantics
- redesigning the on-disk header contract

### Defer until later pass

- any broader binary-format redesign
- any default-on fingerprint validation without measured impact

## Technical-Debt Reduction Register

### Debt safe to reduce now

- undocumented unsafe invariants
- undocumented metadata-only cache validity model
- ambiguous `exists()` handling

### Debt that spans multiple files

- reconciling fast cache freshness with stronger provenance identity
- keeping `LoadOutcome` semantics aligned with `state.rs` load-source reporting

### Debt that should not be “cleaned up” casually

- permissive corrupt-cache fallback behavior
- sidecar/fallback dual-path lookup
- `MaybeUninit` bulk-read optimization

## Performance Opportunity Register

- Safe structural opportunities:
  - none important beyond minor diagnostics/docs work
- Benchmark-sensitive opportunities:
  - stronger default cache validation
  - replacing low-level slab I/O strategy
- Not the right place to chase CUDA performance:
  - yes

## Observability And Diagnostics Gaps

- silent cache read failures
- silent cache write failures
- no local signal distinguishing "cache missing" from "cache unreadable"

## Open Questions And Assumptions

- assumption:
  - the repo intentionally values successful CSV loading over strict cache-failure surfacing
- open question:
  - whether metadata-only freshness is considered sufficient for the supported workflows, given that stronger fingerprinting is already used elsewhere for provenance

## Regression Risk And Feature-Preservation Constraints

- Do not remove permissive corrupt-cache fallback; higher-level flows depend on "load anyway" behavior.
- Do not change cache format or freshness rules casually; this module feeds state loading, validation, campaigns, options replay, and artifact provenance.
- Do not silently move timestamp normalization boundaries without verifying base loads and cross-ticker validation together.
- Do not replace the `MaybeUninit` fast path without measuring load-time impact.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app data_cache -- --nocapture`
2. `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
3. `cargo test -p ta_desktop_demo_app state:: -- --nocapture`
4. `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
   - note: this lane currently has an existing unrelated failure in `commands_validation`
5. `cargo test -p ta_desktop_demo_app commands_campaign -- --nocapture`
6. `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`

## Recommended Action

`Small surgical refactor`

The useful immediate work is to make the current contract explicit and safer to maintain: document the metadata-only freshness rule, annotate the unsafe blocks, and stop flattening filesystem ambiguity into silent cache misses where a better distinction is cheap. Stronger freshness guarantees are worthwhile to evaluate, but they are semantics-sensitive and performance-sensitive enough to defer until measured.
