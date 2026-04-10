# Desktop Artifact Capture Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because stable, diagnosable desktop tests depend on good failure evidence and resilient assertion patterns.
- [WebdriverIO visual testing docs](https://webdriver.io/docs/visual-testing/) are relevant as a cautionary reference: the repo should understand visual tooling without automatically promoting full visual diffing into the desktop smoke path.
- [WebdriverIO screenshot command docs](https://webdriver.io/docs/api/browser/saveScreenshot/) are relevant because local screenshot capture remains a core artifact for desktop smoke failures.
- [Playwright Trace Viewer](https://playwright.dev/docs/trace-viewer) is relevant because browser smoke already benefits from richer failure artifacts, and the desktop harness should be explicit about what equivalent evidence it does and does not provide.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat desktop artifact capture as the minimal structured evidence required to debug native-shell smoke failures locally.
2. Preserve per-run directories, infrastructure logs, environment summaries, and failure screenshots as the baseline artifact set.
3. Document when additional capture is warranted and when existing artifacts should be sufficient.
4. Keep artifact capture repo-local and lightweight unless repeated evidence shows a need for something heavier.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the required artifact files and folders for each desktop smoke run.
- [x] Document how contributors should read driver, preview, and environment logs.
- [x] Keep failure screenshots targeted and per-run rather than building a large always-on image corpus.
- [x] Avoid promoting desktop screenshots into visual baseline authority.
- [x] Keep desktop artifact capture non-options only.
- [x] Review whether any repeated blind spots justify additive capture changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run desktop smoke and verify that run directories still contain logs, screenshots, and environment summaries
- inspect a failing run end to end using only the captured artifact set
- confirm artifact filenames remain safe and stable across repeated runs
- verify any harness change preserves artifact generation even when startup fails early
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current artifact set while formalizing how it is reviewed
- prefer improving naming and structure before adding more artifact types
- if future reporters are considered, require evidence that the existing capture path is insufficient
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on the current desktop harness artifact contract; no additional capture mechanism was required for this item.
- The per-run desktop artifact shape is now concretely evidenced by fresh geometry and stable smoke runs:
  - `output/wdio/desktop-smoke/<runId>/logs/`
  - `output/wdio/desktop-smoke/<runId>/screenshots/`
  - `logs/environment.txt`
  - `logs/preview-server.log`
  - `logs/tauri-driver.log`
  - `logs/wdio/*.log`
- Failure screenshots remain targeted and failure-only:
  - the harness always creates a per-run `screenshots/` folder
  - `browser.saveScreenshot(...)` is only invoked from `afterTest` when a WDIO test fails
  - passing runs therefore keep screenshot storage lightweight instead of becoming an always-on image corpus
- Artifact review expectations are now explicit:
  - `environment.txt` for toolchain and resolved prerequisite context
  - `preview-server.log` for managed host/bootstrap issues
  - `tauri-driver.log` for driver/session failures
  - `logs/wdio/*.log` for per-spec smoke details
  - `screenshots/` for failure-state UI evidence only
- No further reporter stack was justified by the current evidence. The existing repo-local capture layer already covers the main harness, launch, and per-spec failure surfaces without introducing heavier visual-baseline tooling.
- No `README.md` or `REPO_MAP.md` update was required because the current desktop README already describes the artifact root and troubleshooting flow, and the harness output shape still matches that documentation.
- Acceptance evidence:
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck:desktop`
  - `npm.cmd --prefix web run test:desktop:smoke`
  - `npm.cmd --prefix web run test:desktop:geometry`
- Raw evidence paths refreshed in this session:
  - stable desktop smoke: `output/wdio/desktop-smoke/2026-04-10T17-40-18-604Z/`
  - opt-in geometry run: `output/wdio/desktop-smoke/2026-04-10T17-44-33-069Z/`

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/08_fe_ui_baseline_capture_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_2026-04-09.md`
