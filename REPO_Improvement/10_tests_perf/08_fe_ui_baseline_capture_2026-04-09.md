# Frontend UI Baseline Capture Plan

Last updated: 2026-04-09

## Purpose

Define how the non-options `baseline:ui` capture path should produce useful review artifacts without turning screenshot generation into a flaky pseudo-test suite.

## Scope Boundaries

In scope:

- the built-app baseline capture script in `web/scripts/capture-baseline-ui.mjs`
- workspace, workflow-step, and overlay screenshot coverage for the current non-options shell
- captured timing and heap metrics such as `perf_metrics.json`
- review expectations for screenshots generated under `scripts/baselines/ui_capture/`

Out of scope:

- options-related screens or artifacts
- pixel-perfect golden-image diffing as a required gate
- real native-shell verification, which belongs to desktop smoke
- accessibility auditing, which belongs to `a11y:smoke` and related targeted docs

## Current User-Facing Behavior

The repo already has a broad baseline capture script that serves the built `web/dist` bundle, injects mocked-Tauri behavior, optionally holds the startup overlay, captures the major workspaces and workflow steps, and writes coarse load metrics plus a wide set of screenshots under `scripts/baselines/ui_capture/`. It is currently more of a review-evidence generator than a formally owned verification layer, so it needs a sharper contract around what screenshots are authoritative, which metrics matter, and when a UI change is expected to refresh the capture set.

## Current Code Surface

- `web/scripts/capture-baseline-ui.mjs` drives the local static server, mocked-Tauri boot path, screenshot sequence, and metric emission.
- `scripts/baselines/ui_capture/` stores the generated PNG evidence and `perf_metrics.json` review payload.
- `web/package.json` exposes the `baseline:ui` script and therefore defines how contributors actually trigger the capture path.
- `web/src/App.vue` and shell-level components determine which high-level surfaces the capture script is attempting to record.

## Current Verification Surface

- `npm --prefix web run build`
- `npm --prefix web run baseline:ui`
- manual review of regenerated screenshots under `scripts/baselines/ui_capture/`
- manual review of `scripts/baselines/ui_capture/perf_metrics.json` for obviously suspicious load or heap drift

## Strengths Already Present

- the script runs against the built app, so it observes the same bundle shape that release packaging sees
- mocked-Tauri boot makes the run deterministic enough to capture broad shell state without needing the real desktop runtime
- the script already records both visual artifacts and coarse performance telemetry instead of only screenshots
- the capture flow already knows how to hold and dismiss the startup overlay so boot visuals can still be reviewed even while the overlay is disabled in the shipped shell

## Weaknesses / Failure Modes

- the capture set is broad, which makes it easy to collect too many screenshots without a clear review rubric
- mocked-Tauri mode cannot prove native titlebar, desktop window, or live-runtime correctness
- full-page screenshots can be noisy when layout churn is legitimate, so they are useful evidence but a poor hard gate by themselves
- the current metric payload is coarse and can be misread as a benchmark if the doc does not explicitly classify it as review evidence

## Regressions To Avoid

- treating baseline capture as a replacement for component tests, browser UX smoke, or desktop smoke
- allowing screenshot names or output paths to drift without documenting the new review contract
- promoting raw timing values into release-performance authority without a real benchmark harness
- removing high-value screens such as startup, workspace, or results surfaces because they are inconvenient to refresh
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because baseline capture should emphasize stable high-value flows instead of brittle incidental DOM details.
- [Playwright screenshots](https://playwright.dev/docs/screenshots) are relevant because the current script depends on deterministic full-page image capture rather than ad hoc browser screenshots.
- [Playwright visual comparisons](https://playwright.dev/docs/test-snapshots) are relevant as a cautionary reference: the repo should borrow review discipline from visual testing without adopting a full golden-image gate by default.
- [Playwright Trace Viewer](https://playwright.dev/docs/trace-viewer) is relevant because failed browser-harness issues may require richer debug artifacts than static screenshots alone.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Classify `baseline:ui` as a review-evidence generator for broad shell surfaces, not as the primary correctness gate.
2. Define a stable required capture set: startup overlay evidence, top-level workspaces, workflow steps, optimize/analyze result tabs, and settings diagnostics.
3. Keep `perf_metrics.json` narrowly scoped to build review and obvious drift detection, and explicitly separate it from CPU/GPU perf-gate authority.
4. Document when screenshot refresh is expected, when a capture delta is benign, and when the reviewer must escalate the change into component, browser, or desktop tests.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Inventory the currently generated screenshot set and classify each image as required, optional, or redundant.
- [ ] Document the meaning and non-authoritative nature of the captured performance metrics.
- [ ] Tie each required screenshot family to a user-visible shell surface or workflow step.
- [ ] Define a refresh procedure for legitimate UI changes that avoids accidental stale baselines.
- [ ] Define reviewer prompts for layout drift, missing chrome, broken empty states, and bad result readability.
- [ ] Keep the capture path non-options only and explicitly exclude options workspaces or options artifacts.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run baseline:ui` after shell/layout changes that affect broad visual surfaces
- confirm the script still emits the expected screenshot families and `perf_metrics.json`
- spot-check that the captured shell states still align with the current workspace and workflow routing model
- ensure no contributor is treating baseline capture as a substitute for narrower deterministic tests
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- keep the script additive and descriptive before tightening any future gating rules
- preserve existing screenshot artifacts until a replacement capture taxonomy is documented
- if a capture becomes too noisy to be useful, narrow the capture target rather than deleting baseline evidence entirely
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
