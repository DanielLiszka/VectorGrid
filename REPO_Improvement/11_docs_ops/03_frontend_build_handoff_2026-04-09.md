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

## External Research Summary

- [Vite build guide](https://vite.dev/guide/build)
  - relevant because `web/dist` is a standard Vite production output that becomes the desktop web asset package
- [Tauri distribute/build docs](https://v2.tauri.app/distribute/)
  - relevant because Tauri packaging depends on the frontend handoff target being correct
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because generated command inventory is part of keeping the built frontend compatible with the desktop backend
- [GitHub Docs: autolinked URLs and references](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/autolinked-references-and-urls?formCode=MG0AV3)
  - relevant because release/build docs should stay readable and consistent when documenting command/build surfaces

## Proposed Improvement Approach

1. Document the frontend handoff as a single contract:
   - generate/check commands
   - typecheck/build frontend
   - point Tauri at `web/dist`
2. Keep the config mirror and generated-command tooling tied to the same build story.
3. Treat desktop smoke as downstream proof that the handoff remains valid.
4. Preserve current output location and build order unless there is a deliberate migration.

## Checklist

- [ ] Document the current build sequence for the frontend handoff.
- [ ] Document which scripts are contract checks versus plain build steps.
- [ ] Document `frontendDist` ownership in canonical and mirrored Tauri config.
- [ ] Document how desktop smoke depends on the same handoff.
- [ ] Keep release docs aligned with the actual handoff path.
- [ ] Add or tighten checks if the frontend output path or generator flow changes.

## Verification Plan

- run `npm --prefix web run typecheck`
- run `npm --prefix web run build`
- run `npm --prefix web run check:commands`
- run `npm --prefix web run parity`
- run `node scripts/check_tauri_config_sync.mjs`
- run `npm --prefix web run test:desktop:smoke`

## Rollout / Fallback / Compatibility Notes

- preserve the current `web/dist` handoff unless a full migration is coordinated
- if build script ordering changes, update smoke/build docs and command checks together
- keep the config-mirror check active whenever Tauri config changes

## Related Docs

- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_DATE.md`
- `REPO_Improvement/10_tests_perf/47_tauri_config_sync_DATE.md`
- `REPO_Improvement/11_docs_ops/04_bundle_budget_analysis_2026-04-09.md`
