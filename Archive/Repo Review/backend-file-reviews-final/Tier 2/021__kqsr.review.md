# Final Consolidated Review: `crates/app_core/src/kqsr.rs`

- Review date: `2026-04-15`
- Source file: [kqsr.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/kqsr.rs:1>)
- First-pass packet: [kqsr.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/kqsr.rs.review.md:1>)
- Second-pass packet: [kqsr.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kqsr.rs.review.md:1>)
- Third-pass packet: [kqsr.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kqsr.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/021__crates__app_core__src__kqsr.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://serde.rs/field-attrs.html>
  - <https://serde.rs/container-attrs>

## Consolidated Judgment

The fourth-pass packet is valid and should drive the next implementation pass. The live file confirms its most important corrections to the earlier reviews: `build_kqsr_desired_positions()` really does conflate two cases today, but `start_t >= n` can be a legitimate no-trade terminal state and should not be turned into an error casually; `validate_kqsr_params()` allows thresholds that the runtime then interprets against a fixed neutral `50.0` and overlapping PRB tails; and `resolve_output_id()` still falls back to the first declared output after exact and fuzzy matching. The fourth pass also correctly downgrades visibility narrowing from “probably safe” to “public-surface change.”

No wrong `review_fourth_pass.md` is present in this folder. The one caveat that should survive into the final guide is the fourth-pass author’s own caution: the earlier cross-file claim about a live metrics path bypassing `validate_kqsr_params()` was not re-proved from the bundle and should not be treated as the top implementation target unless re-verified directly.

## Safe-Now Priority

- Add a focused test for the legitimate no-trade case where `compute_kqsr_start_t(..., trade_on_next_bar = true)` yields `start_t == close.len()` and `build_kqsr_desired_positions()` returns all-flat.
- If malformed-shape hardening is desired, do it in the existing fallible signal builders rather than by changing the public desired-position builder to `Result`.
- Document the current validator/runtime contract explicitly:
  - exits use a fixed neutral of `50.0`
  - `prb_threshold > 0.5` creates overlapping long/short acceptance regions
  - the desired-position helper may legitimately return all-flat when no tradeable bar remains
- If output-resolution hardening is touched, keep it narrow: multi-output no-match should become an error or diagnostic, while single-output fallback may remain.
- If route observability matters for this strategy, add a diagnostic-only signal when a GPU-preferring call cannot attempt the CUDA helper because prepared inputs are absent.

## Deferred Work

- Do not convert `build_kqsr_desired_positions()` wholesale to `Result<Vec<i32>, String>` as the first move.
- Do not tighten `oversold` / `overbought` or `prb_threshold` acceptance ranges without a product-level compatibility decision.
- Do not narrow the helper surface from `pub` to `pub(crate)` in a local cleanup patch.
- Do not add `deny_unknown_fields` or other serde-tightening changes casually.

## Verification If Changed

- `cargo test -p ta_app_core kqsr -- --nocapture`
- `cargo test -p ta_app_core kqsr_optimization_smoke_runs -- --nocapture`
- Add and run targeted tests for:
  - `start_t == len` / all-flat behavior
  - any new signal-length mismatch hardening in the fallible builders
  - chosen multi-output `resolve_output_id()` behavior
  - current `prb_threshold > 0.5` behavior, unless validation is intentionally tightened
  - diagnostic-only GPU fallback-not-attempted reporting, if added
- If CUDA-helper behavior changes, keep the representative CPU/GPU parity tests in scope.

## Regression And Feature-Preservation Constraints

- Preserve current KQSR entry/exit semantics unless a clearly broken contract is being corrected.
- Treat public-surface changes as compatibility changes, not style cleanup.
- Treat serde changes as persistence and IPC changes.
- Keep route-diagnostic changes diagnostic-only unless higher-level callers are re-audited deliberately.

## Final Conclusion

The implementation-facing conclusion is:

- keep `kqsr.rs` in `Small surgical refactor`
- treat the fourth pass as valid and more precise than the third pass
- preserve the legitimate no-trade `start_t >= n` path
- harden malformed-shape handling earlier in fallible builders, document current threshold semantics, and keep output-resolution tightening narrow
- defer Result-surface changes, validation tightening, visibility narrowing, and serde strictness
