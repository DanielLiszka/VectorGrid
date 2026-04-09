# Generated Command Inventory Plan

Last updated: 2026-04-09

## Purpose

Define the generated frontend Tauri command inventory contract so `web/src/tauri/commands.ts` remains synchronized with the Rust command surface and so literal command drift is caught automatically.

## Scope Boundaries

In scope:

- `web/src/tauri/commands.ts`
- `web/scripts/generate-tauri-commands.mjs`
- `web/scripts/check-tauri-parity.mjs`
- `web/scripts/check-known-tauri-commands.mjs`
- tests that assume the generated inventory is correct

Out of scope:

- Rust command implementation details
- options-specific command semantics
- frontend event channel inventory

## Current User-Facing Behavior

Users never see the generated inventory directly, but much of the frontend’s safety depends on it. The app already uses a generated `TAURI_COMMANDS` list and typed `TauriCommand` union to keep frontend command calls honest. The repo also already has checker scripts that compare frontend inventory against Rust and scan frontend/script files for literal `invoke`, `invokeCommand`, and `invokeByName` command names. The current generated registry contains `98` commands, and the generator/parity/literal checks already pass. This is already a real anti-drift mechanism, not just convenience tooling.

## Current Code Surface

- `web/src/tauri/commands.ts`
  - auto-generated command registry and `isTauriCommand` guard
- `web/scripts/generate-tauri-commands.mjs`
  - parses `src-tauri/src/lib.rs` `generate_handler![...]` and writes `commands.ts`
- `web/scripts/check-tauri-parity.mjs`
  - checks generated frontend command inventory against Rust command surface
- `web/scripts/check-known-tauri-commands.mjs`
  - scans `web/src` and `web/scripts` for literal command names and fails on unknown ones
- `web/src/tauri/invoke.ts`
  - consumes `TauriCommand` typing and `isTauriCommand`
- `web/src/tauri/invoke.contract.test.ts`
  - smokes every registered command through the typed wrapper

## Current Verification Surface

- `web/src/tauri/invoke.contract.test.ts`
- `web/src/tauri/invoke.mockIPC.test.ts`
- `node ./scripts/generate-tauri-commands.mjs --check` from `web/`
- `node ./scripts/check-tauri-parity.mjs` from `web/`
- `node ./scripts/check-known-tauri-commands.mjs` from `web/`

## Strengths Already Present

- the command registry is already generated from Rust rather than copied by hand
- `invokeByName()` already rejects unknown commands before IPC dispatch
- checker scripts already cover both parity and literal command usage
- the typed wrapper test already ensures every registered command is reachable from the frontend layer

## Weaknesses / Failure Modes

- drift can still occur if developers forget to regenerate or check after command-surface changes
- the generator assumes the `generate_handler![...]` block remains parseable with current script rules
- parity and literal-command scripts duplicate some command-surface parsing concerns
- command inventory correctness is foundational but easy to underestimate because the generated file itself is simple

## Regressions To Avoid

- hand-editing `commands.ts` instead of regenerating it
- adding Rust commands without updating the generated inventory
- allowing unknown literal command names to bypass the checker
- weakening the typed `TauriCommand` contract or `isTauriCommand` guard
- breaking the parser if `generate_handler!` formatting changes
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because command-name correctness underpins the frontend-to-Rust boundary
- [Tauri JS Core API](https://v2.tauri.app/reference/javascript/api/namespacecore/)
  - relevant because the generated inventory protects the `invoke()` call surface
- [Vitest: Mocking](https://vitest.dev/guide/mocking.html)
  - relevant because the contract test and mock-runtime tests assume a correct inventory
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat generated command inventory as mandatory build/test hygiene, not optional convenience.
2. Document the current pipeline:
   - Rust `generate_handler![...]`
   - generator script
   - generated `commands.ts`
   - parity checker
   - literal-command checker
   - wrapper/contract tests
3. Consolidate duplicated handler-parsing logic only if the generated output and current checks remain exact.
4. Strengthen future assertions toward command-group or machine-readable inventory checks instead of looser spot-checks.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document how the generator derives command names from Rust.
- [ ] Document when `commands.ts` must be regenerated or checked.
- [ ] Document parity-check and literal-check responsibilities separately.
- [ ] Document how frontend code should consume `TauriCommand` and `isTauriCommand`.
- [ ] Add or tighten tests/scripts if command generation or checking rules change.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `node ./scripts/generate-tauri-commands.mjs --check` from `web/`
- run `node ./scripts/check-tauri-parity.mjs` from `web/`
- run `node ./scripts/check-known-tauri-commands.mjs` from `web/`
- run `npm --prefix web run test -- src/tauri/invoke.contract.test.ts`
- run `npm --prefix web run test -- src/tauri/invoke.mockIPC.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- keep the generated file checked in and reproducible
- if the Rust command macro structure changes, update the generator and parity checks in the same change
- do not accept stale generated output as normal workflow
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/34_frontend_tauri_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/21_tauri_mock_runtime_smoke_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
