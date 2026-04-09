# Static Analysis Quality Gates Plan

Last updated: 2026-04-09

## Purpose

Define the repo-wide static-analysis and code-quality gate plan so this public-facing repository has explicit, repeatable quality checks for Rust, Vue, TypeScript, and Tauri-facing code beyond behavioral tests alone.

## Scope Boundaries

In scope:

- Rust compile-time and lint-time quality commands
- frontend lint and typecheck commands
- formatting expectations that should be enforced consistently
- quality-gate ordering for local verification and future CI reuse
- additive guardrails that reduce public-repo quality drift without removing functionality

Out of scope:

- options-specific verification lanes
- runtime perf gates
- browser or desktop UX smoke flows except where they complement static checks
- introducing heavy new tooling without a clear quality return

## Current User-Facing Behavior

Users do not interact with static-analysis tooling directly, but public trust in the repo depends on it. The current repo already has some useful quality commands:

- frontend linting in `web/package.json` via `npm run lint`
- frontend type checking via `npm run typecheck` and `npm run typecheck:desktop`
- Rust compile verification via `cargo check` and crate-specific `cargo test`
- command-regeneration and parity checks wired into frontend lint/build scripts

At the same time, the current repo still lacks one explicit cross-language quality-gate plan. The repo root does not currently expose a single documented quality sequence for:

- `cargo check`
- `cargo test`
- `cargo fmt --check`
- `cargo clippy`
- `npm --prefix web run lint`
- `npm --prefix web run typecheck`

The result is that code quality is partly present in tooling but under-specified in the planning layer.

## Current Code Surface

- `Cargo.toml`
- `src-tauri/Cargo.toml`
- `crates/app_core/Cargo.toml`
- `web/package.json`
- `web/eslint.config.js`
- `web/tsconfig.json`
- `web/tsconfig.app.json`
- `web/tsconfig.node.json`
- `web/tsconfig.wdio.json`
- `.github/workflows.disabled/`
- `AGENTS.md`
- `RELEASE.md`

## Current Verification Surface

- `cargo check --workspace`
- `cargo test --workspace`
- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run typecheck:desktop`
- targeted desktop/browser smoke runs after static gates pass

## Current Strengths Already Present

- the frontend already has explicit lint and typecheck scripts
- the frontend lint script already includes command-regeneration checks instead of only syntax/style rules
- the repo already values compile/test verification and has many targeted test surfaces
- the improvement-program docs already require explicit verification plans for every implementation item
- the repo already distinguishes parity/perf validation from general correctness validation

## Current Weaknesses and Failure Modes

- there is no first-class implementation-plan lane for static analysis and formatting quality
- Rust lint/format expectations are implied rather than codified
- the repo can drift into “tests passed, code quality unknown” because compile and lint gates are not presented as one public quality bar
- public contributors have no one document telling them what “clean enough to merge” means
- disabled GitHub workflows mean local quality gates matter even more, yet those local gates are not fully centralized in the planning set
- formatting and lint churn can become noisy if they are introduced without scope rules

## Regressions To Avoid

- blocking useful work on flaky or slow quality commands
- treating stylistic preferences as more important than correctness or maintainability
- enabling overly noisy lint categories wholesale without repo-specific tuning
- breaking CPU-only verification paths while adding Rust linting expectations
- adding redundant gates that duplicate existing scripts without clarifying ownership
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Cargo check](https://doc.rust-lang.org/nightly/cargo/commands/cargo-check.html)
  - relevant because `cargo check` provides a faster compile-time correctness gate than full test runs and belongs early in the quality sequence
- [cargo fmt](https://doc.rust-lang.org/cargo/commands/cargo-fmt.html)
  - relevant because formatting is an explicit toolchain-supported contract and should be standardized rather than left to editor defaults
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because Clippy is the primary Rust lint surface for correctness, suspicious code, style, complexity, and perf-oriented warnings
- [TypeScript TSConfig `noEmit`](https://www.typescriptlang.org/tsconfig/noEmit.html)
  - relevant because this repo already uses TypeScript as a type-checking gate separate from build output generation
- [Getting Started with ESLint](https://eslint.org/docs/latest/use/getting-started)
  - relevant because the repo already ships an ESLint-based lint lane and should treat it as part of the public quality bar
- [typescript-eslint Getting Started](https://typescript-eslint.io/getting-started/)
  - relevant because TypeScript-aware linting depends on deliberate rule and parser configuration rather than generic JS linting alone

## Proposed Improvement Approach

1. Define one canonical local quality sequence for non-options work:
   - `cargo check --workspace`
   - `cargo test --workspace` or the relevant narrower target set
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets`
   - `npm --prefix web run lint`
   - `npm --prefix web run typecheck`
2. Make the sequence explicit in repo-facing docs rather than assuming contributors will infer it from scattered scripts.
3. Treat Rust formatting and linting as first-class expectations, but tune them conservatively:
   - avoid enabling entire aggressive lint groups blindly
   - prefer deny-on-merge for correctness-critical issues and explicit allowlists for intentional exceptions
4. Keep frontend linting and typechecking distinct:
   - lint for code-health and rule enforcement
   - typecheck for static type safety
5. Keep smoke, parity, and perf gates downstream from the static-analysis layer rather than using them as substitutes.
6. If CI is re-enabled later, mirror the local quality sequence instead of inventing a different CI-only contract.
7. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Step-by-Step Checklist

- [ ] Inventory the current Rust and frontend static-analysis commands already present in the repo.
- [ ] Decide the canonical non-options local quality-gate order.
- [ ] Document which commands are required on every code change versus only on scoped/high-risk changes.
- [ ] Add a Rust formatting expectation using `cargo fmt --check`.
- [ ] Add a Rust lint expectation using `cargo clippy` with a deliberate scope.
- [ ] Review whether a repo-level `rustfmt.toml` or `clippy.toml` is needed or whether defaults should remain authoritative.
- [ ] Keep frontend lint and typecheck commands explicitly separate in the quality bar.
- [ ] Document when narrower crate/test targets are acceptable instead of full-workspace runs.
- [ ] Tie static-analysis expectations to public-repo contribution guidance once that doc exists.
- [ ] Mirror the same quality sequence into future CI only after the local lane is stable.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `cargo check --workspace`
- run `cargo test --workspace`
- run `cargo fmt --check`
- run `cargo clippy --workspace --all-targets -- -D warnings` only if the repo is intentionally ready for deny-on-warning
- run `npm --prefix web run lint`
- run `npm --prefix web run typecheck`
- run `npm --prefix web run typecheck:desktop` when desktop test harness code changes
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- start by documenting and enforcing the existing high-value commands before adding new tooling
- if `cargo clippy -- -D warnings` is too disruptive initially, begin with a narrower documented baseline and tighten later
- do not add ESLint, TypeScript, or Rust rules that force large unrelated rewrites without a scoped migration plan
- preserve current product behavior; this plan is for code quality, not feature narrowing
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/03_frontend_build_handoff_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/16_public_release_operations_2026-04-09.md`
