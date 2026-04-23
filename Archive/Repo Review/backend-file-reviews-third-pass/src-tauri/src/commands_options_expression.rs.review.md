# Third-Pass Review: `src-tauri/src/commands_options_expression.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_options_expression.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_options_expression.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_expression.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

This file remained one of the highest-ROI third-pass candidates because it is both contract-heavy and side-effect-heavy. The second pass was directionally correct, but another round was needed to identify which defects are safe-now hardening targets and which still belong in the defer bucket.

## Third-Pass Research Log

1. Tauri calling Rust
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - confirm how strongly this command surface should be treated as a public API boundary
   - Third-pass constraint informed:
     - helper semantics exposed through Tauri commands are real product contract, not internal implementation detail

2. Tauri runtime authority
   - Source: <https://v2.tauri.app/security/runtime-authority/>
   - Why consulted in third pass:
     - re-check how much authority this command surface currently holds over the local filesystem
   - Third-pass constraint informed:
     - export/write commands need explicit scope or product-policy justification

3. Rust `canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why consulted in third pass:
     - reassess containment on listing, load, and cache paths
   - Third-pass constraint informed:
     - the remaining weak edge is no longer only cache paths; it is also parent-artifact reopening during listing

4. Rust `write`
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why consulted in third pass:
     - sharpen the export-path review
   - Third-pass constraint informed:
     - `save_options_csv` is a direct caller-controlled write primitive

## What Third Pass Sharpened

### 1. `save_options_csv` is not just a helper; it is a frontend-reachable arbitrary write primitive

- Evidence:
  - the command is registered publicly
  - the caller supplies the output path
  - the helper creates parent directories and writes directly with no containment or export-root rule
- Why this matters:
  - this is a narrower, more actionable issue than the second-pass "broad side-effect surface" framing
- Practical implication:
  - this is a `small surgical refactor now` item inside the larger deferred file
  - the safest fix is explicit export policy, not silently forcing everything into `runs/`

### 2. Artifact listing still has an out-of-root read edge through `parent_artifact_path`

- Evidence:
  - listing iterates artifacts in `runs/`
  - summary construction can reopen `artifact.request.parent_artifact_path` without the stronger `runs/` containment discipline used elsewhere
  - malformed or unreadable artifacts are silently skipped
- Why this matters:
  - a hand-edited or malformed artifact inside `runs/` can widen what the list path reads
  - callers cannot distinguish "no artifacts" from "artifacts were skipped because they were corrupt"
- Practical implication:
  - this is another `small surgical refactor now` candidate inside the larger deferred file

### 3. Saved-artifact inspection is still coupled to current rank order rather than stable tuple identity

- Evidence:
  - tuple and event-path inspection re-run the engine and select by `tuple_index`
- Why this matters:
  - harmless ranking changes, tie-break changes, or exactification changes can make historical artifact inspection brittle even when provenance still matches
- Practical implication:
  - the third pass sharpens this into a concrete recommendation:
    - resolve inspection by stored tuple spec or stable tuple hash, not current rank position

### 4. Cache failure policy is still inconsistent enough to affect user-visible reliability

- Evidence:
  - corrupt signal-manifest cache deserialization is fatal
  - corrupt dataset-cache state can degrade to cache miss
  - the file still spans in-memory caches, persisted caches, and artifact inspection
- Practical implication:
  - the second-pass defer bucket still fits
  - but safe-now fixes should be pulled out explicitly:
    - export-path scoping
    - parent-artifact containment during listing

## Implementation Priority

- Immediate high-ROI hardening inside the deferred bucket:
  - scope `save_options_csv`
  - harden parent-artifact reopening during artifact listing
- Next bounded improvement:
  - decouple inspection from current tuple rank

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`
- `npm --prefix web run test -- src/services/optionsExpressionService.test.ts`

Add targeted coverage for:

- export-path policy on `save_options_csv`
- listing behavior when a run artifact contains an out-of-scope or corrupt `parent_artifact_path`
- historical artifact inspection surviving benign tuple reordering
- corrupt cache behavior remaining non-authoritative

## Regression And Feature-Preservation Constraints

- Preserve `vectorgrid_options_expression_v1` artifact-kind enforcement
- Preserve current legitimate export UX if user-chosen paths are part of product intent
- Keep caches as accelerators only, never sources of truth
- Do not silently narrow artifact browsing or inspection compatibility

## Third-Pass Conclusion

`Bucket unchanged, but two safe-now fixes should be pulled forward`

The file still belongs in `Larger refactor but defer`, but the third pass makes the immediate work much clearer. `save_options_csv` path scoping and parent-artifact containment during listing are bounded hardening tasks that do not need to wait for a full restructure of the options-expression command surface.
