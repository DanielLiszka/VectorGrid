# Release Build Variants Plan

Last updated: 2026-04-09

## Purpose

Define the current shipped non-options build variants so release planning, benchmark claims, and verification all stay aligned with the actual feature matrix.

## Scope Boundaries

In scope:

- CPU-only desktop builds
- GPU-enabled desktop builds
- GPU backtest-kernel desktop builds
- feature wiring across workspace crates
- release documentation and operator-facing variant language

Out of scope:

- options-specific build lanes
- CI pipeline design beyond how it should respect build variants
- benchmark methodology details beyond variant labeling

## Current User-Facing Behavior

The repo already documents three practical Windows build variants:

- CPU-only
- GPU (`--features cuda`)
- GPU backtest-kernel (`--features cuda-backtest-kernel`)

README and `RELEASE.md` already frame these as real product variants rather than purely internal development flags. The current product posture also makes two things explicit:

- CPU-only builds must keep working without CUDA toolchains installed
- GPU builds rely on runtime driver/device availability, not only compile-time features

## Current Code Surface

- `RELEASE.md`
  - current release-facing build-variant description
- `README.md`
  - current product-facing quick-start and benchmark posture
- `Cargo.toml`
  - workspace composition
- `src-tauri/Cargo.toml`
  - desktop-app feature wiring for `cuda` and `cuda-backtest-kernel`
- `crates/app_core/Cargo.toml`
  - core-engine feature wiring into `vector-ta`
- `crates/app_core/build.rs`
  - build-time kernel/PTX behavior tied to GPU-kernel variants
- `src-tauri/tauri.conf.json`
  - bundling target and desktop packaging surface

## Current Verification Surface

- local `cargo run -p ta_desktop_demo_app --release`
- local `cargo run -p ta_desktop_demo_app --release --features cuda`
- local `cargo run -p ta_desktop_demo_app --release --features cuda-backtest-kernel`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- README and `RELEASE.md` consistency review

## Strengths Already Present

- the repo already names the real build variants explicitly
- feature forwarding between `src-tauri` and `ta_app_core` is already deliberate
- GPU kernel builds are already treated as a distinct lane rather than a hidden side effect
- CPU-only behavior is already a stated invariant, not an assumption

## Weaknesses / Failure Modes

- release/build language can drift from actual Cargo feature wiring
- “GPU build” and “GPU available at runtime” are easy to conflate
- benchmark claims can accidentally cite a kernel-specific variant while describing it like a generic product build
- variant-specific verification expectations are currently distributed across multiple docs and scripts

## Regressions To Avoid

- breaking CPU-only builds while improving GPU paths
- describing a build variant more broadly than the actual feature set supports
- letting README/RELEASE naming drift from Cargo feature wiring
- mixing kernel-lane benchmark numbers with non-kernel release wording

## External Research Summary

- [Cargo build command](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
  - relevant because release-variant planning depends on explicit feature-flag behavior
- [The Cargo Book: Features](https://doc.rust-lang.org/cargo/reference/features.html)
  - relevant because this repo’s product variants are feature-driven
- [Tauri build and bundle guide](https://v2.tauri.app/distribute/)
  - relevant because desktop packaging needs to preserve the intended variant semantics
- [Tauri Windows installer docs](https://v2.tauri.app/ja/distribute/windows-installer/)
  - relevant because Windows packaging is the current release target described by the repo docs

## Proposed Improvement Approach

1. Document the canonical variant matrix once:
   - build command
   - feature flags
   - expected runtime prerequisites
   - representative verification expectations
2. Keep release-facing wording aligned with Cargo feature wiring.
3. Separate compile-time variant naming from runtime GPU availability diagnostics.
4. Tie benchmark/report wording back to the exact variant used to produce it.
5. Preserve CPU-only support as a first-class release requirement.

## Checklist

- [ ] Document the current CPU-only, GPU, and GPU-kernel release variants.
- [ ] Document feature forwarding between `src-tauri` and `ta_app_core`.
- [ ] Document runtime prerequisites separately from compile-time features.
- [ ] Document which verification surfaces are variant-specific.
- [ ] Keep README and `RELEASE.md` variant language aligned.
- [ ] Tie benchmark claims to explicit variants.
- [ ] Add or tighten checks if variant wiring changes.

## Verification Plan

- run the documented build commands for each supported non-options variant
- run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` when GPU is available
- verify `README.md`, `RELEASE.md`, and Cargo feature wiring stay consistent

## Rollout / Fallback / Compatibility Notes

- keep variant naming additive and explicit rather than collapsing multiple lanes into one vague “GPU” label
- preserve CPU-only as the safe fallback path
- if variant names change later, update release docs and benchmark wording in the same change

## Related Docs

- `REPO_Improvement/11_docs_ops/02_ptx_packaging_fallbacks_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_DATE.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_DATE.md`
- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_DATE.md`
