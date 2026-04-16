# Fourth-Pass Review: `crates/app_core/src/double_ma_test_support.rs`

**Review date:** `2026-04-16`  
**Source file:** `crates/app_core/src/double_ma_test_support.rs`  
**First-pass packet:** `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_test_support.rs.review.md`  
**Second-pass packet:** `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_test_support.rs.review.md`  
**Third-pass packet:** `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_test_support.rs.review.md`  
**Third-pass action bucket:** `Document only`  
**Fourth-pass action bucket:** `Safe-now hardening`  
**Packet status:** `Reviewed`

## Why This File Got A Fourth Pass

This file kept landing in `Document only` because the live implementation is extremely small: ten forwarding helpers, no branching, no state, no caching, and no production-path execution. A fourth pass is still worthwhile because tiny support seams are where stale lint suppressions and weak architectural stories can survive for a long time without ever becoming urgent enough to challenge.

The live source confirms that `double_ma_test_support.rs` is not a crate-wide test utility and not a runtime seam. It is injected into `double_ma.rs` only under `#[cfg(test)]` (`double_ma.rs:76-78`), then consumed by the local `#[cfg(test)] mod tests` in that same file (`double_ma.rs:14120-14127`). The file itself is a pure forwarding layer to `crate::strategy_presets` (`double_ma_test_support.rs:1-44`). That matters because the right fourth-pass question is not “is this a hidden production risk?” but rather “is the current `Document only` bucket too passive for the small amount of real code-quality debt that is still sitting here?”

Repo guidance also keeps this file in the preset/test-helper lane rather than the runtime lane. `AGENTS.md` lists it under “strategy preset/test helpers,” and `REPO_MAP.md` pairs it with `strategy_presets.rs` as a fast read for representative kernel-ready configurations. That supports a conservative review posture: keep it thin, keep ownership of preset semantics elsewhere, and only recommend changes that are tightly local.

## Fourth-Pass Research Log

I re-read the live shim first, then checked the surrounding ownership points in `double_ma.rs` and `double_ma_validate.rs`, and only after that compared the first-, second-, and third-pass packets. The local source read established four concrete facts.

First, the file is entirely forwarding code: every function delegates directly into `crate::strategy_presets` and returns the upstream result unchanged (`double_ma_test_support.rs:4-43`). Second, the module is test-only and private to `double_ma.rs` (`double_ma.rs:76-78`). Third, the local `double_ma.rs` tests consume these helpers broadly, including CPU-side composite-profile tests and multiple GPU-kernel-oriented tests (`double_ma.rs:16525-16547`, `18980-18990`, `19422-19428`, `20461-20468`, `20662`, `23205-23212`, `23626-23634`). Fourth, nearby validation tests do not go through this shim at all; they call `crate::strategy_presets::*` directly (`double_ma_validate.rs:6268`, `6371`, `6612`, `6774`, `7199`, `8135`, `8225`).

I then used Rust’s primary documentation to calibrate what should and should not matter here. The Rust Book’s test-organization guidance and the Rust Reference’s conditional-compilation rules both support the current pattern of small `#[cfg(test)]` helpers near the code they support. The Rust Reference’s visibility rules also support the current `pub(super)` choice as the narrowest exposure that still lets the sibling `tests` module access the helpers through `super::double_ma_test_support`. On the lint side, the `dead_code` documentation is relevant because this file carries three explicit `#[allow(dead_code)]` suppressions; those attributes are justified only when the unused-status concern is real, not as a generic marker of low importance. Finally, the Rust API Guidelines are useful here only as soft guidance. They help justify a short ownership comment, but because this module is private and test-only, public-API documentation standards should not be applied as if they were a hard contract.

## What Fourth Pass Sharpened

The current third-pass packet got two important things right. It was right to keep this file out of the performance, IPC, persistence, GPU ABI, and public-API risk buckets. It was also right that canonical preset ownership belongs in `strategy_presets.rs`, not here. Any recommendation that moved real preset-building logic into this file would increase the number of semantic owners and make future optimization/validation drift more likely.

Where the third-pass packet is weaker is in how it describes the file’s role. The packet frames this as a “support and instrumentation seam” and warns about global mutable state, publication order, and helper lifetime. None of those concerns are grounded in the live source. There is no instrumentation here, no shared state here, and no helper behavior beyond simple forwarding. That framing is too generic for the actual file and ends up spending review attention on risks that this source simply does not contain.

The third-pass packet also underweights the strongest live hardening opportunity: the three `#[allow(dead_code)]` attributes on `trend_quality_strategy_for_kernel`, `regime_hybrid_strategy_for_kernel`, and `breakout_strategy_for_kernel` (`double_ma_test_support.rs:3`, `8`, `13`). In the live corpus, those functions are not speculative or “maybe reserved” helpers. They are used by real `double_ma.rs` tests, including the CPU-side composite-profile test at `double_ma.rs:16525-16547`, the GPU-kernel export-path test at `double_ma.rs:20662`, and the trade-metrics GPU-kernel test at `double_ma.rs:23626-23634`. That does not prove every possible build matrix sees them as used, but it does make the current suppressions look stale rather than obviously intentional. This is the main reason I would move the file one notch above `Document only`.

Another overstatement in the prior packet is the implied architectural value of the shim itself. The shim does help keep the huge `double_ma.rs` test module from spelling `crate::strategy_presets::*` at every call site. But that convenience boundary is local, not systemic. `double_ma_validate.rs` already bypasses it and imports the preset builders directly from `crate::strategy_presets` in several validation tests. So the current shim is not a crate-wide contract layer and not a strong abstraction boundary that other modules consistently respect. That makes a short local-purpose comment more useful than a broad defense of the shim as a major architectural seam.

The earlier verification guidance is also too weakly grounded. The third-pass packet recommends `cargo test -p ta_app_core double_ma_test_support -- --nocapture`, but the live `double_ma.rs` context does not expose a dedicated `double_ma_test_support` test target. The safer recommendation is to verify the actual consumer tests and the lint path that would be affected by removing stale suppressions.

### Bucket assessment

The third-pass `Document only` bucket should **change**, but only slightly. I would move this file to `Safe-now hardening`, not because it owns meaningful behavior, but because the live source supports one bounded code-quality cleanup beyond documentation: remove or justify the stale `dead_code` suppressions. If the repository is in a zero-churn window, leaving the code alone is still defensible. But on the merits of the live file, there is a real local hardening opportunity that the previous packet underweighted.

## Implementation Priority

### Safe-now hardening

The highest-value safe-now change is to add a short module-level comment explaining the true boundary: this is a `double_ma.rs`-local test forwarding shim, while canonical preset definitions remain in `strategy_presets.rs`. That is more precise than the earlier packet’s broad “support seam” language and matches the actual ownership pattern shown by `double_ma.rs` and `double_ma_validate.rs`.

The second safe-now change is to revisit the three `#[allow(dead_code)]` attributes. Because the live `double_ma.rs` test surface demonstrably uses those helpers, the suppressions should be treated as stale until proven otherwise. If `cargo check -p ta_app_core --tests` and the repo’s lint lane stay clean without them, they should be removed. If some less obvious matrix still needs one of them, the attribute should stay only with an inline comment that names the exact condition being protected. Leaving them as unexplained blanket suppressions is the least review-friendly state.

Everything else should stay conservative. Keep `pub(super)`. Keep the file forwarding-only. Keep `StrategyConfig` construction and the `double_like_kernel_strategy_cases` owner in `strategy_presets.rs`.

### Larger redesign still deferred

A larger cleanup is possible, but it should remain deferred. The first redesign option would be to delete this shim and import `crate::strategy_presets::*` directly inside `double_ma.rs` tests. The second would be to standardize on one genuine shared test-helper surface that both `double_ma.rs` and `double_ma_validate.rs` use. Either path is reasonable, but neither is a good drive-by change for this file alone.

The reason to defer is not risk of production regression; it is review noise and ownership churn. `double_ma.rs` is a very large file with many local references to these helpers, and `double_ma_validate.rs` already follows a different access pattern. A partial consolidation would remove one small duplicate surface while leaving the broader inconsistency intact. That is not a good trade unless someone is already doing a focused test-organization pass across both modules.

## Verification If Changed

If the change is limited to a module comment and removal of the three `#[allow(dead_code)]` attributes, the most relevant fast check is the test-target lint/compile lane rather than a broad workflow run. Start with `cargo check -p ta_app_core --tests`. If the local policy is to treat warnings as handoff blockers, also run `cargo clippy -p ta_app_core --tests -- -D warnings` or the repo-standard quality gate that covers the crate.

For behavioral confidence, verify at least one real `double_ma.rs` consumer test that exercises the shim on CPU and one that exercises it on the GPU-kernel path when that feature matrix is available. The CPU-side `request_dynamic_strategy_ir_cpu_composite_profiles_accept_trade_metrics` test is a good fit because it exercises `trend_quality_strategy_for_kernel`, `regime_hybrid_strategy_for_kernel`, and `breakout_strategy_for_kernel` together. On the GPU side, `gpu_kernel_supports_double_like_regime_strategies` is a strong representative consumer because it iterates the shared `double_like_kernel_strategy_cases` helper.

If anyone goes beyond comment-and-lint cleanup and starts changing imports or deleting the shim, rerun one nearby validation test that still uses `crate::strategy_presets` directly, such as `double_ma_validation_gpu_kernel_matches_cpu_metrics_breakout_momentum_adx_off`. That is not because this file currently affects validation code, but because a broader test-helper cleanup can accidentally create two inconsistent stories about where canonical preset builders are supposed to come from.

## Regression And Feature-Preservation Constraints

Do not move preset semantics into this file. The correct owner of representative strategy construction remains `strategy_presets.rs`, and that should continue to be the single semantic source of truth for both optimization- and validation-adjacent tests.

Do not widen visibility from `pub(super)` unless every consuming site is being updated in the same change for a clear reason. The current visibility is appropriately narrow for a sibling test module, and widening it would add surface area without adding capability.

Do not oversell the risk profile. This file is behind `#[cfg(test)]`, so comment cleanup or dead-code-suppression cleanup has no direct production performance, compatibility, IPC, persistence, or GPU ABI impact. The real regression risk is test-maintenance churn: renaming or deleting helpers forces many edits in `double_ma.rs`, and moving ownership around can slowly desynchronize optimization and validation test setup if both modules stop using the same preset source.

If one of the `dead_code` suppressions turns out to be required in a feature matrix that is not visible from the provided context, preserve it narrowly and explain it locally. The wrong fix would be to remove a necessary suppression, see a warning in some hidden matrix, and then reintroduce the attribute without documenting why it exists.

## Fourth-Pass Conclusion

The current third-pass packet is directionally right about scope but too passive about cleanup and too generic about risk. It is right that `double_ma_test_support.rs` is not a production redesign target, not a performance surface, and not the place to move preset logic. It is wrong, or at least overstated, in treating the file like a broader instrumentation seam and in leaving `Document only` as the best bucket without challenging the stale lint suppressions.

My fourth-pass recommendation is therefore: **change the bucket from `Document only` to `Safe-now hardening`**. The safe-now work is intentionally small: add one precise ownership comment and remove or narrowly justify the three `#[allow(dead_code)]` attributes. Everything larger than that—especially collapsing the shim or standardizing preset-helper access across both optimization and validation tests—should stay deferred until someone is explicitly doing a test-organization cleanup across the surrounding files.
