# Review Packet: `src-tauri/src/commands.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful command-and-type facade, but under-documented enough that maintainers could easily confuse Rust-facade exports with the real Tauri/frontend command contract`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Rust-facade-sensitive`, `Wiring-sensitive`

## What This File Owns

`commands.rs` owns the assembly and re-export spine for the desktop crate's backend command surface.

This file owns:

- child-module assembly for the command/runtime family:
  - `#[path = ...] mod ...;`: `src-tauri/src/commands.rs:1-38`
- the public Rust facade for Tauri command functions and related DTOs:
  - command-family re-exports: `40-153`
- internal crate-visible helper re-exports used across sibling modules:
  - progress sinks: `112`
  - report helpers: `133-135`
  - validation helper: `141-142`
- the public Rust facade for headless/report helpers used outside the Tauri invoke surface:
  - `build_report_xlsx_bytes_headless`: `132`
  - `apply_cross_ticker_validation_double_ma_headless`: `143-145`
  - headless research campaign helpers: `53-60`
- the public type facade for a broad set of `ta_app_core` request/result types:
  - `155-169`

This file does not own:

- actual frontend/Tauri IPC registration
  - that lives in `src-tauri/src/lib.rs:82-192`
- frontend command-registry generation
  - that lives in `web/scripts/generate-tauri-commands.mjs:7-74`
- parity enforcement between backend handler names and frontend registry
  - that lives in `web/scripts/check-tauri-parity.mjs:13-75`
- literal invoke-name scanning in the web surface
  - that lives in `web/scripts/check-known-tauri-commands.mjs`

## Classification

- Public surface: yes
  - as a Rust facade for tests, examples, and helper binaries
  - only a subset is the real frontend invoke surface
- Hot path: no
  - this is assembly/wiring code
- Runtime-critical: moderate
  - bad wiring here can hide or mis-shape backend entrypoints
- CUDA/runtime-critical: indirect only
  - CUDA-sensitive behavior lives behind the re-exported command implementations, not in this file
- Strategy-local: no
- Low-risk support code: yes, but contract-sensitive

## Direct Interactions

### Backend interaction map

- Tauri app bootstrap imports this module wholesale:
  - `use commands::*;`: `src-tauri/src/lib.rs:8`
- actual invoke exposure is determined only by `generate_handler!`:
  - `src-tauri/src/lib.rs:82-192`
- internal sibling modules also route through this facade for shared helpers:
  - `src-tauri/src/commands_options_expression.rs:1`
  - `src-tauri/src/commands_meta.rs:3123`
  - `src-tauri/src/commands_reports.rs:3141`
  - `src-tauri/src/commands_reports.rs:3813`
- external Rust consumers use `ta_desktop_demo_app::commands` as a convenience surface:
  - `src-tauri/src/bin/ta_headless_validate.rs:14-15`
  - `src-tauri/src/bin/ta_campaign_perf.rs:10-14`
  - `src-tauri/src/bin/ta_options_parent_artifact.rs:12-13`
  - `src-tauri/tests/command_runtime_wiring.rs:5-8`
  - `src-tauri/tests/validation_xlsx_report_tests.rs:3`
  - `src-tauri/examples/generate_validation_report_preview.rs:5`

### Frontend contract anchors

- generated frontend command registry:
  - `web/src/tauri/commands.ts:1-119`
- generator reads only `lib.rs`:
  - `web/scripts/generate-tauri-commands.mjs:46-71`
- frontend/backend command-name parity script:
  - `web/scripts/check-tauri-parity.mjs:41-75`
- literal command-name scanner:
  - `web/scripts/check-known-tauri-commands.mjs:57-87`
- registry and wrapper smoke tests:
  - `web/src/tauri/commands.test.ts:4-27`
  - `web/src/tauri/invoke.contract.test.ts:23-65`
- frontend runtime notes explicitly document the generator/parity contract:
  - `web/README.md:36-37`

## Standards And Research Log

This packet was written after targeted external review focused on Rust re-export behavior, rustdoc implications of re-export facades, and Tauri's actual command-registration model.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file exists specifically to support commands defined outside `lib.rs`
   - Standard informed:
     - commands in separate modules should be `pub`
     - the real invoke surface is the command list passed to `generate_handler!`
     - frontend command names remain unscoped even when Rust uses `commands::...`

2. Tauri runtime authority
   - Source: <https://v2.tauri.app/security/runtime-authority/>
   - Why it was consulted:
     - this file looks like a frontend-exposure index, but actual command invocation is mediated later by Tauri runtime registration/authority
   - Standard informed:
     - exposure to the webview is a runtime contract, not just a Rust re-export decision

3. Rust Reference: `use` declarations
   - Source: <https://doc.rust-lang.org/stable/reference/items/use-declarations.html>
   - Why it was consulted:
     - this file is mostly `pub use` / `pub(crate) use`
   - Standard informed:
     - public `use` items re-export names and therefore define public API shape

4. The rustdoc book: re-exports
   - Source: <https://doc.rust-lang.org/rustdoc/write-documentation/re-exports.html>
   - Why it was consulted:
     - re-export-heavy modules influence generated documentation shape and can hide the distinction between facade and canonical definition path
   - Standard informed:
     - a facade like this should be documented intentionally if it is meant to be consumed directly

## Findings

## Correctness And Safety

### 1. No file-local logic bug stands out here, but the file is easy to misread as the canonical frontend command list when it is not

- Evidence:
  - `commands.rs` re-exports command functions broadly: `40-153`
  - `lib.rs` is the actual handler-registration source: `src-tauri/src/lib.rs:82-192`
  - the frontend generator reads only `lib.rs`: `web/scripts/generate-tauri-commands.mjs:49-66`
- Why it matters:
  - a maintainer can add or rename a re-export here and still fail to expose the command to Tauri/frontend consumers
  - the file is not wrong, but its role is easy to misunderstand without reading the adjacent wiring files
- Review judgment:
  - this is a contract-truthfulness issue, not a runtime bug

## API And Naming

### 1. `ta_desktop_demo_app::commands` mixes multiple surfaces in one flat namespace

- Evidence:
  - command re-exports: `40-153`
  - internal crate-visible helpers: `112`, `133-145`
  - `ta_app_core` type re-exports: `155-169`
  - external bins/examples import this namespace directly:
    - `src-tauri/src/bin/ta_headless_validate.rs:14-15`
    - `src-tauri/src/bin/ta_campaign_perf.rs:10-14`
    - `src-tauri/examples/generate_validation_report_preview.rs:5`
- Why it matters:
  - the flat facade makes it hard to tell which names are:
    - frontend-invokable Tauri commands
    - Rust-only headless helpers
    - convenience DTO exports
    - internal support plumbing
- Smallest justified fix:
  - add module-level docs or section comments describing the groups and the source of truth for frontend exposure

### 2. The `commands` facade is broader than the frontend contract, but that distinction is not documented locally

- Evidence:
  - frontend registry is generated from `generate_handler!`, not from `commands.rs`:
    - `web/scripts/generate-tauri-commands.mjs:49-66`
  - the registry tests validate names only:
    - `web/src/tauri/commands.test.ts:4-27`
    - `web/src/tauri/invoke.contract.test.ts:35-65`
- Why it matters:
  - the name `commands.rs` suggests "this is the command surface"
  - in practice it is also a Rust facade for non-IPC consumers
- Review judgment:
  - low-severity naming/contract ambiguity, but worth documenting

## Error Handling

### 1. No independent error-handling logic lives in this file

- Evidence:
  - the file contains no command implementations, only module declarations and re-exports
- Why it matters:
  - there is no file-local runtime error-handling defect to fix here
  - the useful review focus is surface clarity and wiring verification

## Module Boundaries And Technical Debt

### 1. Internal modules depend on the facade path, so this file is more than a thin public assembly module

- Evidence:
  - sibling-module helper imports route through `crate::commands::...`:
    - `src-tauri/src/commands_options_expression.rs:1`
    - `src-tauri/src/commands_meta.rs:3123`
    - `src-tauri/src/commands_reports.rs:3141`
  - `apply_cross_ticker_validation` is re-exported crate-wide with a warning suppression:
    - `src-tauri/src/commands.rs:141-142`
  - one use site is test-oriented support wiring:
    - `src-tauri/src/commands_reports.rs:3813`
- Why it matters:
  - refactoring the facade can break internal sibling modules even when the external Tauri contract is unchanged
  - this increases the blast radius of what looks like a simple public re-export file
- Smallest justified fix:
  - prefer direct sibling-module imports for internal consumers where practical
  - if that is deferred, document that `commands.rs` is also an internal dependency hub

### 2. Frontend drift tooling does not validate most of this file

- Evidence:
  - `check:commands` and parity tooling parse `lib.rs`, not `commands.rs`:
    - `web/scripts/generate-tauri-commands.mjs:49-66`
    - `web/scripts/check-tauri-parity.mjs:44-72`
- Why it matters:
  - the web gates protect handler-name parity
  - they do not protect:
    - the broader Rust facade
    - the `ta_app_core` type-export block
    - internal `pub(crate)` helper re-exports
- Review judgment:
  - this is the main technical-debt point in the file

## Docs And Examples

### 1. The module needs an explicit contract comment describing its three roles

- Missing documentation topics:
  - this file is both a Rust facade and a child-module assembly point
  - frontend invoke exposure is controlled by `lib.rs generate_handler!`, not by re-export alone
  - some exports are intentionally public for headless binaries/examples/tests
  - some exports are only `pub(crate)` support plumbing

### 2. The existing examples and helper binaries prove this facade is intentionally used, but that intent is not documented here

- Evidence:
  - examples and helper binaries import through `ta_desktop_demo_app::commands`
- Why it matters:
  - without docs, future cleanup passes can mistake these exports for accidental leakage

## Tests And Missing Verification

### 1. Frontend command-name drift protection is strong and currently green

- Verified in this review pass:
  - `npm --prefix ./web run check:commands`
    - passed, reporting `109` commands
  - `node ./scripts/check-tauri-parity.mjs` from `web/`
    - passed, reporting `109` backend commands and `109` frontend registry commands
  - `node ./scripts/check-known-tauri-commands.mjs` from `web/`
    - passed
  - `npm --prefix ./web run test -- src/tauri/commands.test.ts src/tauri/invoke.contract.test.ts`
    - passed `6` tests

### 2. Rust-side coverage for the facade itself is thin

- Direct Rust facade coverage found:
  - `src-tauri/tests/command_runtime_wiring.rs:5-40`
    - only covers three cancel commands
  - `src-tauri/tests/validation_xlsx_report_tests.rs:3`
    - exercises `build_report_xlsx_bytes_headless`
- Missing verification:
  - no smoke test locks the broader public `ta_desktop_demo_app::commands` facade
  - no check asserts that key headless exports remain available
  - no compile-time or snapshot-style test distinguishes intentional public exports from incidental ones

### 3. The broad `commands::` Rust test lane is not green, but the failure is downstream, not in this file

- Evidence:
  - `cargo test -p ta_desktop_demo_app commands:: -- --nocapture`
    - failed with `272 passed`, `1 failed`
  - failing test:
    - `commands::commands_validation::tests::cross_ticker_headless_double_ma_recomputes_run_verdict_after_filtering`
- Why it matters:
  - `commands.rs` has no isolated direct test lane of its own
  - the broad aggregate lane is useful, but not an assembly-specific verification surface

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance issue lives in this file

- this module performs no heavy runtime work
- performance-sensitive behavior belongs to the re-exported implementation modules, not the facade

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/lib.rs`
- `web/scripts/generate-tauri-commands.mjs`
- `web/scripts/check-tauri-parity.mjs`
- `web/src/tauri/commands.ts`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports.rs`
- helper binaries under `src-tauri/src/bin/`

### Downstream consumers needing verification

- frontend typed invoke wrappers and command allowlist
- helper binaries that import `ta_desktop_demo_app::commands::*`
- tests/examples using headless report helpers

## Code-Quality Improvement Register

### Safe now

- add a short module-level doc comment describing:
  - facade purpose
  - frontend-source-of-truth location
  - which exports are Rust-only helpers
- add a small Rust facade smoke test for a representative set of public exports

### Coordination required

- splitting the facade into `commands`, `headless`, and `types` namespaces
- moving internal sibling imports away from `crate::commands::...`

### Defer until later pass

- any broad public-API reshaping that would touch helper binaries, examples, or existing tests

## Technical-Debt Reduction Register

### Debt safe to reduce now

- missing module documentation
- missing Rust-facade contract smoke coverage

### Debt that spans multiple files

- dependence on `lib.rs` as the real Tauri source of truth while `commands.rs` acts as the public Rust facade
- internal sibling reliance on facade re-exports

### Debt that should not be “cleaned up” casually

- public headless helper exports that are used by examples/tests/helper binaries
- `ta_app_core` type re-exports if the workspace intentionally treats this crate as the stable app-layer facade

## Performance Opportunity Register

- Safe structural opportunities:
  - none important
- Benchmark-sensitive opportunities:
  - none
- Not the right place to chase performance:
  - yes

## Observability And Diagnostics Gaps

- there is no single local comment in `commands.rs` stating that frontend exposure is driven by `generate_handler!`
- there is no Rust-side export-facade test analogous to the frontend command-registry checks

## Open Questions And Assumptions

- assumption:
  - `ta_desktop_demo_app::commands` is intentionally a public convenience facade for workspace-local binaries/examples/tests
- open question:
  - whether the crate wants to keep that facade stable, or eventually narrow it and move consumers toward more explicit modules/crates

## Regression Risk And Feature-Preservation Constraints

- Do not remove or rename public helper exports casually; several helper binaries, examples, and tests already depend on this facade.
- Do not treat `commands.rs` as the only command-registration source of truth; frontend/Tauri invoke availability is still controlled by `src-tauri/src/lib.rs`.
- Do not collapse the flat namespace without verifying helper binaries and any workspace-local import sites.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`
2. `cargo test -p ta_desktop_demo_app commands:: -- --nocapture`
   - note: currently red due to an existing downstream `commands_validation` failure, not because of `commands.rs` itself
3. `npm --prefix ./web run check:commands`
4. `cd web; node ./scripts/check-tauri-parity.mjs`
5. `cd web; node ./scripts/check-known-tauri-commands.mjs`
6. `npm --prefix ./web run test -- src/tauri/commands.test.ts src/tauri/invoke.contract.test.ts`

## Recommended Action

`Small surgical refactor`

The useful next step is not a structural rewrite. The useful step is to make the facade contract explicit and add a small Rust-side smoke check for representative exports. Broader namespace splitting or import-path cleanup can wait until there is a deliberate app-layer API pass.
