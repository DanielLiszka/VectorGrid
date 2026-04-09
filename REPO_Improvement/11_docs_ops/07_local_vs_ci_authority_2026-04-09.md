# Local Vs CI Authority Plan

Last updated: 2026-04-09

## Purpose

Define which checks in this repo are authoritative locally, which are suitable for CI mirroring, and how local evidence should be treated before release or handoff.

## Scope Boundaries

In scope:

- local build/test/perf/doc checks
- current script-based gates
- local artifact and sample generation
- how CI should mirror rather than reinterpret local authority

Out of scope:

- designing a full CI pipeline
- options-only lanes
- Git hosting policy beyond verification authority

## Current User-Facing Behavior

The repo already treats local verification as the primary truth surface. The strongest evidence currently comes from locally runnable scripts and tests:

- frontend typecheck/test/parity/build
- Rust tests
- workflow/perf gates
- desktop smoke
- artifact/report parity and sample-export scripts

This means the current operating model is local-first, with CI expected to mirror those gates rather than invent a different definition of “green.”

## Current Code Surface

- `README.md`
- `RELEASE.md`
- `web/package.json`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/check_output_parity.mjs`
- `scripts/check_tauri_config_sync.mjs`
- `scripts/export_baseline_samples.mjs`

## Current Verification Surface

- all locally runnable frontend scripts in `web/package.json`
- Rust unit/integration tests under `src-tauri/tests` and crate-local test files
- perf/profiling scripts under `scripts/`
- document-program tracker and meta docs

## Strengths Already Present

- the repo already has executable local gates instead of vague checklists
- perf and benchmark gates already assume controlled local execution
- desktop smoke and GPU tooling are already Windows/local-environment aware
- the doc program already prefers evidence from actual local scripts and artifacts

## Weaknesses / Failure Modes

- CI can drift from local authority if it reimplements checks differently
- environment-sensitive gates can be overpromised if treated as universally CI-friendly
- “CI passed” can be mistaken for “all relevant local evidence exists”
- local-only artifacts can become hard to interpret if there is no written authority model

## Regressions To Avoid

- letting CI replace local perf or desktop authority where hardware/runtime assumptions differ
- adding local gates that cannot be explained or mirrored at all
- treating environment-sensitive failures as flaky instead of documenting the authority boundary
- mixing doc/program quality checks into a different standard than code/build checks

## External Research Summary

- [Cargo build command](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
  - relevant because local Rust builds are part of the current authority model
- [npm CLI docs](https://docs.npmjs.com/cli/v11/commands/npm-run-script)
  - relevant because frontend verification is script-driven
- [GitHub Actions workflow syntax](https://docs.github.com/actions/reference/workflows-and-actions/workflow-syntax)
  - relevant because CI should mirror the local command set rather than invent an unrelated one
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
  - relevant because the authority boundary should be explicit rather than implied

## Proposed Improvement Approach

1. Treat local commands and scripts as the canonical verification contract.
2. Let CI mirror that contract where environment support exists.
3. Keep hardware- or OS-sensitive gates documented as local-authority checks even if CI only runs subsets.
4. Make “not CI-suitable” distinct from “not important.”

## Checklist

- [ ] Document which checks are locally authoritative today.
- [ ] Document which checks are suitable for CI mirroring.
- [ ] Document which checks remain environment-sensitive local proofs.
- [ ] Keep CI/design discussions downstream of the local command contract.
- [ ] Review release/readme wording so it does not overstate CI authority.
- [ ] Add or tighten docs when new local gates are introduced.

## Verification Plan

- verify that documented local authority maps to real runnable commands
- verify that CI-facing docs do not contradict local-first verification
- spot-check new gates for whether they are local-only, CI-mirrorable, or both

## Rollout / Fallback / Compatibility Notes

- preserve local-first verification wording while CI remains partial
- if CI becomes broader later, update it as a mirror of the local contract rather than replacing it
- keep hardware-sensitive gates clearly marked as such

## Related Docs

- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_DATE.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_DATE.md`
- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_DATE.md`
