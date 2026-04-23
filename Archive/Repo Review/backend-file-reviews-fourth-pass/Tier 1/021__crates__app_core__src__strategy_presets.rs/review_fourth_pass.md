# Fourth-Pass Review: crates/app_core/src/strategy_presets.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/strategy_presets.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_presets.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_presets.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_presets.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because the third pass changed the bucket on a very small preset/support module. That is exactly the kind of upgrade that needs an adversarial re-check: the justification has to be concrete and file-local, not just a generally true observation about `Default`.

The live file is still only a handful of preset constructors plus three focused tests (`strategy_presets.rs:3-108`, `strategy_presets.rs:120-246`). The central fourth-pass question is therefore narrower than the third pass framed it: does the presence of `StrategyConfig::default()` in these constructors justify refactoring this file now, or does it justify a more targeted hardening move than “reduce hidden dependency on default values” as a general rule?

My conclusion is that the third-pass bucket can stand, but only with a materially narrower rationale. The third pass identified a real sensitivity, yet it overstates what that sensitivity threatens locally and points too directly toward a higher-churn fix.

## Fourth-Pass Research Log

I re-read the live source file, the first-, second-, and third-pass packets, then checked the adjacent dynamic-strategy and GPU-plan surfaces to see what these presets actually influence.

First, I reviewed `strategy_plan.rs`. That file shows two separate facts that matter here. `CompiledStrategyPlan` carries the full `StrategyConfig` forward (`strategy_plan.rs:84-107`, `strategy_plan.rs:299-315`), but indicator-route selection itself is derived from backend plus indicator metadata / legacy indicator kind, not from arbitrary fields inside `StrategyConfig` (`strategy_plan.rs:130-165`, `strategy_plan.rs:232-243`). That distinction is important because it narrows which third-pass claims are well-supported.

Second, I reviewed `strategy_gpu_program.rs` and `strategy_gpu_runtime.rs`. The compiled-plan fingerprint for GPU work serializes the entire compiled plan (`strategy_gpu_program.rs:184-195`), and the GPU compiled-bundle cache key is built from that fingerprint (`strategy_gpu_runtime.rs:85-100`, `strategy_gpu_runtime.rs:188-220`). So inherited strategy defaults are not inert: they can affect cache identity and benchmark comparability even when indicator routes do not change.

Third, I re-checked repo guidance. `REPO_MAP.md` explicitly still treats `Marrt` and `Vfct` as part of the representative preset surface for this file (`REPO_MAP.md:1058-1077`, `REPO_MAP.md:1201-1206`). The same guidance also distinguishes product backend policy from static plan routing and states that `RegimeHybrid` remains hybrid/CPU-fallback in its current ADX-bearing form (`REPO_MAP.md:1064-1066`; `AGENTS.md:106-110`). That matters because this file’s current tests operate at the static-plan layer, not the product-route-policy layer.

Fourth, I reviewed the official Rust `Default` docs and the Rust API Guidelines documentation/dependability chapters. Those sources support the broad idea that behaviorally important helper surfaces should be documented and validated, but they do not justify exhaustive local literalization by themselves. The code and repo contracts have to decide whether inherited defaults are accidental coupling or deliberate shorthand.

## What Fourth Pass Sharpened

### 1. What the third-pass packet got right

The third pass was right that these presets are not “just comments with code.” Every constructor starts from `StrategyConfig::default()` and only overrides a small subset of fields (`strategy_presets.rs:3-95`). Because the full `StrategyConfig` is cloned into `CompiledStrategyPlan` (`strategy_plan.rs:299-315`) and GPU compiled-bundle keys depend on the serialized compiled plan (`strategy_gpu_program.rs:184-195`; `strategy_gpu_runtime.rs:85-100`, `strategy_gpu_runtime.rs:188-220`), upstream default drift can perturb runtime cache identity and benchmark comparability without this file changing.

The third pass was also right that the file-local tests do not pin every meaningful preset invariant. The current tests cover label uniqueness, grouped-case plan compilation, and three static composite-profile route checks (`strategy_presets.rs:120-246`), but they do not give blanket coverage over every exported helper.

### 2. What the third-pass packet overstated

The third pass overstates the claim that default drift in `StrategyConfig` directly threatens the route coverage exercised by this file. In the adjacent compiler, route choice comes from backend plus indicator metadata / legacy indicator kind (`strategy_plan.rs:130-165`, `strategy_plan.rs:232-243`). So rewriting these presets into exhaustive struct literals would not materially harden the specific fast/slow/regime route assertions already present in `strategy_presets.rs`.

That does not mean default drift is irrelevant. It means the threat model is narrower than the packet claimed. The concrete downstream risks are cache-key identity, benchmark comparability, and higher-level behavior in consumers that interpret the full strategy config. The local route assertions in this file are not where most of that risk is decided.

### 3. What the third-pass packet missed

The most concrete missed local gap is not abstract default sensitivity. It is that two exported representative helpers, `vfct_strategy_for_kernel()` and `marrt_strategy_for_kernel()`, have no file-local smoke coverage at all (`strategy_presets.rs:32-47`). That omission matters because repo guidance still treats `VFCT` and `MARRT` as part of this file’s representative preset surface (`REPO_MAP.md:1062-1063`, `REPO_MAP.md:1201-1206`).

The third pass also missed that some default dependence here is deliberate, not suspicious. `marrt_strategy_for_kernel()` is intentionally tiny: it starts from the default config and flips `long_only` to `false` (`strategy_presets.rs:44-47`). Repo guidance describes `Marrt` as a thin Double-MA variant that is “mostly `long_only=false` preset” (`REPO_MAP.md:1062`). For that helper, exhaustive literalization would make the code less truthful to the preset’s role, not more.

A more meaningful default-sensitive case is `regime_hybrid_strategy_for_kernel()`. The constructor only sets `long_only = false` and `regime_hybrid_enable = true` (`strategy_presets.rs:10-14`), while repo guidance says the current product posture is specifically the ADX-bearing hybrid form (`AGENTS.md:107-110`; `REPO_MAP.md:1065`). If there is one preset here whose inherited defaults deserve selective pinning, it is this one, because its representative meaning depends on more than the two locally assigned fields.

### 4. What recommendation would risk regressions if implemented literally

The third-pass “reduce hidden dependency on `StrategyConfig::default()`” recommendation is too loose because it invites a risky implementation: replacing these helpers with exhaustive struct literals. That would couple a small support file to the full field surface of `StrategyConfig`, which is defined in another crate and already used through `Default` shorthand across nearby tests and helper flows (`strategy_plan.rs:330-340`; `strategy_gpu_program.rs:437-459`, `strategy_gpu_program.rs:539-556`; `strategy_gpu_runtime.rs:369-500`).

That kind of literalization has three downsides. First, it creates maintenance churn whenever `StrategyConfig` grows. Second, it can silently fossilize old defaults in one support file while the rest of the repo continues to use the canonical default baseline. Third, because the compiled-plan fingerprint includes the full strategy config, broad explicit pinning here can change cache identity and benchmark behavior more aggressively than intended.

### 5. What the packet underweighted about policy layering

The third pass did not clearly separate static-plan tests from product backend-policy tests. This file currently checks that certain preset-built plans keep `fast_ma`, `slow_ma`, and selected regime indicators on `GpuKernel` routes (`strategy_presets.rs:135-246`). That is appropriate for this module.

What would be inappropriate is using this file to assert product route policy for `RegimeHybrid`. Repo guidance says `RegimeHybrid` is currently hybrid/CPU-fallback at the product layer because ADX remains part of the present form (`AGENTS.md:107-110`; `REPO_MAP.md:1065`). A follow-up refactor should not “correct” this file by forcing runtime-route expectations into preset tests. That would blur the boundary between preset fixtures and backend-policy owners.

## Implementation Priority

The current third-pass bucket should stand, but the safe-now work should be narrower than the third-pass packet implied.

### Safe-now hardening

- Keep the current `StrategyConfig::default()` construction style. Do not treat its mere presence as a defect.
- Add a short comment above `double_like_kernel_strategy_cases()` clarifying that it is a curated subset, not the full inventory of representative helpers, and that `VFCT` / `MARRT` stay standalone on purpose (`strategy_presets.rs:98-108`; `REPO_MAP.md:1201-1206`).
- Add small file-local smoke coverage for `vfct_strategy_for_kernel()` and `marrt_strategy_for_kernel()`. The goal should be modest: confirm they still compile as representative presets and still expose the intended high-level behavior, not recreate request-builder or validation-policy tests in this file.
- If any inherited-default pinning is added, do it selectively. `regime_hybrid_strategy_for_kernel()` is the strongest candidate for a tiny invariant test because its representative meaning depends on current ADX-bearing defaults more than `MARRT` does.

### Larger redesign still deferred

- Do not freeze the entire `StrategyConfig` schema in this file through exhaustive struct literals or full-struct equality tests.
- Do not add compiled-plan fingerprint or executable-fingerprint goldens here unless the repo explicitly wants versioned cache/baseline contracts for these presets.
- If the repo ultimately wants full independence from upstream `StrategyConfig::default()` drift for benchmark fixtures, solve that once in a shared fixture/baseline layer or in the concrete benchmark/runtime consumers that actually care about those identities. A one-file literalization in `strategy_presets.rs` would be an inconsistent partial fix.

## Verification If Changed

If this file changes, the minimum conservative verification remains:

- `cargo test -p ta_app_core strategy_presets::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma -- --nocapture`

If the hardening goes beyond comments and local smoke tests and starts pinning runtime behavior, broaden verification to a full `ta_app_core` test run because the nearby dynamic-plan and GPU-runtime surfaces are where full-strategy fingerprints and cache identity are actually consumed.

## Regression And Feature-Preservation Constraints

- Preserve exact preset parameter values and current grouped-case membership unless the broader route/parity consumers are re-audited.
- Preserve `MARRT` as a thin default-derived preset. Converting it into an exhaustive literal would make the code less aligned with the role documented in repo guidance (`strategy_presets.rs:44-47`; `REPO_MAP.md:1062`).
- Preserve the distinction between preset-plan checks and product backend-policy checks. Do not add assertions here that claim `RegimeHybrid` is a pure `GpuKernel` product route just because its static plan still contains kernel-routed indicator slots.
- Treat `VFCT` and `MARRT` as live representative helpers even though they are intentionally absent from `double_like_kernel_strategy_cases()` (`strategy_presets.rs:98-108`; `REPO_MAP.md:1201-1206`).
- There is no direct IPC or persistence contract owned by this file. The real downstream risks are benchmark comparability, cache-key identity for dynamic GPU work, and accidental narrowing or widening of representative coverage.

## Fourth-Pass Conclusion

The current third-pass bucket should stand, but the third-pass rationale should not.

The packet got one important thing right: inherited defaults in these presets are real contract inputs, and the current local tests do not pin every meaningful assumption. But it missed a more concrete local problem, namely the absence of file-local smoke coverage for `VFCT` and `MARRT`, and it overstated the claim that `StrategyConfig::default()` primarily threatens the route coverage checked in this module.

The safe interpretation is therefore narrower. Keep `Small surgical refactor`, but make it a genuinely small one: preserve default-based construction, improve the curated-subset documentation, add lightweight coverage for the two untested exported helpers, and only pin selected inherited defaults where the preset’s representative meaning actually depends on them. Exhaustive literalization of `StrategyConfig` should remain deferred because it is a broader fixture-architecture decision, not the right next move for this file.
