# Desktop Harness Contract Plan

Last updated: 2026-04-09

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

## External Research Summary

- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because the desktop smoke suite needs resilient selectors and automatic waiting instead of fragile timing.
- [Tauri distribution docs](https://v2.tauri.app/distribute/) are relevant because the harness is validating a built desktop application path, not only a dev shell.
- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/) is relevant because the harness interacts with real window lifecycle and geometry behavior.
- [Microsoft WebView2 user data folder guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder) is relevant because local desktop-shell runs can inherit environment-specific WebView state.

## Proposed Improvement Approach

1. Define desktop smoke as the real-shell integrity layer that proves launch, chrome, and critical navigation in the native runtime.
2. Keep the stable suite intentionally small, with heavier or more timing-sensitive specs living behind opt-in suites.
3. Preserve managed prerequisite checks, build steps, and artifact capture as first-class parts of the contract.
4. Require new desktop-smoke scenarios to justify why real native runtime is necessary.

## Checklist

- [ ] Document the required prerequisites and accepted override paths.
- [ ] Document the stable default suite and the opt-in suites separately.
- [ ] Document the artifact/log outputs and how they are used during review.
- [ ] Document what belongs in browser smoke instead of desktop smoke.
- [ ] Keep the desktop harness non-options only.
- [ ] Define when a new scenario is heavy enough to stay opt-in.

## Verification Plan

- run `npm --prefix web run test:desktop:smoke` after desktop harness or native-shell changes
- use `test:desktop:smoke:spec` for targeted spec investigation
- review `tauri-driver`, preview-server, and WDIO logs for harness failures before blaming the app itself
- keep the stable suite green independently from geometry or other opt-in suites

## Rollout / Fallback / Compatibility Notes

- preserve the current default stable smoke path while formalizing ownership
- keep prerequisite verification and artifact capture mandatory
- treat opt-in suites as supplemental until they prove stable enough for wider use

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/14_desktop_shell_readiness_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/19_desktop_artifact_capture_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
