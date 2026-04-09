# Bundle Budget Analysis Plan

Last updated: 2026-04-09

## Purpose

Define the current frontend bundle-budget analysis contract so desktop polish and feature growth do not silently bloat the built app shell.

## Scope Boundaries

In scope:

- built frontend asset sizes under `web/dist`
- entry-chunk, lazy-chunk, and total-JS limits
- current bundle-budget checker behavior
- bundle-analysis tooling and review expectations

Out of scope:

- runtime rendering performance beyond bundle-size effects
- options-only frontend growth
- generic web-performance advice detached from this repo’s current build output

## Current User-Facing Behavior

The repo already contains a concrete bundle-budget check for the built Vue frontend. Current behavior includes:

- identifying the main entry chunk from `dist/index.html`
- checking entry-chunk size, largest lazy chunk, and total JS size
- failing when current fixed thresholds are exceeded
- supporting an analysis-mode build for deeper inspection

This is already part of the quality posture for the shipped desktop shell, even though users never see the checker directly.

## Current Code Surface

- `web/scripts/check-bundle-budget.mjs`
  - enforces current entry/lazy/total JS thresholds
- `web/package.json`
  - defines `build`, `analyze:bundle`, and `check:bundle-budget`
- `web/vite.config.ts`
  - Vite build configuration and analysis-mode hooks
- `RELEASE.md`
  - release notes that describe the Vue + Vite handoff and desktop build posture

## Current Verification Surface

- `npm --prefix web run build`
- `npm --prefix web run analyze:bundle`
- `npm --prefix web run check:bundle-budget`

## Strengths Already Present

- bundle size is already treated as an explicit gate, not as an afterthought
- the checker already looks at multiple size classes instead of a single total-only number
- analysis mode already exists for deeper review when a threshold is threatened

## Weaknesses / Failure Modes

- budget values can drift from current product expectations if not reviewed periodically
- size-only checks can miss why the growth happened
- analysis-mode output is useful but easy to neglect without a written workflow
- desktop teams can mistake bundle size for the whole performance story

## Regressions To Avoid

- raising thresholds casually to make failures disappear
- relying on analysis mode without keeping the deterministic budget gate
- letting large lazy chunks accumulate while total JS still looks acceptable
- treating bundle-budget violations as frontend-only noise when they affect desktop startup and shell responsiveness
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vite build guide](https://vite.dev/guide/build)
  - relevant because the bundle budget is enforced on Vite production output
- [Vite performance guide](https://vite.dev/guide/performance)
  - relevant because chunking and dependency discipline matter to build output size and startup
- [Rollup plugin visualizer](https://github.com/btd/rollup-plugin-visualizer)
  - relevant because the repo already uses bundle-analysis tooling beyond raw byte counts
- [web.dev: Code splitting with dynamic imports](https://web.dev/articles/reduce-javascript-payloads-with-code-splitting)
  - relevant because lazy-chunk behavior is part of the current budget model
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Document the current budget numbers and exactly what they protect.
2. Keep deterministic size thresholds as the first gate.
3. Treat analysis-mode output as a follow-up diagnostic, not as the primary gate.
4. Tie budget changes to real product reasoning instead of local convenience.
5. Keep desktop startup/readiness implications in scope when reviewing bundle growth.
6. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current entry/lazy/total JS thresholds.
- [ ] Document how the checker identifies the entry chunk.
- [ ] Document when `analyze:bundle` should be used.
- [ ] Document how budget changes must be justified.
- [ ] Keep bundle-budget review tied to desktop shell/startup concerns.
- [ ] Add or tighten checks if the build-output structure changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- run `npm --prefix web run build`
- run `npm --prefix web run check:bundle-budget`
- run `npm --prefix web run analyze:bundle` when the budget gate is threatened or changed
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the current deterministic budget gate even if analysis tooling improves
- if chunk structure changes materially, update the checker and its documentation together
- treat threshold changes as compatibility-significant for desktop productization
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/11_docs_ops/03_frontend_build_handoff_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/08_fe_ui_baseline_capture_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_2026-04-09.md`
