# Data Repair Replay Risk Plan

Last updated: 2026-04-09

## Purpose

Plan improvements around mutable-dataset replay risk so repair, top-off, startup restore, and downstream analysis do not silently assume that a stable path implies stable content.

## Scope Boundaries

In scope:

- mutable cached-dataset risks after validate, repair, or top-off
- startup snapshot source identity hints
- loaded-data context source identity and base reuse
- path-rewrite and replay-safety implications for cached Polygon datasets

Out of scope:

- full artifact reproducibility redesign
- options artifact manifests
- changing every analysis artifact contract in one pass

## Current User-Facing Behavior

The repo already acknowledges that path identity is not enough. `src-tauri/src/data_cache.rs` tracks source identity using source size and mtime, and also exposes a content fingerprint helper. `src-tauri/src/state.rs` stores `LoadedDataContext` with source identity for loaded base datasets and only reuses an existing loaded base when both path and source identity still match. `web/src/stores/dataStore.ts` persists startup snapshot hints for `baseSourceSizeBytes` and `baseSourceMtimeUtcMs`, rewrites Polygon paths when roots move, and can restore a dataset path plus timeframe on startup.

At the same time, repair and top-off can mutate cached Polygon CSV contents in place or append new bars. That means several currently useful flows carry replay/staleness risk if the same file path later refers to materially different content. The current app partially mitigates this for loaded datasets and startup restore, but not every downstream workflow treats mutated-path risk as a first-class concept yet.

## Current Code Surface

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `web/src/stores/dataStore.ts`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`

## Current Verification Surface

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/services/dataService.test.ts`

## Strengths Already Present

- source identity is already tracked for loaded base datasets instead of relying on path alone
- startup snapshots already persist size and mtime hints, not just a CSV path string
- the repo already has a content-fingerprint helper available for stronger future checks
- path rewriting for moved Polygon roots already treats replay safety as more than a string replacement problem

## Weaknesses / Failure Modes

- repair and top-off mutate cached files in place, which increases replay and staleness risk for any consumer that only remembers a path
- source content fingerprinting exists but is not yet a broad cross-surface contract
- the current improvement docs around validation/artifacts/results do not yet all connect back to mutable-dataset risk explicitly
- there is no dedicated implementation doc that says when size/mtime is good enough and when stronger provenance is needed

## Regressions To Avoid

- assuming a stable CSV path means stable dataset content after repair or top-off
- weakening loaded-base reuse checks so stale datasets can be reused by path alone
- dropping startup snapshot source-identity hints
- adding new replay or restore flows that ignore mutable Polygon dataset risk
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because startup snapshots and data-context objects are effectively persisted compatibility contracts.
- [MDN localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant because replay-sensitive startup state is currently persisted in browser storage.
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259) is relevant because many of these replay-sensitive contracts ultimately live in JSON state and artifact payloads.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document current mutable-dataset risks first, before proposing any new provenance mechanism.
2. Freeze the existing safety checks based on size/mtime-backed source identity and startup snapshot hints.
3. Explicitly call out which flows mutate cached Polygon files and what that means for replay or restore assumptions.
4. Identify where content fingerprinting could later strengthen guarantees without making it a hidden requirement today.
5. Connect this risk doc to results, validation, and artifact plans so mutable-dataset safety is not treated as data-only trivia.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document current source-identity and startup-snapshot safeguards.
- [ ] Document which flows mutate cached Polygon datasets in place.
- [ ] Document where path-only assumptions remain risky.
- [ ] Document the current availability of content fingerprinting for future strengthening.
- [ ] Link mutable-dataset risk into downstream validation/artifact/result docs that depend on data identity.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted data-cache identity and fingerprint tests
- run targeted state reuse tests for loaded base datasets
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts` after startup-snapshot or root-rewrite changes
- perform one manual repair/top-off then restore cycle if replay-sensitive behavior is changed
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current size/mtime identity safeguards as a baseline
- prefer additive provenance strengthening over removing existing restore functionality
- treat mutable-dataset replay risk as a cross-cutting compatibility concern, not a one-off warning
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-09.md`
- `REPO_Improvement/02_data/19_polygon_dataset_repair_2026-04-09.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
