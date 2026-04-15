# Review Packet: `src-tauri/src/settings.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/settings.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small and mostly clean settings helper, but still carrying a couple of implicit contracts around path validation and write durability`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Persistence-sensitive`, `Path-policy-sensitive`, `Market-data-root-sensitive`

## What This File Owns

`settings.rs` owns the desktop app's persisted settings for market-data root resolution and the helper used to move cache directories when that root changes.

This file owns:

- the persisted settings DTO:
  - `AppSettings`: `src-tauri/src/settings.rs:7-13`
- settings file path/load/save helpers:
  - `settings_path`: `15-17`
  - `load_settings`: `19-27`
  - `save_settings`: `29-33`
- configured and effective market-data root resolution:
  - `configured_market_data_root_dir`: `35-56`
  - `resolve_market_data_root_dir`: `58-63`
  - `set_market_data_root_dir`: `65-73`
- recursive move support used by command-layer market-data-root migration:
  - `copy_dir_all`: `75-89`
  - `move_dir_all`: `91-105`

This file does not own:

- the Tauri command surface for changing the market-data directory
  - that lives in `src-tauri/src/commands_data.rs:465-533`
- the actual Polygon cache subtree logic
  - that lives in `src-tauri/src/polygon.rs`

## Classification

- Public surface: internal/backend-only, but broadly reused
- Hot path: no
- Runtime-critical: moderate
  - bad behavior here can redirect or break all market-data-dependent flows
- CUDA/runtime-critical: no
- Strategy-local: no
- Low-risk support code: yes, but persistence-sensitive

## Direct Interactions

### Backend interaction map

- command-layer market-data-root resolution:
  - `src-tauri/src/commands_data.rs:452-459`
- command-layer clearing and setting of the custom market-data root:
  - `src-tauri/src/commands_data.rs:476-531`
- broad downstream consumers resolve the effective market-data root through this file before touching Polygon or options data:
  - `src-tauri/src/commands_data.rs:547-565`, `1018`, `1128`, `1141`, `1191`, `1244`
  - `src-tauri/src/commands_options_expression.rs:655-664`
  - `src-tauri/src/commands_options_data.rs:352`, `831`

### Test anchors

- dedicated settings integration tests:
  - `src-tauri/tests/settings_tests.rs:40-134`

## Standards And Research Log

This packet was written after targeted external review focused on persisted-path validation, write replacement semantics, and serde omission behavior for small settings files.

### Sources consulted

1. `std::fs::rename`
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why it was consulted:
     - this file falls back from rename to copy/delete when moving cache roots
   - Standard informed:
     - rename failure needs a deliberate fallback policy because cross-volume moves behave differently from same-volume renames

2. `std::fs::copy`
   - Source: <https://doc.rust-lang.org/std/fs/fn.copy.html>
   - Why it was consulted:
     - the fallback move implementation recursively copies files
   - Standard informed:
     - copy-based fallback is a pragmatic compatibility path, but it is not the same contract as atomic move

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - the settings DTO uses optional omission semantics for persisted configuration
   - Standard informed:
     - omitted fields should be intentional and compatible with stable defaults

4. `Path::is_absolute`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html#method.is_absolute>
   - Why it was consulted:
     - this file's main configuration invariant is that the optional override must be an absolute path
   - Standard informed:
     - path-policy validation should be explicit and ideally enforced at the API boundary where values are accepted

## Findings

## Correctness And Safety

### 1. `set_market_data_root_dir` trusts caller-side validation instead of enforcing the path contract itself

- Evidence:
  - setter persists whatever path is provided:
    - `src-tauri/src/settings.rs:65-73`
  - reader later rejects non-absolute paths:
    - `35-56`
  - the current command-layer caller does validate absolute/directories before calling:
    - `src-tauri/src/commands_data.rs:481-500`
- Why it matters:
  - the API contract is split between caller and callee
  - any future direct caller can persist an invalid relative path successfully and only discover the problem later on read
- Smallest justified fix:
  - either validate absoluteness inside `set_market_data_root_dir`
  - or make the function's caller-prevalidated contract explicit in docs/name/visibility

## API And Naming

### 1. The small API is coherent, but the distinction between "configured" and "resolved" roots is important enough to document locally

- Evidence:
  - `configured_market_data_root_dir` returns only the explicit override if valid: `35-56`
  - `resolve_market_data_root_dir` falls back to `app_local_data_dir`: `58-63`
- Why it matters:
  - downstream code often needs the difference between:
    - what the user configured
    - what the app will actually use
- Smallest justified fix:
  - add a short module comment documenting that distinction, since it drives UI reporting in `commands_data.rs:452-459`

## Error Handling

### 1. Settings writes are simple but not replacement-safe

- Evidence:
  - `save_settings` uses `fs::write` directly: `29-33`
- Why it matters:
  - this is only one small JSON file, so the risk is low
  - but a partial or interrupted write can still corrupt the persisted market-data-root override
- Smallest justified fix:
  - use temp-file-plus-replace if the project wants stronger durability guarantees for all persisted control files
- Review judgment:
  - low severity, but objective persistence debt

## Module Boundaries And Technical Debt

### 1. `move_dir_all` is intentionally a best-effort compatibility move, not a strict filesystem move

- Evidence:
  - fast path uses `fs::rename`: `97-99`
  - fallback copies recursively and then deletes source: `100-104`
- Why it matters:
  - this is the right broad behavior for cross-volume moves
  - it also means the function is not atomic and does not preserve every filesystem property a native move might preserve
- Review judgment:
  - not a bug, but the function should be understood as "portable data migration" rather than "strict move"
- Smallest justified fix:
  - add a short comment stating that the fallback is copy-then-delete compatibility behavior

## Docs And Examples

### 1. The absolute-path requirement is only partially documented today

- Missing documentation topics:
  - absolute path requirement for overrides
  - difference between configured vs effective root
  - copy-then-delete fallback semantics in `move_dir_all`

## Tests And Missing Verification

### 1. Current coverage is strong for a small support module and currently green

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app --test settings_tests -- --nocapture`
    - passed `7` tests
- Existing strengths:
  - default fallback resolution: `40-48`
  - persisted custom root round-trip: `50-65`
  - clearing behavior: `67-81`
  - rejecting relative configured paths: `83-93`
  - recursive move behavior: `95-107`
  - malformed JSON surfacing: `109-123`
  - save failure when base path is a file: `125-133`

### 2. A small but real direct-API coverage gap remains

- Missing tests found in this review pass:
  - no test that `set_market_data_root_dir` rejects a relative path if called directly
  - no test documenting that `move_dir_all` is allowed to fall back to copy/delete semantics

### 3. If this file changes, verification should stay local but include command-path sanity

- Exact local verification required if changed:
  - `cargo test -p ta_desktop_demo_app --test settings_tests -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance work is justified here

- Evidence:
  - this is a tiny settings/persistence helper
- Review judgment:
  - correctness and durability matter more than any allocation cleanup

## Code-Quality Improvement Register

- Safe now:
  - validate absolute paths inside `set_market_data_root_dir`
  - add a persistence comment or helper if atomic settings writes are desired repo-wide
  - document `move_dir_all` fallback semantics
- Defer:
  - any broader settings-schema expansion until there is more than one or two persisted fields

## Technical-Debt Reduction Register

- Safe to reduce now:
  - caller/callee split on path validation
  - implicit semantics of copy-delete fallback
- Useful debt that should not be "cleaned up" casually:
  - keeping the settings file intentionally tiny and human-readable

## Performance Opportunity Register

- Safe structural improvement:
  - none urgent
- Benchmark-sensitive or semantics-sensitive:
  - none

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - preserve the current default-to-`app_local_data_dir` behavior when unset
  - preserve cross-volume move support
- Regression risk if changed carelessly:
  - market-data-root overrides can become unreadable or unusable
  - data migration during root changes can behave differently across filesystems
- Safe change posture:
  - prefer local validation and documentation hardening over broader redesign
