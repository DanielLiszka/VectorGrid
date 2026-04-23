# Third-Pass Review: `src-tauri/src/commands_reports_shared.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_shared.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_shared.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_shared.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

The second-pass packet was reasonable, but it still framed this file too narrowly as report helper glue. Another pass was warranted because these helpers back public open/reveal commands and therefore sit on a real desktop trust boundary.

## Third-Pass Research Log

1. Tauri calling Rust
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - confirm that these helpers sit behind frontend-callable command boundaries rather than remaining purely internal utilities
   - Third-pass constraint informed:
     - path-open/reveal behavior here is public runtime contract

2. Tauri runtime authority
   - Source: <https://v2.tauri.app/security/runtime-authority/>
   - Why consulted in third pass:
     - check whether command reachability itself imposes local path policy
   - Third-pass constraint informed:
     - authority controls who may call commands, not whether this file itself scopes the target paths

3. Rust `Path`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why consulted in third pass:
     - revisit `exists()` versus `try_exists()` for both probing and launch preflight
   - Third-pass constraint informed:
     - `exists()` collapses permission or metadata failures into `false`, which weakens both collision probing and path diagnostics

4. Rust `Command`
   - Source: <https://doc.rust-lang.org/std/process/struct.Command.html>
   - Why consulted in third pass:
     - verify what `spawn()` success really proves
   - Third-pass constraint informed:
     - the helpers should keep best-effort launch semantics rather than claiming OS-open success

## What Third Pass Sharpened

### 1. This file is a public existing-path opener/revealer boundary, not just filename glue

- Evidence:
  - the helpers back public Tauri commands
  - the only path policy here is effectively "path must exist"
- Why this matters:
  - the second-pass file framing understated that these are arbitrary existing-path launcher helpers unless product scope says otherwise
- Practical implication:
  - the packet should now treat this as a deliberate trust-boundary choice:
    - either keep arbitrary-path behavior and document it honestly
    - or introduce explicit scoping/allowlisting

### 2. The `exists()` issue is stronger than the second pass stated because it affects both collision probing and launch preflight truthfulness

- Evidence:
  - both `existing_path()` and report-path slot selection rely on `exists()`
- Why this matters:
  - inaccessible or metadata-failing paths can be misreported as missing or available
- Practical implication:
  - `try_exists()`-style handling is the highest-ROI bounded hardening here

## Implementation Priority

- Highest-ROI hardening:
  - improve preflight/path-probing truthfulness and decide whether arbitrary existing-path launch is a deliberate product policy
- Keep explicit:
  - `spawn()` success remains best-effort process launch only

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_shared -- --nocapture`

Add targeted coverage for:

- permission-error and metadata-error probing
- collision-slot selection under inaccessible paths
- open/reveal helpers preserving best-effort semantics

## Regression And Feature-Preservation Constraints

- Preserve app-generated report and artifact open/reveal flows
- Do not strengthen `spawn()` into a false "OS definitely opened the target" contract
- If path scoping is added, use explicit allowlists or roots rather than silently breaking current UX

## Third-Pass Conclusion

`Bucket unchanged, but the trust boundary is wider than second pass stated`

The file still fits `Small surgical refactor`, but the third pass upgrades the framing. `commands_reports_shared.rs` is not only report filename glue; it is also the implementation behind public path-launch commands, and its current `exists()`-based preflight is still weaker than the user-facing contract suggests.
