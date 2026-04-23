# Fourth-Pass Review: crates/app_core/src/vram_ma.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/vram_ma.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Document only`
- Fourth-pass action bucket: `Document only`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file earned a fourth pass because the prior packet history converged on the right bucket for the wrong reasons. The bundle rationale still calls `vram_ma.rs` a “shared contract/default surface” (`bundle_manifest.txt:1-5`), and the third-pass packet escalated that framing into Serde/default/label/invariant language (`review__third_pass.md:17-36`, `40-67`). The live source does not support that reading.

`crates/app_core/src/vram_ma.rs` is only a module-level CUDA gate plus three re-exports (`crates/app_core/src/vram_ma.rs:1-4`). More importantly, `crates/app_core/src/lib.rs:88` declares the module as `pub(crate) mod vram_ma;`, so this is crate-private GPU support code, not part of the external `ta_app_core` API surface. That alone materially changes the review posture.

So the fourth pass was needed to answer a narrower question than the earlier packets asked: is there any real hardening to do in the file itself, or is the actual correction mostly to the review rationale? After reopening the live source and the local visibility wiring, the answer is that the code still does not justify a broad action bucket, but the packet absolutely needed sharpening.

## Fourth-Pass Research Log

1. I reopened the live source and the owning crate root first. The source file is only four lines long and contains no local behavior beyond conditional compilation and re-exporting upstream `vector_ta` CUDA MA helpers (`crates/app_core/src/vram_ma.rs:1-4`). The owning crate root keeps the module crate-private (`crates/app_core/src/lib.rs:88`).

2. I checked how this crate exposes symbols from other crate-private GPU modules. `options_expression_gpu` is also crate-private at `crates/app_core/src/lib.rs:52`, but one function is intentionally promoted to the crate’s public surface via `pub use options_expression_gpu::prewarm_options_expression_gpu_runtime;` at `crates/app_core/src/lib.rs:100`. No corresponding public re-export exists for `vram_ma`. That makes the earlier “public seam” language especially weak.

3. I checked the supplied local context and retrieval results for consumers. The bundle and search evidence point to `crates/app_core/src/gpu_backtest_kernel.rs` as the current in-crate consumer of `supports_vram_kernel_ma`, `VramMaComputer`, and `VramMaInputs`, while `REPO_MAP.md` routes this file as part of the GPU/runtime surface. That confirms this is shared internal plumbing, but not a wire-format, persistence, validation, or UI contract owner.

4. I validated the language-level posture against primary Rust docs. The Rust Reference on visibility/privacy and `use` declarations confirms that `pub` on a `use` creates a public re-export only to the extent the enclosing path is publicly reachable, while `pub(crate)` is the precise visibility for crate-internal exposure. The Rust Reference on modules and attributes also confirms that the inner `#![cfg(...)]` at the top of the module file is valid module-level conditional compilation, so the file’s only real local contract is its CUDA feature gating.

## What Fourth Pass Sharpened

The third-pass bucket should stand, but the justification should change.

The strongest fourth-pass conclusion is that `Document only` remains defensible for the source file itself, because there is still no substantive local logic to harden. There are no defaults, sentinels, enum labels, request/response DTOs, persistence shapes, or Serde attributes in this file. The live file is just a crate-private adapter layer over `vector_ta` symbols. That means the earlier packet’s discussion of default behavior, omission behavior, and result-shape compatibility was imported from a generic template rather than inferred from the source.

What the current packet got right is narrower. It correctly treated the file as tiny, feature-gated, and not a place for performance work. It also got the “do not churn this casually” instinct mostly right: changing this file will not improve runtime speed, validation fidelity, or GPU math, and broad cleanup here would create motion without material value. The wrapper does centralize the upstream `vector_ta::cuda::moving_averages::vram_ma` path behind one internal module, which is a reasonable maintenance seam.

What it missed, underweighted, or overstated is more important. First, the repeated “public seam” language in the first-pass and second-pass packets is incorrect against the live source; `pub(crate) mod vram_ma` makes the module crate-private, not externally reachable (`review__first_pass.md:17-25`; `review__second_pass.md:10-20`; `crates/app_core/src/lib.rs:88`). Second, the third-pass Serde/default framing is unsupported by the file’s contents (`review__third_pass.md:24-36`, `40-67`; `crates/app_core/src/vram_ma.rs:1-4`). Third, the verification guidance drifted with that misclassification: `cargo test -p ta_app_core vram_ma -- --nocapture` and “serde/default round-trip or request-contract tests” are not the owning verification surface for a file with no local tests or serialization behavior.

There is, however, one real safe-now hardening opportunity that the earlier packets underweighted: visibility precision. Because the module itself is crate-private, changing the inner re-exports from `pub use` to `pub(crate) use` would better match the actual intent without changing runtime behavior. That is not a bug fix, and it is small enough that I would not overturn the action bucket over it, but it is the one bounded source-level hardening change that is more defensible than adding review-template tests. If the wrapper is kept, a one-line module comment explaining that it is an internal `vector_ta` bridge for GPU kernel plumbing is also reasonable.

Larger redesign should stay deferred. Deleting this wrapper and importing `vector_ta::cuda::moving_averages::vram_ma` directly in `gpu_backtest_kernel.rs` is not obviously wrong, especially if there truly is only one consumer today, but it would trade one tiny internal seam for wider direct dependency on the upstream crate layout. More importantly, promoting this module or its members to the public `ta_app_core` surface would be a materially different decision: it would expose upstream CUDA helper types through the crate’s public API and create semver/dependency-coupling expectations that do not exist today. That should stay out of scope for a cleanup pass.

## Implementation Priority

The highest-value action remains very small.

First, if this file is being touched anyway, the only source-level hardening I would actively recommend is changing the re-exports to `pub(crate) use` and, optionally, adding a short comment that this is an internal re-export seam over `vector_ta` for GPU kernel plumbing. That improves intent expression and reduces the chance that future reviewers call it a public boundary again.

Second, if the file is not otherwise being edited, leaving the source alone is still reasonable. The bigger correction is to the review packet, not to the code. The fourth-pass value here is mostly analytical: stop describing `vram_ma.rs` as a public/default/serde contract owner.

Third, keep any wrapper-removal or public-API promotion discussion deferred. Those are architecture decisions, not safe-now hardening.

## Verification If Changed

If the file is changed only for visibility tightening and/or a module comment, the meaningful verification is compile-path verification, not behavioral re-testing.

Run:

- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`

If a CUDA-capable environment is available and the team wants one representative consumer check, use:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_backtest_kernel -- --nocapture`

If anyone touches the feature gating itself rather than just the re-export visibility, add a CPU-only compile check as well:

- `cargo check -p ta_app_core`

What is not justified here is inventing Serde round-trip tests, DTO contract tests, or file-specific behavioral tests for `vram_ma.rs`; the file does not own that behavior.

## Regression And Feature-Preservation Constraints

Compatibility risk is the main thing to preserve here, but it is internal compatibility, not external wire compatibility. Keep the current import path available to in-crate GPU callers unless you intentionally inline the upstream `vector_ta` path everywhere at once.

GPU ABI risk is effectively nil for a visibility-only or comment-only change. That risk only becomes real if the crate starts publicly exposing these upstream CUDA types, or if the wrapper is repurposed into a different abstraction layer with local type aliases or behavior.

Performance risk is also effectively nil. This file does not participate in the hot path beyond name resolution at compile time, so no performance-improvement claim should be attached to changes here.

IPC, persistence, and validation-contract risk do not materially live in this file. That is exactly why the third-pass Serde/default framing should be retired. Avoid writing recommendations as though `vram_ma.rs` owns request schemas, artifact shapes, or validation semantics.

Finally, preserve the existing CUDA feature gating unless broader build policy is under review. The dual gating — the gated `mod` declaration in `lib.rs` and the module-level `#![cfg(...)]` in `vram_ma.rs` — is the only meaningful local contract this file clearly owns, and changing it can affect CPU-only build posture even though the file has no runtime logic of its own.

## Fourth-Pass Conclusion

The current third-pass bucket should stand, but the current third-pass rationale should not.

`crates/app_core/src/vram_ma.rs` is not a public API seam, not a Serde/default surface, and not a contract owner for labels, sentinels, or DTO behavior. It is a crate-private, feature-gated internal shim over upstream `vector_ta` CUDA moving-average helpers. That means the earlier packets materially overstated the file’s compatibility surface, even though they were directionally right to avoid broad refactoring.

So the fourth-pass call is: keep `Document only` as the action bucket, but rewrite the reasoning around it. The only real safe-now hardening opportunity is tiny visibility tightening (`pub(crate) use`) plus optional intent documentation. Everything larger — wrapper removal, public API promotion, or broader GPU-surface redesign — should stay deferred unless the project is explicitly changing how `ta_app_core` exposes `vector_ta` CUDA primitives.
