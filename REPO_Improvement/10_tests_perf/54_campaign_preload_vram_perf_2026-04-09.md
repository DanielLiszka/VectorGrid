# Campaign Preload VRAM Perf Plan

Last updated: 2026-04-09

## Purpose

Define the headless campaign preload and VRAM staging performance workflow and keep it separate from the main performance gates.

## Scope Boundaries

In scope:

- the campaign perf bin `src-tauri/src/bin/ta_campaign_perf.rs`
- the lower-level prefetch helper `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`
- host-only versus staged modes and related preload-policy questions
- campaign preload timing and staging outcome summaries

Out of scope:

- authoritative CPU or GPU perf gating
- options-related campaign work
- desktop-shell performance
- generic memory telemetry outside campaign preload questions

## Current User-Facing Behavior

The repo already has both a higher-level campaign perf harness and a lower-level staged-prefetch helper. Together they let contributors ask focused questions about host-only versus staged dataset preparation, preload policy, and VRAM-prefetch outcomes. That is valuable, but only if the tooling stays documented as an investigative lane rather than another headline benchmark.

## Current Code Surface

- `src-tauri/src/bin/ta_campaign_perf.rs` implements the higher-level campaign-runner perf harness.
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs` isolates staged versus host-only prefetch behavior.
- `src-tauri/src/commands/*` campaign-related entry points and app-core preload logic feed these bins indirectly.
- output JSON written by the bins under `target/` or other chosen paths records trial summaries and preload outcomes.

## Current Verification Surface

- `cargo run -p ta_desktop_demo_app --bin ta_campaign_perf -- --help`
- `cargo run -p ta_desktop_demo_app --release --bin ta_campaign_perf -- --output-json <path> --mode host-only`
- `cargo run -p ta_app_core --release --bin profile_doublema_campaign_prefetch --features cuda,cuda-backtest-kernel -- --mode staged`
- inspection of emitted JSON summaries and staging outcome fields

## Strengths Already Present

- the repo already separates higher-level campaign timing from lower-level staged-prefetch A/B probing
- the bins emit structured artifacts rather than only console timing
- preload-policy modes are explicit and therefore reviewable
- the tooling helps answer VRAM-staging questions that broader gates intentionally avoid

## Weaknesses / Failure Modes

- the surfaces are specialized and easy to overinterpret
- feature-gated CUDA requirements limit where the lower-level helper can run
- host-only vs staged comparisons do not automatically translate into end-user desktop experience
- there is no single top-level authority wrapper today, which is correct but can confuse expectations

## Regressions To Avoid

- quoting campaign preload numbers as if they were main product-performance authority
- confusing higher-level campaign timing with low-level staged-prefetch timings
- running the staged helper without the correct CUDA feature or environment and misreading failures
- changing preload modes or dataset assumptions without recording them in the output provenance
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because VRAM staging and transfer strategy require careful GPU measurement interpretation.
- [Nsight Systems docs](https://docs.nvidia.com/nsight-systems/) are relevant because campaign preload questions may eventually need timeline-level follow-up.
- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because both bins are invoked directly as investigative tools.
- [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html) is relevant because the lower-level prefetch helper is feature-gated.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep campaign preload and VRAM staging tools documented as investigative aids, not release gates.
2. Separate higher-level campaign timing from lower-level staged-prefetch A/B results clearly.
3. Preserve explicit mode, dataset, and feature provenance in run artifacts and docs.
4. Use these tools when preload or VRAM behavior is the actual question, not as generic benchmark replacements.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document both the higher-level and lower-level campaign/preload tools.
- [ ] Document their feature requirements and timing boundaries.
- [ ] Keep this perf plan non-options only.
- [ ] Review emitted artifacts for mode and staging-outcome clarity.
- [ ] Avoid using campaign preload numbers for broad product claims.
- [ ] Cross-link campaign preload work to deeper GPU profiling only when needed.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the relevant bin in the intended mode after preload-policy changes
- inspect the emitted JSON for staged/host-only outcome fields
- confirm CUDA feature requirements are met before trusting lower-level results
- compare results only across runs with matching mode, dataset, and feature context
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve both levels of tooling while documenting their distinct roles
- prefer better provenance and clearer mode labels over more thresholds
- if one tool becomes the clear authority for a repeated investigation type, document that promotion explicitly
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/37_optimization_memory_capture_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
