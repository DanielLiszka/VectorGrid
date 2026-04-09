# VectorGrid Internal Benchmark Boundaries Plan

Last updated: 2026-04-09

## Purpose

Define the internal benchmark vocabulary so the repo’s many timing surfaces stay distinguishable and honest.

## Scope Boundaries

In scope:

- the timing-boundary taxonomy used across perf gates, helper bins, and faceoff scripts
- same-thread app-path, CLI cold wall-clock, Python-process wall-clock, workflow regression, Criterion, and telemetry-only boundaries
- the relation between internal timing boundaries and public-facing claims
- non-options benchmark boundary consistency across docs and artifacts

Out of scope:

- options-related benchmark surfaces
- changing the underlying benchmark tools themselves
- general benchmarking theory outside the repo’s current surfaces
- marketing or product messaging beyond repo-controlled artifacts

## Current User-Facing Behavior

The repo already has a rich internal timing vocabulary scattered across scripts and bins. That is a strength, but only if the boundaries are named and reused consistently. Today the code is often more honest than the prose, which is exactly why this doc matters.

## Current Code Surface

- `src-tauri/src/bin/ta_app_path_faceoff.rs` emits same-thread app-path boundary data with detailed timing breakdowns.
- `scripts/run_cpu_perf_gate.ps1` and `scripts/run_gpu_perf_gate.ps1` combine workflow and Criterion-style boundaries into authority gates.
- `src-tauri/src/bin/ta_workflow_regression.rs` defines the synthetic workflow-regression boundary and its exclusions.
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` emits comparison outputs across multiple distinct timing boundaries.

## Current Verification Surface

- inspection of summary JSON emitted by the various gates and helper bins
- manual cross-check of README or doc claims against those boundary labels
- rerunning the specific lane when a boundary label looks stale or ambiguous

## Strengths Already Present

- many existing artifacts already carry timing-boundary labels explicitly
- the repo distinguishes micro-bench, workflow, faceoff, and telemetry surfaces in code
- boundary language can be standardized without changing runtime code
- honest internal taxonomy directly improves research, product, and release discussions

## Weaknesses / Failure Modes

- the taxonomy is distributed across many files rather than concentrated in one doc or enum
- different contributors can still paraphrase the same boundary differently in prose
- boundary names are only useful if they are repeated consistently in summaries and docs
- readers may still prefer simplified claims, increasing pressure to flatten unlike surfaces

## Regressions To Avoid

- inventing new prose labels for existing boundaries instead of reusing the canonical ones
- merging unlike timing surfaces into one internal chart or narrative
- allowing doc claims to outlive the artifact boundary labels they were based on
- treating telemetry-only or helper-bin outputs as gate authority

## External Research Summary

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because several internal boundaries are Criterion-based while others are not.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because multiple internal surfaces ultimately originate from Cargo benchmark semantics.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU timing boundaries especially require precise measurement context.
- [Python `venv` docs](https://docs.python.org/3/library/venv.html) are relevant because Python-process wall-clock boundaries depend on controlled comparison environments.

## Proposed Improvement Approach

1. Document the internal timing-boundary vocabulary in one place and reuse it consistently across docs.
2. Tie each boundary label to one or more concrete scripts or bins.
3. Use the taxonomy as a review tool whenever benchmark prose is written or updated.
4. Prefer preserving multiple honest boundaries over creating one oversimplified aggregate number.

## Checklist

- [ ] Inventory the active internal timing boundaries and their canonical names.
- [ ] Map each boundary to its main scripts or bins.
- [ ] Keep this boundary taxonomy non-options only.
- [ ] Review benchmark prose and release notes against the canonical vocabulary.
- [ ] Mark telemetry-only and helper-bin boundaries explicitly as non-authoritative.
- [ ] Avoid introducing new ad hoc boundary names without need.

## Verification Plan

- inspect current artifact outputs for boundary-name consistency
- cross-check docs and summaries against those outputs
- re-run a lane when its emitted boundary metadata is unclear or stale
- treat inconsistent boundary naming as a documentation regression

## Rollout / Fallback / Compatibility Notes

- preserve existing explicit boundary labels and standardize around them
- prefer documentation and review discipline over new runtime complexity
- if a truly new boundary is added, add it explicitly to the taxonomy and surrounding docs

## Related Docs

- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
