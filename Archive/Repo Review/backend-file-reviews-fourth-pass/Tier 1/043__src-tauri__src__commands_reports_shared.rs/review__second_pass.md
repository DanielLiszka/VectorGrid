# Second-Pass Review: `src-tauri/src/commands_reports_shared.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_shared.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_shared.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already rated this helper file as mostly sound but contract-sensitive because it owns shared filename sanitization, report-path generation, and shell-launch behavior.

## Stale Check

The live source still owns the same persistent contracts:

- filename sanitization and truncation
- collision probing
- best-effort open/reveal shell behavior

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `Path::try_exists`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html#method.try_exists>
   - Why consulted in second pass:
     - refine the first-pass concern around `Path::exists()` ambiguity
   - Additional constraint informed:
     - the current probing logic can still misread permission or metadata failures as "slot free"

2. Rust `std::process::Command`
   - Source: <https://doc.rust-lang.org/std/process/struct.Command.html>
   - Why consulted in second pass:
     - verify what `spawn()` success really means for open/reveal behavior
   - Additional constraint informed:
     - launch success is still best-effort process creation, not proof that the OS opened the target

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground the shared-helper contract review
   - Additional constraint informed:
     - persisted path and naming helpers should be treated as durable output contracts

## Delta vs First Pass

## Refined Existence-Probing Finding

### 1. The `Path::exists()` concern is broader than simple readability because it also affects collision probing

- Evidence:
  - filename slot selection still depends on existence checks
- Why this matters:
  - permission or metadata failures can be misread as a free filename slot
- Practical implication:
  - any cleanup here should treat probing and error reporting together

## Refined Launch-Semantics Finding

### 2. `spawn()` success must remain best-effort launch semantics, not proof of successful file reveal/open

- Evidence:
  - the current helpers still rely on process spawn as the OS integration boundary
- Why this matters:
  - frontend fallback logic already assumes "launch attempted" rather than "open guaranteed"
- Practical implication:
  - keep that best-effort contract explicit if this helper is revised

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_shared -- --nocapture`

Add targeted coverage for:

- permission-error probing
- unsupported-platform launch behavior
- collision-path regression

## Regression And Feature-Preservation Constraints

- Keep filename truncation and suffix behavior stable unless all naming tests move with it
- Keep the reports directory under the app-local base
- Preserve best-effort open/reveal semantics so frontend fallback remains valid

## Second-Pass Conclusion

`Bucket unchanged, but probing and launch semantics are sharper`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mostly clarifies two things: collision probing still has `exists()` ambiguity, and `open`/`reveal` helpers should remain explicitly best-effort OS launch attempts rather than stronger success claims.
