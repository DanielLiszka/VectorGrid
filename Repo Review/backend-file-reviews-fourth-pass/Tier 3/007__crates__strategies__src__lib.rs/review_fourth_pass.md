# Fourth-Pass Review: `crates/strategies/src/lib.rs`

- Review date: `2026-04-16`
- Source file: `crates/strategies/src/lib.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/lib.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/lib.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/strategies/src/lib.rs.review.md`
- Third-pass action bucket: `Document only`
- Fourth-pass action bucket: `Document only`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file is so small that a fourth pass is only justified if the earlier packet’s reasoning drifted away from what the live source actually contains. The live `lib.rs` is still just three public module declarations:

```rust
pub mod composite_cpu;
pub mod double_ma;
pub mod records;
```

That keeps the earlier `Document only` bucket plausible, but it also means the third-pass packet needed to stay tightly anchored to public-module exposure and crate-front-page documentation. Instead, the third-pass narrative expanded into Serde-visible defaults, permissive normalization, zeroed fallback behavior, and compatibility fixtures. Those may be valid concerns elsewhere inside `ta_strategies`, but they are not findings on this file.

So the fourth pass was not mainly about finding a hidden bug. It was about checking whether the third-pass justification still matched the file. The answer is: the bucket should stand, but the reasoning needs to be narrowed back to the real scope of `lib.rs`.

## Fourth-Pass Research Log

I reread the live source, compared it against all three earlier packets, and checked the repo guidance to confirm the crate’s role. The repo map still treats `crates/strategies` as the shared strategy definitions and logic crate, which makes this crate root public-surface-sensitive even though the file itself is operationally trivial.

I then revalidated the documentation and API-surface guidance against current Rust primary sources. The rustdoc book says crate documentation should begin with front-page docs in `src/lib.rs`, and that those first lines should use `//!`.[^1] The Rust Book shows the same crate-level `//!` pattern and explicitly frames `pub use` as a tool for reshaping a public API only when the internal module structure is inconvenient for consumers.[^2] The Rust Reference confirms that anything externally usable must be public from the crate root down to the destination item.[^3] Cargo’s SemVer guide classifies renaming, moving, or removing public items as a major change, and treats adding new public items as minor but not risk-free.[^4] Finally, the rustdoc documentation guidance notes that `#![warn(missing_docs)]` or `#![deny(missing_docs)]` can help keep libraries documented, but that is inherently a crate-wide policy move rather than a tiny local tweak.[^5]

That research materially supports only two conclusions here: first, a small crate-level rustdoc block is justified; second, structural API changes from this file should be treated conservatively because this is the root that exposes the crate’s module paths.

## What Fourth Pass Sharpened

### The third-pass bucket should stand

`Document only` is still the correct bucket. This file does not contain executable logic, data-shape logic, validation logic, CUDA branching, IPC handling, persistence behavior, or serialization behavior. There is no evidence here for a bounded code hardening change beyond documentation.

### What the third-pass packet got right

The third-pass packet was right to keep the bucket unchanged and to treat this file as a compatibility-sensitive crate-root seam rather than a refactor target. It was also right that changing or removing exports here would create downstream churn, because this file is the root of the crate’s public module paths.

### What it missed or underweighted

The packet underweighted the most concrete, evidence-backed improvement: a short crate-front-page rustdoc block. The earlier first-pass packet was closer to the mark on that point than the third-pass packet.

It also failed to separate change classes clearly enough. A docs-only edit is one thing; a public-export change is another. Those two edits have very different verification and regression profiles, and a review of this file should say that explicitly.

Finally, it should have said plainly that performance, GPU ABI, IPC, persistence, and validation behavior do not live in this file. Those concerns only matter here indirectly through import-path stability.

### What the third-pass packet overstated or weakly supported

The third-pass packet’s discussion of Serde container attributes, Serde field attributes, permissive normalization, and zeroed fallback behavior is not supported by the live `lib.rs`. None of those mechanisms appear in this file, and none of them can be inferred from three `pub mod` declarations.

Likewise, “prefer contract fixtures and serde-visible regression tests over refactor symmetry” is not a strong recommendation for this file. That guidance belongs, if anywhere, to the concrete config and metrics types inside the exported modules. It is not a `lib.rs` finding.

The same applies to broader “shared strategy contract” language. The crate root is indeed part of the public contract, but the contract risk here is structural path stability, not behavior semantics. The file exposes modules; it does not itself implement strategy normalization, config defaults, or metrics serialization.

A smaller overstatement is the implied need for heavier consumer verification even for doc-only edits. If the only safe-now change is crate-front-page rustdoc, the highest-signal verification is documentation/build sanity, not cross-crate behavioral fixtures.

## Implementation Priority

### Safe-now hardening

The only safe-now hardening clearly justified by the live file is a crate-level rustdoc comment at the top of `lib.rs`.

That comment should stay intentionally small. It should explain that `ta_strategies` is the shared strategy crate and briefly describe the role of `double_ma`, `composite_cpu`, and `records`, ideally with intra-doc links to those modules. A short overview is enough. Forcing a top-level doctest or a heavier tutorial into this file would be disproportionate to the surface area and would likely age poorly.

### Larger redesign still deferred

A public-API reshape from this file is still deferred. That includes changing `pub mod` to a different export pattern, flattening the surface with `pub use`, renaming modules, or hiding modules behind a different façade. Rust’s own guidance presents `pub use` as a convenience tool when the internal hierarchy is cumbersome for users; there is no evidence of that problem here because the current surface is already only three first-level modules.[^2]

Crate-wide documentation enforcement is also deferred. Adding `#![warn(missing_docs)]` or `#![deny(missing_docs)]` may be desirable in a broader documentation campaign, but it is not a bounded `lib.rs` cleanup. It could create a large amount of follow-on work across the crate’s exported items, so it should not be smuggled in as a tiny crate-root hardening patch.

Conditional export changes should remain deferred as well. Cargo’s SemVer guidance explicitly notes that changing which public items exist, including via `cfg`, can be breaking.[^4] This file should therefore stay boring unless there is a coordinated, repo-wide API migration plan.

## Verification If Changed

For the only safe-now change recommended here, verification can stay light:

- `cargo doc -p ta_strategies --no-deps`
- `cargo check -p ta_strategies`

If someone chooses to add a doctest at the crate root, add:

- `cargo test -p ta_strategies --doc`

If someone changes the export surface anyway, that leaves the `Document only` bucket and needs broader verification because the public module paths may break consumers. In that case, at minimum run downstream compile coverage for direct workspace users such as `ta_app_core` and `ta_optimizer`, and preferably a workspace-wide check.

## Regression And Feature-Preservation Constraints

Preserve the exact public module paths exposed today:

- `ta_strategies::composite_cpu`
- `ta_strategies::double_ma`
- `ta_strategies::records`

Do not treat crate-root cleanup as permission to reshape the public API. Even a cosmetically nicer export layout can create downstream import churn, alter documentation discoverability, or quietly expand the apparent supported surface.

Do not attach performance, GPU ABI, IPC, persistence, or validation claims to a change in this file unless some broader refactor has escaped scope. A correct safe-now edit here should leave those dimensions untouched.

Do not introduce crate-wide documentation lint policy from this file alone unless the full crate has been audited for the fallout. That is a separate standards decision, not a local hardening tweak.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: `Document only` remains the correct action bucket for `crates/strategies/src/lib.rs`.

What changes in the fourth pass is the reasoning. The third-pass packet got the conservative bucket right, but it justified that result with concerns that belong to deeper strategy modules, not to this crate root. The live `lib.rs` supports one small, high-confidence improvement: add a concise crate-level rustdoc overview. It does **not** support a stronger call for contract fixtures, Serde-focused hardening, or API reshaping.

In other words, the current action bucket is still defensible, but only when framed narrowly: this is a minimal public-module seam, and the right hardening move is to document that seam rather than redesign it.

[^1]: Rustdoc Book, “How to write documentation”: <https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html>
[^2]: The Rust Programming Language, Chapter 14.2, crate-level docs and `pub use`: <https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html>
[^3]: Rust Reference, “Visibility and privacy”: <https://doc.rust-lang.org/reference/visibility-and-privacy.html>
[^4]: Cargo Book, “SemVer Compatibility”: <https://doc.rust-lang.org/cargo/reference/semver.html>
[^5]: Rustdoc Book, “What to include (and exclude)”: <https://doc.rust-lang.org/rustdoc/write-documentation/what-to-include.html>
