# Fourth-Pass Review: src-tauri/src/data_cache.rs

- Review date: `2026-04-16`
- Source file: `src-tauri/src/data_cache.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file warranted a fourth pass because the current third-pass packet kept the right bucket but drifted toward a more abstract framing than the live code supports. The live source still reads like a compatibility-sensitive cache helper that should not be broadly redesigned, but the sharpest remaining risks are narrower and more concrete than “lexical path identity” or a generic persistence discussion.

The current third-pass bucket should stand. The file still belongs in `Small surgical refactor`, not a larger redesign bucket. The difference is that the highest-confidence next steps are not a path-identity rethink. They are a short list of local hardening fixes around malformed-cache rejection, same-path concurrent writers, operation-first filesystem handling, and an objective big-endian compile defect in the guarded recovery branches.

## Fourth-Pass Research Log

This pass re-checked the live source against current Rust standard-library guidance rather than relying on the prior packet’s framing.

1. Rust `std::fs` and `std::path::Path` documentation was consulted for the current TOCTOU guidance around existence checks. The relevant standard point is that `exists()` flattens some errors to `false`, `try_exists()` preserves more error information, but `try_exists()` still does not remove TOCTOU risk by itself. That matters here because `data_cache.rs` currently pre-checks existence in both `try_load_cache` and `write_cache`, and the earlier packet’s “use `try_exists()`” suggestion is directionally better than `exists()` but still not the narrowest fix.

2. Rust `std::fs::File` documentation was consulted for `create_new`, `sync_all`, and `sync_data`. Two constraints matter here. First, `create_new` is the standard atomic way to avoid check-then-create races for temp files. Second, stronger crash-durability claims require explicit syncing work; a buffered write followed by rename is not the same as durable persistence.

3. Rust `core::mem::MaybeUninit` and `Vec` documentation was consulted to re-check the unsafe slab-read pattern. The current read helpers are still a reasonable performance-oriented shape for POD-like numeric slabs, but they need their invariants documented and they should reject malformed lengths before any large allocation path is reached.

4. Rust `std::collections::hash_map::DefaultHasher` documentation was consulted because the third-pass packet leaned on path-identity language. The standard library still documents the default hasher’s algorithm as unspecified over releases. That makes the fallback-cache filename hash a best-effort reuse key, not a stable long-term cache identity. That is useful context for what should stay deferred: path-key redesign is real technical debt, but it is not the sharpest safe-now correctness item.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The current third-pass packet was right to keep the bucket unchanged, right to avoid recommending broad structural churn, and right to treat persistence semantics as compatibility-sensitive. It also implicitly kept the most important preservation constraint intact: corrupt or unreadable cache artifacts should continue to degrade to a cache miss rather than fail the dataset load outright.

That conservative posture still fits the live file. `data_cache.rs` is shared runtime support code with disk-format implications, so widening scope casually would be more dangerous than helpful.

### What it missed or underweighted

The largest miss is that the live file still has a concrete malformed-header hardening gap that is more immediate than the third-pass packet’s path-identity discussion. `expected_cache_size(len)` uses unchecked multiplication (`src-tauri/src/data_cache.rs:156-158`), the loader trusts header-driven `len` after only a `u64 -> usize` conversion (`src-tauri/src/data_cache.rs:346`), and the read helpers then allocate and form raw byte slices from that `len` (`src-tauri/src/data_cache.rs:226-255`). In the normal path this is fine, but for a crafted `.vta` file it means the “corrupt cache falls back to CSV” contract is not fully hardened: an oversized `len` can reach an enormous allocation path instead of being rejected as malformed up front. That is a better safe-now target than a path-identity rewrite.

The second missed issue is a same-path writer race in `write_cache`. The temp path is deterministic (`cache_path.with_extension("vta.tmp")` at `src-tauri/src/data_cache.rs:404`), and the writer removes the destination before renaming (`src-tauri/src/data_cache.rs:418-421`). Two concurrent writers targeting the same cache file can therefore trample the same temp file, and the losing writer can delete a cache the winning writer just installed. That does not corrupt the candles already returned to the caller, but it does make cache persistence non-deterministic and can erase a valid cache artifact.

The third underweighted point is that the earlier packet’s filesystem recommendation was slightly off-target. Replacing `exists()` with `try_exists()` is an improvement only in the narrow sense that it preserves more error information. It is not the best local fix here. In `try_load_cache`, the tighter pattern is to attempt `File::open` or `metadata` directly and map `NotFound` to `Ok(None)`. In `write_cache`, the tighter pattern is to attempt `remove_file` directly and ignore `NotFound`. That removes extra syscalls and avoids a needless check/use split while preserving today’s behavior.

The fourth missed item is that the objective big-endian compile defect is still present in the live file. Both read helpers bind `out` immutably and then try to iterate it mutably inside the `#[cfg(target_endian = "big")]` branches (`src-tauri/src/data_cache.rs:237-245`, `259-268`). Even if the product’s real deployment targets are little-endian, this is still a real portability defect and a very small fix.

The fifth underweighted issue is that the timestamp-unit boundary is still more concrete than the third-pass packet gave it credit for. `AppState::load_price_data` normalizes timestamps after the cache layer returns (`src-tauri/src/state.rs:221-224`, `1523-1532`), so `data_cache.rs` itself still returns raw timestamp units. That split contract is more actionable than the third-pass packet’s general “identity boundaries” wording because it directly affects any caller that consumes `data_cache` outside the `AppState` base-load path.

### What the current packet overstated or weakly supported

The third-pass packet over-centered “identity-sensitive path handling.” That is not wholly wrong, but it is not the sharpest remaining risk in the live file. The fallback cache path is a best-effort cache filename, not a security boundary, and the on-disk header still validates reuse using source metadata. If this area changes now, the higher-value work is not canonical-path theory; it is preventing malformed cache files and same-path writer races from breaking the intended “safe miss” behavior.

It also treated overwrite hardening too generically. True cross-platform atomic-and-durable replacement is not a one-line cleanup here. The code currently writes through a `BufWriter` and flushes it (`src-tauri/src/data_cache.rs:405-416`), but stronger durability would require explicit file syncing and likely parent-directory syncing as well. That is a semantics-sensitive I/O decision on a shared load path. It belongs in deferred redesign discussion, not in the first local patch unless there is a measured requirement.

Finally, the broader “pin persistence behavior” advice was too vague. The next tests should be more specific: malformed-header rejection without panic paths, same-path concurrent writer behavior, and a compile-only or cross-target guard for the big-endian branches.

## Implementation Priority

### Safe-now hardening

1. **Add checked length arithmetic before any allocation path.**

   Reject cache headers whose `len` cannot produce a valid payload size before entering `Vec::with_capacity` or forming `len * 8` byte slices. The right local change is to make the header-size and slab-size math explicitly checked and to treat overflow as `Ok(None)` in `try_load_cache`, preserving the current “bad cache becomes miss” contract.

   This is the highest-confidence correctness hardening item in the file because it tightens the corrupt-cache boundary without changing the wire format or the higher-level API.

2. **Make temp-file naming unique per write.**

   Keep the final cache filename and the sidecar/fallback lookup order exactly as they are, but stop using a single deterministic `*.vta.tmp` path for all writers of the same cache file. A local nonce or PID/timestamp suffix plus `File::create_new` is enough. That is a narrow fix for the writer race without turning this helper into a file-locking subsystem.

   This is preferable to a broader locking design in the current bucket. It addresses the concrete race while preserving current behavior when only one writer exists.

3. **Replace pre-checks with operation-first handling instead of introducing `try_exists()`.**

   In `try_load_cache`, attempt the open/read path directly and treat `NotFound` as a miss. In `write_cache`, attempt `remove_file` directly and ignore `NotFound`. This is a better safe-now change than swapping `exists()` for `try_exists()` because it removes the extra check/use split instead of just changing how that pre-check reports errors.

4. **Fix the big-endian mutability bug and keep the repair tiny.**

   This should remain a surgical portability fix: make the vectors mutable in the guarded conversion branches and add a compile-oriented verification step if the toolchain is available. Do not let this expand into a broader endian rewrite.

5. **Add local safety comments next to the unsafe slab I/O helpers.**

   The earlier packets were right that the unsafe reasoning should be written down. That remains worthwhile after the allocation guard is added because these helpers are still safe only under explicit assumptions about complete initialization before exposure as `Vec<i64>` / `Vec<f64>`.

6. **Optionally reject unknown field-mask bits if the header logic is touched anyway.**

   This is lower priority than the preceding items, but it is a cheap extra corruption screen and a more concrete header hardening step than trying to invent new semantics for the currently unused `min_ts` / `max_ts` header bytes.

### Larger redesign still deferred

1. **Do not default-on stronger freshness validation in this pass.**

   Metadata-only freshness remains imperfect, but moving cache hits to fingerprint-backed validation would add I/O to a shared load path and would require a deliberate decision about performance versus confidence. That is not a safe-now local tweak.

2. **Do not market this pass as an atomic-and-durable rewrite.**

   A true cross-platform replacement story would need more than the current temp-write-plus-rename shape. It would need explicit durability semantics, likely `sync_data` / `sync_all` decisions, and probably directory-sync review. That is bigger than the current bucket.

3. **Do not move timestamp normalization boundaries locally.**

   The split between `data_cache.rs` and `state.rs` is still awkward, but moving normalization into the cache helper would change what downstream callers observe. That should only happen with cross-file verification and an explicit contract decision.

4. **Do not spend this pass on path-key redesign.**

   If the product ever needs stable fallback-cache names across compiler or toolchain changes, `DefaultHasher` is the wrong long-term primitive for that job. But solving that is a deliberately chosen cache-key policy change, not the best next local hardening step.

## Verification If Changed

At minimum, keep the existing direct cache coverage and expand it in the places the current file is weakest.

1. Run the existing cache-local unit lane:
   - `cargo test -p ta_desktop_demo_app data_cache -- --nocapture`

2. Re-run the state-facing load path because `LoadOutcome` semantics and timestamp normalization are consumed there:
   - `cargo test -p ta_desktop_demo_app state:: -- --nocapture`

3. Re-run the data-command surface that fronts this helper:
   - `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`

4. Add a new targeted unit test that builds a tiny cache file with a maliciously oversized header `len` and verifies that `load_or_build` treats it as a cache miss rather than entering a panic-sized allocation path.

5. Add a new targeted unit test or deterministic regression helper for the same-path writer race. The goal is not to prove perfect cross-platform locking; it is to prove that two writes to the same cache path no longer leave the final cache missing.

6. If the big-endian branch is touched, add at least one compile-oriented verification step for a big-endian target when the toolchain is available. A compile-only guard is enough here.

7. If timestamp normalization boundaries are touched anyway, add an integration test that makes the intended raw-versus-normalized timestamp contract explicit across `data_cache::load_or_build_with_identity` and `AppState::load_price_data`.

## Regression And Feature-Preservation Constraints

Preserve the current fallback posture: a bad cache artifact should remain a miss-like condition, not a hard load failure. That resilience is part of the current module contract and is more important than surfacing every cache error synchronously.

Preserve the on-disk format version unless a migration is explicitly planned. The safe-now items above do not require a format bump.

Preserve the current final cache filenames and the sidecar-first, fallback-second lookup order. A unique temp-file change should harden writes without changing the final cache path that callers report today.

Do not change `LoadOutcome` semantics casually. `state.rs` derives frontend-facing `load_source`, `cache_hit`, and `cache_path` behavior from this structure (`src-tauri/src/state.rs:268-286`).

Do not replace the current slab I/O fast path with a “cleaner” but slower fully initialized read path unless it is measured. This file sits on dataset-load boundaries and the current low-level approach is justified once the invariants and malformed-header guards are tightened.

Do not default-enable stronger freshness checks or heavier sync behavior without measurement. Both would add I/O cost to a shared backend surface.

No GPU ABI or frontend IPC redesign is justified here. Any changes in this pass should remain backend-local and disk-format compatible.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**. `src-tauri/src/data_cache.rs` still belongs in `Small surgical refactor`.

What changes in the fourth pass is not the bucket, but the priority order inside it. The current third-pass packet was correct to avoid broad refactoring, but it underweighted more concrete live issues: unchecked header-driven length math, a same-path temp-file writer race, the fact that `try_exists()` is not the best local fix for the current pre-check pattern, and a still-live big-endian compile defect.

The safe-now work is therefore narrow and evidence-backed: reject malformed cache lengths before allocation, use unique temp files, move to operation-first filesystem handling, fix the big-endian mutability bug, and document the unsafe invariants. Larger redesign remains deferred: fingerprint-backed freshness, true atomic-and-durable replacement, timestamp-boundary unification, and any deliberate change to fallback cache-key stability.

The main regression risk is overreaching. A patch that quietly changes cache freshness semantics, timestamp normalization boundaries, or persistence durability policy would carry more compatibility risk than benefit for this pass. A narrow hardening patch would improve this file substantially without changing its current role in the repository.
