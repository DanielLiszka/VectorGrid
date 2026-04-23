# Second-Pass Review: `src-tauri/src/commands.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a broad facade whose main risk is API blast radius rather than internal complexity.

## Stale Check

The live source still serves as:

- the public Rust re-export surface for command modules
- a shared facade used by sibling backend modules and helper binaries

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust reference on `use` declarations
   - Source: <https://doc.rust-lang.org/stable/reference/items/use-declarations.html>
   - Why consulted in second pass:
     - confirm whether these public `use` statements are real API surface
   - Additional constraint informed:
     - this file is a true re-export boundary, not merely internal organization

2. Rustdoc re-export guidance
   - Source: <https://doc.rust-lang.org/rustdoc/write-documentation/re-exports.html>
   - Why consulted in second pass:
     - verify the public-doc and API implications of cleanup here
   - Additional constraint informed:
     - re-export churn changes Rust-facing surface even if frontend IPC is unchanged

3. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - distinguish Rust exports from actual invoke registration
   - Additional constraint informed:
     - frontend drift tooling covers `lib.rs`, not necessarily this facade

## Delta vs First Pass

### 1. Rust-only export regressions can hide here even when the Tauri command list still compiles

- Why this matters:
  - helper binaries or sibling modules can depend on these re-exports
- Practical implication:
  - cleanup remains small and deliberate work, not wholesale facade pruning

## Verification If Changed

- `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`
- `npm --prefix ./web run check:commands`
- `web/scripts/check-tauri-command-parity.mjs`
- `web/scripts/generate-tauri-commands-doc.mjs`

## Regression And Feature-Preservation Constraints

- Preserve public headless/helper exports until dependents migrate
- Treat this file as Rust API surface, not just Tauri glue
- Keep facade cleanup small and dependency-aware

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that this facade is a real Rust re-export boundary, so export cleanup can break consumers even when frontend command parity still passes.
