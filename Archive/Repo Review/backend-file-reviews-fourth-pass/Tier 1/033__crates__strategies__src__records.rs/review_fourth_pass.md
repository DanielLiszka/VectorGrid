# Fourth-Pass Review: crates/strategies/src/records.rs

- Review date: `2026-04-15`
- Source file: `crates/strategies/src/records.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/records.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/records.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/strategies/src/records.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file did not need a fourth pass because it is large or algorithmically complex. It needed a fourth pass because the third-pass packet identified two real issues — malformed parallel-vector state and non-transactional batch append — but left the implementation priority slightly too aggressive for the actual public API shape.

The live file is a small public transport layer built around a serializable SoA representation (`TradeRecord` / `TradeRecords`) with public fields and lightweight conversion helpers. That matters because it sharply limits what can be “enforced” without redesigning the type itself. In other words, the third pass was directionally right about the risks, but it did not fully account for the fact that this file has already committed to a public, mutable representation choice.

My fourth-pass view is therefore narrower and more conservative: the current bucket should stand, but the recommended work should shift away from behavior-changing enforcement and toward explicit contract documentation, a couple of missing edge-case tests, and at most debug-only invariant assertions.

## Fourth-Pass Research Log

I re-read the live `records.rs` source first, then re-read the third-pass packet, and only then used the earlier packets for drift detection. I also cross-checked `crates/strategies/src/double_ma/mod.rs` and `crates/strategies/src/composite_cpu/mod.rs` because `records.rs` does not define its own trade-domain types; it converts to `Trade` and reproduces `TradeStats` policy that already exists elsewhere in the crate family.

The main live-source anchors for this pass were:

- `TradeRecord` conversion boundaries in `records.rs:16-45`
- public SoA layout in `records.rs:49-61`
- invariant checking in `records.rs:78-90`
- accessor and scan semantics in `records.rs:93-147`
- per-column trade-stat aggregation in `records.rs:150-226`
- current file-local tests in `records.rs:230-345`
- the source `Trade` shape in `double_ma/mod.rs:325-333`
- in-memory trade generation from slice indices in `double_ma/mod.rs:1459-1535`
- the shared `TradeStats` policy as implemented in `double_ma/mod.rs:1662-1695` and `composite_cpu/mod.rs:1055-1087`

I also consulted a small amount of official Rust / Serde guidance to bound what is a safe recommendation here:

1. Rust API Guidelines documentation guidance, especially documenting panic and error behavior for public APIs.[^1]
2. Rust API Guidelines future-proofing guidance, especially the warning that public fields prevent a type from maintaining invariants on its own contents.[^2]
3. Serde container attributes, specifically `#[serde(try_from = "FromType")]`, to confirm what a validated-deserialization redesign would actually look like if the repo ever wants that boundary.[^3]
4. The Rust Book discussion of indexing versus `get`, because `TradeRecords::get` looks like a non-panicking collection accessor even though malformed SoA state can still panic inside it.[^4]

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**. This is still a `Small surgical refactor`, not a redesign candidate.

There is no evidence here of a strategy bug, hot-path regression, GPU ABI issue, or validation-model error. The risk is contract clarity around a public serializable DTO. That is exactly the kind of problem that should remain in a small bucket unless the repo deliberately chooses to change the public representation.

### What the third-pass packet got right

The third pass was correct about two things.

First, malformed SoA state is not merely theoretical. `TradeRecords` derives `Serialize` / `Deserialize`, keeps every vector public, and only checks equal lengths in `validate()` (`records.rs:52-90`). That means invalid state can be created by direct field mutation or by deserializing malformed payloads, and the accessor paths do not defend against that.

Second, `try_extend_trades()` is not transactional in the strict sense (`records.rs:124-135`). If an earlier trade converts successfully and a later trade overflows in `try_from_trade()`, the earlier records stay appended.

Those are real observations and worth keeping.

### What the third-pass packet overstated

The third-pass packet slightly overstated how much “enforcement” can be done inside this file without crossing into redesign territory.

The main reason is structural: `TradeRecords` has public fields (`records.rs:53-60`). Once a type makes that commitment, it cannot truly maintain its own invariants, because callers can always mutate the fields directly after construction. That is exactly the kind of representation lock-in the Rust API Guidelines warn about.[^2] So “make accessors require or enforce validated state” is not really a small-surgical recommendation in this API. The file can document the invariant, assert it in debug builds, or add checked alternatives, but it cannot actually guarantee it without changing the public shape.

The third pass also made the append-failure concern sound broader than it is. `try_push_trade()` is atomic per trade because conversion happens before the record is pushed (`records.rs:118-121`). `try_from_trades()` is effectively atomic to the caller because it builds a fresh `TradeRecords` and drops it on error (`records.rs:64-67`). The only visible partial-mutation surface is `try_extend_trades()` on an already-existing collection, and even there the error can only happen when a later `Trade` index exceeds `u32::MAX`. In local context, those indices come from in-memory slice positions (`double_ma/mod.rs:1459-1535`), so this is an extreme-size edge case rather than a routine operational hazard.

That does not make the observation wrong. It does make “make append failure atomic” too strong as the default next step.

### What the third-pass packet missed or underweighted

The most important missed point is that malformed-state behavior is **asymmetrical**, not just “may panic.” `len()` is defined as `self.col.len()` (`records.rs:70-75`). Every scan uses that as the row count (`records.rs:138-181`). So if `col` is longer than another field, methods can panic; but if another field is longer than `col`, the extra tail data is silently unreachable and ignored. In practice, `col` is the de facto canonical row count, but that fact is undocumented.

That asymmetry matters because it changes what a safe refactor looks like. A naive iterator cleanup that zips columns together would silently truncate to the shortest vector and hide malformed persisted or IPC state. That would be worse than the current behavior unless the contract explicitly changed.

The third pass also underweighted the API-shape surprise in `get()`. In Rust, `get()` conventionally signals non-panicking access that returns `None` when an element is unavailable.[^4] Here, `TradeRecords::get()` only protects against `i >= self.len()`; it still uses direct indexing into the other columns (`records.rs:93-105`). That is not necessarily wrong, but it is exactly the kind of panic caveat that should be documented on a public method.[^1]

Finally, the third pass spent more energy on malformed-state mechanics than on **semantic parity risk**. `trade_stats_for_col()` duplicates the same trade-summary policy used elsewhere in this crate family (`records.rs:150-226`, `double_ma/mod.rs:1662-1695`, `composite_cpu/mod.rs:1055-1087`). That duplication is a more realistic maintenance risk than the overflow path. The current tests do not pin the parity-sensitive edge cases already implied by the live formula, especially:

- flat closed trades count toward `closed_trades` and expectancy, but not toward the win-rate denominator;
- all-win / no-loss sets produce `profit_factor = f64::INFINITY`.

Those semantics are observable policy, not mere implementation detail, and this file should test them if it continues to own an independent aggregation copy.

## Implementation Priority

### Safe-now hardening

The safest useful work is still small.

First, add explicit rustdoc to `TradeRecords`, `validate()`, `get()`, `to_trades_for_col()`, `trade_stats_for_col()`, and `try_extend_trades()`. The docs should say, plainly:

- all column vectors are required to have the same length;
- `col.len()` is the iteration anchor used by the current helpers;
- externally constructed or deserialized values should be validated before use;
- accessors assume valid SoA state and may panic or ignore unreachable tail data if that invariant is broken;
- `try_extend_trades()` may partially append earlier records before returning an overflow error on a later trade.

That documentation is not fluff. It is the most direct fix for a public serializable type whose failure behavior is otherwise only obvious from reading implementation details.

Second, add targeted edge-case tests for `trade_stats_for_col()`. The highest-value additions are a flat closed trade case and an all-win / no-loss case. Those tests do more to preserve compatibility than another generic mismatch-length test, because they lock this file to the same `TradeStats` policy already implemented in `double_ma` and `composite_cpu`.

Third, if you want one small code hardening step without changing release behavior, add `debug_assert!(self.validate().is_ok())` at the start of the public accessors and mutators that assume a valid SoA state. That improves failure locality in debug builds without redefining the release contract or conflating malformed state with `None`.

### Larger redesign still deferred

If the repo actually wants invariant enforcement rather than documentation, that is a different class of change.

The clean version would be to stop exposing raw mutable fields directly, or to keep the public serialized shape as a wire type and convert through a validated domain type using something like Serde’s `try_from` support.[^3] That is a legitimate design direction, but it is not a small follow-up. It changes how external callers construct the type, how deserialization is handled, and potentially how stored artifacts remain compatible.

Likewise, if the repo truly wants atomic batch append, the honest implementation is to pre-convert into temporary `TradeRecord`s before mutating `self`, or to provide a distinct atomic helper instead of retrofitting the current method. That introduces extra work or memory for a failure mode that is currently very narrow. I would not prioritize it unless there is an actual caller that depends on transactional semantics.

I would also defer any attempt to replace the current per-column scans with indexing structures. This file is not the right place to chase speculative performance, and the existing SoA design is simple and transparent.

## Verification If Changed

If this file changes, the minimum local verification remains:

- `cargo test -p ta_strategies -- --nocapture`

If you add the hardening I recommend, the test delta should specifically cover:

- flat closed trade handling in `trade_stats_for_col()`;
- all-win / no-loss `profit_factor` behavior;
- any newly documented partial-append behavior, **but only if** you are comfortable freezing that behavior as contract.

If the change crosses into deserialization behavior, field visibility, or wire-shape changes, verification has to widen beyond this crate because `TradeRecord` and `TradeRecords` are public serializable transport types. At that point, artifact load/save and IPC boundaries need explicit downstream re-checks.

## Regression And Feature-Preservation Constraints

Preserve the current serialized field names, field types, and SoA layout unless the downstream crates and any persisted payloads move together. This file is part of a public transport surface, not just a private helper.

Preserve the current `TradeStats` policy unless the rest of the crate family changes with it. In particular, do not casually “fix” flat closed trades entering `closed_trades` / expectancy while staying outside the win-rate denominator, and do not silently clamp or normalize the current infinite-profit-factor branch in this file alone.

Do not replace the current indexed scans with `zip()`-style shortest-slice iteration as an incidental cleanup. That would silently truncate malformed state and make bad persisted or externally supplied data harder to detect.

Do not make the fields private, add validating deserialization, or change `get()` to collapse invariant failures into `None` under the banner of a small cleanup. Those are contract changes, not maintenance polish.

Do not overcorrect on `try_extend_trades()`. Making it atomic is only worthwhile if the repo explicitly wants that stronger contract. Otherwise, document the current behavior and leave the method simple.

## Fourth-Pass Conclusion

The third-pass action bucket should **remain `Small surgical refactor`**.

The third-pass packet was right to call out malformed-state exposure and non-transactional extension, but it slightly overreached by treating accessor enforcement as the natural next step. In this file, public fields are the decisive fact. They make true invariant enforcement a redesign problem, not a surgical one.

The best fourth-pass correction is therefore to narrow the action list, not to escalate it: document the invariant and panic surface explicitly, add the missing `TradeStats` edge-case tests that preserve parity with the rest of the crate, consider debug-only invariant assertions, and defer any private-field / validated-deserialization / atomic-append redesign until the repo is willing to make a deliberate public-contract change.

[^1]: Rust API Guidelines, “Documentation,” especially C-FAILURE and related public-doc guidance: <https://rust-lang.github.io/api-guidelines/documentation.html>
[^2]: Rust API Guidelines, “Future proofing,” especially C-STRUCT-PRIVATE: <https://rust-lang.github.io/api-guidelines/future-proofing.html>
[^3]: Serde container attributes, `#[serde(try_from = "FromType")]`: <https://serde.rs/container-attrs.html>
[^4]: The Rust Programming Language, vectors chapter, on indexing versus `get`: <https://doc.rust-lang.org/book/ch08-01-vectors.html>
