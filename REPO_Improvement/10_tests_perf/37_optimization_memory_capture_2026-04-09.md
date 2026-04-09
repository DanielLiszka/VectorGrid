# Optimization Memory Capture Plan

Last updated: 2026-04-09

## Purpose

Define the supplemental process-memory capture workflow around `ta_workflow_regression` and keep it clearly separate from authority perf gates.

## Scope Boundaries

In scope:

- the wrapper `scripts/capture_optimization_memory.ps1`
- peak working-set and private-byte sampling over a synthetic workflow-regression run
- memory telemetry JSON output under `scripts/baselines/ui_capture/optimization_memory_metrics.json`
- the relation between this capture and post-release supplemental review

Out of scope:

- VRAM profiling
- authoritative performance gating
- options-related flows
- frontend browser heap metrics, which are captured separately by the baseline UI script

## Current User-Facing Behavior

The current memory-capture script builds or reuses the `ta_workflow_regression` executable, launches it with a synthetic workload, samples process working set and private bytes on a fixed interval, and writes a JSON payload that explicitly marks itself as telemetry-only. That is exactly the right direction, but without a dedicated doc it is too easy for this JSON to be mistaken for a full optimization-memory or GPU-memory benchmark.

## Current Code Surface

- `scripts/capture_optimization_memory.ps1` orchestrates the sampling run and telemetry JSON emission.
- `src-tauri/src/bin/ta_workflow_regression.rs` provides the synthetic workload surface being sampled.
- `scripts/baselines/ui_capture/optimization_memory_metrics.json` is the default output target used downstream.
- `scripts/post_release_telemetry_review.mjs` consumes this telemetry as supplemental evidence.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/capture_optimization_memory.ps1 -BuildProfile release`
- review of the emitted JSON payload and its `telemetry_only` flag
- comparison with workflow summary output when correlating runtime and memory data

## Strengths Already Present

- the script already labels itself as telemetry-only
- it samples a deterministic synthetic workflow instead of arbitrary ad hoc runs
- it writes a simple machine-readable payload that can be aggregated later
- it captures memory without needing a heavyweight profiler for basic trend inspection

## Weaknesses / Failure Modes

- sampling can miss short-lived spikes
- the measured process includes the synthetic workflow bin more broadly than just “optimization”
- it captures host memory only, not VRAM
- the default output location alongside UI metrics can make it look more authoritative than it is

## Regressions To Avoid

- treating sampled peak working set as a release gate without additional rigor
- describing the output as optimization-only when it covers a broader synthetic workflow run
- dropping the `telemetry_only` framing from the payload or doc language
- assuming host-memory telemetry says anything direct about GPU memory staging
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Cargo build command docs](https://doc.rust-lang.org/cargo/commands/cargo-build.html) are relevant because the wrapper explicitly builds the workflow bin when needed.
- [Cargo profiles reference](https://doc.rust-lang.org/cargo/reference/profiles.html) are relevant because the script distinguishes `dev` and `release` capture modes.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant as a contrast because host-memory sampling is not a substitute for device-memory analysis.
- [Microsoft WebView2 user data folder guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder) is relevant by contrast because this telemetry script bypasses the shell entirely and therefore must not be confused with desktop-memory behavior.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep optimization memory capture documented as host-memory telemetry over a synthetic workflow run.
2. Preserve the telemetry-only framing in both payload and surrounding docs.
3. Use the output to flag suspicious trends and to support post-release review, not as an authority gate by itself.
4. Escalate real memory concerns into targeted host or GPU profiling workflows when telemetry suggests a problem.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the exact sampled process and workload boundary.
- [ ] Document the meaning of working-set versus private-byte fields.
- [ ] Keep the telemetry-only framing explicit.
- [ ] Keep this capture non-options only.
- [ ] Cross-link the output into post-release review rather than a main perf gate.
- [ ] Avoid implying VRAM insight from host-memory telemetry.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the script after memory-sensitive workflow changes
- inspect the JSON payload for expected schema fields and telemetry labels
- compare repeated runs for coarse trend detection only
- follow up with deeper profiling if telemetry shows unusual growth
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current lightweight telemetry path while clarifying its limits
- prefer additive notes and provenance over more aggressive thresholds
- keep VRAM-specific concerns in separate tools and docs
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/54_campaign_preload_vram_perf_2026-04-09.md`
