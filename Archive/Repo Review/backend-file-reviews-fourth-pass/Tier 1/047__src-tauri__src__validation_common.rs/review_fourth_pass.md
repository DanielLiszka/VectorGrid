# Fourth-Pass Review: src-tauri/src/validation_common.rs

- Review date: `2026-04-16`
- Source file: `src-tauri/src/validation_common.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file stayed worth a fourth pass because it is small, but it is not low-consequence. `validation_common.rs` sits in the shared Tauri validation support layer, not in strategy-local leaf code, and the repo map still treats it as part of the validation entry surface rather than incidental glue. Its current responsibilities include cross-ticker dataset dedupe, candidate-key canonicalization, cross-ticker score folding, Pareto marking, and final ordering. Those are all places where a “cleanup” can quietly change validation outcomes, artifact matching, or route behavior.

The third-pass packet was directionally right to keep the bucket at `Small surgical refactor`, and that bucket should stand. The point of this pass was not to reopen the bucket lightly; it was to challenge whether the prior packet had picked the smallest safe fix. After re-reading the live file, the earlier packets, the surrounding validation pipeline, and the relevant Rust standard-library behavior, the answer is: the bucket stays the same, but one recommended direction was a little too aggressive, and one route-level consequence was underweighted.

## Fourth-Pass Research Log

1. Re-read the live source with attention to the contracts that actually move behavior: `normalize_path_key()` (`45-47`), `sanitize_cross_ticker_config()` (`129-158`), `candidate_key_canonical()` (`221-224`), the local quantile/median helpers (`236-290`), and `finalize_cross_ticker_candidates()` (`296-380`).

2. Re-read the first-, second-, and third-pass packets to separate what was already correct from what was only plausible. The earlier packets were strongest on the semantics-sensitive nature of path and ranking helpers, and weakest where they drifted toward implying a filesystem-backed fix without fully pricing its new contract.

3. Compared the local stats and ranking semantics with `crates/app_core/src/validation_pipeline.rs`. That file still uses the same rounded nearest-rank quantile indexing pattern (`638-659`) and still sorts finalized candidates by folded robust score (`350-357`). That makes a unilateral “median cleanup” here even less safe than it first sounds.

4. Checked nearby runtime identity behavior in `src-tauri/src/state.rs`. Loaded base datasets are matched using `data_cache::identity_for_source(...)` and then compared using both the original path string and `source_identity`, not by any shared lexical path-normalization helper (`214-222`, `720-738`). That is relevant because it weakens the case for dropping `std::fs::canonicalize()` into this helper as if the repo already had a single filesystem-identity model.

5. Checked retrieved validation-route tests in `double_ma_validate.rs`. The current suite explicitly treats `cross_ticker: Some(CrossTickerConfig { datasets: Vec::new(), ... })` as enough to force `GpuKernel->CpuOnly` fallback in direct validation tests. That means “sanitized to empty” is not operationally the same as “cross-ticker absent.”

6. Rechecked the relevant Rust standard-library behavior in official docs:
   - `std::path::MAIN_SEPARATOR` / `std::path::Path`: `/` is the primary separator on Unix, while `\` is the primary separator on Windows.
   - `std::fs::canonicalize`: returns an absolute canonical path and resolves symlinks.
   - slice `sort_by`: stable sort, preserving the initial order of equal elements.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet correctly identified that the path-key helper is not cosmetic. `normalize_path_key()` is used directly by `sanitize_cross_ticker_config()` (`129-134`), so any lossiness there changes which datasets survive into cross-ticker validation. The third pass was also right to keep the file in `Small surgical refactor` instead of escalating to a redesign bucket. The live source still does not justify larger algorithmic churn.

The earlier packet was also correct to treat nearest-rank median semantics and canonical JSON key generation as active behavior, not cleanup fodder. The local median remains `quantile(sorted, 0.5)` (`257-259`), and the app-core validation pipeline still uses the same rounded-quantile family rather than an arithmetic middle-pair average. Changing only this file would create cross-layer semantic drift, not “cleanup.”

### What the current packet still missed or underweighted

#### 1. The path-normalization bug is stronger than case-folding alone

The third-pass packet focused on lowercasing as the main cross-platform input-loss bug. That was real, but it was not the whole problem. The more important sharpened finding is that `normalize_path_key()` also rewrites every `/` into `\` (`45-47`). On Unix-like platforms, `/` is the actual path separator and `\` is an ordinary filename character. That means the helper can alias two lexically distinct inputs even when case is identical.

The concrete bad shape is not just `AAPL.csv` vs `aapl.csv`. It is also `data/a.csv` vs `data\a.csv`. On Unix, those are not equivalent strings or equivalent paths. The current helper makes them equal anyway. That is a stronger correctness finding than “Windows-biased case handling,” and it makes a pure “add a case-sensitive test” follow-up too narrow.

#### 2. The earlier `canonicalize` direction was too strong for this layer

The first and third packets were right to consult `std::fs::canonicalize()`, but they leaned a bit too close to treating it as the natural fix. For this file, that is overstated.

`sanitize_cross_ticker_config()` is currently a pure, in-place sanitizer over request data. Replacing or front-loading it with `canonicalize` would introduce filesystem I/O, existence dependence, absolute-path rewriting, symlink resolution, and new error behavior into a helper that currently cannot fail (`129-158`). That is not a “small surgical refactor” unless the repo intentionally wants a new filesystem-backed identity contract for cross-ticker dataset lists.

This matters even more because nearby repo code already has a stronger but different identity model: loaded-data reuse in `state.rs` uses `SourceIdentity`, not a generic canonical-path string (`214-222`, `720-738`). So the safe-now fix is not “canonicalize here.” The safe-now fix is: make the lexical normalization non-lossy on non-Windows platforms, document that it is lexical dedupe rather than filesystem identity, and leave true filesystem identity as deferred design work.

#### 3. Empty-after-sanitize configs still carry validation-route risk

This was the most important behavioral consequence underweighted by the earlier packet.

`sanitize_cross_ticker_config()` can remove every dataset entry and still leave a syntactically present `CrossTickerConfig` behind (`129-158`). The current validation tests in `double_ma_validate.rs` show that `cross_ticker: Some(CrossTickerConfig { datasets: Vec::new(), ... })` is still enough to trigger `GpuKernel->CpuOnly` fallback in direct GPU-kernel validation coverage. In other words, “empty datasets” and “no cross-ticker request” are not the same thing in the surrounding system.

That means a path-normalization fix is not only a correctness question. It is also a performance and compatibility question. If the caller preserves `Some(empty)` after sanitization, the request can still take the CPU-safe route. If the caller starts dropping empty configs entirely, that can change backend-route selection. The earlier packet mentioned downstream verification, but it did not call out this route-level consequence strongly enough.

The safe recommendation is therefore narrower and more explicit: if the product contract is “no surviving datasets means cross-ticker is absent,” that null-out behavior should be implemented and tested in the command/orchestration layer, not smuggled into this helper without route verification.

#### 4. `finalize_cross_ticker_candidates()` has implicit freshness and alignment invariants

The earlier packets focused on ranking semantics, but they underweighted the contract weakness in `finalize_cross_ticker_candidates()` itself.

The helper silently skips candidates when `accs.len() < candidates.len()` (`306-309`) and also skips any accumulator whose `sharpes` list is empty (`310-313`). In both cases, it leaves the candidate untouched. If this helper is only ever called with fresh candidate objects and perfectly aligned accumulators, that is fine. But the function does not enforce that assumption, and the type does not help it: `CrossTickerAcc` keeps all metric vectors public (`11-17`), `ds_n` is taken only from `sharpes.len()` (`310`), and there is no check that `dds`, `pnls`, `trades`, and `pnl_pos` are aligned with that count.

I am not recommending an API redesign here. The safe-now point is smaller: the one-shot/freshness/alignment assumptions are implicit today, and they should be pinned with debug assertions and tests instead of being left as silent contract debt.

#### 5. Non-finite metric folding is not aligned with the broader validation helper family

The earlier packets mentioned missing tests around median semantics, but the sharper issue is broader: the local cross-ticker helpers coerce selected non-finite float results to `0.0` (`236-283`), while the app-core validation pipeline’s nearest-rank selector returns the chosen value directly and tests it bit-for-bit against the shared sorted-quantile behavior (`638-659`, plus the quantile-parity tests nearby).

That does not automatically make the local behavior wrong. Cross-ticker aggregation may intentionally want a “non-finite becomes neutral zero” policy. But the divergence is real, undocumented, and currently untested in this file. So the right fourth-pass position is conservative: do not change it casually, but do stop pretending the only statistical contract question here is the even-cardinality median name.

#### 6. Tie behavior should be pinned, not “improved”

The first-pass packet correctly asked for tie-behavior testing, but the fourth-pass refinement is that the likely safe test is for *current stable behavior*, not for a new tie-break rule.

`finalize_cross_ticker_candidates()` finishes with `candidates.sort_by(...)` (`380`). In Rust, slice `sort_by` is stable. So equal `robust_score` candidates preserve whatever relative order they already had when they entered this function. Adding a secondary tie-breaker here would not be harmless determinism cleanup; it would be a ranking-policy change that could alter top-candidate order, report order, and replay expectations.

## Implementation Priority

### Safe-now hardening

1. **Fix `normalize_path_key()` as a platform-aware lexical helper, not as pseudo-filesystem identity.**
   - On Windows, slash harmonization and case-folding may still be acceptable if the product wants Windows-friendly dedupe.
   - On Unix-like platforms, do not rewrite `/` into `\`, and do not lowercase by default.
   - Add a brief doc comment that the helper is lexical request dedupe only, not canonical filesystem identity.

2. **Add tests that cover the actual cross-platform failure modes.**
   - A Unix-style separator-alias case such as `data/a.csv` vs `data\\a.csv`.
   - Platform-specific case behavior.
   - Stable equal-score ordering for `finalize_cross_ticker_candidates()`.
   - Non-finite metric folding semantics.
   - Canonical JSON parity against the app-core helper, if the app-core helper remains separate.

3. **Add caller-side handling for “sanitized to empty.”**
   - This is likely a `commands_validation` concern rather than a `validation_common.rs` concern.
   - The contract must be explicit: either keep `Some(empty)` intentionally and accept the current CPU-fallback implications, or drop cross-ticker entirely once no datasets survive.
   - Whichever contract is chosen, test it against current GPU-fallback expectations before merging.

4. **Harden `finalize_cross_ticker_candidates()` with cheap invariant checks.**
   - `debug_assert_eq!(candidates.len(), accs.len())` or equivalent.
   - `debug_assert!` alignment checks for the metric vectors and `pnl_pos <= ds_n`.
   - A comment that the helper assumes fresh candidate state and accumulator alignment.

5. **Add context to canonical-key errors.**
   - The earlier packet was right here. `candidate_key_canonical()` should not keep returning bare serializer strings if a small context wrapper can make diagnostics clearer without changing outward behavior.

### Larger redesign still deferred

1. **Repo-wide path-identity consolidation.**
   The earlier packet was right that validation, campaign, and ML-signal path normalization should not drift forever. But that is cross-file work and should stay deferred until the repo chooses one explicit identity model.

2. **Any move to true filesystem-backed identity.**
   If the product eventually wants symlink-aware, absolute-path, existence-dependent dataset identity for cross-ticker requests, that should be a deliberate orchestration-layer change. It is not a safe local substitution for `normalize_path_key()`.

3. **Stats-helper centralization with app-core.**
   The local non-finite folding and nearest-rank behavior are close enough to app-core to create drift risk, but not close enough to rewrite casually in this pass. If the repo wants one canonical quantile/median policy, that belongs in a coordinated shared-helper pass.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app validation_common -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
- If cross-ticker presence/emptiness semantics change, run the direct GPU-fallback validation coverage that currently asserts `cross_ticker: Some(empty)` still yields `GpuKernel->CpuOnly` in `double_ma_validate.rs`.
- If path-identity policy is unified across workflows, also run the affected `commands_ml_signal` and `commands_campaign` suites.
- If candidate-key or ranking semantics move at all, recheck report/export surfaces that consume `CrossTickerStats` and finalist ordering.

## Regression And Feature-Preservation Constraints

- Preserve the current nearest-rank quantile/median behavior in this file unless the app-core validation pipeline moves with it.
- Preserve array-order-sensitive canonical JSON behavior. Sorting arrays “for convenience” would change candidate identity.
- Preserve current stable equal-score ordering unless a repo-wide ranking-policy change is explicitly intended.
- Do not introduce filesystem I/O, path-existence requirements, or symlink resolution into `sanitize_cross_ticker_config()` as an unannounced behavior change.
- Treat empty cross-ticker configs as route-sensitive. Changing whether `Some(empty)` survives can change validation backend selection and therefore performance characteristics.
- There is no direct GPU ABI risk in this file, but there is real validation-route and performance risk because cross-ticker presence already participates in CPU fallback safety.
- Candidate-key string format should remain byte-stable unless all producing and consuming layers move together; it feeds cross-layer candidate matching and downstream summaries.

## Fourth-Pass Conclusion

The third-pass bucket should stand.

The current third-pass packet got the main call right: this file is still `Small surgical refactor`, not a redesign candidate. It also correctly elevated path-key handling to a correctness issue. What the fourth pass changes is the shape of the recommended fix. The safer hardening is narrower than “move toward `canonicalize`”: first remove the lossy lexical behavior, especially the Unix-hostile `/` -> `\\` rewrite, then make the empty-after-sanitize route contract explicit at the caller boundary, and add tests that pin current ranking and non-finite semantics.

So the action bucket remains unchanged, but the packet should be sharpened in two ways: the path-loss bug is stronger than case-folding alone, and the fix should stay lexical-and-local for now, with route-sensitive caller handling tested separately. That is the highest-confidence way to improve this file without causing validation-order, compatibility, or GPU-route regressions.
