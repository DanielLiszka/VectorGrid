# Fourth-Pass Review: `crates/app_core/src/composite_validate.rs`

- **Review date:** `2026-04-15`
- **Source file:** `crates/app_core/src/composite_validate.rs`
- **First-pass packet:** `Repo Review/backend-file-reviews/crates/app_core/src/composite_validate.rs.review.md`
- **Second-pass packet:** `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_validate.rs.review.md`
- **Third-pass packet:** `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_validate.rs.review.md`
- **Third-pass action bucket:** `Larger refactor but defer`
- **Fourth-pass action bucket:** `Small surgical refactor`
- **Packet status:** `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because the third-pass packet changed the action bucket from `Small surgical refactor` to `Larger refactor but defer`, and that escalation needed pressure-testing against the live source.

After rereading the file, the third pass was directionally right about two things: the public contract is broader than the app-core implementation actually honors, and route-diagnostic handling is observably biased toward the first candidate family. But the bucket change itself is too strong. The live source still leaves a bounded, low-regression hardening path available inside this file: deterministic route-diagnostic aggregation, local reopt completeness checks that remove production panic paths, and rustdoc that accurately describes wrapper-owned or ignored request fields. The larger DTO split remains worthwhile, but it is not the smallest correct next move.

## Fourth-Pass Research Log

I reread the live source with emphasis on the public DTOs and entrypoints (`composite_validate.rs:9583-9765`, `12262-12931`), the first-family GPU dispatcher (`9309-9360`), per-candidate CPU execution (`9894-10056`, `12365-12616`), and the GPU reopt helpers that still contain production `panic!` lookups (`1460-1470`, `2023-2039`, `2979-2998`, `3486-3505`, `4731-4750`, `5796-5812`, `6968-6987`, `7754-7773`, `8251-8270`, `9011-9030`). I also checked the public re-export surface in `lib.rs:138-141`, because that makes request/result cleanup compatibility-sensitive.

For standards, I used official guidance rather than informal blog posts. The [Rust API Guidelines dependability guidance](https://rust-lang.github.io/api-guidelines/dependability.html) is relevant here because it explicitly says APIs should validate inputs whenever practical and should not silently accept broader contracts than they honor. The [Rust API Guidelines documentation guidance](https://rust-lang.github.io/api-guidelines/documentation.html) matters because these public DTOs and entrypoints currently have no rustdoc despite being re-exported. The Rust Book’s [“To panic! or Not to panic!”](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html) section is the right reference for the remaining GPU reopt panic sites, because it treats `Result` as the default for recoverable failures and reserves `panic!` for genuine contract-violation or test-only cases. For GPU-sensitive changes, NVIDIA’s [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/) remains the right source for the APOD cycle and for verification discipline: benchmark-sensitive cleanup should be measured, and correctness/performance checks should be run against known-good reference outputs before and after optimization.

One important scope note: I did **not** let unbundled wrapper or pipeline claims carry the bucket by themselves. The current bundle re-demonstrates the app-core side of the `cross_ticker` and `validation_context` drift, but it does not itself re-demonstrate the wrapper-side `runtime_ms` overwrite claim. Likewise, it shows one explicit SPA skip path in this file, but not the full `validation_pipeline.rs` contract. Those may still matter repo-wide, but they are not strong enough in this bundle to justify keeping this file in the larger bucket on their own.

## What Fourth Pass Sharpened

### The current third-pass packet got the core diagnosis right, but over-escalated the bucket

The third-pass packet was right to stop treating this as a docs-only cleanup. It correctly recognized that `cross_ticker` and `validation_context` make the public request appear stronger than the entrypoint actually is, and it correctly identified that route-diagnostic state is keyed to the first candidate family rather than the full batch.

Where it goes too far is the conclusion that this must now be treated as `Larger refactor but defer`. The live source still admits a bounded hardening path that preserves current behavior for successful runs and avoids public-contract narrowing. That is the signature of `Small surgical refactor`, not of “only redesign remains.”

### A safe mixed-family route-diagnostics fix exists, and the current packet missed how small it can be

The existing helpers only clear and collect route diagnostics for the first candidate family (`327-355`). The main validation flow, however, still evaluates candidates one by one through `run_validation_candidates`, and `build_candidate_series` validates only the per-candidate strategy shape, not batch homogeneity (`9894-10056`, `12365-12616`). So mixed-family CPU batches are not hypothetical. They are allowed by control flow.

The third-pass packet is therefore right that mixed-family requests can miss later-family diagnostics or retain stale state from a prior run. But it understates how small the fix can be. The least risky fix is **not** to reject mixed-family requests. That would be a semantic narrowing of the public API, and the live source gives no evidence that callers are forbidden to batch heterogeneous families on the CPU path.

The smaller and safer fix is to clear **all currently diagnostic-capable families** unconditionally at run start and then collect nonempty diagnostics from all of them in a fixed order at run end. In this file today, that means the ASR, VCC, VEMR, VCET, KQSR, and KVDF collectors that are already imported and wired (`2-5`, `115-149`, `327-355`). That eliminates stale carryover without changing request legality. The collection order should be deterministic to avoid artifact churn.

The current packet also missed that there is already a natural targeted regression test for this: build a mixed-family request that combines the two families with existing forced-fallback route-diagnostic fixtures, KQSR and KVDF (`22119-22179`, `23684-23744`). That would lock the exact bug without needing a redesign.

### The first-family GPU dispatcher is real, but it is not the load-bearing reason to keep the larger bucket

The GPU dispatcher chooses a family from the first candidate (`9352-9360`), which is exactly why the third-pass packet flagged it. But the live source also shows that the family-specific GPU helpers revalidate uniformity internally. The ASR helper is representative: it iterates the whole batch and errors unless all candidates use ASR and share the required uniform parameters (`788-860`).

That means the first-family GPU dispatcher is not the main correctness hole for metrics. Mixed-family GPU batches already fall back or error because the family-specific prep rejects them. The observable bug in the current file is the route-diagnostic state handling around that fallback path, not silent wrong metric computation. That distinction matters, because it turns the immediate fix back into a small one.

### `cross_ticker` is even weaker at the app-core layer than the current packet says, but the correct next step is documentation, not field surgery

The public request exposes `cross_ticker` (`9604`), candidate results expose `cross_ticker` and `cross_ticker_pareto` (`9655-9657`), and the top-level result exposes `cross_ticker` (`9761`). But inside this file, the only live behavior tied to `cross_ticker` is the GPU feature blocker (`615-619`). Candidate assembly hardcodes the per-candidate cross-ticker outputs to `None` on both the GPU-assisted and CPU-only branches (`12477-12478`, `12606-12607`), and the final result hardcodes the top-level summary to `None` (`12927`).

So the second-pass “route-sensitive capability boundary” framing was too generous. Within **this** entrypoint, `cross_ticker` is not a capability the CPU path “still owns.” The core function never produces cross-ticker stats at all. On a GPU request it only influences fallback messaging before the code continues into a CPU path that still ignores the feature.

The third-pass packet is right that this is public-surface drift. But it is too strong to treat that drift as forcing immediate structural separation. The request and result types are public and re-exported from `lib.rs:138-141`; removing or repurposing fields now would be a breaking IPC/serialization change. The safe-now action is rustdoc and module-level contract text that says, plainly, that `cross_ticker` is outer-orchestration state at present and is ignored by the app-core blocking entrypoint.

A later redesign can and probably should split wrapper-facing DTOs from a narrower core request/result surface. That redesign should stay deferred.

### `validation_context` is pure surface drift here, but core-side rejection would be the risky move

`validation_context` is part of the public request DTO (`9626`), yet the entrypoint never uses it. The run function copies many request fields into locals (`12320-12331`), but `validation_context` is absent from that set and there is no in-file execution-path read of the field.

That means the third-pass packet is right that the current type suggests stronger ownership than this file actually implements. But the safe response is still documentation and, later, deprecation or DTO separation. Adding a new hard error at the core boundary would be behavior-changing, and the current bundle does not prove that all wrapper or direct callers already strip or avoid the field.

### The highest-confidence code fix still available is the GPU reopt panic cleanup, but the implementation detail matters

The third-pass packet underweighted the strongest bounded code fix still left in the file: the ten production `panic!` sites in the GPU reopt helpers (`1466`, `2034`, `2990`, `3497`, `4742`, `5807`, `6979`, `7765`, `8262`, `9022`). These are all on the public library execution path, not in tests. A representative example is the base-composite lookup closure in `1460-1470`, which panics if a `(start, end)` window or MA pair is missing from the prebuilt metric cache.

The current packet was directionally right that this should not remain a production panic path in a `Result<_, String>` API. But “replace each `panic!` with `Err(...)`” is slightly underspecified here because the panic sits inside the nonfallible closure passed to `compute_reopt_walk_forward_stats`. The smallest correct implementation is therefore **not** a broad redesign of `validation_pipeline` just to make the closure fallible.

The smallest correct implementation is local completeness validation before the helper call. For each reopt helper, after each per-range GPU evaluation is produced, verify that the expected number of metrics was returned or that every expected pair/key exists in the range map. If that invariant fails, return `Err(...)` **before** entering `compute_reopt_walk_forward_stats`. Once the completeness invariant is established locally, the downstream lookup closure becomes a real invariant instead of a panic-backed hope.

That is still a surgical refactor. It is localized to this file, preserves successful behavior, and avoids widening a shared pipeline contract prematurely.

### `max_candidates` is more order-sensitive than the current packet stressed

The run entrypoint rewrites `max_candidates == 0` to `32` and truncates oversized candidate lists **before** any candidate validation happens (`12290-12295`). That means the request contract is order-sensitive. A later invalid candidate can be silently dropped instead of rejected, while an earlier invalid candidate still fails the run.

The current in-file invalid-candidate test covers only the “bad candidate retained” case: it passes `[bad, ok]` with `max_candidates: 1` and expects an error (`16180-16202`). It does not cover the mirror case where the bad candidate is truncated away and the run succeeds.

This is not a redesign issue, but it is a real public-surface behavior edge. The third-pass packet underweighted it. The safe-now move is to document the truncation order explicitly, or add a focused test that locks the current semantics so future cleanup does not accidentally change them.

### The SPA point is too weakly supported in this bundle to justify the larger bucket here

The third-pass packet says this file inherits the stronger SPA ambiguity from `validation_pipeline.rs`. That may still be true repo-wide, but it is not strong enough in the current bundle to carry the bucket. In this file, when SPA is requested and there are not enough post-warmup bars, the code returns a concrete `SpaValidationStats` object with `skipped_reason: Some("spa_skipped_not_enough_bars_after_warmup")` rather than silently using `None` (`12788-12887`).

So the bundle does not show this file as the primary owner of a bucket-changing SPA contract bug. At most, it shows a cross-file follow-up that should stay aligned with any future `validation_pipeline.rs` cleanup.

## Implementation Priority

The third-pass bucket should **change back** to `Small surgical refactor`.

### Safe-now hardening

First, fix route-diagnostic handling without changing request legality. Clear all currently diagnostic-capable families at run start, collect all nonempty diagnostics in deterministic order at run end, and add one mixed-family regression test that proves later-family diagnostics are no longer dropped.

Second, remove the ten production GPU reopt panic paths by adding local cache-completeness validation before each call to `compute_reopt_walk_forward_stats`. Do not redesign the shared pipeline API unless another validation owner independently needs the same change.

Third, add rustdoc to the public DTOs and entrypoints that are re-exported from `lib.rs:138-141`. The docs should state three things explicitly: `cross_ticker` is not executed by this app-core entrypoint, `validation_context` is outer-orchestration context rather than a core execution input, and `max_candidates` normalization/truncation happens before candidate validation.

Fourth, add one small contract test for truncation order. The current invalid-candidate test only locks one half of the behavior.

### Larger redesign still deferred

A real DTO split between wrapper-owned orchestration state and app-core execution inputs is still the right long-term cleanup. That includes deciding whether `cross_ticker`, `validation_context`, and any wrapper-owned timing/provenance fields should remain on the app-core request/result surface at all.

Breaking this file into smaller family-specific or GPU-vs-CPU units is still deferred. The file is unquestionably too broad, but contract stabilization should come before decomposition so the extraction does not freeze the wrong boundaries.

The benchmark-sensitive GPU cleanup items remain deferred as well. That includes the `VCC`/`CARM`/`CDMC` “GPU route but CPU exact metric source-of-truth” behavior and the family-specific GPU prep reuse work. Those are real opportunities, but they need profiling and parity evidence first.

## Verification If Changed

At minimum, rerun the existing core smoke and contract anchors that already cover the shared validation path:

- `composite_validation_smoke_runs_and_returns_stats`
- `composite_validation_iterative_engine_emits_delta_fields`
- `composite_mo_candidates_validate_consistently`
- `composite_validation_rejects_invalid_candidate_params`
- `composite_validation_reopt_walk_forward_smoke`

If route-diagnostic handling changes, rerun the existing single-family diagnostics tests and add one mixed-family coverage test:

- `kqsr_validation_gpu_kernel_reports_route_diagnostics`
- `kvdf_validation_gpu_kernel_reports_route_diagnostics`
- new mixed-family KQSR+KVDF route-diagnostics test that proves both families survive one run

If the GPU reopt panic cleanup is implemented, rerun the family reopt parity anchors that already exist in this file:

- `vcet_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `asr_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `rocb_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `vemr_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `vcc_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `kqsr_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `carm_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `kvdf_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `cdmc_validation_gpu_kernel_reopt_walk_forward_matches_cpu`
- `mkcs_validation_gpu_kernel_reopt_walk_forward_matches_cpu`

If only rustdoc and comments change, compile/test drift is low, but the public-surface docs should still be reviewed against the live behavior in `12277-12931` rather than copied from wrapper assumptions.

## Regression And Feature-Preservation Constraints

Preserve mixed-family CPU batch acceptance. Nothing in the live source establishes that heterogeneous candidate batches are illegal, and the main validation loop already evaluates them one candidate at a time. Do not reject mixed-family requests just to make route diagnostics easier.

Preserve the current serialized request/result shape unless a compatibility-reviewed redesign is underway. These DTOs are public, derive serialization traits, and are re-exported from `lib.rs`. Field removal or semantic repurposing is therefore an IPC and persistence risk, not a cosmetic cleanup.

Do not add new core-side hard errors for `cross_ticker` or `validation_context` without confirming wrapper and direct-caller behavior first. Documentation and future deprecation are safer than silently turning previously accepted requests into failures.

Keep route-diagnostic collection deterministic. If the fix aggregates multiple family collectors, the output order should be fixed so that artifacts and test snapshots do not churn needlessly.

Do not “clean up” benchmark-sensitive route labels, fallback wording, or the `VCC`/`CARM`/`CDMC` GPU/CPU exact-metric split during the same pass. Those changes affect benchmark interpretation and likely downstream artifact expectations.

## Fourth-Pass Conclusion

The current third-pass packet correctly identified real contract drift and real mixed-family route-diagnostics leakage, but its bucket escalation is too strong.

This file should move back to `Small surgical refactor`. The live source still has a bounded, evidence-backed hardening path that does **not** require immediate wrapper/core separation: deterministic all-family route-diagnostic handling, local reopt completeness checks that remove ten production panic paths, and public docs that accurately describe ignored orchestration fields and truncation behavior.

The larger redesign still belongs on the roadmap. A narrower app-core DTO surface, better wrapper/core separation, file decomposition, and deeper GPU-path cleanup all remain sensible deferred work. They just are not the next mandatory step.
