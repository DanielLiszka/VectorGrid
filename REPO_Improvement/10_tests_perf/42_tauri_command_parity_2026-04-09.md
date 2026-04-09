# Tauri Command Parity Plan

Last updated: 2026-04-09

## Purpose

Define how backend command registration and the generated frontend command registry stay in sync.

## Scope Boundaries

In scope:

- the parity checker `web/scripts/check-tauri-parity.mjs`
- backend command registration in `src-tauri/src/lib.rs`
- frontend command registry in `web/src/tauri/commands.ts`
- contract tests around invoke-command registration and unknown-command handling

Out of scope:

- options-specific command surfaces as a planning focus
- behavioral correctness of each command implementation
- desktop runtime smoke
- generated command inventory documentation already owned by a separate doc

## Current User-Facing Behavior

The repo already has a parity checker that parses the `generate_handler!` registration list from the Tauri backend and compares it to the generated frontend command registry. This is a strong guard against frontend/backend drift, but the surrounding docs need to make the distinction between command-parity correctness and command-behavior correctness explicit.

## Current Code Surface

- `web/scripts/check-tauri-parity.mjs` implements the parity check.
- `src-tauri/src/lib.rs` is the canonical backend registration site for Tauri commands.
- `web/src/tauri/commands.ts` is the generated frontend registry consumed by invoke helpers.
- `web/src/tauri/invoke.contract.test.ts` helps protect the frontend-side command contract around known vs unknown commands.

## Current Verification Surface

- `npm --prefix web run parity`
- `npm --prefix web run check:commands`
- `npm --prefix web run test -- src/tauri/invoke.contract.test.ts`
- manual diff review if generated command counts change materially

## Strengths Already Present

- the repo already treats command drift as a checkable contract problem
- the checker compares backend and frontend registries directly instead of relying on memory
- frontend invoke tests complement the parity check with behavior around known versus unknown commands
- the contract is lightweight enough to run often

## Weaknesses / Failure Modes

- the parser depends on the current formatting shape of `generate_handler!`
- generated registry drift can still show up in release notes or prose even when the script is correct
- parity says a command exists on both sides, not that it behaves correctly
- count changes are easy to miscommunicate if the doc does not separate registry count from parity health

## Regressions To Avoid

- loosening the checker until formatting drift becomes silent contract drift
- treating command parity as proof that all commands still behave correctly
- forgetting to regenerate frontend command artifacts after backend registration changes
- copying stale command counts into docs after parity changes
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because command parity exists specifically to protect the frontend-to-Rust invocation boundary.
- [Tauri capabilities overview](https://v2.tauri.app/security/capabilities/) is relevant because the registered command surface also interacts with the app’s allowed native capability posture.
- [Node.js `fs` API docs](https://nodejs.org/api/fs.html) are relevant because the parity checker is a source-parsing script and its file-reading semantics should remain straightforward and deterministic.
- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant by analogy because parity is a contract-validation concern even when implemented through source parsing rather than JSON schemas.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep Tauri command parity documented as a registry-sync contract, not a behavior test.
2. Preserve a single canonical backend registration source and a single generated frontend registry.
3. Treat parser changes and generation changes as contract changes that need review.
4. Cross-link parity results into the generated-command inventory and release-note hygiene docs rather than duplicating counts loosely.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the canonical backend and frontend command sources.
- [ ] Document what the parity script proves and what it does not prove.
- [ ] Keep parity checks non-options focused for this program.
- [ ] Run parity and command-generation checks together when command registration changes.
- [ ] Review doc and release-note command counts after parity-affecting changes.
- [ ] Avoid claiming parity implies behavior correctness.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run parity` after backend command registration changes
- run `npm --prefix web run check:commands` after generation-affecting changes
- run the invoke contract test after command-registry changes
- inspect parser behavior if `generate_handler!` formatting changes in `lib.rs`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current lightweight parity script and improve explanation around it
- prefer one canonical registry source over multiple parallel inventories
- if parity parsing becomes brittle, fix the generation pipeline rather than weakening the contract
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/48_generated_command_inventory_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/47_tauri_config_sync_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/06_repo_map_readme_hygiene_2026-04-09.md`
