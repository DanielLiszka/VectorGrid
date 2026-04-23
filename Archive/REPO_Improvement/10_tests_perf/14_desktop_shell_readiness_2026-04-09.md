# Desktop Shell Readiness Plan

Last updated: 2026-04-10

## Purpose

Define the minimal real-shell readiness assertions that the stable desktop smoke path must always protect.

## Scope Boundaries

In scope:

- the `desktop-shell-readiness.spec.ts` smoke contract
- real-shell visibility of titlebar, shell header, workspace bar, and run footer
- basic top-level workspace-label sanity in the native runtime
- readiness helpers that wait for the shell to navigate away from `about:blank` and expose core chrome

Out of scope:

- deep workspace navigation or overlay flows that have their own docs
- options-related shell surfaces
- pixel-level visual checks
- native window geometry or maximize/restore behavior

## Current User-Facing Behavior

The current desktop shell readiness spec launches the real Tauri shell, waits for core chrome to appear, and asserts that the workspace bar contains the expected top-level sections. The shared wait helper also handles the blank-shell bootstrap case by explicitly navigating to the managed preview host if necessary. This is the root health check for the desktop harness, and it should stay small, stable, and semantically meaningful.

## Current Code Surface

- `web/e2e/desktop/specs/desktop-shell-readiness.spec.ts` holds the current readiness assertions.
- `web/e2e/desktop/support/waits.ts` owns the blank-shell recovery and shell-ready checks.
- `web/e2e/desktop/support/selectors.ts` defines the stable chrome selectors.
- `web/src/components/DesktopTitlebar.vue` and `web/src/components/VueWorkspaceShell.vue` render much of the chrome that readiness smoke asserts.

## Current Verification Surface

- `npm --prefix web run test:desktop:smoke:spec -- .\e2e\desktop\specs\desktop-shell-readiness.spec.ts`
- the default `npm --prefix web run test:desktop:smoke` suite
- desktop smoke artifacts under `output/wdio/desktop-smoke/` when readiness fails

## Strengths Already Present

- the readiness spec is intentionally small and therefore suitable for the stable green path
- the shared wait helper already handles the most important harness bootstrap edge case
- assertions target core shell presence rather than incidental data state
- workspace-label checks make sure the shell is meaningfully mounted instead of merely present as an empty window

## Weaknesses / Failure Modes

- the readiness contract can become noisy if it starts asserting too much copy or content detail
- real-shell launch failures can come from harness issues as well as app issues, so the doc needs clear triage guidance
- shared selectors are reused across many specs, so weak ownership can cause broad smoke breakage
- desktop readiness can be falsely “green” if the shell mounts but key runtime wiring is degraded elsewhere

## Regressions To Avoid

- bloating readiness smoke with navigation and workflow-specific assertions
- weakening readiness to a single-element check that would miss partial shell failure
- replacing explicit readiness waits with fixed pauses
- treating readiness failure as an app-only bug before checking harness logs
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because the readiness spec should rely on stable selectors and built-in assertions.
- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/) is relevant because the harness is validating a real window lifecycle, not a plain web page.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are still relevant by analogy: shell readiness should test observable outcomes rather than implementation internals.
- [WAI-ARIA tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because the workspace bar is a top-level navigation surface with selection semantics.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep desktop shell readiness as the smallest real-shell contract that still proves meaningful app chrome is live.
2. Preserve the shared wait helper as the place where blank-shell recovery and readiness logic live.
3. Assert only the top-level shell chrome and stable workspace taxonomy in this spec.
4. Push any deeper behavior into other desktop or browser specs rather than growing readiness smoke.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the minimum chrome that must be visible for the shell to count as ready.
- [x] Document the blank-shell recovery path and why it exists.
- [x] Keep readiness assertions independent from dynamic run data.
- [x] Keep selectors stable and shared-safe.
- [x] Keep the spec non-options only.
- [x] Use readiness as a prerequisite for heavier desktop flows rather than duplicating its checks everywhere.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the targeted readiness spec after shell bootstrap or chrome changes
- run the full desktop smoke suite after major desktop-shell changes
- inspect harness logs when readiness fails before broadening the selector set
- confirm the workspace bar still exposes the expected top-level sections in the native shell
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current readiness spec as a stable smoke entry point
- prefer additive helper improvements over expanding readiness assertions
- if shell taxonomy changes, update the readiness contract deliberately and consistently
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on the current helper-plus-spec contract with fresh isolated and full-suite evidence.
- `waitForShellReady()` is the authoritative readiness helper:
  - detects `about:blank`
  - navigates the desktop webview to the managed preview host
  - waits for the URL handoff
  - then requires the core chrome to render
- The minimum chrome/taxonomy enforced by the shared helper and verified by the spec is now explicit:
  - desktop titlebar
  - workspace shell header
  - workspace bar
  - run footer
  - top-level workspace labels `Workflow`, `Run Library`, `Aggregate Analysis`, `Pro Tools`, and `Settings`
- The readiness contract remains independent from dynamic run data and continues to serve as the prerequisite for the heavier desktop shell journeys.
- No additional cross-doc sync was required because the shared selector/wait surfaces already matched the current shell taxonomy after the `Pro Tools` workspace alignment.
- Acceptance evidence:
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck:desktop`
  - `npm.cmd --prefix web run test:desktop:smoke:spec -- .\\e2e\\desktop\\specs\\desktop-shell-readiness.spec.ts`
  - `npm.cmd --prefix web run test:desktop:smoke`
- Raw evidence paths refreshed in this session:
  - targeted readiness run: `output/wdio/desktop-smoke/2026-04-10T17-33-01-029Z/`
  - stable desktop smoke: `output/wdio/desktop-smoke/2026-04-10T17-40-18-604Z/`

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/15_desktop_native_titlebar_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/16_desktop_task_center_nav_2026-04-09.md`
