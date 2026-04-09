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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Cargo build command](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
  - relevant because release-variant planning depends on explicit feature-flag behavior
- [The Cargo Book: Features](https://doc.rust-lang.org/cargo/reference/features.html)
  - relevant because this repo’s product variants are feature-driven
- [Tauri build and bundle guide](https://v2.tauri.app/distribute/)
  - relevant because desktop packaging needs to preserve the intended variant semantics
- [Tauri Windows installer docs](https://v2.tauri.app/ja/distribute/windows-installer/)
  - relevant because Windows packaging is the current release target described by the repo docs
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

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
6. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current CPU-only, GPU, and GPU-kernel release variants.
- [ ] Document feature forwarding between `src-tauri` and `ta_app_core`.
- [ ] Document runtime prerequisites separately from compile-time features.
- [ ] Document which verification surfaces are variant-specific.
- [ ] Keep README and `RELEASE.md` variant language aligned.
- [ ] Tie benchmark claims to explicit variants.
- [ ] Add or tighten checks if variant wiring changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- run the documented build commands for each supported non-options variant
- run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` when GPU is available
- verify `README.md`, `RELEASE.md`, and Cargo feature wiring stay consistent
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep variant naming additive and explicit rather than collapsing multiple lanes into one vague “GPU” label
- preserve CPU-only as the safe fallback path
- if variant names change later, update release docs and benchmark wording in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/02_ptx_packaging_fallbacks_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_2026-04-09.md`
