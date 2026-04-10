# Profile Runner Bins Plan

Last updated: 2026-04-10

## Purpose

Define the role and limits of the standalone profiling/helper binaries that sit below the main gates and playbooks.

## Scope Boundaries

In scope:

- profiling/helper bins such as `profile_gpu_dynamic_runner`, `profile_doublema_campaign_prefetch`, and `ta_app_path_faceoff`
- their build features, timing boundaries, and intended use cases
- the difference between helper bins and authority gates
- repo-local workflows that depend on these bins for deeper investigation

Out of scope:

- options-related bins
- full frontend or desktop integration
- authority perf gating logic itself
- general CLI design outside these profiling/helper tools

## Current User-Facing Behavior

The repo already contains several helper binaries that exist specifically to make focused profiling or comparison possible. They are powerful because they isolate questions that would be awkward in the full app, but they also create a documentation burden: helper bins can easily be mistaken for authoritative benchmark surfaces unless their boundaries are made explicit.

## Current Code Surface

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs` provides a GPU-focused runner surface for profiling and ratio checks.
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs` isolates host-only versus staged campaign prefetch behavior.
- `src-tauri/src/bin/ta_app_path_faceoff.rs` provides a same-thread app-state faceoff surface with detailed timing breakdown.
- `src-tauri/src/bin/ta_campaign_perf.rs` provides headless campaign preload/perf investigation above the lower-level prefetch helper.

## Current Verification Surface

- `cargo build -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel`
- `cargo run -p ta_app_core --release --bin profile_doublema_campaign_prefetch --features cuda,cuda-backtest-kernel -- --mode staged`
- `cargo run -p ta_desktop_demo_app --release --bin ta_app_path_faceoff -- --help`
- `cargo run -p ta_desktop_demo_app --release --bin ta_campaign_perf -- --help`

## Strengths Already Present

- the bins isolate specific profiling or comparison questions cleanly
- they let the repo investigate lower-level behavior without distorting the main app surface
- some already include inline tests or structured output artifacts
- they are reusable building blocks for the gate and profiling scripts

## Weaknesses / Failure Modes

- their timing boundaries differ substantially from one another
- feature gating and binary-specific prerequisites make them easy to misuse
- helper-bin outputs can look more authoritative than they really are
- bin proliferation can create confusion if their roles are not cataloged clearly

## Regressions To Avoid

- quoting helper-bin results as if they were equivalent to gate outputs
- forgetting feature requirements and then interpreting build/run failures as product regressions
- using a helper bin when a simpler gate or smoke run would answer the question
- adding new helper bins without documenting their exact boundary and intended consumer
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Cargo build command docs](https://doc.rust-lang.org/cargo/commands/cargo-build.html) are relevant because these bins are often built explicitly with feature flags rather than run through higher-level wrappers.
- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because these tools are used directly as investigative binaries.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because several of these bins exist to isolate GPU/runtime behavior that broader app paths obscure.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant as a contrast because helper bins are not statistical benchmark authorities by default.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Catalog helper bins by question type and timing boundary rather than by file location alone.
2. Make it explicit which higher-level scripts or gates depend on each helper bin.
3. Keep helper-bin results subordinate to the gates or playbooks that interpret them.
4. Require new helper bins to declare their boundary and intended use before they are treated as part of the repo’s evidence surface.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory the current non-options profiling/helper bins and their timing boundaries.
- [x] Document feature and environment prerequisites for each one.
- [x] Map helper bins to the scripts or docs that rely on them.
- [x] Keep helper-bin guidance non-options only.
- [x] Avoid presenting helper-bin outputs as authority unless a separate gate wraps them.
- [x] Review whether any helper bin has become obsolete or redundant.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `--help` or a minimal execution path for helper bins when their interfaces change
- verify feature-gated build commands still succeed for the intended bins
- check that higher-level scripts still reference the expected helper bins and output shapes
- confirm helper-bin output language does not overstate its authority
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve current useful helper bins while tightening their documentation
- prefer better cataloging over adding wrappers if the existing bins already answer the question
- retire or de-emphasize helper bins only after confirming no gate or playbook still depends on them
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on fresh helper-bin build/interface checks plus one real staged helper-bin run rather than a code change.
- Fresh acceptance evidence for this closure:
  - `cargo build -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel`
  - `cargo run -p ta_app_core --release --bin profile_doublema_campaign_prefetch --features cuda,cuda-backtest-kernel -- --mode staged`
  - `cargo run -p ta_desktop_demo_app --release --bin ta_app_path_faceoff -- --help`
  - `cargo run -p ta_desktop_demo_app --release --bin ta_campaign_perf -- --help`
  - `nvidia-smi --query-gpu=name,driver_version --format=csv,noheader`
- Verified local CUDA provenance for the helper-bin tranche:
  - GPU: `NVIDIA GeForce RTX 4090`
  - driver: `595.71`
- The helper-bin surfaces exercised in this closure matched the documented catalog:
  - `profile_gpu_dynamic_runner` built successfully in `release` with `cuda,cuda-backtest-kernel`, which keeps the lower-level GPU runner surface available to `10_tests_perf/27`, `10_tests_perf/33`, and `10_tests_perf/34`
  - `profile_doublema_campaign_prefetch` completed a real staged run and returned structured JSON with `stage_attempts=3`, `stage_successes=3`, `stage_errors=0`, and `GpuKernel` on both first and second route observations for every measured iteration
  - `ta_app_path_faceoff --help` confirmed the app-path faceoff CLI still exposes the documented same-thread faceoff boundary, including required `--csv` and `--output-json` arguments plus backend and trial controls
  - `ta_campaign_perf --help` confirmed the headless campaign perf harness still exposes the documented preload/staging boundary, including `--mode`, `--scope`, `--backend`, and `--output-json`
- The current repo-local dependency map remains intact rather than indicating obsolescence:
  - `profile_gpu_dynamic_runner` is still the runner surface underneath the GPU perf gate and Nsight playbooks
  - `profile_doublema_campaign_prefetch` still isolates staged prefetch behavior for deeper campaign/runtime investigation
  - `ta_app_path_faceoff` still provides the app-path faceoff surface
  - `ta_campaign_perf` still provides the higher-level headless campaign preload/perf harness
- No `README.md` or `REPO_MAP.md` update was required because the helper-bin inventory and boundary catalog were already present; this item verified those existing surfaces rather than adding or retiring bins.

## Related Docs

- `REPO_Improvement/10_tests_perf/33_ncu_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/34_nsys_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/36_gpu_route_matrix_profile_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/54_campaign_preload_vram_perf_2026-04-09.md`
