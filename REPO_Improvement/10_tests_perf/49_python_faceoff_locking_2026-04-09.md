# Python Faceoff Locking Plan

Last updated: 2026-04-09

## Purpose

Define the reproducibility and environment-locking expectations for the Python side of the VectorBT comparison harness.

## Scope Boundaries

In scope:

- Python environment expectations for `run_vectorbt_vs_vectorgrid_faceoff.ps1`
- locked requirements and scenario reproducibility for the VectorBT comparison path
- the relation between local `.venv-vectorbt-faceoff`, interpreter choice, and saved summary artifacts
- environment snapshotting for rerun credibility

Out of scope:

- options-related Python tooling
- general Python packaging policy for the whole repo
- non-Python performance gates
- benchmark-claim wording beyond environment reproducibility

## Current User-Facing Behavior

The faceoff harness already supports a locked requirements file and scenario-driven runs, but it still relies on the local Python interpreter and environment being prepared correctly. That means rerun credibility depends on environment discipline more than the script itself. The doc therefore needs to be explicit about how Python state is controlled and recorded when comparison artifacts are generated.

## Current Code Surface

- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` orchestrates the Python-based comparison path.
- `scripts/vectorbt_double_ma_faceoff.py` is the Python comparator surface.
- `scripts/requirements-vectorbt-faceoff.lock.txt` is the current locked dependency manifest for the faceoff environment.
- `scripts/vectorbt_double_ma_faceoff_scenario_*.json` files define repeatable comparison scenarios.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 -ScenarioPath <scenario.json>`
- inspection of emitted summary JSON and environment details under the chosen output directory
- manual environment snapshotting via `py -m pip freeze` when rerun credibility matters

## Strengths Already Present

- the repo already has a locked requirements file for the Python comparison lane
- the faceoff harness is scenario-driven rather than ad hoc
- comparison artifacts can already capture substantial context
- the Python lane is isolated from the main Rust build and gate surfaces

## Weaknesses / Failure Modes

- environment drift is still possible if the wrong interpreter or venv is used
- local `.venv-vectorbt-faceoff` use is preferred by convention rather than enforced automatically everywhere
- Python, vectorbt, and Numba behavior can vary across environments in ways that are hard to spot after the fact
- comparison numbers are only as trustworthy as the environment snapshot that accompanies them

## Regressions To Avoid

- rerunning faceoff artifacts from an untracked or mixed Python environment
- refreshing comparison numbers without preserving the environment context
- treating the locked requirements file as sufficient if the actual active interpreter does not use it
- using the Python comparison lane for claims while ignoring environment drift risk
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Python `venv` docs](https://docs.python.org/3/library/venv.html) are relevant because comparison reproducibility depends on controlled Python environments.
- [`pip freeze` docs](https://pip.pypa.io/en/stable/cli/pip_freeze.html) are relevant because environment snapshots are necessary for rerun credibility.
- [Requirements file format docs](https://pip.pypa.io/en/stable/reference/requirements-file-format/) are relevant because the faceoff lane already uses a locked requirements manifest.
- [VectorBT docs](https://vectorbt.dev/) are relevant because the comparison lane must remain clear about the Python library and runtime stack it is actually exercising.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat the Python faceoff environment as a controlled comparison instrument, not a casual local script context.
2. Require environment snapshotting or explicit interpreter provenance for any refreshed comparison artifact intended to be reused.
3. Prefer a dedicated local venv for the comparison lane and keep the lock file current when intentional dependency changes happen.
4. Separate environment reproducibility from claim interpretation so comparison results stay auditable.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the expected Python environment setup for faceoff runs.
- [ ] Document how the locked requirements file is used and refreshed.
- [ ] Document how environment provenance is captured alongside results.
- [ ] Keep this plan non-options only.
- [ ] Use a dedicated comparison venv rather than a generic Python environment.
- [ ] Avoid publishing or repeating comparison results without environment context.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- confirm the intended Python interpreter and venv before a comparison run
- capture `pip freeze` output or equivalent provenance when results matter beyond one local session
- check that the scenario file, interpreter, and lock file align before refreshing artifacts
- inspect the output directory for environment/provenance context after the run
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the locked-requirements approach and improve discipline around actual venv use
- prefer a smaller number of trustworthy comparison reruns over many loosely reproducible ones
- if Python environment drift remains common, add stronger automation before trusting refreshed claims
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_2026-04-09.md`
