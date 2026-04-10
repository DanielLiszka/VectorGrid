# Desktop Geometry Behavior Plan

Last updated: 2026-04-10

## Purpose

Define the opt-in desktop-smoke contract for deterministic resize behavior and post-resize shell usability.

## Scope Boundaries

In scope:

- the `desktop-window-geometry.spec.ts` opt-in suite
- real-shell resize, post-resize usability, and restore-to-original-geometry checks
- the boundary between geometry smoke and persisted window-state behavior
- environment tolerance expectations for geometry assertions on Windows

Out of scope:

- default stable desktop-smoke green-path requirements
- full benchmarking of resize performance
- options-related window flows
- maximize/restore titlebar behavior, which belongs to the titlebar doc

## Current User-Facing Behavior

The repo already keeps the geometry spec behind the opt-in `test:desktop:geometry` suite. The spec resizes the window to a deterministic target, waits for the size to land within a tolerance, checks that the core shell remains usable, opens settings, and then restores the original geometry. This is a useful native-runtime check, but it is intentionally not part of the default stable suite because window sizing is more environment-sensitive than ordinary shell smoke.

## Current Code Surface

- `web/e2e/desktop/specs/desktop-window-geometry.spec.ts` contains the current resize/restore journey.
- `web/wdio.desktop.conf.ts` keeps geometry in a separate opt-in suite.
- `web/e2e/desktop/support/waits.ts` and `selectors.ts` provide the shell and settings readiness helpers used after resize.
- `web/src/components/DesktopTitlebar.vue` and Tauri window integration influence the broader shell behavior around desktop window state.

## Current Verification Surface

- `npm --prefix web run test:desktop:geometry`
- `npm --prefix web run test:desktop:smoke:all` when running the full suite intentionally
- desktop artifact logs and screenshots when geometry operations fail

## Strengths Already Present

- the geometry spec is already isolated from the stable default smoke path
- it verifies both the resize itself and post-resize shell usability
- the tolerance-based check already acknowledges desktop-environment variability
- the restore step protects against one-way destructive geometry changes during a run

## Weaknesses / Failure Modes

- window geometry is inherently more environment-sensitive than most other desktop-smoke surfaces
- geometry success can depend on OS/window-manager timing outside the app itself
- the spec covers current usability after resize but not long-term persistence across relaunch
- if contributors misunderstand its role, they may promote it into mandatory green-path coverage too early

## Regressions To Avoid

- moving geometry into the stable default suite without evidence that it is reliable enough
- tightening tolerances unrealistically for desktop environments that do not honor exact pixels
- dropping the restore step and leaving the local environment altered after a run
- confusing geometry smoke with persisted window-state validation
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/) is relevant because resize and restore operations depend on real window lifecycle semantics.
- [Tauri window-state plugin docs](https://v2.tauri.app/plugin/window-state/) are relevant because geometry smoke sits adjacent to persisted window-state behavior but is not identical to it.
- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because geometry assertions need explicit waits and tolerant conditions rather than fixed timing.
- [Microsoft WebView2 user data folder guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder) is relevant because desktop shell environment state can influence repeated local runs.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep geometry behavior as an opt-in suite until its stability clearly justifies broader inclusion.
2. Treat the spec as a shell-usability-after-resize check, not as the authority on persisted window-state correctness.
3. Preserve tolerance-based assertions and explicit restore logic.
4. Escalate repeated geometry instability into harness or environment analysis before turning the spec off or making it mandatory.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the geometry suite as opt-in, not stable-default coverage.
- [x] Keep resize assertions tolerance-based and environment-aware.
- [x] Keep restore-to-original-geometry mandatory in the spec.
- [x] Separate geometry smoke from window-state persistence concerns.
- [x] Keep the geometry suite non-options only.
- [x] Capture and review artifact logs whenever geometry behavior becomes flaky.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.
- [ ] Re-close only after a current `test:desktop:geometry` run proves restore-to-original-geometry for position as well as size on the real desktop harness.

## Verification Plan

- run `npm --prefix web run test:desktop:geometry` after geometry or window-state related desktop changes
- review logs and screenshots when resize/restore behavior fails
- verify the shell remains usable after the resize step
- avoid treating geometry green-ness as a blocker for unrelated desktop-shell work unless the changed code touches window behavior directly
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- keep geometry opt-in until stability data supports a wider role
- preserve the current restore behavior even if the assertion set evolves
- if geometry flakes, first improve waits and diagnostics before loosening the contract
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Reopened on 2026-04-10 after a fresh desktop geometry run showed that restore-to-original-geometry is still not proven for position on the current harness.
- The spec continues to pass the size/usability portion of the journey:
  - resize to a deterministic target still lands within the configured width/height tolerance (`24px`)
  - the shell remains usable after resize
  - settings still opens after the resize step
- The unresolved gap is restore position:
  - the original window position at run start was `x=2973`, `y=2245`
  - after the resize command, both WebDriver `getWindowRect()` and page-level `screenX/screenY` reported `x=5946`, `y=4490`
  - reissuing the original rectangle in the restore step left the window at `x=5946`, `y=4490`
  - that means this doc cannot stay closed while its scope still claims restore-to-original-geometry coverage
- The geometry suite remains explicitly opt-in:
  - `test:desktop:geometry` remains the focused verification command
  - `test:desktop:smoke` remains the stable default suite and does not include geometry
- Geometry smoke remains distinct from persisted window-state correctness:
  - the failing surface is current-run resize/restore geometry, not relaunch persistence
  - window-state persistence and titlebar maximize/restore semantics stay documented separately
- Cross-doc sync updated in this session:
  - `web/e2e/desktop/README.md` now points troubleshooting at `preview-server.log`, matching the actual harness artifact name
- Refresh evidence from the reopen audit:
  - latest failing opt-in geometry run: `output/wdio/desktop-smoke/2026-04-10T17-58-34-582Z/`
  - previously passing size-only geometry run that is no longer sufficient for closure: `output/wdio/desktop-smoke/2026-04-10T17-44-33-069Z/`
  - companion stable desktop smoke run from the earlier tranche: `output/wdio/desktop-smoke/2026-04-10T17-40-18-604Z/`

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/15_desktop_native_titlebar_2026-04-09.md`
- `REPO_Improvement/09_live/08_system_gpu_window_surface_2026-04-09.md`
