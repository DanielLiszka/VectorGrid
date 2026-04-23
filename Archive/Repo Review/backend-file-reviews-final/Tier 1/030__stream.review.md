# Final Consolidated Review: `crates/optimizer/src/stream.rs`

- Review date: `2026-04-15`
- Source file: [stream.rs](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/stream.rs:1>)
- First-pass packet: [stream.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/optimizer/src/stream.rs.review.md:1>)
- Second-pass packet: [stream.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/stream.rs.review.md:1>)
- Third-pass packet: [stream.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/stream.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/030__crates__optimizer__src__stream.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid, but normalize prompt-artifact references`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html>
  - <https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by>
  - <https://doc.rust-lang.org/std/ptr/fn.fn_addr_eq.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

All four passes agree that this file does not justify a large rewrite. The core top-k path is compact, conventional, and already fairly well characterized. The review progression is mainly about identifying the real correctness surface. The first pass leaned too hard on documentation clarity. The second and third passes correctly moved the focus to `merge()`. The fourth pass sharpens that the defect is not simply "partial merge exists," but that auxiliary fidelity is receiver-direction-dependent.

That sharpened conclusion is valid. `merge()` can preserve or drop `all` and `heatmap` state differently depending on which aggregator is `self`, while still merging `num_combos`, `best`, and heap state. The strongest safe-now fix is not a full merge rejection. It is a truthful auxiliary-state policy that preserves valid core ranking data while invalidating or refusing auxiliary detail when exact fidelity is no longer available.

No wrong fourth-pass packet is present in this folder. The packet is about the correct file. The problem is packet cleanliness, not misplacement: it still contains `source__...` and `context__...` prompt-artifact references and should not be treated as final wording.

## Safe-Now Priority

- Keep the current core compatibility gate for `objective`, candle count, score-function identity, pair-score identity, and `top_k`.
- Make `all` and `heatmap` truthful on merge:
  - merge them only when exact compatibility exists
  - when exact merged fidelity no longer exists, degrade the affected auxiliary channel to `None` instead of silently preserving or dropping one side based on receiver order
  - preserve explicit empty-side adoption only when it still represents the true accumulated state
- Add focused tests for:
  - receiver-direction asymmetry
  - one-sided `all` presence mismatch
  - heatmap shape mismatch after otherwise compatible core merge
  - empty-side adoption behavior
- Document the real merge contract once the semantics are tightened.

## Deferred Work

- Defer any deterministic equal-score tie-policy redesign.
- Defer redesign of function-pointer identity as the merge-compatibility key.
- Defer a richer merge-status API that would surface auxiliary degradation explicitly to callers.

## Verification If Changed

- `cargo test -p ta_optimizer -- --nocapture`
- Add targeted `stream.rs` tests covering merge direction, detail-policy mismatch, and heatmap mismatch.
- Recheck representative `ta_app_core` optimization flows that request `include_all` and/or heatmaps so serialized result shape remains intentional.

## Regression And Feature-Preservation Constraints

- Preserve current core ranking and top-k behavior for compatible merges.
- Do not silently discard valid `best` / `top` state just because auxiliary detail channels differ.
- Treat any `Some(...) -> None` change in `all` or `heatmap` as an intentional payload-shape change that needs explicit tests.
- Do not mix auxiliary merge hardening with equal-score ordering cleanup or heap redesign.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `stream.rs` in `Small surgical refactor`
- harden `merge()` so auxiliary outputs are truthful rather than receiver-order artifacts
- add targeted merge-edge-case coverage before touching anything else
- defer tie-policy work, merge-status redesign, and function-identity cleanup until a separate contract pass
