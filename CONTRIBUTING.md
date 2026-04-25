# Contributing to VectorGrid

Thanks for taking the time to improve this repository.

This project is maintained by a single owner and the contribution bar is intentionally narrow: small, well-scoped changes with clear verification are much more likely to be accepted than broad refactors or speculative expansion.

## Before You Start

Read these first:

- `README.md` for product scope and the public quality bar

If you are proposing a larger workflow change, open an issue first so the shape can be agreed before significant implementation work starts.

## Good Contribution Shapes

Good fits for this repo:

- targeted bug fixes
- scoped UX, docs, or verification improvements
- focused performance or diagnostics work backed by evidence
- additive tests near the changed surface
- narrow release/build/documentation hardening that matches the existing product scope

Poor fits for drive-by pull requests:

- broad cleanup-only refactors
- unrelated formatting churn
- large architectural rewrites without prior discussion
- changes that widen product scope without an agreed issue
- generated build outputs, local profiler captures, or local-only environment files

## Bug Reports And Feature Requests

Use GitHub Issues for reproducible bugs, scoped feature requests, and usage questions.

A useful report includes:

- the exact commit or release tag
- Windows version
- whether you used the CPU-only, GPU (MA sweep), or GPU (kernel) path
- NVIDIA GPU model and driver version when GPU paths are involved
- the exact command, workflow, or screen you used
- the observed result, the expected result, and any saved logs or screenshots

Please keep one issue per bug or request.

## Pull Request Expectations

Keep pull requests small and explicit:

- explain the problem being solved
- describe the chosen scope and anything intentionally left out
- update adjacent docs when behavior or operator expectations change
- add or adjust tests close to the changed code when behavior changes
- avoid mixing behavior work with unrelated cleanup

If your change touches performance-sensitive or route-selection code, include the gate, benchmark, or profiling evidence that justifies the change.

## Verification Expectations

Run the narrowest relevant verification before asking for review. Common examples:

- `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1`
- `cargo test --workspace`
- `npm.cmd --prefix web run lint`
- `npm.cmd --prefix web run typecheck`

If you intentionally use a narrower command, say why.

## Docs And Artifact Sync

When behavior changes, update the public docs that users and contributors actually rely on:

- `README.md`
- adjacent crate, script, or frontend documentation when that surface needs it

Do not commit large generated artifacts, local benchmark byproducts, or local profiler captures unless the change explicitly requires a checked-in artifact and the reason is documented in the pull request.

## Review And Merge

There is no guaranteed review SLA. Reviews are best-effort and may be limited to the latest tagged release line or the current default branch.
