# PTX Packaging Fallbacks Plan

Last updated: 2026-04-09

## Purpose

Define the current PTX packaging and fallback contract so GPU-kernel builds remain reproducible, honest, and compatible across developer and packaging environments.

## Scope Boundaries

In scope:

- build-time PTX generation and fallback logic
- prebuilt PTX usage
- NVCC-required versus NVCC-optional behavior
- environment toggles controlling PTX selection
- release/build documentation for kernel packaging

Out of scope:

- CUDA kernel algorithm changes
- options-specific roadmap decisions beyond their current PTX artifact handling
- runtime GPU profiling methodology

## Current User-Facing Behavior

The repo already supports two practical kernel-artifact paths for the desktop GPU-kernel build:

- use prebuilt PTX
- compile PTX locally with NVCC

Current behavior is intentionally compatibility-oriented:

- `VECTORBT_USE_PREBUILT_PTX=1` forces prebuilt PTX usage
- `VECTORBT_REQUIRE_NVCC=1` turns missing/failing NVCC into a hard failure
- otherwise the build prefers local NVCC when available and falls back to prebuilt PTX when allowed

This is already a product-facing contract because the Windows release docs tell users the CUDA Toolkit is not required when prebuilt PTX is used.

## Current Code Surface

- `crates/app_core/build.rs`
  - owns PTX/cubin build selection, fallback logic, required-symbol validation, and environment toggle behavior
- `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
  - current packaged fallback PTX for the backtest kernel
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`
  - packaged fallback PTX for the options-expression kernel
- `RELEASE.md`
  - release-facing PTX fallback wording
- `src-tauri/Cargo.toml`
  - variant feature wiring that activates kernel builds

## Current Verification Surface

- `cargo build -p ta_desktop_demo_app --features cuda-backtest-kernel`
- targeted local builds with `VECTORBT_USE_PREBUILT_PTX=1`
- targeted local builds with `VECTORBT_REQUIRE_NVCC=1`
- GPU perf gate and profiling flows after a successful kernel build

## Strengths Already Present

- the fallback logic is already centralized in one build script
- prebuilt PTX is validated for required kernel symbols before being accepted
- NVCC failure and NVCC absence are already distinguished from hard-requirement mode
- release docs already acknowledge the fallback model instead of assuming a developer-only environment

## Weaknesses / Failure Modes

- legacy environment-variable names still exist and can obscure current product wording
- stale prebuilt PTX can silently become invalid if symbol validation is incomplete
- developer expectations can drift from packaging expectations if NVCC and prebuilt lanes are not documented together
- kernel packaging behavior spans build script, release docs, and feature selection, which makes drift likely without one dedicated doc

## Regressions To Avoid

- shipping a build that relies on stale or symbol-incomplete PTX
- breaking the prebuilt fallback lane while hardening NVCC builds
- documenting NVCC as required when the packaged fallback lane is intentionally supported
- changing environment-toggle behavior without updating release guidance
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
  - relevant because the PTX selection contract is implemented in `build.rs`
- [NVIDIA CUDA Compiler Driver NVCC](https://docs.nvidia.com/cuda/cuda-compiler-driver-nvcc/)
  - relevant because local PTX generation depends on NVCC behavior and architecture support
- [Tauri distribute/build docs](https://v2.tauri.app/distribute/)
  - relevant because release packaging needs to honor the same artifact-selection rules as local builds
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf)
  - relevant because packaging a verified fallback path is part of making GPU behavior dependable across environments
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Document the PTX artifact contract directly from `build.rs`.
2. Keep prebuilt-versus-NVCC behavior explicit in release docs and implementation docs.
3. Preserve symbol validation as a non-negotiable acceptance check for packaged PTX.
4. Treat environment toggles as compatibility-critical behavior, not as incidental dev flags.
5. Keep kernel packaging truth aligned with the documented Windows release story.
6. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document current PTX artifact inputs and outputs.
- [ ] Document current prebuilt fallback conditions.
- [ ] Document current NVCC-required conditions.
- [ ] Document required-symbol validation behavior.
- [ ] Document the environment toggles and their compatibility implications.
- [ ] Keep release/build docs aligned with actual `build.rs` behavior.
- [ ] Add or tighten build verification if PTX selection logic changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- run `cargo build -p ta_desktop_demo_app --features cuda-backtest-kernel`
- rerun with `VECTORBT_USE_PREBUILT_PTX=1`
- rerun with `VECTORBT_REQUIRE_NVCC=1` in an NVCC-equipped environment
- verify the built artifact path still passes the repo’s GPU perf/profiling gates
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together

## Rollout / Fallback / Compatibility Notes

- preserve the prebuilt PTX fallback lane unless it is explicitly retired with replacement guidance
- keep legacy environment-variable names documented until they are intentionally migrated
- if symbol requirements change, regenerate and revalidate the packaged PTX in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/33_ncu_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/41_gpu_alma_contract_gate_2026-04-09.md`
