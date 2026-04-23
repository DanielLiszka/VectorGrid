# Review Packet: `src-tauri/src/lib.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/lib.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Central and mostly coherent bootstrap code, but still carrying a few implicit startup assumptions that should be made explicit because this file is the actual custom-IPC exposure root for the desktop app`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Desktop-shell-sensitive`, `Startup-sensitive`, `IPC-surface-sensitive`

## What This File Owns

`lib.rs` owns the top-level Tauri application bootstrap for the desktop crate.

This file owns:

- root module exports for the desktop crate:
  - `pub mod ...`: `src-tauri/src/lib.rs:1-6`
- the application bootstrap entrypoint:
  - `run_tauri`: `15-194`
- plugin registration:
  - log plugin: `17-33`
  - notification plugin: `34`
  - window-state plugin: `35`
- managed state injection:
  - `AppState`: `36`
  - `BackgroundTaskState`: `37`
- manual main-window creation and presentation:
  - `setup` closure: `38-56`
- optional CUDA/options-expression runtime prewarm on startup:
  - `58-76`
- the canonical `generate_handler!` command-registration list:
  - `82-192`

This file does not own:

- actual command implementations
  - they live under `src-tauri/src/commands*.rs`
- frontend command-registry generation
  - that lives in `web/scripts/generate-tauri-commands.mjs`
- capability JSON definitions
  - that lives in `src-tauri/capabilities/main.json`

## Classification

- Public surface: yes
  - this is the real custom-command exposure root for the app
- Hot path: no
  - startup/bootstrap only
- Runtime-critical: high
  - plugin/state/window/IPC registration all depend on it
- CUDA/runtime-critical: moderate
  - startup prewarm decisions live here when CUDA features are enabled
- Strategy-local: no
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- imports the flattened command facade:
  - `use commands::*;`: `src-tauri/src/lib.rs:8`
- registers the custom IPC surface through `generate_handler!`:
  - `82-192`
- wires in managed state used broadly by command modules:
  - `36-37`
- delegates command implementations into `src-tauri/src/commands.rs` and its child modules

### Config, security, and runtime anchors

- main window config is defined in `tauri.conf.json`:
  - `src-tauri/tauri.conf.json:20-30`
  - note in particular:
    - label `main`: `22`
    - `create: false`: `24`
    - `visible: false`: `29`
- active capability mapping is configured in `tauri.conf.json`:
  - `src-tauri/tauri.conf.json:15-18`
- the `main-window` capability grants window/log/notification/window-state plugin permissions:
  - `src-tauri/capabilities/main.json:3-15`

### Frontend contract anchors

- frontend command registry is generated from this file's `generate_handler!` block:
  - `web/scripts/generate-tauri-commands.mjs:49-66`
- frontend/backend command-name parity is checked against this file:
  - `web/scripts/check-tauri-parity.mjs:44-72`
- frontend runtime notes describe that contract explicitly:
  - `web/README.md:36-37`

### Test anchors

- mock runtime only verifies managed-state resolution using a separate test builder:
  - `src-tauri/tests/mock_runtime_smoke.rs:8-23`
  - `src-tauri/tests/support/mod.rs:3-8`
- command-runtime wiring tests exercise selected commands, not `run_tauri()` itself:
  - `src-tauri/tests/command_runtime_wiring.rs`

## Standards And Research Log

This packet was written after targeted external review focused on Tauri command registration, managed state, and capability-based IPC exposure.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file defines the `generate_handler!` list
   - Standard informed:
     - commands in separate modules are registered here
     - command names must be unique
     - this list is the actual frontend-invokable custom command surface

2. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - this file uses `.manage(...)` for shared app state
   - Standard informed:
     - managed state is part of the application bootstrap contract and should remain explicit and coherent

3. Tauri capabilities
   - Source: <https://v2.tauri.app/security/capabilities/>
   - Why it was consulted:
     - this file interacts with `tauri.conf.json` and capability JSON to define the runtime IPC boundary
   - Standard informed:
     - windows/webviews without matching capabilities have no IPC access
     - when `app_manifest.commands` is not used, registered app commands are allowed by default

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this is the top-level application API/wiring surface
   - Standard informed:
     - central public-surface code should make boundary ownership and failure behavior clear

## Findings

## Correctness And Safety

### 1. Main-window bootstrap is order-sensitive because it selects the first configured window instead of the window labeled `main`

- Evidence:
  - startup window config selection uses `app.config().app.windows.get(0)`: `39-41`
  - the intended window is identified by label in config:
    - `src-tauri/tauri.conf.json:20-30`
    - `label: "main"` at `22`
- Why it matters:
  - if another window is added earlier in the config list, this bootstrap path can create or restore the wrong window
  - the current config is safe because there is only one window, but the code is more implicit than it needs to be
- Smallest justified fix:
  - look up the window by label instead of by index, or explicitly assert that the first entry is the `main` window

## API And Naming

### 1. `lib.rs` is the canonical custom-command exposure root, but that is easy to confuse with `commands.rs`

- Evidence:
  - `commands.rs` is imported wholesale at `8`
  - only the `generate_handler!` list at `82-192` becomes frontend-invokable
  - the web generator parses only this file:
    - `web/scripts/generate-tauri-commands.mjs:49-66`
  - parity tooling compares frontend registry to this file:
    - `web/scripts/check-tauri-parity.mjs:44-72`
- Why it matters:
  - this file is the widest custom-IPC surface in the app
  - a new re-export in `commands.rs` does not become callable until it is added here
- Review judgment:
  - not a bug, but an important boundary fact that should be stated explicitly

### 2. The app currently exposes a broad custom-command surface to the single `main-window` capability

- Evidence:
  - `tauri.conf.json` binds the app to capability `main-window`: `15-18`
  - `main-window` grants plugin permissions but does not narrow custom command names: `src-tauri/capabilities/main.json:3-15`
  - Tauri capability guidance says registered app commands are allowed by default unless app-manifest command restrictions are used
- Why it matters:
  - every command named in `generate_handler!` should be treated as part of the live desktop IPC boundary
- Review judgment:
  - this is a public-surface sensitivity note, not a defect by itself

## Error Handling

### 1. Window-state restore failure is intentionally non-fatal, but startup prewarm failures are only log-visible

- Evidence:
  - restore-state failure logs warn and continues: `49-51`
  - CUDA prewarm failures also log warn and continue: `58-76`
- Why it matters:
  - the current behavior is operationally reasonable
  - it also means startup degradations are only visible in logs, not in structured runtime diagnostics or tests
- Smallest justified fix:
  - document the best-effort policy; only add stronger surfacing if the product needs it

## Module Boundaries And Technical Debt

### 1. `run_tauri()` combines plugin setup, state injection, manual window creation, startup prewarm, and 109-command registration in one place

- Evidence:
  - all of that behavior lives in `15-194`
- Why it matters:
  - this is the correct place for central bootstrap logic
  - it also means small edits here can affect multiple orthogonal startup concerns at once
- Review judgment:
  - acceptable for current size, but changes should remain surgical and well-verified

### 2. The WebView data-directory policy is hard-coded and implicit

- Evidence:
  - `std::env::temp_dir().join("vectorgrid").join("webview2")`: `43`
  - that path is then passed to `.data_directory(...)`: `45-47`
- Why it matters:
  - persistence, isolation, and cleanup expectations for the embedded webview profile are implicit rather than documented
- Smallest justified fix:
  - add a short comment describing why temp storage is used here, or derive the path from a more explicit policy if that is the intended product direction

## Docs And Examples

### 1. The startup coupling between `tauri.conf.json` and the manual `setup` path should be documented locally

- Missing documentation topics:
  - `create: false` means the window is created manually here
  - `visible: false` plus later `show()` avoids early display until setup is done
  - the intended bootstrap target is the window labeled `main`

### 2. The custom-command exposure contract should be documented near `generate_handler!`

- Missing documentation topics:
  - this list is the real app-command IPC surface
  - frontend registry generation and parity checks derive from this exact block

## Tests And Missing Verification

### 1. Command-name parity automation is strong and currently green

- Verified in this review pass:
  - `npm --prefix ./web run check:commands`
    - passed, reporting `109` commands
  - `cd web; node ./scripts/check-tauri-parity.mjs`
    - passed, reporting `109` backend commands and `109` frontend registry commands

### 2. Rust-side bootstrap verification is thin

- `cargo test -p ta_desktop_demo_app --test mock_runtime_smoke -- --nocapture`
  - passed `1` test
- What that test actually proves:
  - managed `AppState` and `BackgroundTaskState` can be resolved from a mock app
  - refs:
    - `src-tauri/tests/mock_runtime_smoke.rs:8-23`
    - `src-tauri/tests/support/mod.rs:3-8`
- What it does not prove:
  - the real `run_tauri()` builder path works
  - plugin registration succeeds
  - the manual `setup` window creation path succeeds
  - `restore_state` / `show` behaviors are correct
  - the CUDA prewarm path behaves as intended

### 3. Real bootstrap changes still require manual desktop verification

- especially if changing:
  - plugin list
  - capability wiring
  - manual window creation/show flow
  - startup prewarm logic

## Performance, Allocation, And CUDA Concerns

### 1. GPU runtime prewarm is startup-sensitive, not throughput-sensitive

- Evidence:
  - best-effort background prewarm: `58-76`
- Why it matters:
  - changes here affect cold-start behavior and first-use latency more than steady-state throughput
- Review judgment:
  - if the prewarm policy changes, measure startup UX impact rather than backend compute performance

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/commands.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/main.json`
- `web/scripts/generate-tauri-commands.mjs`
- `web/scripts/check-tauri-parity.mjs`
- `src-tauri/tests/mock_runtime_smoke.rs`
- `src-tauri/tests/support/mod.rs`

### Downstream consumers needing verification

- frontend typed command registry
- desktop shell/window bootstrap
- OS notification integration
- window-state restore/save behavior
- any CUDA-enabled startup path that depends on prewarm behavior

## Code-Quality Improvement Register

### Safe now

- select the startup window by label instead of list index
- add short comments explaining the manual window-creation path
- add a short note that `generate_handler!` is the canonical custom-command IPC surface

### Coordination required

- changing capability policy for custom commands
- changing WebView data-directory policy
- changing startup prewarm behavior

### Defer until later pass

- any broader bootstrap refactor that splits `run_tauri()` without a clear testing benefit

## Technical-Debt Reduction Register

### Debt safe to reduce now

- order-sensitive window lookup
- missing local documentation for startup assumptions

### Debt that spans multiple files

- command-surface parity between `lib.rs`, `commands.rs`, and the web generator
- capability policy and any future command scoping work

### Debt that should not be “cleaned up” casually

- manual window creation path
- best-effort non-fatal restore/prewarm policy
- current managed-state injection setup

## Performance Opportunity Register

- Safe structural opportunities:
  - none important beyond documentation and narrow bootstrap cleanup
- Benchmark-sensitive opportunities:
  - changing GPU prewarm policy
- Not the right place to chase backend throughput:
  - yes

## Observability And Diagnostics Gaps

- startup restore/prewarm issues are log-visible but not otherwise surfaced
- there is no direct test exercising the real `run_tauri()` path

## Open Questions And Assumptions

- assumption:
  - manual window creation is intentional because the configured main window uses `create: false`
- open question:
  - whether the hard-coded WebView data directory under temp is a deliberate product policy or an implementation convenience that should be documented more explicitly

## Regression Risk And Feature-Preservation Constraints

- Do not change the `generate_handler!` list casually; it is the effective custom-command IPC boundary.
- Do not alter manual main-window creation without verifying desktop startup behavior against `tauri.conf.json`.
- Do not change prewarm behavior casually on CUDA builds; it can affect perceived cold-start and first-use latency.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app --test mock_runtime_smoke -- --nocapture`
2. `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`
3. `npm --prefix ./web run check:commands`
4. `cd web; node ./scripts/check-tauri-parity.mjs`
5. Manual desktop verification:
   - cold app launch
   - main window appears correctly
   - window state restores as expected
   - notification-dependent surfaces still behave
   - CUDA-enabled build still starts cleanly if prewarm logic changed

## Recommended Action

`Small surgical refactor`

The highest-value change is to make the startup assumptions explicit and remove the order-sensitive main-window lookup. The rest of the file is mostly doing the right job already; what it needs is better local clarity because this is the real custom-command exposure and desktop bootstrap root for the app.
