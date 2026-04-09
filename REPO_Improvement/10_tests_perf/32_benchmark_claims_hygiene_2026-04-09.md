# Benchmark Claims Hygiene Plan

Last updated: 2026-04-09

## Purpose

Define the wording and evidence rules that keep benchmark numbers, faceoff charts, and performance claims honest and comparable.

## Scope Boundaries

In scope:

- benchmark wording in repo docs such as `README.md` and release-adjacent notes
- timing-boundary labeling across Criterion, app-path, CLI, Python, workflow, and telemetry surfaces
- faceoff-summary interpretation discipline
- claim review requirements before numbers are repeated externally

Out of scope:

- options-related benchmarks
- the raw implementation of the perf gates themselves
- marketing copy outside repo-controlled artifacts
- benchmark creation unrelated to current repo surfaces

## Current User-Facing Behavior

The repo already embeds more timing-boundary honesty than many projects: several scripts write boundary labels directly into output artifacts. However, the README and other summary surfaces can still drift, and numbers collected under different lanes or on different dates can be easy to flatten into one claim. This doc exists to stop that failure mode before it becomes institutional memory.

## Current Code Surface

- `README.md` currently contains benchmark-oriented narrative that must stay aligned with actual saved artifacts.
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` emits explicit boundary metadata into comparison summaries.
- `src-tauri/src/bin/ta_app_path_faceoff.rs` and the perf gates already define several boundary labels that should be reused, not paraphrased loosely.
- `scripts/baselines/*` and `target/*` hold the saved evidence that benchmark claims are supposed to reference.

## Current Verification Surface

- manual comparison between docs/claims and fresh summary artifacts
- rerunning the underlying gate or faceoff script when numbers look stale
- review of generated `summary.json` and `summary.md` outputs from comparison scripts

## Strengths Already Present

- many existing scripts already emit machine-readable timing-boundary notes
- the repo keeps checked-in baselines and scenario files rather than only ephemeral local runs
- the distinction between perf gates and supplemental telemetry already exists in code
- claim hygiene can be enforced at doc-review time without new runtime code

## Weaknesses / Failure Modes

- claim drift can happen in prose even when the underlying scripts are careful
- different boundary names are easy to forget or oversimplify
- headline comparisons can persist long after the underlying artifacts are stale
- readers often prefer one number, which increases pressure to collapse unlike timing surfaces into a single story

## Regressions To Avoid

- quoting a number without naming its timing boundary
- merging warm same-thread, CLI cold, Python, and Criterion numbers into one chart or statement
- reusing stale artifacts after major engine or environment changes
- letting README claims outlive the script outputs they were based on

## External Research Summary

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because it illustrates the difference between statistically measured micro-benchmarks and broader workflow timings.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because several repo claims ultimately trace back to Cargo benchmark surfaces.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU claims especially need precise measurement context and comparable conditions.
- [VectorBT getting started docs](https://vectorbt.dev/) are relevant because claims against VectorBT are strongest only when the compared workload and boundary are named explicitly.

## Proposed Improvement Approach

1. Require every benchmark claim to name its timing boundary, scenario, and artifact provenance.
2. Prefer multiple smaller honest statements over one oversimplified headline number.
3. Treat stale benchmark prose as a regression even when the code has not changed.
4. Use machine-readable boundary labels from script outputs whenever possible instead of inventing prose substitutes.

## Checklist

- [ ] Document the accepted timing-boundary vocabulary used in repo outputs.
- [ ] Review README and release-adjacent benchmark prose against current artifacts.
- [ ] Require scenario identity and date context for faceoff-derived claims.
- [ ] Keep benchmark-claims hygiene non-options only.
- [ ] Mark supplemental telemetry as supplemental everywhere it appears.
- [ ] Avoid comparing unlike benchmark surfaces without an explicit caveat.

## Verification Plan

- inspect current benchmark prose and cross-check it against fresh or intentional artifacts
- re-run the underlying script when evidence is stale or ambiguous
- verify that new benchmark docs reuse the canonical boundary labels emitted by the scripts
- treat missing provenance as a blocker for repeating a number

## Rollout / Fallback / Compatibility Notes

- apply claim hygiene first in repo-controlled docs and summaries
- prefer tightening wording over inventing new aggregate metrics
- if older claims cannot be revalidated, remove or downgrade them rather than guessing

## Related Docs

- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/51_vectorgrid_internal_benchmark_boundaries_DATE.md`
- `REPO_Improvement/11_docs_ops/06_repo_map_readme_hygiene_2026-04-09.md`
