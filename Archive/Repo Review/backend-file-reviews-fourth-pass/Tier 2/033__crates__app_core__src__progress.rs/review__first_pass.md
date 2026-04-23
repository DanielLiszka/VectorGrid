# Review Packet: `crates/app_core/src/progress.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/progress.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Structurally sound and small, but not fully public-review ready because it is a shared public coordination contract with weakly typed phase vocabulary, best-effort delivery semantics, and almost no inline documentation explaining thread-safety, ordering, or drop behavior`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Cross-cutting-support`, `Runtime-adjacent`, `Semantics-sensitive`, `Low-compute`

## What This File Owns

`progress.rs` owns the shared backend progress-reporting contract used by optimization, validation, and campaign orchestration code.

It owns:

- the serialized progress payload:
  - `DoubleMaProgressPayload`: `crates/app_core/src/progress.rs:4-9`
- the VRAM prefetch feedback enum:
  - `DoubleMaCampaignVramPrefetchFeedback`: `11-17`
- the sink trait:
  - `ProgressSink`: `19-33`
- the optional fan-out helper:
  - `emit_double_ma_progress`: `35-46`

It does not own:

- Tauri event emission:
  - `src-tauri/src/commands_optimize.rs`
  - `src-tauri/src/commands_campaign.rs`
- optimization or validation progress scheduling:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/validation_pipeline.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
- GPU VRAM prefetch execution:
  - `crates/app_core/src/gpu_backtest_kernel.rs`

## Classification

- Public surface: yes
  - the module is exported from `crates/app_core/src/lib.rs:56`
- Hot path: indirectly
  - the file itself is tiny, but its sink is called from hot optimization/validation loops
- CUDA/runtime-critical: runtime-adjacent only
  - the VRAM-prefetch extension point participates in campaign staging
- Strategy-local: no
  - this is shared infrastructure reused across several backend families
- Low-risk support code: no
  - implementation is tiny, but contract changes here have broad downstream blast radius

## Direct Interactions

### Backend interaction map

- Tauri progress sinks:
  - `src-tauri/src/commands_optimize.rs:28-52`
  - `src-tauri/src/commands_campaign.rs:512-545`
- GPU campaign prefetch path:
  - `crates/app_core/src/gpu_backtest_kernel.rs:12074-12167`
- Double-MA progress forwarding and parity tests:
  - `crates/app_core/src/double_ma.rs:5519-5531`
  - `16344-16439`
- validation progress forwarding and monotonicity tests:
  - `crates/app_core/src/validation_pipeline.rs:585-610`
  - `1376-1411`
  - `crates/app_core/src/validation_core.rs:167-182`
  - `277-335`
- composite and meta-select callers reusing the same contract:
  - `crates/app_core/src/composite_cpu.rs:12303-12308`
  - `crates/app_core/src/composite_mo.rs:28-41`
  - `crates/app_core/src/meta_select.rs:788-795`

### Downstream verification impact if changed

- changing payload fields or phase semantics affects frontend event consumers immediately
- changing sink semantics affects every optimization or validation path that reports progress
- changing the prefetch callback contract affects campaign GPU staging and warning/fatal feedback handling

## Standards And Research Log

This packet was written after targeted web research focused on public trait-object design, thread-safety marker semantics, documentation obligations for public trait contracts, and Tauri event payload expectations.

### Sources consulted

1. Rust API Guidelines flexibility guidance, `C-OBJECT`
   - Source: <https://rust-lang.github.io/api-guidelines/flexibility.html#traits-are-object-safe-if-they-may-be-useful-as-a-trait-object-c-object>
   - Why it was consulted:
     - this file exposes `Option<&dyn ProgressSink>` as the central calling pattern
   - Standard informed:
     - trait-object-based public traits should keep object-safety and behavior expectations explicit

2. Rust API Guidelines documentation guidance, `C-FAILURE`
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why it was consulted:
     - delivery semantics here are best-effort and non-fallible at the API boundary
   - Standard informed:
     - public trait and function behavior should document error/drop semantics when they matter operationally

3. `std::marker::Send`
   - Source: <https://doc.rust-lang.org/std/marker/trait.Send.html>
   - Why it was consulted:
     - `ProgressSink` requires `Send`
   - Standard informed:
     - implementors are explicitly allowed to cross thread boundaries

4. `std::marker::Sync`
   - Source: <https://doc.rust-lang.org/std/marker/trait.Sync.html>
   - Why it was consulted:
     - `ProgressSink` also requires `Sync`
   - Standard informed:
     - shared references to sink implementations must be safe across threads, which pushes synchronization obligations onto implementors

5. Tauri “Calling the Frontend from Rust”
   - Source: <https://v2.tauri.app/develop/calling-frontend/>
   - Why it was consulted:
     - downstream sinks emit this payload through Tauri events
   - Standard informed:
     - event payloads should be serializable and clone-friendly, and event-delivery semantics should be treated deliberately

## Findings

## Correctness And Safety

### 1. Thread-safety is required, but the contract does not document ordering or blocking expectations

- Evidence:
  - `ProgressSink` is `Send + Sync`:
    - `crates/app_core/src/progress.rs:19`
  - all methods take `&self`, so implementations must manage their own synchronization:
    - `19-33`
  - the validation test sink uses `Mutex<Vec<_>>` explicitly:
    - `crates/app_core/src/validation_core.rs:167-182`
- Why this matters:
  - the trait is designed for shared cross-thread use, but callers and implementors are not told whether ordering, reentrancy, or nonblocking behavior are expected
- Smallest justified fix:
  - add rustdoc clarifying that implementations must be thread-safe and should avoid blocking hot loops

## API And Naming

### 2. The public progress contract is stringly typed in the places that matter most

- Evidence:
  - `DoubleMaProgressPayload.phase` is a raw `&'static str`:
    - `crates/app_core/src/progress.rs:5-8`
  - the helper accepts `phase: &'static str` directly:
    - `35-39`
  - the VRAM-prefetch request hook forwards raw string identifiers:
    - `22-24`
- Why this matters:
  - the contract is lightweight, but there is no compile-time phase vocabulary or typed request object for the prefetch hook
  - downstream consumers learn semantics through convention, not types
- Smallest justified fix:
  - add a small phase enum or named constants for known phase labels
  - consider a typed prefetch request struct if that hook needs to evolve

### 3. Public naming is overly Double-MA-specific for a cross-cutting contract

- Evidence:
  - every exported name is `DoubleMa*`:
    - `4-20`
  - the same contract is reused by validation, composite, meta-select, and campaign flows:
    - `crates/app_core/src/validation_pipeline.rs:585-610`
    - `crates/app_core/src/composite_cpu.rs:12303-12308`
    - `crates/app_core/src/composite_mo.rs:28-41`
    - `crates/app_core/src/meta_select.rs:788-795`
- Why this matters:
  - the naming is operationally workable, but it weakens the semantic clarity of a public cross-cutting API
  - consumers must understand that `"double_ma_progress"` also carries progress for non-Double-MA workflows
- Smallest justified fix:
  - document the historical naming explicitly
  - defer renaming unless the repo is ready to update all event consumers together

## Error Handling

### 4. Delivery semantics are intentionally lossy and currently undocumented

- Evidence:
  - helper drops progress events silently when there is no sink:
    - `crates/app_core/src/progress.rs:40`
  - the trait surface returns no `Result`
  - downstream Tauri sinks ignore emission failures:
    - `src-tauri/src/commands_optimize.rs:32-34`
    - `src-tauri/src/commands_campaign.rs:525-527`
- Why this matters:
  - this is a valid “best effort” design, but callers cannot tell whether lost events are acceptable, observable, or retryable
- Smallest justified fix:
  - document the contract as best-effort and nonblocking
  - avoid turning this into fallible reporting unless there is a demonstrated operational need

## Module Boundaries And Technical Debt

### 5. The VRAM-prefetch extension point is tightly coupled to runtime internals

- Evidence:
  - the hook exposes `&Candles` directly:
    - `crates/app_core/src/progress.rs:2`
    - `22-24`
  - the main consumer is the GPU campaign prefetch path:
    - `crates/app_core/src/gpu_backtest_kernel.rs:12074-12167`
- Why this matters:
  - the sink trait now couples shared progress infrastructure to a concrete data type and a positional callback shape
  - that makes future evolution harder than a typed request payload would
- Smallest justified fix:
  - keep the hook as-is for now, but document it as a specialized extension point and consider a small request object if the hook grows

## Docs And Examples

### 6. The file exposes a public coordination contract with no rustdoc and no examples

- Evidence:
  - the public surface spans `4-46`
  - there are no rustdoc comments in the file
- Why this matters:
  - users of `ProgressSink` have no local documentation for:
    - ordering expectations
    - drop semantics
    - phase vocabulary
    - whether implementations may block
    - what the prefetch hook is allowed to do
- Smallest justified fix:
  - add rustdoc to `DoubleMaProgressPayload`, `DoubleMaCampaignVramPrefetchFeedback`, `ProgressSink`, and `emit_double_ma_progress`

## Tests And Missing Verification

### 7. There are no file-local tests even though the contract is reused widely

- Evidence:
  - `crates/app_core/src/progress.rs` has no local test module
  - meaningful indirect coverage exists in:
    - `crates/app_core/src/validation_core.rs:167-182`
    - `277-335`
    - `crates/app_core/src/double_ma.rs:16344-16439`
    - `src-tauri/src/commands_campaign.rs:4152-4182`
- Why this matters:
  - behavior is currently specified by downstream consumers rather than by the contract module itself
- Smallest justified fix:
  - if the contract evolves, add one small file-local sink stub test module to lock in helper/drop semantics

## Performance, Allocation, And CUDA Concerns

### 8. The file is low-compute, but sink behavior can still perturb hot loops

- Evidence:
  - the helper is tiny and allocation-free:
    - `35-46`
  - however, sinks are called from hot optimization and validation loops:
    - `crates/app_core/src/validation_pipeline.rs:610`
    - `crates/app_core/src/double_ma.rs:8174`
    - `crates/app_core/src/composite_cpu.rs:12308`
- Why this matters:
  - any change that adds allocation, blocking, or richer synchronization inside the sink path can become user-visible
- Safe-now conclusion:
  - keep the helper simple and nonblocking
- Benchmark-sensitive conclusion:
  - avoid making progress emission materially heavier without measurement

## Code-Quality Improvement Register

- Safe now:
  - document best-effort delivery semantics
  - document thread-safety and nonblocking expectations for sink implementations
  - introduce named phase constants or an enum if the vocabulary is stable
- Defer:
  - broad public renaming away from `DoubleMa*`
  - reshaping the prefetch hook unless there is a concrete next feature that needs it

## Technical-Debt Reduction Register

- Safe now:
  - add local rustdoc
  - clarify the specialized purpose of the prefetch hook
- Defer:
  - renaming public events or trait methods that many downstream consumers already rely on
- Do not casually remove:
  - the best-effort optional sink design
  - the trait-object-friendly shape of `ProgressSink`

## Performance Opportunity Register

- Safe structural work:
  - none beyond preserving low-overhead semantics
- Throughput-sensitive:
  - any change that adds dynamic allocation, locking, or fallible retry logic in the hot emission path

## Recommended Action

`Small surgical refactor`

The implementation is fine. The contract needs hardening:

- document it
- make the “best effort” semantics explicit
- reduce stringly vocabulary where it is cheap to do so

## Exact Local Verification Required If Changed

1. Double-MA progress parity:
   - `cargo test -p ta_app_core cpu_dynamic_strategy_progress_parity_matches_legacy_results -- --nocapture`
2. Validation progress monotonicity:
   - `cargo test -p ta_app_core shared_validation_core_progress_phase_names_and_monotonic_counts -- --nocapture`
3. Campaign prefetch-config coverage:
   - `cargo test -p ta_desktop_demo_app --lib commands::commands_campaign::tests::doublema_vram_prefetch_config_accepts_exact_kernel_and_learned_auto -- --nocapture`
   - `cargo test -p ta_desktop_demo_app --lib commands::commands_campaign::tests::doublema_vram_prefetch_config_rejects_non_plain_shapes -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core cpu_dynamic_strategy_progress_parity_matches_legacy_results -- --nocapture`
  - `cargo test -p ta_app_core shared_validation_core_progress_phase_names_and_monotonic_counts -- --nocapture`
  - `cargo test -p ta_desktop_demo_app --lib commands::commands_campaign::tests::doublema_vram_prefetch_config_accepts_exact_kernel_and_learned_auto -- --nocapture`
  - `cargo test -p ta_desktop_demo_app --lib commands::commands_campaign::tests::doublema_vram_prefetch_config_rejects_non_plain_shapes -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change phase names or payload fields casually; frontend listeners and downstream tests already assume them
- Do not make sink emission blocking or fallible without explicit product need
- Do not rename the prefetch hook without updating GPU campaign staging and command-layer adapters together
