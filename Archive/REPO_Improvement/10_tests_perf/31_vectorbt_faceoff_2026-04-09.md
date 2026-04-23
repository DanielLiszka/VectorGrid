# VectorBT Faceoff Plan

Last updated: 2026-04-09

## Purpose

Define the comparison-harness contract for the VectorGrid versus VectorBT faceoff path and keep its timing-boundary claims honest.

## Scope Boundaries

In scope:

- the faceoff wrapper `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- the Python comparator `scripts/vectorbt_double_ma_faceoff.py` and scenario files
- the VectorGrid-only helper `scripts/run_vectorgrid_double_ma_faceoff.ps1`
- timing-boundary labeling for app-path, CLI-cold, Python-process, and Criterion-style outputs

Out of scope:

- options-related faceoffs
- general product marketing copy beyond what the harness can substantiate
- GPU perf gates
- benchmarking claims that ignore the comparison mode and scenario used

## Current User-Facing Behavior

The repo already has a substantial faceoff harness that can compare VectorGrid and VectorBT under different timing modes and scenarios, emitting structured outputs that label the timing boundary explicitly. It is useful because it already encodes more honesty than many benchmark scripts, but it remains easy to misuse: app-path same-thread runs, CLI cold launches, and Python wall-clock results are not interchangeable, and the metric-contract lane can shift between legacy search-only and richer real-workflow summaries.

## Current Code Surface

- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` orchestrates the comparison workflow and artifact generation.
- `scripts/vectorbt_double_ma_faceoff.py` implements the Python/vectorbt side of the comparison.
- `scripts/vectorbt_double_ma_faceoff_scenario_*.json` files define comparable scenario inputs.
- `scripts/run_vectorgrid_double_ma_faceoff.ps1` and `src-tauri/src/bin/ta_app_path_faceoff.rs` provide the VectorGrid-specific timing surfaces used by the faceoff.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 -ScenarioPath <scenario.json>`
- summary JSON and Markdown output under the chosen `OutDir`
- VectorGrid-only helper outputs under `target/vectorgrid_double_ma_faceoff*` when isolating one side

## Strengths Already Present

- the faceoff harness already emits explicit timing-boundary metadata instead of burying it
- scenarios are checked in and can be rerun intentionally
- the harness supports multiple VectorGrid modes, which helps separate warm app-path timing from colder CLI timing
- environment and artifact outputs make post-run review possible instead of leaving only console text

## Weaknesses / Failure Modes

- the comparison environment spans PowerShell, Rust, Python, and potentially Numba/vectorbt runtime behavior, so reproducibility is fragile
- mixed timing boundaries are very easy to misstate in external summaries
- the Python environment is only as reproducible as the local venv and pinned requirements discipline around it
- the scenario files and README claims can drift apart if reruns are not carefully curated

## Regressions To Avoid

- quoting faceoff numbers without naming the exact scenario and timing boundary
- mixing app-path same-thread numbers with CLI or Python wall-clock numbers
- refreshing Python-side dependencies or vectorbt versions without recording the environment change
- using stale faceoff artifacts to justify current performance claims
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [VectorBT getting started docs](https://vectorbt.dev/) are relevant because the comparison harness needs to stay clear about what VectorBT itself is optimizing for and how it runs.
- [Python `venv` docs](https://docs.python.org/3/library/venv.html) are relevant because comparison reproducibility depends on a stable Python environment.
- [`pip freeze` docs](https://pip.pypa.io/en/stable/cli/pip_freeze.html) are relevant because environment snapshots are necessary if faceoff results are meant to be revisitable.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant as a contrast: some VectorGrid numbers are Criterion-like while others are app-path or CLI wall clock, and the harness must preserve that distinction.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the faceoff harness classified as a comparison tool with explicit timing-boundary labeling, not a single universal benchmark.
2. Record scenario identity, environment identity, and comparison mode alongside every saved result.
3. Separate reproducibility concerns for Python/vectorbt from those for the Rust-side harness.
4. Require any external claim derived from faceoff artifacts to name the exact lane, scenario, and timing boundary used.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the supported comparison modes and their timing boundaries.
- [ ] Document the scenario files that currently matter and what they measure.
- [ ] Document Python-environment expectations and how they are frozen or inspected.
- [ ] Keep the faceoff harness non-options only.
- [ ] Review README claims against fresh summary artifacts before repeating them elsewhere.
- [ ] Avoid merging incomparable lanes into one headline number.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the faceoff script with a named checked-in scenario after meaningful engine changes
- inspect the emitted summary JSON and Markdown for timing-boundary labels
- capture or review the Python environment state before trusting refreshed comparison numbers
- cross-check app-path and CLI modes separately instead of averaging them mentally
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current multi-lane honesty of the harness rather than collapsing it into simplified claims
- prefer a smaller set of well-understood scenarios over a large uncurated scenario catalog
- if environment drift is suspected, regenerate the comparison under a controlled Python setup before changing docs
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/49_python_faceoff_locking_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/51_vectorgrid_internal_benchmark_boundaries_2026-04-09.md`
