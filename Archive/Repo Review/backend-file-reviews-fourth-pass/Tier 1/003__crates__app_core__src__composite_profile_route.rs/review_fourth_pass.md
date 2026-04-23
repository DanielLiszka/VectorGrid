# Fourth-Pass Review: `crates/app_core/src/composite_profile_route.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/composite_profile_route.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_profile_route.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_profile_route.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_profile_route.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Document only`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third pass made an important correction: the strings emitted here are not cosmetic. `CompositeTrendCpu`, `RegimeHybridCpu`, `BreakoutMomentumCpu`, `GpuKernel`, and `GpuHybridKernel` are part of an existing compatibility surface, and they should not be renamed casually.

But that same observation makes the third-pass bucket worth challenging rather than affirming. The live file is still a tiny crate-private policy helper (`crates/app_core/src/lib.rs:15`), its behavior is internally coherent (`composite_profile_route.rs:11-63`), and its local tests already lock the exact label/target distinction that the third pass wants clarified (`69-145`, `171-237`). The fourth pass was needed to answer the narrower question: does this file actually warrant an in-file refactor, or does the current evidence support documentation hardening only?

My conclusion is that the third-pass compatibility concern should stand, but the third-pass bucket should change. The safe-now work in this file is documentation and boundary clarification, not a structural refactor.

## Fourth-Pass Research Log

1. Live source and local tests
   - Reviewed `crates/app_core/src/composite_profile_route.rs` directly, with focus on classification precedence (`11-20`), runtime-label behavior (`24-31`), GPU-target policy (`42-63`), and the current test contract (`65-239`).

2. Local backend-context verification
   - Reviewed `crates/app_core/src/composite_validate.rs` to confirm how this helper actually influences validation.
   - Important local anchors:
     - validation `backend_route` strings are built from `Backend` request/runtime helpers, not from `composite_profile_runtime_label` or `composite_profile_gpu_target_label` (`279-323`)
     - plain base-composite admission uses `composite_profile_from_strategy` only to reject non-`CompositeCpu` candidates (`656-667`, `5843-5894`)

3. Repo guidance and route-policy context
   - Reviewed `AGENTS.md` and `REPO_MAP.md` for the current ownership model.
   - Relevant current guidance:
     - `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` optimize through the `double_ma` family, not the `CompositeCpu` family (`AGENTS.md:106-110`)
     - the current target route policy is `GpuKernel` or `GpuHybridKernel` depending on ADX/profile, with CPU fallback still possible downstream (`AGENTS.md:107-110`, `REPO_MAP.md:1064-1076`)

4. External API-design guidance used only to calibrate review posture
   - Rust API Guidelines — About
     - <https://rust-lang.github.io/api-guidelines/about.html>
     - used to confirm these are recommendations, not mandates, which matters because this module is crate-private
   - Rust API Guidelines — Naming
     - <https://rust-lang.github.io/api-guidelines/naming.html>
     - used to evaluate whether `runtime_label` communicates its semantics clearly enough
   - Rust API Guidelines — Documentation
     - <https://rust-lang.github.io/api-guidelines/documentation.html>
     - used to calibrate how much rustdoc is justified for internal helpers; the key phrase here is “applied within reason,” not “document everything like a public crate API”
   - Rust Programming Language Book — How to Write Tests
     - <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
     - used to re-check whether the existing unit tests already lock the real behavior that matters

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third pass is right about one thing that should now be treated as settled: label churn here is compatibility-sensitive, not cosmetic. The file hard-codes stable string outputs and the local unit tests intentionally assert them (`24-39`, `69-145`, `171-237`). Any rename or repurposing would need a broader audit.

It is also right to prefer additive clarification over renaming. That remains the correct posture.

### What the current third-pass packet underweighted or overstated

The bucket increase from `Document only` to `Small surgical refactor` is not supported strongly enough by the live bundle.

The strongest reason is simple: the current safe-now improvement inside this file is explanatory documentation, not code surgery. The third pass argues that the label strings are part of a live contract, but that is an argument against refactoring, not an argument for it. Contract sensitivity narrows the acceptable change set. It does not, by itself, create a need for structural cleanup.

The third pass also underweights that the key distinction it wants to “clarify and stabilize” is already encoded in the local tests. `composite_trend_without_adx_targets_full_gpu` and `breakout_without_adx_targets_full_gpu` already assert the intentionally mixed shape of this helper surface: runtime-family label remains `*Cpu` while target policy is `GpuKernel` (`69-115`). `regime_hybrid_targets_hybrid` does the same for `RegimeHybridCpu` plus `GpuHybridKernel` (`129-145`). So the file is not missing a test that demonstrates the distinction; it already has one.

More importantly, the current local context weakens the third-pass claim that these helpers themselves are already the exported `backend_route` contract. In the visible validation owner, `backend_route` is built from `Backend` request/runtime helpers such as `validation_backend_label` and `resolve_validation_backend_runtime`, not from `composite_profile_runtime_label` or `composite_profile_gpu_target_label` (`composite_validate.rs:279-323`). That does not prove no downstream optimization/report surface uses these labels, but it does mean the current packet overstates the certainty of that claim from the available bundle. The safer conclusion is narrower: these strings are contract-sensitive enough that they should not be renamed casually, but the current evidence does not justify turning this file into a refactor target.

The current packet also underweights phase-specific route divergence. This file returns intended target policy (`42-63`), but repo guidance explicitly says realized routing can still differ by stage and fallback posture. `CompositeTrend` can target `GpuKernel` in optimization while fixed-candidate validation still preserves parity by falling back to CPU on some ADX-off paths (`AGENTS.md:106-110`, `REPO_MAP.md:1064-1076`). That makes it especially important not to let this file drift toward pretending it owns exact realized backend-route semantics.

### What earlier packets suggested that is riskier than it looks

The first-pass suggestion to deduplicate classifier logic against `strategy_dynamic.rs` may be reasonable in a broader campaign, but it is cross-owner work and is not revalidated by the current fourth-pass bundle. That recommendation should not drive this file’s bucket. If taken on, it should be tracked as a separate cross-file hardening task with parity tests at the affected boundaries.

The earlier idea of removing the `StrategyProfileKind::DoubleMa` match arms is also weaker than it looks. Those arms are unreachable from the current `StrategyConfig -> StrategyProfileKind` classifier (`11-20`), but they still keep the label/target helpers total over a shared enum domain (`24-31`, `42-60`). Deleting them without first narrowing the helper input domain would trade defensive exhaustiveness for cosmetic tidiness. That is not a good bargain in a contract-sensitive policy module.

A final testing nuance: downstream `backend_route` assertions should stay near the downstream owners that actually emit/export route strings. Pulling those assertions into this file’s unit tests would over-couple a tiny policy helper to downstream route-string assembly and make future maintenance harder, not easier.

## Implementation Priority

### Safe-now hardening

1. Add short module-level rustdoc explaining the ownership boundary:
   - this module classifies current composite profile policy from `StrategyConfig`
   - it returns intended target labels, not guaranteed realized runtime routes
   - optimization and validation may still realize different backend routes downstream

2. Add one-line function docs clarifying the two label axes:
   - `composite_profile_runtime_label` returns the stable composite runtime-family / CPU-family label currently used by downstream diagnostics
   - `composite_profile_gpu_target` and `composite_profile_gpu_target_label` return intended target policy, not capability detection and not stage-specific realization

3. Document precedence explicitly:
   - `BreakoutMomentum > RegimeHybrid > CompositeTrend > CompositeCpu`
   - mixed flags are classified here, not validated here

4. Preserve current strings, precedence, and total match arms exactly as they are.

### Larger redesign still deferred

1. Cross-file deduplication with `strategy_dynamic.rs` and any frontend mirror should be deferred to a dedicated parity-hardening task. That work is broader than this file and should only happen with direct review of all mirrored owners.

2. If the team ever wants cleaner naming, do it as a repo-wide route-vocabulary migration, not an isolated rename here. The better long-term shape would be three clearly distinct concepts:
   - profile-family label
   - intended target label
   - realized backend-route label
   But that is a broader design cleanup, not a safe local edit.

3. Only reconsider the `DoubleMa` match arms if the helper inputs are first narrowed or split so the smaller domain is explicit and testable. Until then, keep the defensive total mapping.

## Verification If Changed

If only comments/rustdoc change, the local unit tests are still enough to protect against accidental edit drift:

1. `cargo test -p ta_app_core composite_profile_route -- --nocapture`

If behavior or strings change, widen verification because the risk is semantic, not performance-related:

2. `cargo test -p ta_app_core composite_validate -- --nocapture`
3. `cargo test -p ta_app_core double_ma_validate -- --nocapture`
4. targeted `double_ma` route tests that cover composite-profile backend routing and fallback strings
5. frontend parity verification for the out-of-scope mirror:
   - `web/src/strategies/compositeProfileGpuPolicy.test.ts`

I would not require a perf gate for this file. There is no meaningful throughput, allocation, or GPU-ABI upside available here.

## Regression And Feature-Preservation Constraints

Preserve the current emitted strings exactly unless a broader compatibility audit is underway. Even though the module is crate-private, those strings are already treated as stable expectations by local tests and likely by downstream route/report consumers.

Preserve the current precedence order. This helper does more than label policy; `composite_validate.rs` uses it to gate plain `CompositeCpu` validation admission (`656-667`, `5843-5894`). A precedence change would therefore alter both route classification and validation behavior.

Do not reinterpret `composite_profile_gpu_target` as realized backend execution. The local context shows that realized validation route strings are assembled elsewhere (`279-323`), and repo guidance shows that current fixed-candidate validation can diverge from optimization-target policy on parity or implementation grounds (`REPO_MAP.md:1064-1076`).

Do not remove the `DoubleMa` arms as a casual cleanup. That would reduce totality over the shared enum with no demonstrated correctness or maintenance payoff.

Do not add new local tests that assert downstream `backend_route` composition here. Verify those surfaces when this file changes, but keep ownership of route-string assembly tests with the emitters that actually build/export those strings.

From a risk taxonomy perspective:
- performance risk: negligible
- GPU ABI risk: none in-file
- IPC / persistence / artifact risk: indirect but real if emitted string values are changed
- validation risk: real if precedence or target semantics are changed

## Fourth-Pass Conclusion

The third-pass packet is directionally right to treat this file as contract-sensitive, but its action bucket should change.

`Small surgical refactor` is too strong for the live source. The file is small, coherent, allocation-free, crate-private, and already locally well-tested. The strongest safe-now action is to document what the file does not own: it does not define realized backend-route strings for every phase, and it does not validate mixed-flag legality. It classifies current composite profile policy and returns intended label/target surfaces.

So the fourth-pass bucket is `Document only`.

Keep the current behavior. Keep the current strings. Add short clarifying rustdoc/comments. Defer any deduplication, renaming, or domain narrowing to a separate cross-file route-contract task where all downstream emitters and mirrors are in scope.
