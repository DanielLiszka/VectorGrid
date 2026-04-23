# Second-Pass Review: `src-tauri/src/commands_options_expression.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_options_expression.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_options_expression.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as contract-heavy because it owns the live options-expression workflow, replay reconstruction, cache policy, artifact inspection, and several path-sensitive helper layers.

## Stale Check

The live source still shows the same concentration of responsibilities:

- run execution and feasibility preview
- artifact listing/load/export
- replay reconstruction from parent artifacts plus imported datasets
- in-memory and persisted replay caches

The second pass found that the file’s side-effect surface is wider and more internally inconsistent than the first pass rated it.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm that these helpers are a public command/API surface, not just internal composition
   - Additional constraint informed:
     - read-only command semantics matter here because the Tauri layer exposes them directly

2. Rust `canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why consulted in second pass:
     - refine the containment review for artifact and parent-artifact paths
   - Additional constraint informed:
     - stronger containment checks should be applied consistently across load and list paths

3. Rust `create_dir_all`
   - Source: <https://doc.rust-lang.org/std/fs/fn.create_dir_all.html>
   - Why consulted in second pass:
     - verify whether apparently read-only helpers are mutating disk
   - Additional constraint informed:
     - several list/load/cache helpers still create directories eagerly

4. Bincode 1.3.3 documentation
   - Source: <https://docs.rs/crate/bincode/1.3.3>
   - Why consulted in second pass:
     - assess how much built-in format/version protection the persisted caches actually have
   - Additional constraint informed:
     - the cache format has no built-in version/header guardrails

## Delta vs First Pass

## Stronger Read-Only Finding

### 1. Several apparently read-only paths still mutate disk through eager cache and runs-dir creation

- Evidence:
  - `runs_dir`, replay-cache helpers, and dataset-store helpers still call `create_dir_all`
- Why this matters:
  - the file’s command surface is more side-effectful than the first pass rated it
- Practical implication:
  - the safest conclusion is no longer "small only" across the whole file; the side-effect model is broad enough to justify deferring deeper cleanup to a larger refactor plan

## Stronger Containment Finding

### 2. Artifact listing still reopens `parent_artifact_path` without the stronger containment discipline used elsewhere in the file

- Evidence:
  - parent-artifact handling is stricter in some code paths than others
- Why this matters:
  - list-style and inspect-style paths should not be the weaker containment edge in an artifact-heavy workflow
- Practical implication:
  - if this surface is hardened, containment policy should be unified rather than patched one call site at a time

## Stronger Cache-Policy Finding

### 3. Cache corruption handling is internally inconsistent and the persisted cache format has weak self-description

- Evidence:
  - some corrupt caches degrade to cache miss, while others fail the workflow
- Why this matters:
  - this is no longer just "cache cleanup"; it is a workflow-policy inconsistency spread across multiple responsibilities
- Practical implication:
  - the second-pass action bucket should move upward even if no immediate rewrite is recommended

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`

Add targeted coverage for:

- all-negative summary values
- corrupt signal-manifest cache fallback
- parent-artifact containment during listing
- read-only list/load paths avoiding directory creation

## Regression And Feature-Preservation Constraints

- Preserve `vectorgrid_options_expression_v1` artifact-kind enforcement
- Preserve `runs/` containment for artifact load and inspect paths
- Keep replay caches as accelerators only, never sources of truth
- Preserve dataset-manifest compatibility and `aggregate_price_field = "close"` enforcement

## Second-Pass Conclusion

`First-pass bucket changed`

The second pass upgrades this file from `Small surgical refactor` to `Larger refactor but defer`. The issue is not one isolated bug; it is that read-only semantics, containment discipline, and cache-policy behavior are spread across too many responsibilities to treat as one more small cleanup packet.
