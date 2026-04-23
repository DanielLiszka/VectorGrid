# Fourth-Pass Review: crates/optimizer/src/stream.rs

- Review date: `2026-04-15`
- Source file: `crates/optimizer/src/stream.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass packet correctly moved the discussion away from “comments only” and toward a real merge-contract weakness. The live file still makes `merge()` the only place where this module can silently produce a semantically misleading combined result, while the rest of the file remains intentionally simple and reasonably bounded (`source__crates__optimizer__src__stream.rs:299-399`).

That said, the third-pass prescription is still slightly too blunt. It is right that incompatible detail policies are a problem, but it underdescribes the exact failure mode and overstates the safest immediate fix. The stronger live issue is not merely that one side’s `include_all` rows can be dropped. It is that auxiliary result fidelity is direction-dependent: the merge receiver can keep, drop, or adopt one-sided `all` / `heatmap` state depending on which side happens to be `self` (`source__crates__optimizer__src__stream.rs:319-377`). That is the point a fourth pass needed to sharpen.

The current bucket should stand. This is still a small, semantics-sensitive hardening pass, not a redesign. But the safe-now target should be a real `merge()` behavior correction with focused tests, not just more commentary and not a whole-sale ranking rewrite.

## Fourth-Pass Research Log

I re-read the live source first, then the current third-pass packet, and only then checked the earlier packets for drift. I also checked the local result contract in `crates/optimizer/src/lib.rs`, because the safest fix depends on whether auxiliary fields are already modeled as optional. They are: both `all` and `heatmap` are `Option` fields on `OptimizationResult`, and they are omitted from serialized output when absent (`context__01__crates__optimizer__src__lib.rs:53-69`). That matters because it gives this file a truthful “degrade detail fidelity to None” escape hatch without changing the public result type.

I also verified three Rust-library behaviors that matter to the current packet’s claims. First, `BinaryHeap::into_iter()` yields elements in arbitrary order, so any equal-score order that survives finalization is not a stable semantic guarantee by itself. Second, `slice::sort_by()` is stable, which means this file’s final sort preserves whatever pre-sort order it receives for equal scores; here that preserved order still originates from the heap’s arbitrary iterator order. Third, Rust’s current function-pointer comparison guidance is weaker than the earlier packets implied: official docs and lint text explicitly warn that function-address equality can produce false positives and false negatives, so merge compatibility keyed on function addresses is only a best-effort internal heuristic, not a strong semantic identity mechanism.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right on three important points.

First, the bucket should remain `Small surgical refactor`. Nothing in the live file justifies a larger rewrite. The hot push path is short, the heap logic is conventional, and the ranking code should not be disturbed casually (`source__crates__optimizer__src__stream.rs:243-292`).

Second, the packet correctly identified `merge()` as the real correctness surface. The earlier packets leaned too hard on documentation clarity, while the live source shows observable behavior differences when detail policies diverge (`source__crates__optimizer__src__stream.rs:299-377`).

Third, the packet was right not to conflate this with a heap redesign. For compatible aggregators, the existing top-k maintenance is simple and appropriate, and changing ordering internals without a stronger product requirement would be regression-prone (`source__crates__optimizer__src__stream.rs:277-291`, `383-386`).

### What it missed or underweighted

The main missed point is that the defect is directional, not just partial.

On the `all` side, if `self` has `Some(Vec<_>)` and `other` has `None`, the merge keeps only `self`’s historical rows while still merging `num_combos`, `best`, and `top`. If the receiver is reversed, `other`’s exact rows are dropped outright because the append only happens for `(Some, Some)` (`source__crates__optimizer__src__stream.rs:319-323`).

On the `heatmap` side, the asymmetry is even sharper. If `self.heatmap` is `None` and `other.heatmap` is `Some`, the code adopts `other`’s heatmap wholesale (`373-375`) even though `self.num_combos`, `best`, and `heap` have already been merged from both sides (`319-347`). That yields a combined result whose heatmap only covers one side. If `self.heatmap` is `Some` and `other.heatmap` is `None`, the merged result keeps a one-sided heatmap. If both are `Some` but configured differently, the function returns only after core state has already been merged (`350-360`). In other words, the current behavior is not just “detail-policy mismatch exists.” It is “detail-policy mismatch makes merged auxiliary output depend on receiver choice and branch path.”

That is stronger than the third-pass phrasing and should be the lead finding.

### What the current packet overstated

The third-pass implementation priority says to reject incompatible `include_all` and detail-policy merges before any deeper merge cleanup (`review__third_pass.md:47-50`). That is too strong as the default safe-now fix.

For core ranking state, `objective`, `num_candles`, `score_fn`, `pair_score_fn`, and `top_k` are the real compatibility gate (`source__crates__optimizer__src__stream.rs:299-317`). If those align, then `num_combos`, `best`, and `top` can still be merged meaningfully even when auxiliary detail fidelity cannot. Rejecting the entire merge because `all` or `heatmap` policy differs would throw away valid best/top information and undercount processed combinations. That is a larger semantic change than necessary.

A narrower, safer change is available because the public result type already models detail channels as optional (`context__01__crates__optimizer__src__lib.rs:53-69`). This file can preserve core merge behavior while degrading incompatible auxiliary outputs to `None`, and it can still adopt exact auxiliary state when one side is actually empty. That is the smallest behavior hardening that makes the combined result more truthful without discarding valid search results.

### What the test story still does not support

The broader packet family overstates local test strength. The live tests cover matching-configuration heatmap merge, custom score functions, pair score clamping, and single-aggregator include-all order (`source__crates__optimizer__src__stream.rs:443-523`). They do **not** cover:

- `all` mismatch across merges,
- heatmap presence mismatch,
- heatmap shape mismatch after non-empty core merge,
- receiver-direction asymmetry,
- empty-side adoption behavior,
- equal-score cutoff membership across merges.

So the remaining safe-now work is not “comments only.” There is a clear need for behavior-locking tests around the actual merge edge cases.

### What is still deferred, not denied

The file still has a second-order determinism issue, but it is not the right immediate patch.

`BinaryHeap::into_iter()` produces items in arbitrary order, and this file finalizes by collecting that iterator and applying a stable `sort_by()` on score only (`source__crates__optimizer__src__stream.rs:383-386`). Stable sort does **not** solve equal-score determinism here; it only preserves the heap iterator’s incoming order. Combined with strict `>` comparisons in `push()` and `merge()` (`255-291`, `325-345`), equal-score cutoff membership remains arrival-order-sensitive. That is real, but it is redesign-level because any serious fix needs an explicit product tie policy for both selection and final ordering. It should not be mixed into the immediate merge hardening patch.

The same is true for function-pointer compatibility. The live code compares `score_fn` and `pair_score_fn` by function address after casting to `usize` (`304-312`). The earlier packets treated this mostly as a documentation issue. That is too weak. Current Rust guidance says function-address equality is not a strong notion of identity; it can fail in both directions. That does not make the current code an urgent bug for this repo’s likely desktop targets, but it does make it a poor long-term merge key. That belongs in deferred redesign, not in a quick comment-only fix.

## Implementation Priority

### Safe-now hardening

The third-pass bucket should stand, but the safe-now work should be slightly different from what the packet proposes.

The first change should keep the existing **core** merge gate exactly where it is: objective, candle count, scoring mode identity, and `top_k` mismatch should still short-circuit the entire merge (`source__crates__optimizer__src__stream.rs:299-317`). Those fields control the meaning of `best` and `top`.

After that, auxiliary state should be handled truthfully rather than directionally.

For `all`, the smallest safe rule is:

- if both sides carry exact `all` rows, append them;
- if one side has rows and the other side is empty, preserve the exact side;
- otherwise, set `all = None` because the combined exact row set is no longer available.

For `heatmap`, the smallest safe rule is similar:

- if both sides have the same heatmap contract, merge as today;
- if only one side has a heatmap and the other side is empty, preserve the exact side;
- otherwise, set `heatmap = None` because the combined heatmap would be partial or contract-mismatched.

That is narrower and safer than rejecting the whole merge, and it uses the existing optional result shape rather than inventing a new one.

The second safe-now change is test coverage, not algorithm churn. This file needs merge tests that prove the intended behavior for receiver direction, empty-side adoption, and auxiliary invalidation. Without those tests, a future “cleanup” could easily reintroduce the same semantic corruption.

The third safe-now change is documentation, but only after the behavior is corrected. `merge()` should state that core compatibility mismatches are a no-op, while auxiliary detail fields are preserved only when they remain exact. That is a more precise and honest contract than the current inferred behavior.

### Larger redesign still deferred

A stronger scoring-identity key is still deferred. If this API ever needs robust custom-score merging across broader contexts, function-address comparison should be replaced by an explicit identity token or typed scoring mode. Simply swapping to a different pointer-comparison helper would not really solve the semantic weakness.

A deterministic tie policy is also still deferred. If the product eventually requires reproducible ordering and reproducible top-k membership for equal scores, then both heap selection and final ordering need a documented tiebreaker. Changing only the final sort key would not be enough.

Returning a merge status or result enum is deferred as well. It may be the right API eventually, but it is a broader shape change than the current file needs right now.

## Verification If Changed

If the safe-now hardening above is implemented, the minimum verification should include the local crate tests plus representative downstream optimizer consumers.

At minimum:

- `cargo test -p ta_optimizer -- --nocapture`
- focused `stream.rs` tests covering auxiliary mismatch and receiver direction
- representative `ta_app_core` optimization tests that exercise merged optimizer results, especially flows that request `include_all` and/or heatmaps

The downstream verification matters because `OptimizationResult` is `Serialize`/`Deserialize`, and the optional `all` / `heatmap` fields are skipped when `None` (`context__01__crates__optimizer__src__lib.rs:53-69`). Even though this file is not a GPU ABI surface, a merge hardening that changes `Some(...)` into `None` on mismatch can still change IPC/report payload shape.

## Regression And Feature-Preservation Constraints

The important constraints are narrower than they looked in the earlier packets.

Do not change score normalization. The current `NEG_INFINITY` clamping for non-finite scores is part of the file’s ranking behavior and is already tested (`source__crates__optimizer__src__stream.rs:213-225`, `486-496`).

Do not change compatible-merge ranking behavior in the same patch. For compatible aggregators, `best`, `top`, and `num_combos` should keep their current semantics. The safe-now patch should only make auxiliary detail fidelity truthful when policies diverge.

Do not try to “fix” tie behavior opportunistically while touching merge. Equal-score selection and equal-score final ordering are a separate policy question, and any change there can alter finalist membership.

Do not oversell function-pointer comparison as a stable identity contract. If left in place for now, document it as an internal compatibility heuristic and keep any deeper redesign out of the surgical hardening patch.

Finally, be explicit about payload-shape risk. Clearing `all` or `heatmap` on mismatch is safer than returning partial data, but it is still an observable behavior change for any downstream caller that currently receives partial auxiliary output and treats it as present. That is why downstream verification is required even though the core ranking path should remain intact.

## Fourth-Pass Conclusion

The third-pass action bucket should **stand**: this file is still a `Small surgical refactor`, not a redesign.

The third-pass packet got the center of gravity right by focusing on merge compatibility instead of heap internals. But it still missed the most precise statement of the bug: auxiliary merge behavior is direction-dependent and can produce semantically false `all` / `heatmap` output depending on which side is the receiver.

It also overreached on the immediate fix. Rejecting the whole merge on detail-policy mismatch would discard valid best/top work. The smaller, safer, more evidence-backed correction is to preserve core merge behavior for score-compatible aggregators while invalidating auxiliary detail channels whenever they can no longer remain exact, with empty-side adoption when that exactness is still preserved.

So the file does not need a bigger bucket. It needs a sharper one: **small merge hardening, focused tests, and no opportunistic tie-policy or scoring-identity redesign in the same patch**.
