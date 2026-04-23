# Review Packet: `src-tauri/src/commands_system.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_system.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small and mostly sound command surface, with the main remaining need being clearer platform-contract documentation rather than code churn`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Public-surface-sensitive`, `Platform-sensitive`, `Desktop-shell-sensitive`

## What This File Owns

`commands_system.rs` owns a small set of desktop/system-facing Tauri commands.

This file owns:

- the serialized GPU diagnostics payload:
  - `GpuStatus`: `src-tauri/src/commands_system.rs:3-13`
- feature-gated CUDA availability probing:
  - `cuda_available_now`: `15-24`
- public Tauri commands for:
  - GPU status: `26-42`
  - application exit: `44-48`
  - desktop window effect application: `50-105`
- one direct unit test for the GPU diagnostics shape:
  - `118-144`

This file does not own:

- frontend window-effect option filtering
  - `web/src/utils/windowEffects.ts`
- optimize-service GPU status normalization
  - `web/src/services/optimizeService.ts`

## Classification

- Public surface: yes
  - commands are re-exported by `src-tauri/src/commands.rs:140`
  - and registered in `src-tauri/src/lib.rs:134-188`
- Hot path: no
  - these commands are sporadic UI/runtime actions
- Runtime-critical: low to moderate
  - exit and window effects are UX-critical, but not computationally heavy
- CUDA/runtime-critical: low
  - GPU status is diagnostic, not execution-path logic
- Strategy-local: no
- Low-risk support code: yes

## Direct Interactions

### Backend interaction map

- command re-exports:
  - `src-tauri/src/commands.rs:140`
- invoke registration:
  - `src-tauri/src/lib.rs:134-188`
- CUDA device probing dependency:
  - `ta_app_core::preferred_cuda_device()` from `get_gpu_status`

### Frontend contract anchors

- optimize-service GPU refresh:
  - `web/src/services/optimizeService.ts:1243-1248`
  - heavily exercised in `web/src/services/optimizeService.test.ts`
- settings/window-effect bridge:
  - `web/src/services/settingsService.ts:1-8`
  - `web/src/services/settingsBootstrap.test.ts:126-176`
- platform-specific effect filtering:
  - `web/src/utils/windowEffects.ts:1-43`
- close/exit UX:
  - `web/src/components/DesktopTitlebar.vue:186-196`
  - `web/src/components/DesktopTitlebar.test.ts:151-174`
- command allowlist:
  - `web/src/tauri/commands.ts:54-108`

## Standards And Research Log

This packet was written after a targeted external review focused on Tauri command contracts and the platform guarantees of the `window_vibrancy` crate used by `set_window_effect`.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file is entirely Tauri command surface
   - Standard informed:
     - public commands in separate modules should be `pub`
     - command-return contracts and error transport are part of the frontend API

2. `window_vibrancy` crate docs
   - Source: <https://docs.rs/window-vibrancy/latest/window_vibrancy/>
   - Why it was consulted:
     - `set_window_effect` is a thin wrapper around this crate
   - Standard informed:
     - Linux is unsupported
     - Windows/macOS support is effect-specific and version/platform constrained

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - even small public commands should have explicit contract clarity and failure semantics
   - Standard informed:
     - shared command surfaces benefit from clear, unsurprising return contracts and documented failure behavior

## Findings

## Correctness And Safety

### 1. `set_window_effect` is intentionally best-effort on unsupported platforms, but that contract is not documented

- Evidence:
  - non-Windows/macOS branch returns `Ok(())`: `src-tauri/src/commands_system.rs:99-104`
  - `window_vibrancy` documents Linux as unsupported
  - frontend platform filtering already restricts non-Windows/macOS users to `none`:
    - `web/src/utils/windowEffects.ts:5-30`
- Why it matters:
  - the behavior is defensible, but readers could easily mistake it for a bug or assume unknown effects will error consistently on every platform
- Review judgment:
  - acceptable as-is, but the behavior should be documented explicitly

### 2. Window-effect clearing errors are silently ignored

- Evidence:
  - Windows clear calls use `let _ = ...`: `67-69`
  - macOS clear call uses `let _ = ...`: `94`
- Why it matters:
  - if a previous effect fails to clear, the command may still report success after applying or skipping the new effect
- Review judgment:
  - low severity for a cosmetic command, but this is still hidden behavior
- Smallest justified fix:
  - document that clearing is best-effort, or surface/log clear failures if the repo wants stricter desktop diagnostics

## API And Naming

### 1. `get_gpu_status` and `exit_application` expose `Result` signatures even though they do not currently return command-level errors

- Evidence:
  - `get_gpu_status` always returns `Ok(GpuStatus { ... })`: `26-42`
  - `exit_application` always returns `Ok(())` after `app.exit(0)`: `44-48`
- Why it matters:
  - the command signature suggests rejectable command-level failure when the real contract is “always resolve, with error details embedded in payload or handled by the platform”
- Review judgment:
  - this is a mild API-shape nit, not a correctness problem
- Smallest justified fix:
  - either document the behavior or simplify the signatures if the frontend contract can absorb it

### 2. `GpuStatus` payload design is reasonable and frontend-aligned

- Positive note:
  - `auto_device_error` stays in the payload instead of turning expected GPU-probe failures into command rejection
  - this matches the optimize-service consumer, which treats GPU status as diagnostics, not fatal workflow gating

## Error Handling

### 1. GPU probe failure is intentionally modeled as data, not command failure

- Evidence:
  - `preferred_cuda_device()` errors become `auto_device_error` in the payload: `28-34`
- Why it matters:
  - this keeps UI refresh non-throwing and is aligned with frontend usage in `optimizeService.ts`
- Review judgment:
  - good choice for diagnostics, but should be documented because the `Result` return type does not reveal that policy

### 2. Unknown effect handling is platform-asymmetric by design

- Evidence:
  - Windows/macOS return `Err("Unknown window effect: ...")` for unsupported strings: `78-82`, `96-103`
  - non-supported platforms return `Ok(())`: `99-104`
- Why it matters:
  - this asymmetry is acceptable because the frontend prefilters supported options, but it is worth stating explicitly in a review packet

## Module Boundaries And Technical Debt

### 1. The file is cohesive and does not carry major structural debt

- Evidence:
  - small command-focused module with three related responsibilities
- Review judgment:
  - no larger refactor is justified here

## Docs And Examples

### 1. The platform contract should be written down near `set_window_effect`

- Missing documentation topics:
  - Linux/other platforms are best-effort no-op
  - Windows/macOS effect names are platform-specific
  - clear operations are best-effort

### 2. The diagnostic contract should be written down near `get_gpu_status`

- Missing documentation topics:
  - command-level `Err` is not presently used
  - probe failures arrive inside `GpuStatus.auto_device_error`

## Tests And Missing Verification

### 1. Rust-side test coverage is very thin

- Direct Rust coverage:
  - one unit test for `get_gpu_status`: `118-144`
- Missing Rust coverage:
  - `set_window_effect` unknown-effect path
  - non-Windows/macOS no-op behavior
  - `exit_application` contract

### 2. Frontend coverage is stronger and should be part of any verification plan

- exercised through:
  - `web/src/services/settingsService.test.ts`
  - `web/src/services/settingsBootstrap.test.ts`
  - `web/src/services/optimizeService.test.ts`
  - `web/src/components/DesktopTitlebar.test.ts`
  - `web/src/tauri/commands.test.ts`

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance issue stands out here

- `get_gpu_status` is diagnostic and sporadic
- `set_window_effect` is UI-only desktop glue
- this file is not a benchmark target

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `web/src/services/optimizeService.ts`
- `web/src/services/settingsService.ts`
- `web/src/utils/windowEffects.ts`
- `web/src/components/DesktopTitlebar.vue`
- `src-tauri/src/lib.rs`

### Downstream consumers needing verification

- optimize panel GPU status refresh
- settings bootstrap/application of window effects
- window close/exit flow

## Code-Quality Improvement Register

### Safe now

- add short contract comments for `get_gpu_status` and `set_window_effect`
- add a small test or manual-verification note for unknown-effect behavior

### Coordination required

- changing command return types from `Result` to direct values
- changing unsupported-platform behavior for `set_window_effect`

### Defer until later pass

- any broader error-type normalization across the entire Tauri command surface

## Technical-Debt Reduction Register

### Debt safe to reduce now

- missing contract comments
- missing small tests around command semantics

### Debt that spans multiple files

- whether small always-OK diagnostic commands should continue to use `Result`

### Debt that should not be “cleaned up” casually

- frontend platform filtering for window effects
- non-throwing GPU diagnostics consumption in optimize flows

## Performance Opportunity Register

- Safe structural opportunities:
  - none important
- Benchmark-sensitive opportunities:
  - none
- Not the right place to chase performance:
  - yes

## Observability And Diagnostics Gaps

- clear failures for window effects are currently silent
- the always-OK diagnostics pattern is not documented in the backend file itself

## Open Questions And Assumptions

- assumption:
  - best-effort no-op behavior for unsupported platforms is intentional because the frontend already hides non-supported effects
- open question:
  - whether the repo wants command-level errors for cosmetic effect failures, or prefers the current quiet behavior

## Regression Risk And Feature-Preservation Constraints

- Do not change `set_window_effect` unsupported-platform behavior casually; settings bootstrap and platform filtering currently assume it is harmless on unsupported systems.
- Do not turn GPU diagnostics into hard command failure unless the optimize frontend is updated accordingly.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_system -- --nocapture`
2. `npm --prefix ./web run test -- src/components/DesktopTitlebar.test.ts src/services/settingsBootstrap.test.ts src/services/settingsService.test.ts src/services/optimizeService.test.ts src/tauri/commands.test.ts`
3. Manual desktop verification if `set_window_effect` changes:
   - on Windows, try `mica`, `acrylic`, and `blur`
   - on macOS, try `vibrancy`
   - on unsupported platforms, confirm the command remains harmless

## Recommended Action

`Document only`

This file does not need meaningful structural work. The useful improvements are to make the existing command contracts explicit and keep verification focused on the real frontend consumers.
