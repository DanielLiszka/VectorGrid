# Desktop Artifact Capture Plan

Last updated: 2026-04-09

## Purpose

Define the artifact, log, and screenshot contract for desktop smoke so failures remain diagnosable without bloating the harness.

## Scope Boundaries

In scope:

- desktop smoke artifact capture under `output/wdio/desktop-smoke/`
- driver, preview-server, and environment logs
- failure screenshot naming and storage rules
- how desktop artifact capture complements Playwright traces and browser artifacts

Out of scope:

- broad visual regression gating
- options-related artifact capture
- persistent production telemetry collection
- third-party reporter integration unless it materially improves the existing repo-specific capture path

## Current User-Facing Behavior

The current desktop smoke harness already creates a per-run artifact context, writes driver and preview logs, records an environment summary, and stores failure screenshots under a stable directory rooted at `output/wdio/desktop-smoke/`. This is strong for a repo-local harness, but there is no dedicated implementation doc describing which artifacts are authoritative, when additional capture is worth adding, and how to keep this capture layer from devolving into unstructured log sprawl.

## Current Code Surface

- `web/e2e/desktop/support/artifacts.ts` owns run IDs, directories, file-safe naming, and log/screenshot path helpers.
- `web/wdio.desktop.conf.ts` wires artifact creation into the desktop smoke lifecycle.
- `web/e2e/desktop/README.md` documents the current artifact outputs and troubleshooting workflow.
- `output/wdio/desktop-smoke/` is the current local artifact root used by the harness.

## Current Verification Surface

- `npm --prefix web run test:desktop:smoke`
- `npm --prefix web run test:desktop:smoke:debug`
- manual review of driver logs, preview logs, environment summaries, and failure screenshots

## Strengths Already Present

- the harness already captures the most important local debug artifacts without extra tooling
- artifact naming is file-safe and grouped by run, which makes repeated local debugging manageable
- logs cover both harness infrastructure and app launch context
- desktop artifact capture is already treated as part of the harness rather than an afterthought

## Weaknesses / Failure Modes

- artifact review expectations are still implicit and can vary by contributor
- the harness captures enough information to debug many failures, but there is no written threshold for when more capture is justified
- desktop artifact capture is local-file based and therefore easier to ignore if not documented as part of the workflow
- it is possible to overreact to missing artifacts by bolting on heavyweight reporter dependencies instead of improving the current focused capture path

## Regressions To Avoid

- removing or weakening existing logs just because the harness appears noisy
- adding large reporter stacks before proving the current capture layer is insufficient
- treating failure screenshots as broad visual baselines
- allowing artifact file naming or folder structure to drift without documenting the change

## External Research Summary

- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because stable, diagnosable desktop tests depend on good failure evidence and resilient assertion patterns.
- [WebdriverIO visual testing docs](https://webdriver.io/docs/visual-testing/) are relevant as a cautionary reference: the repo should understand visual tooling without automatically promoting full visual diffing into the desktop smoke path.
- [WebdriverIO screenshot command docs](https://webdriver.io/docs/api/browser/saveScreenshot/) are relevant because local screenshot capture remains a core artifact for desktop smoke failures.
- [Playwright Trace Viewer](https://playwright.dev/docs/trace-viewer) is relevant because browser smoke already benefits from richer failure artifacts, and the desktop harness should be explicit about what equivalent evidence it does and does not provide.

## Proposed Improvement Approach

1. Treat desktop artifact capture as the minimal structured evidence required to debug native-shell smoke failures locally.
2. Preserve per-run directories, infrastructure logs, environment summaries, and failure screenshots as the baseline artifact set.
3. Document when additional capture is warranted and when existing artifacts should be sufficient.
4. Keep artifact capture repo-local and lightweight unless repeated evidence shows a need for something heavier.

## Checklist

- [ ] Document the required artifact files and folders for each desktop smoke run.
- [ ] Document how contributors should read driver, preview, and environment logs.
- [ ] Keep failure screenshots targeted and per-run rather than building a large always-on image corpus.
- [ ] Avoid promoting desktop screenshots into visual baseline authority.
- [ ] Keep desktop artifact capture non-options only.
- [ ] Review whether any repeated blind spots justify additive capture changes.

## Verification Plan

- run desktop smoke and verify that run directories still contain logs, screenshots, and environment summaries
- inspect a failing run end to end using only the captured artifact set
- confirm artifact filenames remain safe and stable across repeated runs
- verify any harness change preserves artifact generation even when startup fails early

## Rollout / Fallback / Compatibility Notes

- preserve the current artifact set while formalizing how it is reviewed
- prefer improving naming and structure before adding more artifact types
- if future reporters are considered, require evidence that the existing capture path is insufficient

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/08_fe_ui_baseline_capture_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_DATE.md`
