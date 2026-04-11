# Frontend Build Handoff Plan

Last updated: 2026-04-09

## Purpose

Define the current frontend build handoff into the Tauri desktop packaging flow so `web/dist` production output, command generation, and desktop bundling stay synchronized.

## Scope Boundaries

In scope:

- `web/` production build output
- Tauri `frontendDist` handoff
- generated command checks that gate frontend builds
- local desktop smoke/build assumptions that depend on the handoff

Out of scope:

- runtime UI behavior
- options-only build details
- CI workflow orchestration beyond the handoff contract itself

## Current User-Facing Behavior

The current desktop app ships a Vue + Vite frontend that is built into `web/dist` and then consumed by Tauri packaging. The repo already treats this as a real contract:

- frontend build happens through `npm --prefix web run build`
- Tauri config points at `web/dist`
- the frontend build itself depends on typed command-generation checks and related script validation

Desktop smoke and bundle builds already assume that this handoff is deterministic and reproducible.

## Current Code Surface

- `web/package.json`
  - defines `build`, `typecheck`, parity, and command-generation checks
- `web/scripts/generate-tauri-commands.mjs`
  - keeps generated command inventory in sync before/alongside build hygiene
- `web/scripts/check-known-tauri-commands.mjs`
  - validates command usage assumptions
- `src-tauri/tauri.conf.json`
  - canonical `frontendDist` handoff target
- `tauri.conf.json`
  - mirrored root Tauri config
- `RELEASE.md`
  - describes the Vue + Vite handoff in release-facing terms
- `web/scripts/desktop/build-smoke-app.ts`
  - consumes the frontend build as part of desktop smoke setup

## Current Verification Surface

- `npm --prefix web run typecheck`
- `npm --prefix web run build`
- `npm --prefix web run parity`
- `npm --prefix web run check:commands`
- `node scripts/check_tauri_config_sync.mjs`
- `npm --prefix web run test:desktop:smoke`

## Strengths Already Present

- the frontend handoff target is already explicit in Tauri config
- build hygiene already includes command-generation and parity checks
- the desktop smoke build path already exercises the frontend handoff rather than assuming stale output
- release docs already describe the migration to Vue + Vite and the new build location

## Weaknesses / Failure Modes

- frontend output location and Tauri config can drift if not checked together
- build hygiene can look broader than it is unless the ordering of checks is documented
- browser-mode and desktop-mode assumptions can drift if `web/dist` content is treated as generic static output rather than part of a desktop contract

## Regressions To Avoid

- changing `frontendDist` or build output location without updating Tauri config and smoke tooling
- allowing stale generated command inventory into a production build
- letting README/RELEASE describe a different frontend handoff than the actual Tauri config
- weakening deterministic build hygiene to save local build time
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vite build guide](https://vite.dev/guide/build)
  - relevant because `web/dist` is a standard Vite production output that becomes the desktop web asset package
- [Tauri distribute/build docs](https://v2.tauri.app/distribute/)
  - relevant because Tauri packaging depends on the frontend handoff target being correct
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because generated command inventory is part of keeping the built frontend compatible with the desktop backend
- [GitHub Docs: autolinked URLs and references](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/autolinked-references-and-urls?formCode=MG0AV3)
  - relevant because release/build docs should stay readable and consistent when documenting command/build surfaces
- [GitHub repository documentation](https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  - relevant because public-repo docs and release operations should be maintained as first-class contributor and reviewer surfaces

## Proposed Improvement Approach

1. Document the frontend handoff as a single contract:
   - generate/check commands
   - typecheck/build frontend
   - point Tauri at `web/dist`
2. Keep the config mirror and generated-command tooling tied to the same build story.
3. Treat desktop smoke as downstream proof that the handoff remains valid.
4. Preserve current output location and build order unless there is a deliberate migration.
5. Treat docs/ops quality as part of the implementation scope: keep public-facing instructions, release steps, support posture, and maintenance rules specific, reproducible, and easy for outside contributors to follow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current build sequence for the frontend handoff.
- [x] Document which scripts are contract checks versus plain build steps.
- [x] Document `frontendDist` ownership in canonical and mirrored Tauri config.
- [x] Document how desktop smoke depends on the same handoff.
- [x] Keep release docs aligned with the actual handoff path.
- [x] Add or tighten checks if the frontend output path or generator flow changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the public-repo sync surfaces and minimum-bar artifacts that must be updated together when this docs/ops change lands.

## Verification Plan

- run `npm --prefix web run typecheck`
- run `npm --prefix web run build`
- run `npm --prefix web run check:commands`
- run `npm --prefix web run parity`
- run `node scripts/check_tauri_config_sync.mjs`
- run `npm --prefix web run test:desktop:smoke`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify every changed doc/ops surface against the repo minimum bar and update all linked public docs, release notes, and contributor-facing routing files together
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current `web/dist` handoff unless a full migration is coordinated
- if build script ordering changes, update smoke/build docs and command checks together
- keep the config-mirror check active whenever Tauri config changes
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on fresh handoff verification across the generated-command checks, the Tauri config mirror, the frontend build, and a real desktop smoke run.
- The current handoff authority surfaces are aligned across:
  - `web/package.json`
  - `web/scripts/generate-tauri-commands.mjs`
  - `web/scripts/check-known-tauri-commands.mjs`
  - `src-tauri/tauri.conf.json`
  - `tauri.conf.json`
  - `RELEASE.md`
  - `REPO_MAP.md`
- Fresh acceptance evidence:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run parity`
  - `npm --prefix web run check:commands`
  - `node scripts/check_tauri_config_sync.mjs`
  - `npm --prefix web run build`
  - `npm --prefix web run test:desktop:smoke`
- The fresh pass also surfaced why this doc matters:
  - `npm --prefix web run check:commands` initially failed on a stale generated-command inventory
  - rerunning `npm --prefix web run gen:commands` restored the generated file to the expected state and the follow-up `check:commands` pass succeeded
- Desktop smoke then revalidated that the same `web/dist` handoff still drives the actual packaged shell path:
  - all 6 WDIO desktop smoke specs passed
  - the titlebar native-controls spec also passed, which reconfirmed the earlier selector/label mismatch is no longer blocking this lane
- No additional public-doc edits were required for closure because `RELEASE.md` and `REPO_MAP.md` already describe the same `web/dist` handoff, config-mirror ownership, and command-generation checks as the live build/test path.

## Related Docs

- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/47_tauri_config_sync_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/04_bundle_budget_analysis_2026-04-09.md`
