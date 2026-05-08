# Master Code Review Tracker

## Purpose

This document tracks a full file-visible code review for `ta_desktop_demo` before public-facing polish and release work. It is the control sheet for review coverage, quality decisions, follow-up fixes, and verification evidence.

The goal is not only to find defects. The goal is to make the repository read like a maintainable public project: clear ownership, bounded modules, explicit contracts, strong tests, predictable errors, and no avoidable tangles in control flow or responsibility.

## Scope

- Every tracked, non-generated project file must either receive a file-level review row or be marked `N/A` with a reason.
- Public-facing root files, documentation, release notes, security/privacy/support docs, and scripts are in scope.
- Rust, Vue, TypeScript, JavaScript, PowerShell, Python, CUDA, config, and test files are in scope.
- Vendored third-party code is reviewed for provenance, license, integration risk, patch status, and binary/source consistency. It is not style-reviewed unless this repo carries local modifications.
- Generated binaries, PTX, CUBIN, screenshots, sample datasets, and packaged artifacts are reviewed for provenance, rebuildability, public suitability, and release hygiene rather than normal source style.
- Ignored build outputs such as `target/`, `dist/`, `node_modules/`, `.codex_tmp*/`, and local benchmark output are out of scope unless a release process depends on them.

## Inventory Snapshot

Initial tracked-file counts from `git ls-files` at tracker creation:

| Scope | Count | Review treatment |
|---|---:|---|
| All tracked files | 2104 | Full inventory source |
| Non-vendor tracked files | 930 | Normal file-level review unless generated/data |
| `vendor/` tracked files | 1174 | Provenance, license, integration, binary consistency |
| `web/` | 545 | Frontend source, tests, assets, public UX |
| `crates/` | 125 | Core Rust crates and tests |
| `docs/` | 103 | Public and internal documentation quality |
| `src-tauri/` | 77 | Tauri command boundary, runtime, desktop integration |
| `scripts/` | 45 | Gates, release helpers, profiling and maintenance scripts |

Before a review wave is marked complete, refresh these counts and record any delta in the review notes.

## Status Legend

| Status | Meaning |
|---|---|
| `TODO` | File has not been reviewed yet. |
| `IN REVIEW` | Review is active but findings are not final. |
| `NEEDS WORK` | File has one or more open required changes. |
| `BLOCKED` | Review cannot finish without missing context, tooling, or a prior fix. |
| `APPROVED` | File meets the quality bar and has appropriate evidence. |
| `N/A` | File is intentionally excluded from source review, with a reason. |

## Severity Legend

| Severity | Meaning |
|---|---|
| `P0` | Public-release blocker, correctness risk, data loss, security/privacy risk, or misleading user-facing result. |
| `P1` | High-confidence maintainability, correctness, reproducibility, or testing issue that should be fixed before public polish is considered done. |
| `P2` | Quality improvement with clear value but lower immediate risk. |
| `P3` | Cleanup, naming, comments, minor docs, or follow-up polish. |

## File Review Definition Of Done

A file cannot be marked `APPROVED` until the reviewer has checked each applicable item:

- [ ] The file has one clear responsibility and belongs in its current module or directory.
- [ ] Public contracts, request/response shapes, data formats, and side effects are explicit.
- [ ] Error handling is deliberate; production paths avoid unjustified `unwrap`, `expect`, panics, silent fallbacks, and swallowed errors.
- [ ] Control flow is readable, locally testable, and not dependent on hidden global state unless the architecture requires it.
- [ ] Shared abstractions reduce real complexity instead of hiding strategy-specific behavior.
- [ ] Performance-sensitive paths avoid accidental allocation, needless cloning, avoidable full fills, and benchmark-hostile work.
- [ ] CPU-only behavior remains valid without CUDA unless the file is explicitly CUDA-only.
- [ ] Async, threading, locking, cancellation, and event emission behavior is documented or locally obvious.
- [ ] Frontend UI code has coherent state handling, accessibility posture, responsive behavior, and no misleading copy.
- [ ] Tests exist close to the changed behavior or the absence of tests is recorded as a follow-up risk.
- [ ] Public docs, release docs, and scripts match the actual command surface and repo behavior.
- [ ] Dependencies, vendored code, licenses, credentials, and local artifacts are appropriate for a public repo.
- [ ] Verification evidence is recorded, or the reason verification was not run is explicit.

## Global Review Checklist

- [ ] Refresh the tracked-file inventory and record any difference from the initial snapshot.
- [ ] Expand the file ledger so every non-vendor tracked file has its own row or an explicit `N/A` row.
- [ ] Add a vendor provenance ledger for `vendor/vector-ta-0.2.4`.
- [ ] Review public root files for project framing, support posture, privacy/security language, and release suitability.
- [ ] Review Rust workspace manifests and feature wiring, especially CPU-only and CUDA feature boundaries.
- [ ] Review Tauri command boundary files for validation, error shape, state ownership, and command drift.
- [ ] Review app-core files by strategy or runtime area, not by random file order.
- [ ] Review optimizer and strategy crates for reusable primitive quality and test coverage.
- [ ] Review Vue/TypeScript frontend by workflow surface, store/service boundary, and component responsibility.
- [ ] Review scripts as production tooling: idempotence, failure behavior, path safety, benchmark serialization, and clear output.
- [ ] Review docs for public accuracy, duplication, stale claims, and routing clarity.
- [ ] Review assets, samples, and binary artifacts for provenance, size, licensing, and public necessity.
- [ ] Convert findings into fix tickets or implementation checklist rows with severity and owner area.
- [ ] Re-review every file touched by fixes before marking the area complete.
- [ ] Run the appropriate quality gates and record evidence before closing the review.

## Review Order

1. Root public surface and repository policy files.
2. Workspace manifests and dependency/feature wiring.
3. Tauri command boundary and desktop runtime.
4. Core Rust crates by strategy and runtime ownership.
5. Frontend services, stores, strategy models, and components.
6. Scripts, gates, benchmark/profiling helpers, and release tooling.
7. Public docs and internal routing docs.
8. Vendored dependency provenance and binary artifact posture.
9. Final cross-cutting pass for duplication, naming, dead code, and stale claims.

## Area Expansion Checklist

Use this checklist to seed the file-level ledger. Directory-level boxes only mean the ledger rows have been created; they do not mean the files are approved.

- [ ] Root files: create or update one row per root visible file.
- [ ] `.cargo/`: create one row per tracked file.
- [ ] `.github/`: create one row per tracked file.
- [ ] `configs/`: create one row per tracked file.
- [ ] `crates/app_core/`: create one row per tracked source, test, bench, manifest, kernel, and support file.
- [ ] `crates/optimizer/`: create one row per tracked source, test, and manifest file.
- [ ] `crates/strategies/`: create one row per tracked source, test, and manifest file.
- [ ] `src-tauri/`: create one row per tracked Rust, config, capability, test, and asset file.
- [ ] `web/`: create one row per tracked component, composable, store, service, script, test, style, asset, and config file.
- [ ] `scripts/`: create one row per tracked script, registry, baseline contract, and helper file.
- [ ] `docs/`: create one row per current documentation file and one `N/A` row for archived-only material if excluded.
- [ ] `GPT5.5-review/`: classify public relevance, then create source-review or `N/A` rows.
- [ ] `vendor/vector-ta-0.2.4/`: create provenance rows for package metadata, local patches if any, source subtrees, kernels, PTX/CUBIN artifacts, and licenses.
- [ ] Root sample data/artifacts: create provenance and public-suitability rows.

## Root File Ledger

| Status | File | Review focus | Findings / follow-up | Evidence |
|---|---|---|---|---|
| TODO | `.gitignore` | Public hygiene, ignored outputs, generated artifacts |  |  |
| TODO | `Cargo.toml` | Workspace membership, dependency policy, feature routing |  |  |
| TODO | `Cargo.lock` | Dependency lock posture, public reproducibility |  |  |
| TODO | `CONTRIBUTING.md` | Contributor workflow, quality bar, command accuracy |  |  |
| TODO | `DISCLAIMER.md` | Public legal/product risk language |  |  |
| TODO | `LICENSE` | License presence and consistency |  |  |
| TODO | `PRIVACY.md` | Data, credentials, telemetry, and storage claims |  |  |
| TODO | `README.md` | Product framing, setup, public claims, screenshots, gates |  |  |
| TODO | `SECURITY.md` | Vulnerability reporting and security posture |  |  |
| TODO | `SUPPORT.md` | Support expectations and public maintenance posture |  |  |
| TODO | `rust-toolchain.toml` | Toolchain pin and developer reproducibility |  |  |
| TODO | `tauri.conf.json` | Root mirror consistency and public packaging behavior |  |  |
| TODO | `tauri.windows.conf.json` | Windows package/release behavior |  |  |
| TODO | `SERENITY-0.46-final-trial-102-SMACrossover_precalc-Bitfinex Spot-BTC-USD-1h-interval-90-2018-01-01-2023-09-01.xlsx` | Public suitability, provenance, size, and necessity |  |  |

## File-Level Ledger Template

Add one row per reviewed file. Do not mark a directory complete unless every file row under that directory is `APPROVED` or `N/A`.

| Status | File | Area | Primary quality focus | Findings / required changes | Evidence |
|---|---|---|---|---|---|
| TODO | `path/to/file.ext` | `area` | `correctness / maintainability / tests / docs / release / provenance` |  |  |

## Finding Template

Use this format when a file gets `NEEDS WORK` or `BLOCKED`:

```text
ID:
Severity:
File:
Lines:
Issue:
Risk:
Required change:
Verification:
Status:
```

## Verification Evidence Log

Record gates and targeted tests here as review waves complete.

| Date | Scope | Command / evidence | Result | Notes |
|---|---|---|---|---|
| TODO | TODO | TODO | TODO | TODO |

## Open Review Risks

- [ ] Complete file inventory has not yet been expanded from the initial area checklist.
- [ ] Vendor and generated-artifact review rules need to be applied before public release decisions.
- [ ] Existing uncommitted worktree changes mean review status must distinguish pre-existing changes from review fixes.
- [ ] Final approval will require targeted tests plus the relevant repo quality gates, not only static reading.
