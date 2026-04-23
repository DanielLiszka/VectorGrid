# Fourth-Pass Review: crates/app_core/src/lib.rs

- **Review date:** `2026-04-16`
- **Source file:** `crates/app_core/src/lib.rs`
- **First-pass packet:** `review__first_pass.md`
- **Second-pass packet:** `review__second_pass.md`
- **Third-pass packet:** `review__third_pass.md`
- **Third-pass action bucket:** `Small surgical refactor`
- **Fourth-pass action bucket:** `Small surgical refactor`
- **Packet status:** `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet kept the bucket unchanged, but its reasoning drifted into serde/default/invariant language that fits DTO-heavy files better than a crate-root facade. The live `lib.rs` is almost entirely module declarations and public re-exports (`lib.rs:1-296`), so the real contract here is not field-default behavior. It is path stability, feature-gated symbol availability, discoverability, and how much public API this crate root is intentionally promising.

This pass re-opened the live source to challenge the current packet on narrower, safer grounds: whether the existing `Small surgical refactor` bucket still holds, whether the “docs-first” framing remains right, and whether the current packet underweights any compatibility, GPU ABI, or feature-matrix risks.

## Fourth-Pass Research Log

1. **Live-source reread**
   - Re-read the current `lib.rs` line by line.
   - Re-compared the live file with the first-, second-, and third-pass packets instead of assuming the earlier framing was still the best one.

2. **Repo-contract reread**
   - Re-checked the supplied repo guidance to confirm that `ta_app_core` is a workspace core crate with `cuda`, `cuda-backtest-kernel`, and optional `cli` feature surfaces, and that CPU-only builds must continue working.

3. **Official Rust documentation re-check**
   - **Rust Reference — use declarations:** `pub use` is a real re-export and therefore part of the crate’s public name surface.
   - **Rust Reference — visibility and privacy:** re-exports can short-circuit the privacy chain, so a root re-export is a real compatibility promise even when the canonical module path is private.
   - **Rust Reference — conditional compilation** and **Cargo Book — features:** `#[cfg(...)]` actually removes items when predicates are false, so feature combinations materially change the compiled public surface.
   - **Rustdoc Book — how to write documentation:** crate-front-page docs belong at the top of `lib.rs` via `//!`.
   - **Rustdoc Book — re-exports:** `#[doc(inline)]` and `#[doc(no_inline)]` affect how re-exports are shown; non-inlined re-exports do not automatically make the re-export-site attributes the main documentation signal.
   - **Rustdoc-specific lints:** `rustdoc::missing_crate_level_docs` is a narrow enforcement tool; `missing_docs` is much broader.
   - **Rustdoc unstable features:** `#[doc(cfg)]` / `#[doc(auto_cfg)]` remain unstable, so they are not a good default recommendation for a conservative stable-facing hardening pass.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**. This is still a `Small surgical refactor` file, not a redesign candidate.

The reason the bucket stands, however, is narrower than the third-pass packet implies. The safe work here is crate-front-door hardening: crate docs, clearer feature-surface explanation, better verification of the compile surface, and more explicit labeling of advanced operational exports. It is **not** serde/default invariant work on the facade itself.

### What the third-pass packet got right

The current packet is directionally right on several important points.

First, it correctly treats `lib.rs` as a **compatibility seam**, not as a free cleanup zone. That is still the right posture. This file defines which modules and symbols downstream consumers can import, and that alone makes it high-blast-radius.

Second, it correctly resists broad visibility pruning. There is still no evidence in the live file that a large-scale “make more of this private” pass would be safe without a real consumer audit.

Third, it correctly points toward **documentation-first** hardening over runtime or performance work. `lib.rs` is not a hot path; the risk is contract churn, not execution cost.

### What the third-pass packet missed or underweighted

#### 1. The serde/default framing is not well grounded in the live file

The third-pass packet repeatedly frames this file as a “contract/default surface” and prioritizes tests for default behavior, serde behavior, omission behavior, and exported labels. That is not the strongest reading of the live source.

`lib.rs` does not define request structs, enum defaults, serde aliases, or wire-format behavior of its own. It declares modules and re-exports symbols (`lib.rs:1-296`). The serialization-sensitive behavior lives in modules like `double_ma`, `options_types`, `strategy_ir`, and related request/result owners, not in the crate facade itself.

That means the third-pass recommendation to prioritize serde/default round-trips is **overstated for this file**. The relevant file-local contract is import-path and feature-surface stability.

#### 2. The public feature surface is three-tier, not merely “CPU vs CUDA”

The earlier packet notices feature-gated exports, but it still reads a bit too much like a two-lane split. The live file shows a more important distinction:

- **Always available**: for example `cuda_device` is public and its helpers are re-exported at the root (`lib.rs:19`, `95-98`), and GPU-program/runtime planning surfaces are also always public (`lib.rs:66-67`, `259-270`).
- **`cuda` only**: dynamic-indicator CUDA helpers and dispatch stats are gated only on `feature = "cuda"` (`lib.rs:169-173`, `288-289`).
- **`cuda + cuda-backtest-kernel`**: strategy-local GPU kernel modules, `gpu_backtest_kernel`, the options GPU prewarm export, and the kernel upload/eval helpers are gated by `all(feature = "cuda", feature = "cuda-backtest-kernel")` (`lib.rs:3-4`, `7`, `10`, `13`, `30`, `36`, `39`, `45`, `52`, `60`, `63`, `80`, `83`, `86`, `88`, `99-100`, `291-296`).

That distinction matters for both documentation and verification. A file review that only validates default build and “full GPU build” still misses the standalone `cuda` lane.

#### 3. The file already shows meaningful curation, so “broad accidental exposure” is too coarse

The first-pass packet was right to worry about API breadth, but fourth-pass rereading shows a sharper nuance: this crate root already draws real internal/public boundaries.

A substantial set of modules is `pub(crate)`, including `alloc_policy`, `composite_profile_route`, `double_ma_backend`, `double_ma_mode`, `double_ma_result`, `options_expression_gpu`, `request_normalization`, `validation_core`, and `vram_ma` (`lib.rs:1`, `13`, `15`, `21-25`, `27`, `52`, `55`, `57`, `63`, `74`, `88`).

So the file is not simply “everything public.” It is already curated; the question is whether the *remaining* public surface is intentionally supported or just historically convenient. That makes blanket privatization recommendations even weaker than the earlier packet suggested.

#### 4. The root exports include ABI/schema/version and runtime-cache control surfaces

The current packet underweights how sensitive some of the root exports already are.

This is not just a bag of convenience types. The facade re-exports:

- **schema/version constants** for options and strategy IR/plan surfaces (`lib.rs:242-264`, `273-276`);
- **GPU ABI-facing types and version constants** like `CompiledGpuPredicateProgramAbi`, `GpuConditionAbi`, `GpuIndicatorSlotAbi`, `GpuSignalBindingsAbi`, `GpuValueOperandAbi`, `STRATEGY_GPU_EXECUTABLE_ABI_VERSION`, and `STRATEGY_GPU_PROGRAM_SCHEMA_VERSION` (`lib.rs:259-265`);
- **advanced operational/runtime-cache controls** like `reset_dynamic_gpu_preflight_cache`, `stage_double_ma_campaign_vram_prefetch`, `prewarm_options_expression_gpu_runtime`, `take_prepared_cuda_dispatch_stats`, `gpu_strategy_executable_cache_stats`, and `reset_gpu_strategy_executable_cache` (`lib.rs:105`, `108`, `100`, `289`, `268-269`).

Those are higher-risk than ordinary flat request/result re-exports. They touch GPU ABI expectations, cache lifecycle, profiling/diagnostic behavior, or persisted artifact/version compatibility. Any cleanup that renames, rehomes, or hides them is more dangerous than the third-pass packet implies.

#### 5. The root facade is not exhaustive

The earlier packets push a “root re-exports versus nested modules” framing, which is directionally right, but fourth pass sharpens an important nuance: the root export list is not a complete public facade.

Several public modules remain module-only entry points, including `indicator_factory`, `ma_policy`, `matrix_index`, `nvtx`, `progress`, `stats_common`, `strategy_presets`, `validation_pipeline`, and `validation_stats` (`lib.rs:31-32`, `40-41`, `48`, `56`, `64`, `70`, `75`, `77`). The strategy-local GPU kernel modules are also public under kernel features without corresponding flat root re-exports (`lib.rs:4`, `7`, `10`, `36`, `39`, `45`, `60`, `80`, `83`, `86`).

That means a future crate doc should describe **common entry points**, not imply that the root re-export list is the whole supported API.

#### 6. Verification should include the explicit `cuda` lane

This is the cleanest concrete miss.

The earlier packet called for default-build and `cuda,cuda-backtest-kernel` compile checks. That is not enough for this file because it owns `cuda`-only root exports at `lib.rs:169-173` and `288-289`.

For this file, a real compile-surface review should explicitly include:

- default / CPU-only,
- `--features cuda`,
- `--features cuda,cuda-backtest-kernel`.

That is a meaningful hardening correction, not a cosmetic one.

### What the third-pass packet overstated

#### 1. “Invariant coverage before refactoring” is the wrong primary lever here

For `lib.rs`, the best verification is compile-surface reachability and documentation rendering, not serde/default invariant tests. A crate root with no file-local serde behavior does not benefit much from request-contract round-trips at the facade layer.

Those tests still matter in the owning modules. They are just not the best next step for this file.

#### 2. The docs should not invent a stronger stability policy than the code currently encodes

The earlier review packets lean toward distinguishing “stable convenience exports” from “public-by-necessity internals.” That may be true socially, but the code does not currently enforce that distinction.

A safer docs pass should describe:

- what the crate is for,
- which surfaces are commonly imported,
- which features are required for which symbols,
- which exports are advanced/operational.

It should **not** prematurely promise that certain still-public modules are semi-private or headed for future restriction unless the project explicitly wants to formalize that policy.

### Safe-now hardening

These are the changes that still fit the current bucket.

1. **Add crate-level `//!` docs at the top of `lib.rs`.**
   - Explain the crate’s role as the backend/runtime hub.
   - Group the public surface by workflow: optimization, validation, dynamic indicators, meta/ML, options, GPU program/runtime.
   - Explicitly describe the three availability tiers: always available, `cuda`, and `cuda + cuda-backtest-kernel`.
   - Keep any example CPU-safe by default, or mark feature-specific examples so they do not become brittle doctests.

2. **Add a narrow documentation guard, not a broad one.**
   - `#![warn(rustdoc::missing_crate_level_docs)]` is a reasonable safe-now follow-up once the crate header exists.
   - Do **not** jump straight to `#![warn(missing_docs)]` in this pass; that would spill across roughly the entire public surface and turn a surgical review into a repo-wide docs project.

3. **Document the high-value namespaced modules that are clearly real entry points.**
   - Priority candidates are `double_ma`, `composite_cpu`, `options_types`, `progress`, `nvtx`, `strategy_ir`, `strategy_gpu_program`, and `strategy_gpu_runtime`.
   - This is safer and more honest than pretending the flat root re-export list is the whole surface.

4. **Mark advanced operational exports as advanced.**
   - Cache reset/prefetch/prewarm/stats helpers should not be presented like first-stop user APIs.
   - Even a short crate-level note that these are advanced runtime/profiling controls would materially improve public-readiness.

5. **Only consider selective rustdoc inlining after inspecting generated docs.**
   - A small number of handpicked `#[doc(inline)]` uses might improve discoverability for dependency re-exports or especially important top-level entry points.
   - That should be driven by generated-doc inspection, not applied blindly.

### Larger redesign still deferred

These ideas still do **not** belong in the current bucket.

- Broad visibility narrowing of public modules.
- Root-versus-module API rationalization.
- Prelude-style or multi-facade redesign.
- Blanket `#![warn(missing_docs)]` / `#![deny(missing_docs)]`.
- Any use of unstable `#[doc(cfg)]` / `#[doc(auto_cfg)]` as part of a stable-facing docs hardening pass.
- Renaming, moving, or removing root exports, especially version constants, ABI structs, request/result types, or runtime-cache controls.

## Implementation Priority

1. **Keep the bucket unchanged, but correct the target.**
   - The next pass should be documentation and compile-surface hardening, not serde/default invariant work on the facade.

2. **Add the crate header and feature-surface explanation first.**
   - This is the highest-value change with the lowest regression risk.

3. **Add the narrow crate-doc lint second.**
   - `rustdoc::missing_crate_level_docs` protects the specific improvement you just made without forcing a much larger documentation migration.

4. **Then document the namespaced entry points and advanced controls that the root list does not explain well.**
   - This keeps the review conservative while still improving public-readiness.

5. **Defer any public-surface pruning until a consumer inventory exists.**
   - The live file already supports both root and nested import styles. That is too compatibility-sensitive to “clean up” as part of this pass.

## Verification If Changed

Minimum verification for this file should be:

```text
cargo test -p ta_app_core --lib --no-run
cargo test -p ta_app_core --features cuda --lib --no-run
cargo test -p ta_app_core --features cuda,cuda-backtest-kernel --lib --no-run
```

If crate docs or rustdoc presentation are changed, also run:

```text
cargo doc -p ta_app_core --no-deps
```

If the docs intentionally mention optional-feature APIs in runnable examples, either:

- run the matching doc-test feature lane as well, or
- make the example `no_run` / `ignore` / explicitly feature-scoped so default CPU builds do not become brittle.

If any `pub mod` visibility or `pub use` names change, the verification bar rises materially. At that point I would want at least:

```text
cargo test --workspace --no-run
cargo test -p ta_desktop_demo_app --lib --no-run
cargo bench -p ta_app_core --no-run
```

A broad non-options export/visibility change is also a reasonable point to use the repo quality gate from the supplied guidance.

## Regression And Feature-Preservation Constraints

- Preserve the current dual import posture: both flat root re-exports and nested module paths are active consumers today.
- Preserve the three compile surfaces owned here: default/CPU-only, `cuda`, and `cuda + cuda-backtest-kernel`.
- Keep CPU-only builds working with no CUDA toolchain installed.
- Do not document GPU-named symbols as feature-gated merely because of their names. Some GPU-adjacent surfaces are always public here, including `cuda_device` helpers and the strategy GPU program/runtime planning types.
- Treat `STRATEGY_GPU_EXECUTABLE_ABI_VERSION`, `CompiledGpuPredicateProgramAbi`, the `Gpu*Abi` types, `STRATEGY_*_SCHEMA_VERSION`, and `OPTIONS_*_VERSION` as GPU ABI / persistence / IPC-sensitive surfaces. Small-pass cleanup should not rename or hide them.
- Do not use `#[doc(hidden)]` as a docs-cleanup shortcut on still-public APIs. That reduces discoverability without removing the compatibility obligation.
- Do not rely on unstable `#[doc(cfg)]` / `#[doc(auto_cfg)]` for a conservative stable-facing hardening pass.
- Do not add feature-specific doctests that fail in the default build.
- Do not justify public-surface churn with performance. This file is not a runtime hot path.

## Fourth-Pass Conclusion

The **third-pass action bucket should stand**: `crates/app_core/src/lib.rs` remains a `Small surgical refactor`.

But the fourth pass does sharpen the packet materially. The current third-pass review is partially mis-aimed because it treats the crate root too much like a serde/default seam. The live file is better understood as a public-facade and compile-surface map with three distinct feature tiers, a mix of flat and namespaced entry points, and several ABI/cache-control exports that are more sensitive than ordinary convenience re-exports.

So the safe-now work stays small, but it is more specific than the current packet says:

- improve crate-front-door documentation,
- explicitly explain the feature matrix,
- add a narrow crate-doc lint,
- verify the missing middle `cuda` lane,
- and avoid inventing a stronger API-tier policy or pruning public surface without a real consumer audit.

That is the narrowest evidence-backed hardening path that improves public-readiness without inviting compatibility regressions.
