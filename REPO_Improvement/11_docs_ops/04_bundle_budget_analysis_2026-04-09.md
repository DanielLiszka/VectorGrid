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

## External Research Summary

- [Vite build guide](https://vite.dev/guide/build)
  - relevant because the bundle budget is enforced on Vite production output
- [Vite performance guide](https://vite.dev/guide/performance)
  - relevant because chunking and dependency discipline matter to build output size and startup
- [Rollup plugin visualizer](https://github.com/btd/rollup-plugin-visualizer)
  - relevant because the repo already uses bundle-analysis tooling beyond raw byte counts
- [web.dev: Code splitting with dynamic imports](https://web.dev/articles/reduce-javascript-payloads-with-code-splitting)
  - relevant because lazy-chunk behavior is part of the current budget model

## Proposed Improvement Approach

1. Document the current budget numbers and exactly what they protect.
2. Keep deterministic size thresholds as the first gate.
3. Treat analysis-mode output as a follow-up diagnostic, not as the primary gate.
4. Tie budget changes to real product reasoning instead of local convenience.
5. Keep desktop startup/readiness implications in scope when reviewing bundle growth.

## Checklist

- [ ] Document the current entry/lazy/total JS thresholds.
- [ ] Document how the checker identifies the entry chunk.
- [ ] Document when `analyze:bundle` should be used.
- [ ] Document how budget changes must be justified.
- [ ] Keep bundle-budget review tied to desktop shell/startup concerns.
- [ ] Add or tighten checks if the build-output structure changes.

## Verification Plan

- run `npm --prefix web run build`
- run `npm --prefix web run check:bundle-budget`
- run `npm --prefix web run analyze:bundle` when the budget gate is threatened or changed

## Rollout / Fallback / Compatibility Notes

- keep the current deterministic budget gate even if analysis tooling improves
- if chunk structure changes materially, update the checker and its documentation together
- treat threshold changes as compatibility-significant for desktop productization

## Related Docs

- `REPO_Improvement/11_docs_ops/03_frontend_build_handoff_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/08_fe_ui_baseline_capture_DATE.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_DATE.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_DATE.md`
