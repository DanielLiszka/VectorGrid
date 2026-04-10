# Desktop Harness Contract Plan

Last updated: 2026-04-10

## Purpose

Define the role, limits, prerequisites, and artifact contract for the Windows-first WebdriverIO desktop smoke harness.

## Scope Boundaries

In scope:

- the WebdriverIO desktop smoke harness configuration and README contract
- managed build, preview-server, and `tauri-driver` orchestration
- artifact/log output under `output/wdio/desktop-smoke/`
- the boundary between desktop smoke and both browser smoke and lower-level tests

Out of scope:

- options-related desktop flows
- broad performance benchmarking or profiling
- browser-only mocked-Tauri coverage
- deep backend command correctness that belongs in targeted Rust or service tests

## Current User-Facing Behavior

The desktop harness already verifies prerequisites, builds the frontend and desktop binary, launches a managed Vite preview server, starts `tauri-driver`, warms a desktop session, and then runs a selected WebdriverIO smoke suite against the real Tauri shell. The README clearly describes the current local workflow, but the harness contract still needs a formal implementation doc that keeps it scoped to real-shell integrity rather than turning it into a huge end-to-end test matrix.

## Current Code Surface

- `web/e2e/desktop/README.md` defines the current intended use and local workflow.
- `web/wdio.desktop.conf.ts` orchestrates build, preview, driver startup, suites, and artifact handling.
- `web/scripts/desktop/verify-prereqs.ts` owns local prerequisite discovery for `tauri-driver` and EdgeDriver.
- `web/scripts/desktop/build-smoke-app.ts` owns the managed frontend + Rust build path.
- `web/e2e/desktop/support/appPaths.ts`, `waits.ts`, `selectors.ts`, and `artifacts.ts` provide the shared harness support layer.

## Current Verification Surface

- `npm --prefix web run test:desktop:smoke`
- `npm --prefix web run test:desktop:smoke:debug`
- `npm --prefix web run test:desktop:smoke:spec -- <spec>`
- `npm --prefix web run test:desktop:geometry`
- artifact and log review under `output/wdio/desktop-smoke/`

## Strengths Already Present

- the harness already exercises the real Tauri shell instead of a mocked runtime
- it intentionally separates stable default smoke from the opt-in geometry suite
- prerequisite verification and artifact capture are already built into the harness rather than left implicit
- the harness avoids `cargo tauri dev`, reducing dev-server timing and cwd-related noise

## Weaknesses / Failure Modes

- desktop smoke is inherently heavier and more environment-sensitive than browser smoke
- the harness is Windows-first and depends on matching local driver prerequisites
- real-shell coverage can be overtrusted if its responsibilities are not kept small and explicit
- build and launch orchestration live in one config file, so local maintenance drift can accumulate without ownership

## Regressions To Avoid

- using desktop smoke as the primary proof layer for behaviors that should stay covered by unit or browser tests
- adding many slow or flaky scenarios to the default stable suite
- weakening artifact capture or prerequisite checks just to make the harness appear greener
- treating the geometry suite as required green-path coverage before it is demonstrably stable
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because the desktop smoke suite needs resilient selectors and automatic waiting instead of fragile timing.
- [Tauri distribution docs](https://v2.tauri.app/distribute/) are relevant because the harness is validating a built desktop application path, not only a dev shell.
- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/) is relevant because the harness interacts with real window lifecycle and geometry behavior.
- [Microsoft WebView2 user data folder guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder) is relevant because local desktop-shell runs can inherit environment-specific WebView state.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Define desktop smoke as the real-shell integrity layer that proves launch, chrome, and critical navigation in the native runtime.
2. Keep the stable suite intentionally small, with heavier or more timing-sensitive specs living behind opt-in suites.
3. Preserve managed prerequisite checks, build steps, and artifact capture as first-class parts of the contract.
4. Require new desktop-smoke scenarios to justify why real native runtime is necessary.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the required prerequisites and accepted override paths.
- [x] Document the stable default suite and the opt-in suites separately.
- [x] Document the artifact/log outputs and how they are used during review.
- [x] Document what belongs in browser smoke instead of desktop smoke.
- [x] Keep the desktop harness non-options only.
- [x] Define when a new scenario is heavy enough to stay opt-in.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run test:desktop:smoke` after desktop harness or native-shell changes
- use `test:desktop:smoke:spec` for targeted spec investigation
- review `tauri-driver`, preview-server, and WDIO logs for harness failures before blaming the app itself
- keep the stable suite green independently from geometry or other opt-in suites
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current default stable smoke path while formalizing ownership
- keep prerequisite verification and artifact capture mandatory
- treat opt-in suites as supplemental until they prove stable enough for wider use
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on current desktop harness evidence; no additional harness refactor was required for this item.
- The stable default suite is now concretely frozen in `web/wdio.desktop.conf.ts` as:
  - `desktop-shell-readiness.spec.ts`
  - `desktop-workspace-navigation.spec.ts`
  - `desktop-command-palette.spec.ts`
  - `desktop-task-center.spec.ts`
  - `desktop-settings-diagnostics.spec.ts`
  - `desktop-titlebar-native-controls.spec.ts`
- The geometry path remains opt-in only through `test:desktop:geometry`; `test:desktop:smoke:debug` remains a troubleshooting path, not closure evidence.
- The README already documents prerequisite discovery, accepted override paths, the browser-vs-desktop verification boundary, and the stable-vs-opt-in suite split, so no additional `README.md` or `REPO_MAP.md` sync was required for this closure.
- The artifact contract is run-scoped under `output/wdio/desktop-smoke/<runId>/`:
  - `logs/environment.txt`
  - `logs/preview-server.log`
  - `logs/tauri-driver.log`
  - `logs/wdio/*.log`
  - `screenshots/` for failures
- Acceptance evidence:
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `npm.cmd --prefix web run typecheck:desktop`
  - `npm.cmd --prefix web run test:browser:ux`
  - `npm.cmd --prefix web run test:desktop:smoke`
- Raw evidence paths refreshed in this session:
  - stable desktop smoke: `output/wdio/desktop-smoke/2026-04-10T17-40-18-604Z/`
  - earlier targeted titlebar run retained under `output/wdio/desktop-smoke/2026-04-10T17-29-35-668Z/`

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/14_desktop_shell_readiness_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/19_desktop_artifact_capture_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
