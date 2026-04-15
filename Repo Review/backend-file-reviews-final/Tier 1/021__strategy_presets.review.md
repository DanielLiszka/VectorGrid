# Final Consolidated Review: `crates/app_core/src/strategy_presets.rs`

- Review date: `2026-04-15`
- Source file: [strategy_presets.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_presets.rs:1>)
- First-pass packet: [strategy_presets.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/strategy_presets.rs.review.md:1>)
- Second-pass packet: [strategy_presets.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_presets.rs.review.md:1>)
- Third-pass packet: [strategy_presets.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_presets.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/021__crates__app_core__src__strategy_presets.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/default/trait.Default.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth pass is the right final packet for this file. The earlier upgrade from `Document only` to `Small surgical refactor` is still justified, but the rationale must stay narrow. The issue is not that `StrategyConfig::default()` is inherently wrong here. The issue is that this file is a curated preset/fixture surface whose representative intent is only partially documented and only partially covered by file-local tests.

The right near-term move is selective fixture hardening, not exhaustive literalization of every default-derived preset.

## Safe-Now Priority

- Keep the current `StrategyConfig::default()` construction style.
- Add a short comment above `double_like_kernel_strategy_cases()` explaining that it is a curated subset, not the full inventory of representative preset helpers, and that `VFCT` / `MARRT` are intentionally outside that grouped loop.
- Add file-local smoke coverage for:
  - `vfct_strategy_for_kernel()`
  - `marrt_strategy_for_kernel()`
- If inherited defaults are pinned at all, do it selectively. `regime_hybrid_strategy_for_kernel()` is the clearest candidate because its representative meaning depends on inherited regime-hybrid defaults more than `MARRT` does.
- Preserve exact preset values and grouped membership unless broader route/parity consumers are re-audited.

## Deferred Work

- Do not replace these helpers with exhaustive struct literals in a small patch.
- Do not add broad fingerprint goldens or freeze the full `StrategyConfig` schema in this file.
- If the repo eventually wants full insulation from `StrategyConfig::default()` drift for benchmark fixtures, solve that in a shared fixture/baseline layer rather than by literalizing this one support file.

## Verification If Changed

- `cargo test -p ta_app_core strategy_presets::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma -- --nocapture`
- If the change grows beyond comments and local smoke tests, broaden verification to the nearby dynamic-plan and runtime consumers that depend on these presets indirectly.

## Regression And Feature-Preservation Constraints

- Preserve current preset parameter values and grouped-case membership unless intentional downstream revalidation happens.
- Preserve `MARRT` as a thin default-derived preset.
- Keep the distinction between preset-plan tests and product/backend-policy tests clear.
- Treat `VFCT` and `MARRT` as part of the live representative preset surface even though they are intentionally absent from `double_like_kernel_strategy_cases()`.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `strategy_presets.rs` at `Small surgical refactor`
- preserve default-based construction
- improve curated-subset documentation and add lightweight smoke coverage for `VFCT` and `MARRT`
- defer exhaustive literalization and broader fixture-architecture changes to a separate, repo-wide decision
