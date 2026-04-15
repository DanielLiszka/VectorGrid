# Second-Pass Review: `src-tauri/src/lib.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/lib.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/lib.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as the real Tauri runtime boundary because command registration, plugin/capability composition, and manual window bootstrap all converge here.

## Stale Check

The live source still owns:

- the `generate_handler!` command registration surface
- startup/plugin/capability composition
- manual window/bootstrap choices

The first-pass packet remains current.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - reconfirm what `generate_handler!` represents at runtime
   - Additional constraint informed:
     - this macro block is the effective custom-command IPC boundary

2. Tauri capabilities
   - Source: <https://v2.tauri.app/security/capabilities/>
   - Why consulted in second pass:
     - distinguish capability policy from custom-command registration
   - Additional constraint informed:
     - the capabilities file mostly scopes plugin permissions, not the custom command list itself

3. Tauri runtime authority
   - Source: <https://v2.tauri.app/security/runtime-authority/>
   - Why consulted in second pass:
     - confirm how invoke requests are authorized at runtime
   - Additional constraint informed:
     - custom command registration and capability policy are related but not interchangeable boundaries

## Delta vs First Pass

### 1. The command-registration block is the actual custom-command IPC boundary because no narrower app-command capability policy exists

- Why this matters:
  - command additions or removals here have immediate runtime-authority implications
- Practical implication:
  - keep this file under explicit contract discipline rather than treating it as bootstrap boilerplate

### 2. Startup window choice still depends on config index order

- Why this matters:
  - adding another configured window could silently change bootstrap behavior
- Practical implication:
  - preserve the current manual-window assumptions unless multi-window policy is intentionally revisited

## Verification If Changed

- `cargo test -p ta_desktop_demo_app --test mock_runtime_smoke -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`
- `npm --prefix ./web run check:commands`
- manual desktop launch

## Regression And Feature-Preservation Constraints

- Preserve the current command list unless dependents are moved in lockstep
- Preserve manual-window bootstrap and best-effort restore/prewarm behavior
- Treat this file as the effective custom-command boundary, not mere startup glue

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that `generate_handler!` is the real custom-command boundary and that startup window selection is still more configuration-order-sensitive than it first appears.
