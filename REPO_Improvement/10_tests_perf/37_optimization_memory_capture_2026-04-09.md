# Optimization Memory Capture Plan

Last updated: 2026-04-10

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

- [x] Document the exact sampled process and workload boundary.
- [x] Document the meaning of working-set versus private-byte fields.
- [x] Keep the telemetry-only framing explicit.
- [x] Keep this capture non-options only.
- [x] Cross-link the output into post-release review rather than a main perf gate.
- [x] Avoid implying VRAM insight from host-memory telemetry.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

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

## Execution Notes

- This item closed on a fresh release-profile memory capture, a refreshed downstream supplemental review, and the workflow-regression bin’s inline tests rather than a code change.
- Fresh acceptance evidence for this closure:
  - `powershell -ExecutionPolicy Bypass -File scripts/capture_optimization_memory.ps1 -BuildProfile release -SampleIntervalMs 50`
  - `node scripts/post_release_telemetry_review.mjs`
  - `cargo test -p ta_desktop_demo_app --bin ta_workflow_regression`
- The release memory-capture script refreshed the tracked telemetry artifact:
  - `scripts/baselines/ui_capture/optimization_memory_metrics.json`
- The refreshed telemetry payload preserved the intended non-options framing and release-bin provenance:
  - `timing_boundary`: `process-memory-sampling`
  - `timing_boundary_note`: `Peak process working-set/private-byte sampling over a synthetic ta_workflow_regression run. Supplemental telemetry only; not a perf gate and not optimization-only.`
  - `build_profile`: `release`
  - `telemetry_only`: `true`
  - sampled executable: `target/release/ta_workflow_regression.exe`
  - sampled arguments: `--out target/workflow_regression_summary.json --candles-len 10000 --train-assets 10 --holdout-assets 6`
  - `runtime_ms_wall`: `283`
  - `peak_working_set_mb`: `40.95`
  - `peak_private_mb`: `38.55`
- The downstream supplemental review also refreshed the tracked consumer artifacts:
  - `scripts/baselines/post_release/telemetry_review.json`
  - `scripts/baselines/post_release/telemetry_review.md`
- The refreshed post-release review passed `17/17` checks and now explicitly preserves the supplemental-only framing rather than reading like a perf-authority gate.
- The targeted bin tests also passed:
  - `tests::stage_budget_skips_zero_baseline_and_applies_multiplier`
  - `tests::workflow_summary_deserializes_legacy_payload_without_envelope`
- No `README.md` or `REPO_MAP.md` update was required because this item verified and refreshed the existing telemetry workflow rather than changing product behavior, perf-gate policy, or route contracts.

## Related Docs

- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/54_campaign_preload_vram_perf_2026-04-09.md`
