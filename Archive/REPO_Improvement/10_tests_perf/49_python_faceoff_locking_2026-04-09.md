# Python Faceoff Locking Plan

Last updated: 2026-04-10

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

The faceoff harness is still scenario-driven and lockfile-backed, but it now records the actual Python executable, lockfile SHA-256, same-interpreter `pip freeze`, and `pip --version` artifacts into each run directory and summary. That means rerun credibility no longer depends purely on operator discipline after the run; the saved artifact itself now carries the core provenance needed to audit which Python environment actually produced the comparison numbers.

## Current Code Surface

- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` orchestrates the Python-based comparison path.
- `scripts/vectorbt_double_ma_faceoff.py` is the Python comparator surface.
- `scripts/requirements-vectorbt-faceoff.lock.txt` is the current locked dependency manifest for the faceoff environment.
- `scripts/vectorbt_double_ma_faceoff_scenario_*.json` files define repeatable comparison scenarios.
- `README.md` documents the dedicated `.venv-vectorbt-faceoff` setup and the explicit `-PythonExe` invocation path for reusable faceoff reruns.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 -ScenarioPath <scenario.json>`
- inspection of emitted summary JSON and environment details under the chosen output directory
- inspection of the same-run `python-freeze.txt` and `python-pip-version.txt` artifacts written with the selected interpreter

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

- [x] Document the expected Python environment setup for faceoff runs.
- [x] Document how the locked requirements file is used and refreshed.
- [x] Document how environment provenance is captured alongside results.
- [x] Keep this plan non-options only.
- [x] Use a dedicated comparison venv rather than a generic Python environment.
- [x] Avoid publishing or repeating comparison results without environment context.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- confirm the intended Python interpreter and venv before a comparison run
- capture `pip freeze` output or equivalent provenance with the same interpreter used for the actual faceoff run
- check that the scenario file, interpreter, and lock file align before refreshing artifacts
- inspect the output directory for environment/provenance context after the run
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the locked-requirements approach and improve discipline around actual venv use
- prefer a smaller number of trustworthy comparison reruns over many loosely reproducible ones
- if Python environment drift remains common, add stronger automation before trusting refreshed claims
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closure date: `2026-04-10`
- Implemented automatic Python provenance capture in `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`:
  - same-interpreter `python-freeze.txt`
  - same-interpreter `python-pip-version.txt`
  - lockfile path and SHA-256 in `summary.json`
  - selected interpreter provenance in `summary.json`
- Implemented interpreter-path capture in `scripts/vectorbt_double_ma_faceoff.py` so the saved environment block now records `sys.executable`.
- Updated `README.md` to require the dedicated `.venv-vectorbt-faceoff` setup, install from `scripts/requirements-vectorbt-faceoff.lock.txt`, and pass the explicit venv interpreter when refreshing reusable faceoff artifacts.
- Verified with a bounded smoke rerun that avoids a fresh release rebuild:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 -ScenarioPath scripts/vectorbt_double_ma_faceoff_scenario.json -OutDir target/vectorbt_vs_vectorgrid_faceoff_verify_smoke_2026-04-10 -PythonExe .\.venv-vectorbt-faceoff\Scripts\python.exe -VectorGridBackends cpu -WarmupTrials 0 -Trials 1 -SkipBuild`
- Verified output artifact location:
  - `target/vectorbt_vs_vectorgrid_faceoff_verify_smoke_2026-04-10/legacy_58300_search_only-search_only_legacy/summary.json`
  - `target/vectorbt_vs_vectorgrid_faceoff_verify_smoke_2026-04-10/legacy_58300_search_only-search_only_legacy/python-freeze.txt`
  - `target/vectorbt_vs_vectorgrid_faceoff_verify_smoke_2026-04-10/legacy_58300_search_only-search_only_legacy/python-pip-version.txt`
- Verified provenance captured in the saved summary:
  - Python executable: `C:\Rust Projects\ta_desktop_demo\.venv-vectorbt-faceoff\Scripts\python.exe`
  - Python version: `3.10.6`
  - platform: `Windows-10-10.0.26200-SP0`
  - `using_preferred_venv`: `true`
  - requirements lock SHA-256: `57185314dab0cd87be8df63cd1b599b97501f6ae0558953cd8081c184204fca8`
  - `pip.freeze_status`: `captured`
  - `pip.version_status`: `captured`
  - VectorBT stack: `vectorbt 0.28.4`, `numpy 2.2.6`, `pandas 2.3.3`, `numba 0.64.0`
- This change does not alter the timed benchmark boundaries; the provenance capture happens outside the measured faceoff trial timings.

## Related Docs

- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_2026-04-09.md`
