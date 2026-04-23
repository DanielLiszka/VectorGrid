# Fourth-Pass Review: crates/optimizer/src/lib.rs

- Review date: `2026-04-16`
- Source file: `crates/optimizer/src/lib.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/lib.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/lib.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/lib.rs.review.md`
- Third-pass action bucket: `Document only`
- Fourth-pass action bucket: `Small hardening`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file is still a small crate root and support seam, but it is not a trivial one. The live source at `crates/optimizer/src/lib.rs:1-85` defines the shared optimizer vocabulary (`ObjectiveKind`, `OptimizationMode`), a public extension seam (`BacktestEngine`), a convenience adapter (`CpuEngine`), and serde-exposed result DTOs (`OptimizationResult`, `OptimizationHeatmap`). That is a real compatibility surface even if it is not a hot path.

The third-pass packet was directionally conservative, which was good, but its rationale leaned too hard on ordering concerns that are better owned by `grid` / `stream` or downstream callers. The sharper fourth-pass question is whether this file still deserves a pure `Document only` bucket once you look at the actual contract pressure: public serde shape, future evolution of an unsealed trait, and DTO invariants that are currently implicit.

My conclusion is that the third-pass caution was correct, but the bucket is slightly undercalled. This file still does **not** justify a structural refactor, performance work, or a semver-risky API cleanup. It **does** justify a small hardening pass centered on explicit contract docs and narrow contract tests, with one optional debug-only guard at the adapter boundary.

## Fourth-Pass Research Log

The fourth pass re-read the live source first, then challenged the third-pass framing against the supplied local context and the external API-contract guidance that is actually relevant to this kind of file.

First, the live source confirmed that `lib.rs` is a public contract file, not an algorithm owner. The file exposes three submodules (`grid`, `mo`, `stream`), then immediately defines public enums, a public trait, a public adapter struct, and public serde DTOs (`crates/optimizer/src/lib.rs:1-85`). That makes compatibility and contract clarity the main review lens, not ranking internals.

Second, the supplied local consumer context showed that the shared enums are not merely internal convenience types. `CompositeMoRequest` deserializes `ObjectiveKind` and `OptimizationMode` directly, and `CompositeGpuPrescanConfig` also carries `OptimizationMode` (`crates/app_core/src/composite_mo.rs:86-129`, `155-192`). More importantly, the supplied consumer explicitly ignores `req.mode` for multi-objective execution today (`crates/app_core/src/composite_mo.rs:1145-1146`). That materially strengthens the case for documenting semantic limits of `OptimizationMode` instead of talking mainly about emitted ordering.

Third, the supplied bundle shows nearby serde-facing enums using explicit wire-format attributes. `CompositeGpuPrescanObjectives` is declared with `#[serde(rename_all = "snake_case")]` (`crates/app_core/src/composite_mo.rs:92-98`), while `ObjectiveKind` and `OptimizationMode` are plain derives with no rename attributes (`crates/optimizer/src/lib.rs:9-28`). That does not prove a bug, but it does prove that enum wire spelling is part of the nearby request contract story and should be treated deliberately.

Fourth, bundle-local search did not surface live users of `OptimizationResult` or `OptimizationHeatmap` in the supplied context beyond the crate root and repo map, while it did surface direct use of the enums in request DTOs. That weakens the third-pass emphasis on result ordering as the main local concern and strengthens the case for focusing on DTO shape semantics, wire compatibility, and future-proofing.

Finally, external guidance from the Rust API Guidelines and Serde docs supports that emphasis. The documentation guidance says public items should document usage and failure behavior, and the future-proofing guidance says traits that are not meant for downstream implementation should be sealed. Serde’s field and container attribute docs are also directly relevant here because this file exposes derived serde enums and DTOs whose field names, variant spellings, omission behavior, and later evolution are observable API.

## What Fourth Pass Sharpened

### What the third-pass packet got right

The third-pass packet was right about three important things.

It correctly treated this file as public-surface-sensitive rather than as a performance target. There is no meaningful CPU or GPU optimization win inside `lib.rs` itself.

It also correctly warned against casual changes to DTO shape. Even in the supplied bundle, `ObjectiveKind` and `OptimizationMode` are already imported into app-core request types, so public surface churn would not stay local.

And it correctly resisted broad refactoring. Nothing in the live source justifies redesigning the crate root just because it is small.

### What it missed or underweighted

The strongest missed point is that the real contract pressure here is **serde-visible vocabulary and future evolution**, not heap ordering.

`ObjectiveKind` and `OptimizationMode` are defined as serde enums at `crates/optimizer/src/lib.rs:9-28`, and they are already consumed by request DTOs in `composite_mo.rs`. That means their exact spellings and semantics are part of IPC / persistence / request compatibility. The supplied context even shows one caller ignoring `OptimizationMode` entirely for multi-objective execution today (`crates/app_core/src/composite_mo.rs:1145-1146`). So the biggest documentation gap is not “how top-k ordering works”; it is “what these modes mean, where they are actually honored, and what callers should not assume.”

The second missed point is the unsealed public trait. `BacktestEngine` is public at `crates/optimizer/src/lib.rs:30-32` and has no rustdoc describing whether downstream implementations are intended. That is the strongest future-proofing constraint in the file. If outside crates start implementing it, adding methods later becomes much harder. The third-pass packet did not identify that as the dominant evolution risk.

The third missed point is DTO invariant clarity. `OptimizationResult` uses parallel and mode-dependent fields:

- `best_score` may or may not exist.
- `top_scores` is logically coupled to `top`.
- `all_scores` is logically coupled to `all`.
- `top` and `all` are tuple pairs, which serialize as positional tuples rather than named entry objects.
- `heatmap` is optional, and `OptimizationHeatmap` itself flattens matrix-like data into `Vec<Option<f64>>` with dimension metadata stored separately.

Those are not fatal design flaws, but they are real observable contract details. The file does not document any of them.

A smaller but useful nuance the third pass also missed is that `OptimizationHeatmap` already contains compatibility-aware serde hardening on later-added optional fields: `mean_values` and `sample_counts` use `skip_serializing_if` plus `default` (`crates/optimizer/src/lib.rs:81-84`). That is evidence that compatibility already mattered here. The next justified step is not a structural rewrite; it is to pin the serde contract with focused tests instead of leaving the contract entirely implicit.

### Claims that were too strong or too weak

The third-pass emphasis on BinaryHeap / slice sorting / emitted ordering was too strong for this specific file. `lib.rs` does not implement heap maintenance or ranking logic. Those concerns are real somewhere in the crate, but the evidence is better matched to `grid`, `stream`, or downstream aggregation code than to this root contract file.

The third-pass bucket rationale was too weak in a different direction. Calling this “document only” undersells two bounded, nonbreaking hardening opportunities that are squarely appropriate for a public contract surface: explicit contract tests for serde shape and a minimal invariant guard for batch cardinality at the adapter boundary.

### Safe-now hardening

I would treat the following as the justified bounded work for this file now.

First, add real rustdoc to every public item in `lib.rs`, but make it contract-specific rather than generic. The docs should state:

- `ObjectiveKind` names a metric only; maximize vs minimize is caller-defined.
- `OptimizationMode` is a request vocabulary, not a promise that every caller honors every mode.
- `BacktestEngine::eval_batch` is expected to return one `Metrics` row per input combo in input order, and any panic / validation assumptions belong in the docs.
- `CpuEngine` is a thin adapter over `ta_strategies::double_ma::double_ma_batch_cpu`, not a separate engine implementation.
- `OptimizationResult` field-presence rules and pairing invariants (`top` with `top_scores`, `all` with `all_scores`) are deliberate contract surface.
- `OptimizationHeatmap` uses flattened storage plus dimension metadata, and its optional aggregate arrays are optional compatibility-bearing fields.

Second, add crate-local contract tests for the serde surface. That is the cleanest nonbreaking hardening step the earlier packets underweighted. At minimum, pin:

- enum round-trips for `ObjectiveKind` and `OptimizationMode`, so accidental wire-format drift is caught;
- sparse deserialization of `OptimizationResult` with omitted optional fields;
- sparse deserialization of `OptimizationHeatmap` with and without `mean_values` / `sample_counts`.

Third, if this file gets one code-level guard beyond docs and tests, make it extremely small: a debug-only cardinality assertion in `CpuEngine::eval_batch` that the delegated batch evaluator returns the same number of rows as `combos.len()`. That is a real implied contract of a batch engine, it is low risk in release builds, and it is much better targeted than any attempt to redesign the public DTOs in this pass.

That combination is enough to justify changing the bucket from `Document only` to `Small hardening`.

### Larger redesign still deferred

The file still does **not** justify a deeper API rewrite in this pass.

Do **not** try to seal `BacktestEngine` in a small hardening pass. That may be the right long-term design if downstream implementation is not intended, but making a currently public trait sealed is a compatibility event, not a patch cleanup.

Do **not** replace the tuple-based `top` / `all` entries with named entry structs in this pass, even though that would make the JSON shape more self-describing. That would change persistence and consumer expectations.

Do **not** add `#[non_exhaustive]`, `rename_all`, or new enum spellings casually. Those are all wire-contract or match-behavior changes that need coordinated downstream review.

Do **not** retype `CpuEngine` around richer MA identifiers or change the trait to return `Result`. Those may be reasonable future API improvements, but they are outside the scope of a conservative fourth-pass hardening pass on a Tier-3 file.

## Implementation Priority

The implementation priority should be narrow and explicit.

Priority one is contract documentation plus serde contract tests. That is the highest-value work because it improves the public surface without changing behavior.

Priority two, only if the file is already being touched for code, is the debug-only batch-cardinality assertion in `CpuEngine::eval_batch`.

Everything else should stay deferred. There is no direct performance lane to chase here, no GPU ABI to harden here, and no reason to spend compatibility budget on public-surface redesign.

## Verification If Changed

If this file changes, verification should stay proportional to the risk.

At minimum:

- `cargo test -p ta_optimizer -- --nocapture`

If serde contract tests are added, ensure they explicitly cover omission / round-trip behavior for:

- `ObjectiveKind`
- `OptimizationMode`
- `OptimizationResult`
- `OptimizationHeatmap`

If the optional debug assertion is added to `CpuEngine`, also run representative caller coverage that exercises the shared Double-MA CPU adapter path, because that is the one behavioral seam in this file.

No special GPU perf, CUDA ABI, or report-generation verification is justified for a `lib.rs`-only hardening pass.

## Regression And Feature-Preservation Constraints

Preserve the current serde-visible shape unless consumers and saved artifacts are updated in lockstep. That includes enum spellings, public field names, optional-field omission behavior, and tuple ordering inside `top` / `all`.

Do not add `deny_unknown_fields` to the DTOs. This file is a compatibility surface, and aggressively rejecting unknown fields would make future evolution and backward/forward artifact handling more brittle.

Do not over-document `OptimizationMode` as if every consumer honors every variant. The supplied `CompositeMo` context already shows that assumption would be false today.

Do not treat `BacktestEngine` as an internal-only trait in code unless you are willing to do a real compatibility review. As written, it is public and part of the crate surface.

Do not make performance claims for this file. The only meaningful risk here is compatibility: request decoding, persistence shape, downstream trait evolution, and implicit DTO invariants.

## Fourth-Pass Conclusion

The current third-pass packet was **directionally right but not sharp enough**. It was right to stay conservative and to avoid broad refactors. It was not right to center the rationale on ordering behavior, because `crates/optimizer/src/lib.rs` is primarily a public contract file, not the ranking implementation.

I would **change** the action bucket from `Document only` to **`Small hardening`**.

That does **not** mean redesigning the crate root. It means doing the narrow work this file actually justifies: document the public contract precisely, pin the serde contract with focused tests, and optionally add one debug-only cardinality assertion at the batch adapter boundary. Anything larger than that risks semver, IPC, and persistence regressions without enough payoff for a Tier-3 file.
