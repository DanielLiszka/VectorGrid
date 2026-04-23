# Fourth-Pass Review: `crates/app_core/src/validation_pipeline.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/validation_pipeline.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_pipeline.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_pipeline.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_pipeline.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file remained worth a fourth pass because the third-pass packet correctly identified SPA-state ambiguity, but it did not fully answer the Tier-1 question in this bundle: does the current defer bucket still hold once the live source is pressure-tested for smaller bounded hardening steps?

The answer is yes, the current bucket should stand. The larger semantic cleanup is still too cross-cutting to treat as a safe local refactor. However, the fourth pass found a better safe-now target than the third-pass packet emphasized: this file has multiple implicit lifecycle invariants that are currently enforced only by caller discipline. Those invariants can be hardened with debug assertions, targeted tests, and clearer contract notes without changing public behavior.

The fourth pass also found two places where the prior packets were not quite sharp enough. First, the run-verdict contract is weaker than the SPA-only focus suggested: several top-level summary fields are anchored to the top-ranked candidate, not the first approved candidate. Second, the earlier concern about unstable Rayon collection order is overstated for this exact use site.

## Fourth-Pass Research Log

I re-read the live source with emphasis on the contract-bearing paths rather than the broad module split:

- `compute_probability_metrics()` (`268-297`)
- `mark_robust_pareto()` and `fold_cscv_into_robust_and_sort()` (`300-357`)
- `apply_candidate_hard_gates()` and `build_run_verdict()` (`359-579`)
- `parallel_collect_validation_candidates()` (`581-616`)
- `compute_walk_forward_stats()` (`668-736`)
- `finalize_with_probability_and_optional_cscv()` (`1367-1447`)
- `compute_spa_validation_stats()` (`1519-1627`)
- file-local tests (`1726-2312`)

I also re-checked the current packet against the earlier review history to separate genuine live-code issues from packet drift.

For external validation, I used primary or implementation-facing references only:

- Rayon docs for `ParallelIterator`, `IndexedParallelIterator`, and `enumerate()` on `docs.rs`, to verify whether the second-pass order warning actually applies to `&[P].par_iter().enumerate().collect()`.
- Hansen’s SPA reference and the `arch` SPA documentation, to sanity-check whether the current single-`p_value` stationary-bootstrap contract is a local implementation choice or a broader methodological simplification.
- Bailey / López de Prado on PSR and DSR, only to confirm that claims about statistical fidelity should stay bounded to what this file actually computes versus what it delegates.

That research changed two judgments from the earlier packets:

1. The Rayon-order warning should not be carried forward as a core risk for this exact function.
2. The third-pass proposal to add richer SPA status is directionally sensible, but it is not a truly local hardening step once shared serialization and downstream consumers are considered.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right on the most important broad point: `compute_spa_validation_stats()` overloads `Option` too aggressively. The live code returns `None` both when inputs are structurally invalid (`1527-1529`) and when cancellation fires during bootstrap (`1588-1590`), while `build_run_verdict()` treats `None` as plain absence and not as “requested work failed or was aborted” (`522-539`). That is a real contract weakness.

The third-pass packet was also right to prefer additive contract cleanup over in-place semantic rewrites. This file is shared validation infrastructure, and the verdict and stats types are `Serialize`/`Deserialize`-backed public-facing data shapes (`39-95`). A careless semantic rewrite here would not be a local cleanup; it would be an IPC, artifact, and report-shape change.

### What the third-pass packet missed or underweighted

#### 1. The sharper safe-now hardening target is lifecycle invariants, not SPA status expansion

The third-pass packet focused on SPA state, but the stronger bounded hardening opportunity is the set of caller-owned invariants that this file currently assumes.

The clearest example is robust-score ownership. `mark_robust_pareto()` recomputes a score from metrics internally (`308-346`), but `fold_cscv_into_robust_and_sort()` does not use that same recomputation; it folds CSCV into the preexisting caller-owned `cand.robust_score()` and sorts on that (`350-356`). That leaves this file with two potential score sources of truth: the local formula used for Pareto marking and the externally seeded value used for final ranking.

That is not just generic trait broadness. It is a concrete invariant gap. If a caller forgets to seed `robust_score`, or seeds it with a stale or different formula, this file can mark Pareto dominance using one score while sorting and reporting with another. The safest bounded response is not to silently change ranking behavior now. It is to encode the invariant with targeted assertions and tests first.

A similar hidden invariant exists between `compute_probability_metrics()` and `build_run_verdict()`. Approval requires `min_track_record_length_bars()` to be present and satisfied (`448-457`), but `build_run_verdict()` does not defend that precondition itself. If a caller reaches run-verdict construction without first populating probability metrics and minimum track-record length (`268-297`), every candidate becomes unapproved by construction even if hard gates passed. The existing tests do not currently characterize that ordering requirement.

#### 2. `sample_obs` silently falling back to `2` masks caller bugs

`compute_probability_metrics()` uses `sample_obs.get(i).copied().unwrap_or(2).max(2)` (`288`). That fallback is easy to gloss over, but it is not benign. A caller length mismatch silently converts into a two-bar evidence assumption, which then feeds PSR/DSR and minimum-track-record computation (`289-296`).

That is exactly the sort of quiet fallback that makes a validation engine harder to trust in a public repository. The safest bounded hardening step here is a debug assertion and a targeted regression test, not a release-mode panic or a behavior change.

#### 3. The run-verdict summary is anchored to `out.first()`, not to the approved recommendation

This is the strongest contract issue the earlier packets still underweighted.

`build_run_verdict()` computes `first_approved_rank` from the filtered approved set (`448-462`), but the `top_candidate_*` summary fields and propagated warning flags are taken from `out.first()` (`485-497`). Those are not the same concept once the first approved rank is greater than `1`.

In other words, the run-verdict payload can say the run was approved while its `top_candidate_track_record_bars`, `top_candidate_psr`, `top_candidate_dsr`, and copied warning flags still describe a candidate that was rejected or not approved. That is a deeper issue than the coarse recommendation enum alone. It means the verdict payload currently mixes “best raw rank after robust-score sorting” with “first approved recommendation” inside one summary object.

This also explains why `ApprovedWithWarnings` is semantically overloaded. The branch is taken not only when warning flags exist, but also when the first approved candidate is not rank 1 (`556-559`). So the recommendation text can imply explicit warning-state even when the real condition was “the recommended candidate was not the top-ranked candidate after post-processing.” The current tests cover approved-with-warning flags and minTRL failure, but they do not cover the top-ranked-versus-first-approved mismatch.

#### 4. Empty walk-forward downside-ratio data currently collapses to a real `0.0`

`compute_walk_forward_stats()` pushes downside ratios only when `downside_vol_ratio_from_metrics()` returns `Some` (`699-700`). If every window yields `None`, the vector stays empty, and the median helper returns `0.0` on empty input (`643-656`, `727-728`).

That matters because both `robust_score_from_metrics()` and `risk_warning_flags_from_metrics()` treat any finite walk-forward downside ratio as real data (`179-187`, `212-220`). So an unavailable walk-forward downside-ratio path becomes an explicit `0.0`, which suppresses fallback to the in-sample ratio and removes any downside penalty.

This is a real contract smell, and it is more actionable than the earlier generic “statistical defaults are underdocumented” framing. But it is not obviously safe-now to fix in place, because changing the sentinel from `0.0` to a different representation would likely ripple into serialized stats, report consumers, and artifact compatibility.

### What the earlier packets overstated or weakly supported

#### 5. The second-pass Rayon ordering concern is overstated for this exact code path

The earlier packet warned that later consumers should not assume stable collection order from `parallel_collect_validation_candidates()`. That is too pessimistic for the live implementation.

This function iterates over a slice with `par_iter().enumerate()` (`597-605`). In Rayon, this is an indexed parallel iterator path, and the collection examples on `ParallelIterator` / `IndexedParallelIterator` show vector collection preserving the same element order as the sequential iterator, with `collect_into_vec()` likewise producing ordered vectors. So order instability is not the main risk here.

The real nuance is cancellation granularity, not ordering. The helper checks `cancel` before starting each item (`601-603`), but it does not provide structured mid-item cancellation itself. Any long-running per-candidate work must still cooperate separately if prompt cancellation matters.

#### 6. The third-pass statistical-label critique reached beyond what this file alone proves

The third-pass packet argued that PSR/DSR / minimum-track-record reporting here overstates methodological specificity. That concern is directionally plausible, but it is not fully supported by this file in isolation.

The live file delegates the actual PSR/DSR computation to `crate::validation_stats::compute_psr_dsr()` and minimum-track-record computation to `crate::sharpe_stats::minimum_track_record_length()` (`281-292`). What this file does clearly own is the choice to pass fixed arguments into minimum-track-record length and to merge those results into verdict logic. What it does not show, on its own, is the full underlying statistical implementation detail that would justify a stronger public claim about the PSR/DSR labels themselves.

So the stronger evidence-backed statement is narrower: this file hardcodes important PSR/DSR / minTRL inputs and verdict use, but the packet should not overstate what it can prove about the deeper statistical formulae without reviewing the delegated modules too.

#### 7. The third-pass “best bounded fix” for SPA status is probably not actually bounded

The earlier recommendation to distinguish `Skipped`, `Completed`, and `Aborted/Invalid` is directionally right, but it is not a tiny local change in this repo.

`SpaValidationStats` is part of the shared validation surface, and the repo context explicitly routes validation outputs through Tauri commands, reports, saved artifacts, and frontend consumers. Expanding SPA state means touching a shared serialized contract, not just swapping an internal enum inside this file. So that recommendation belongs in larger redesign still deferred, not in the safest immediate hardening bucket.

## Implementation Priority

### Safe-now hardening

The current third-pass bucket should stand, but there is a bounded hardening step that the packet missed.

The best immediate work is to make the pipeline’s implicit prerequisites explicit without changing shipped semantics:

1. Add debug-only invariants and focused tests around caller-seeded state.

   Concretely:

   - In or near `compute_probability_metrics()`, assert in debug builds that `sample_obs.len()` matches `out.len()` when sample observations are expected candidate-by-candidate, so the fallback-to-`2` path is no longer silent.
   - In or near `build_run_verdict()`, assert in debug builds that candidates which passed hard gates have `track_record_bars` and `min_track_record_length_bars` populated before approval logic runs.
   - In or near `finalize_with_probability_and_optional_cscv()`, assert in debug builds that the caller-provided `robust_score()` matches `robust_score_from_metrics(...)` within tolerance before CSCV folding and final sorting. That gives a safe way to detect score-source drift before centralizing ownership.

   These are high-value because they strengthen the file’s real weak spots without changing public result shapes.

2. Add targeted characterization tests for run-verdict identity semantics.

   The current test suite is good, but it still does not cover:

   - rank-1 candidate rejected, rank-2 candidate approved
   - `ApprovedWithWarnings` triggered because `first_approved_rank > 1` rather than because warning flags were propagated
   - `top_candidate_*` fields describing a different candidate from the approved recommendation

   These tests would not fix the contract mismatch, but they would make future changes deliberate instead of accidental.

3. Correct local documentation and future review guidance on parallel collection.

   The safe correction is not code; it is to stop warning about unstable output order here and instead document the actual nuance: order is stable on this indexed collection path, while cancellation remains cooperative only at item boundaries unless the per-item work also checks cancellation.

### Larger redesign still deferred

The bigger redesign remains deferred, and the bucket should stay unchanged.

1. Redesign `ValidationRunVerdict` so it no longer mixes top-ranked candidate data with approved-candidate data.

   The clean fix is likely additive: keep the existing top-ranked snapshot for compatibility, but add an explicit first-approved snapshot or recommendation-target summary. Renaming or repurposing the current `top_candidate_*` fields in place would be risky because these structs are serialized and likely flow into reports, artifacts, and IPC.

2. Redesign SPA status representation.

   The live `Option<SpaValidationStats>` contract still conflates “not requested,” “invalid inputs,” and “cancelled during bootstrap.” Fixing that should be done through an explicit status model, but that is a shared contract change and should be audited across all consumers first.

3. Make robust-score ownership single-source.

   Right now, this file both computes robust score locally for Pareto logic and trusts caller-owned `robust_score()` for final fold-and-sort. A cleaner design would make this module the single owner of post-candidate robust-score computation or split the trait so ownership is unambiguous. That likely changes ranking semantics and should not be slipped in as a hidden cleanup.

4. Make optional walk-forward risk measures explicit.

   The downside-ratio `0.0` sentinel issue points to a broader contract problem: some shared stats are using scalar fields where “unavailable” would be semantically distinct from “zero.” Fixing that probably means shared-type changes, serialization review, and report compatibility work.

5. Only expand SPA methodology labels or outputs if the product actually needs it.

   External SPA implementations often expose richer recentering choices and multiple p-value variants. This file currently exposes one p-value, one pass/fail bit, and one skipped reason. That narrower surface is not automatically wrong, but expanding it is a product and contract choice, not a safe backend-only cleanup.

## Verification If Changed

If only the safe-now invariant checks, tests, or comments change, the minimum verification should be:

- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

Add specific new tests for:

- short `sample_obs` / mismatched candidate count
- caller-provided `robust_score` drift versus `robust_score_from_metrics(...)`
- first approved rank greater than `1`
- run verdict where `top_candidate_*` differs from the approved recommendation
- SPA requested but unavailable versus SPA explicitly skipped

If any serialized output shape changes, widen verification beyond app-core unit tests:

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `npm --prefix web run parity:outputs`
- any artifact-loading or report-generation flows that consume validation verdicts

No GPU ABI risk is evident in this file. The real risks are validation semantics, ranking drift, IPC/report compatibility, and saved-artifact compatibility.

## Regression And Feature-Preservation Constraints

Preserve current validation behavior unless a bug or contract weakness is explicit. In practice that means:

- Do not rewrite `passed`, `approved`, `final_recommendation`, or `top_candidate_*` semantics in place without a downstream consumer audit.
- Do not silently change robust-score ownership or ranking order. Assert the invariant first; centralize later.
- Do not replace the current scalar sentinel behavior for walk-forward stats without checking serialization and report consumers.
- Do not treat the SPA status expansion as a file-local cleanup. It is a shared validation contract change.
- Do not carry forward the older warning that `parallel_collect_validation_candidates()` produces unstable order. That is not the primary risk on this indexed collection path.
- Do preserve the existing deterministic tests. They are doing real contract-lock work already; the gap is that a few important invariants are still untested.

## Fourth-Pass Conclusion

The current third-pass bucket should stand: `Larger refactor but defer` is still the right action bucket.

What changed in the fourth pass is not the bucket, but the priority inside it.

The third-pass packet was right that SPA-state ambiguity is real and that additive changes are safer than semantic rewrites. But it still underweighted a better bounded hardening step: encode the file’s implicit lifecycle invariants around caller-seeded `robust_score`, populated probability metrics, and aligned `sample_obs` before attempting any public-contract redesign.

The fourth pass also surfaced a more concrete verdict-contract weakness than the earlier packet emphasized. `ValidationRunVerdict` currently mixes two candidate identities: the top-ranked candidate in `out.first()` and the first approved candidate derived later. That is the real reason a larger redesign is still deferred. Fixing it cleanly means touching serialized output semantics, not merely polishing local control flow.

So the conservative outcome is:

- safe now: harden invariants with debug assertions, characterization tests, and clearer local contract notes
- still deferred: SPA status redesign, verdict identity cleanup, robust-score ownership centralization, and optional-stat representation cleanup

That preserves current behavior while making this file materially safer to evolve.
