# Fourth-Pass Review: `crates/app_core/src/matrix_index.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/matrix_index.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/matrix_index.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/matrix_index.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/matrix_index.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the third-pass packet reached the right bucket but did not fully separate **truly safe-now hardening** from **public-API and wire-shape changes that are only “small” if you ignore compatibility**.

The live source is still compact and algorithmically clean. The real risk is not runtime performance and not CUDA/runtime complexity. The real risk is that this is a **publicly exported module** (`context__01__crates__app_core__src__lib.rs:41`) whose main type is both **publicly constructible** and **Serde-deserializable** while carrying non-trivial derived invariants (`source__crates__app_core__src__matrix_index.rs:19-29`, `32-68`). That makes some earlier “just tighten the boundary” ideas directionally correct but too casual about SemVer and artifact compatibility.

So the goal of this pass was to challenge the existing packet on two points:

1. Which recommendations are actually safe in a public repository right now.
2. Which concrete edge cases matter more than the third-pass packet’s generic talk about “defaults” and “omission behavior.”

My conclusion is that the **current third-pass bucket should stand**. But the safe-next-work is **narrower** than the earlier packet implied: focused invariant tests, sharper rustdoc, and explicit deferral of public-shape changes.

## Fourth-Pass Research Log

This pass re-checked the live file against the official Rust and Serde guidance most relevant to a tiny invariant-bearing helper that is nevertheless public.

1. **Rust API Guidelines — Future proofing / `C-STRUCT-PRIVATE`**
   - Relevance to this file: the main type exposes all of its invariant-bearing fields publicly (`source__crates__app_core__src__matrix_index.rs:20-29`).
   - What this sharpened: the design concern identified in the earlier packets is real, but the guidance cuts both ways. Public fields are a problem for invariants, yet once a public type ships that way, changing that shape is no longer a casual refactor.

2. **Rust API Guidelines — Dependability / `C-VALIDATE`**
   - Relevance to this file: `new()` already validates sorted non-empty axes before any search logic is used (`33-57`, `110-119`).
   - What this sharpened: the constructor is doing the right kind of work. The weakness is not the algorithm; it is that callers can bypass the validating entry point.

3. **Cargo Book — SemVer Compatibility**
   - Relevance to this file: the type is public, all fields are public, and the enum is public.
   - What this sharpened: adding private fields to an all-public struct, or adding `#[non_exhaustive]` to an existing public struct/enum, is treated as a major compatibility change. That makes earlier “privatize fields” or “future-proof the enum” suggestions compatibility-sensitive, not safe-now cleanup.

4. **Rust Reference — `#[non_exhaustive]`**
   - Relevance to this file: `MatrixOrder` is currently a single-variant public enum (`3-9`).
   - What this sharpened: if future variants are ever desired, `#[non_exhaustive]` would have helped only if present from the start. Adding it now would change downstream construction and matching rules.

5. **Standard library slice docs (`binary_search`, `partition_point`)**
   - Relevance to this file: the implementation relies on sorted-slice search in both construction and lookup (`48`, `74-75`, `94`).
   - What this sharpened: the current algorithm is sound under the constructor’s guarantees, and the standard library explicitly treats unsorted input as producing unspecified/meaningless search results. That reinforces that the problem to fix is invariant leakage, not search complexity.

6. **Serde container attributes (`try_from`)**
   - Relevance to this file: the current type derives `Deserialize` directly (`19`), so deserialization bypasses `new()`.
   - What this sharpened: there is a real future path to validated deserialization without hand-writing the whole impl, but that should be treated as deliberate compatibility work, not as a “small cleanup” bolted into this pass.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep the bucket at `Small surgical refactor`. It was also right that this file does **not** justify algorithmic churn or performance work. The construction path is linear in the fast-axis length with one sorted-slice boundary search per fast period (`45-57`), and the lookup helpers are cheap, direct, and appropriately use sorted-slice search (`70-107`). There is no credible performance case here for replacing the current logic with something trickier.

The third pass was also right that **invariant coverage is higher value than refactor motion**. That is still the correct center of gravity.

### What the third-pass packet missed or underweighted

The biggest miss is that the earlier packets were still too loose about the cost of changing the public shape.

`matrix_index` is a public module, not a crate-private helper (`context__01__crates__app_core__src__lib.rs:41`). `DoubleMaFastLtSlowIndex` is a public struct with all-public fields and derived Serde (`source__crates__app_core__src__matrix_index.rs:19-29`). That means the intuitive fix — “just make the fields private and force `new()`” — is **not** a risk-free small refactor in a public-facing repository. The compatibility risk is the main reason the bucket stays small.

The third pass also did not surface the most important concrete edge case in the live code: **non-empty axes can still produce an empty matrix**.

That falls directly out of the implementation. `validate_strictly_increasing_u16()` rejects empty axes but does not require any `fast < slow` pair to exist (`110-119`). In `new()`, a fast period with no larger slow periods yields `start == slow_len`, `count == 0`, and therefore no additional columns (`48-57`). So `cols == 0` is a valid constructed state, not accidental tolerance. That should be tested and documented explicitly.

Relatedly, the file’s derived offsets are intentionally only **nondecreasing**, not necessarily strictly increasing. Once some fast period has zero valid `slow` partners, all later fast periods will also contribute zero columns, so `fast_offsets` can legitimately end with duplicates (`47`, `54-57`). Any future “invariant tightening” that incorrectly insists on strictly increasing offsets would reject valid matrices.

### What the third-pass packet overstated or framed too generically

The third-pass packet leaned on “defaults, omission behavior, and exported labels” language that does not fit this source very tightly.

There is no `Default` implementation here, no optional-field filling logic, and no subtle omission semantics on the struct itself. The relevant Serde issue is much narrower and more concrete:

- the type serializes **redundant derived caches** (`slow_start_idx`, `fast_offsets`, `cols`) alongside canonical axes; and
- the type deserializes those caches back in without re-validation.

That is a better statement of the actual problem than generic DTO/default language.

The third-pass packet also under-explained how silent the current bad-state behavior can be once construction is bypassed. This is not only a matter of returning `None` for corrupt input. Because the fields are public and lookups use `Option` plus saturating arithmetic (`82`, `95`, `99-100`), malformed instances can violate round-trip expectations without any loud failure.

A concrete example using only currently public fields:

```rust
DoubleMaFastLtSlowIndex {
    order: MatrixOrder::FastMajor,
    fast_periods: vec![1, 2],
    slow_periods: vec![2, 3],
    slow_start_idx: vec![0, 0],
    fast_offsets: vec![1, 1],
    cols: 1,
}
```

For that value, `params_for_col(0)` returns `Some((1, 2))`, but `col_id(1, 2)` returns `None` under the current method bodies. That is a stronger demonstration of the invariant problem than the earlier packet provided.

### Claims from earlier packets that are too strong

The strongest claim that needs narrowing is the idea that field privacy tightening is a straightforward safe-now fix. It is not. In this public module, that is compatibility work.

Likewise, the earlier suggestion that `MatrixOrder` might be narrowed because it only has one variant is too aggressive for the available evidence. A single-variant public enum can still be a deliberate wire/schema marker. Since this enum already derives Serde and already has a serialized case policy (`3-5`), the safer read is:

- **document its role more clearly now**;
- do **not** remove it, collapse it, or add `#[non_exhaustive]` casually.

### Safe-now hardening

The safe-now work is real, but it is narrower than “tighten the whole type boundary.”

First, add stronger **behavioral tests on valid constructed state**, not representation churn.

The highest-value addition here is a small exhaustive test matrix over tiny strictly-increasing axes. For each generated `(fast_periods, slow_periods)` pair:

- build the index through `new()`;
- assert that every `Some(col)` from `col_id(fast, slow)` round-trips through `params_for_col(col)`;
- assert that every `col in 0..cols` round-trips through `params_for_col(col)` and back through `col_id`;
- explicitly include cases where `cols == 0`.

That gives much stronger evidence than the current three hand-picked tests (`126-158`) while preserving the exact existing API.

Second, tighten the **public rustdoc contract**.

This file should say plainly that:

- `new()` is the authoritative constructor;
- `fast_periods` and `slow_periods` must be strictly increasing and non-empty (`33-35`, `110-119`);
- `cols == 0` is valid if no `fast < slow` pairs exist;
- `slow_start_idx` and `fast_offsets` are derived caches, not primary authoring inputs;
- `MatrixOrder::FastMajor` is the current stable ordering contract.

Third, if the project intentionally treats this type as part of a serialized artifact or cross-process contract, add a **current-wire-shape round-trip test for valid instances only**. But that should be done knowingly. If there is no deliberate contract here, do not accidentally cement the current redundant wire shape just because Serde derive happens to exist today.

### Larger redesign still deferred

The bigger fix is still deferred, and it should stay deferred.

The real long-term hardening path is to stop trusting caller-supplied derived caches on deserialization. The cleanest direction would be a separate wire type plus validated conversion, or `#[serde(try_from = ...)]`, with derived fields recomputed from canonical axes instead of being accepted verbatim. That would directly address the invariant leak.

But that is **not** a zero-risk patch for this repository. It changes deserialization acceptance rules, and depending on how this type is used in saved artifacts or any external tooling, it may reject payloads that currently deserialize. Even if those payloads are semantically invalid, the change is still compatibility-sensitive and should be staged intentionally.

Likewise, **field privatization**, **adding `#[non_exhaustive]`**, **removing `MatrixOrder`**, or **changing the serialized field set** all belong in a planned compatibility window, not in the safe-now lane.

## Implementation Priority

### 1. Safe-now hardening

The current third-pass bucket should stand, and the first implementation step should be additive and low-risk.

Start with tests and docs:

- extend unit coverage from a few examples to small exhaustive round-trip checks over valid constructed grids;
- add one explicit zero-column test, because that behavior is currently valid but undocumented;
- add rustdoc on `new()`, `col_id()`, and `params_for_col()` describing the ordering and valid-empty-matrix case.

That work improves confidence without changing the public or serialized surface.

### 2. Compatibility-aware next step, only if needed

If there is confirmed serialized or external use of this type, the next hardening step should be **validated deserialization while preserving the current wire shape for valid payloads**.

That is where a private invariant validator and a `TryFrom`-based Serde path become worthwhile. But I would not do that until the project has decided whether existing saved payloads or callers need to keep deserializing exactly as they do now.

### 3. Larger redesign still deferred

Any of the following should remain deferred until there is explicit appetite for compatibility-managed cleanup:

- making fields private;
- adding `#[non_exhaustive]` to the struct or enum;
- collapsing or redesigning `MatrixOrder`;
- removing derived caches from the serialized representation;
- replacing the public `String` constructor errors with a typed public error.

Those are plausible future improvements, but none are “small surgical” once public consumers and wire shape are taken seriously.

## Verification If Changed

For safe-now hardening in this file, the verification bar should stay lightweight and local:

- `cargo test -p ta_app_core matrix_index::tests:: -- --nocapture`
- `cargo test -p ta_app_core`

If any Serde-visible behavior changes, add:

- a valid-instance serialize/deserialize round-trip test for this type;
- `cargo test --workspace`

No CPU or GPU perf gate is justified here. This file is not a hot numerical loop, not a CUDA ABI surface, and not a runtime dispatch seam.

## Regression And Feature-Preservation Constraints

Preserve these constraints if the file changes:

1. Keep the current **FastMajor** mapping semantics exactly as implemented today (`17-18`, `60-66`).
2. Do **not** reinterpret `cols == 0` as invalid. Non-empty axes with zero valid pairs are currently supported by the constructor.
3. Do **not** tighten invariants in a way that rejects valid nondecreasing `fast_offsets` suffixes caused by zero-count rows.
4. Do **not** move business/domain validation into this helper. This file should validate ordering and internal mapping invariants, not strategy legality such as “is period 0 allowed for a specific indicator family.”
5. Treat all public-shape changes as compatibility work:
   - field visibility changes;
   - `#[non_exhaustive]` additions;
   - enum variant additions or renames;
   - Serde field renames or field-set changes.
6. Be explicit about risk categories:
   - **performance risk:** negligible;
   - **GPU ABI / FFI risk:** none apparent from the live source;
   - **validation/result-correlation risk:** real if column ordering changes;
   - **IPC/persistence risk:** plausible because the type is public and serializable, but not directly proven by the supplied bundle, so recommendations should stay conditional rather than overstated.

## Fourth-Pass Conclusion

`Bucket unchanged.` The current third-pass bucket of `Small surgical refactor` should stand.

But the fourth pass narrows what that bucket actually means.

The live file does **not** need algorithm work, performance work, or enum/representation churn. The real safe-now work is:

- stronger invariant tests on **valid** `new()`-constructed state;
- explicit docs for ordering, constructor expectations, and the valid-zero-column case.

The earlier packets were directionally right about invariant leakage, but too loose about compatibility cost. In this public module, the tempting fixes — private fields, `#[non_exhaustive]`, enum surgery, or wire-shape cleanup — are not surgical unless done as deliberate compatibility-managed changes.

So the sharpened recommendation is:

- **safe now:** tests and docs;
- **defer:** validated-deserialization redesign and any public-shape tightening.

That is the most conservative path that still improves the file in a meaningful, evidence-backed way.
