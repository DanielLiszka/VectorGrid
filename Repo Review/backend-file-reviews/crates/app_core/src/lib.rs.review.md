# Review Packet: `crates/app_core/src/lib.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/lib.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Functionally acceptable as the crate facade, but not fully public-review ready because it lacks crate-level rustdoc, exposes a very broad and only partially curated public surface, and does not explain how CPU-only versus CUDA-gated APIs are intended to be consumed`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Facade-critical`, `Feature-gated`, `Semantics-sensitive`, `Compatibility-sensitive`

## What This File Owns

`lib.rs` is the crate-root facade for `ta_app_core`.

It owns:

- module declarations for the backend implementation tree:
  - `crates/app_core/src/lib.rs:1-88`
- the public export boundary for strategy, optimization, validation, dynamic-indicator, options, ML, and GPU/runtime APIs:
  - `89-296`
- the CPU-only versus CUDA-gated crate surface:
  - module-level `#[cfg(...)]` declarations:
    - `3-4`
    - `6-7`
    - `9-10`
    - `12-13`
    - `29-30`
    - `35-39`
    - `44-45`
    - `51-52`
    - `59-60`
    - `62-63`
    - `79-88`
  - export-level `#[cfg(...)]` re-exports:
    - `99-100`
    - `169-173`
    - `288-296`
- cross-crate public re-exports from `ta_strategies`:
  - `89-92`

This file does not own:

- algorithmic implementation details for any strategy or runtime path
- validation or optimization semantics
- CUDA kernel code

Its job is not execution. Its job is to define what downstream callers are allowed to see and depend on.

## Classification

- Public surface: yes
  - this is the primary public contract for `ta_app_core`
- Hot path: no
  - it is not an execution-time performance path
- CUDA/runtime-critical: indirectly yes
  - it controls exposure of GPU/runtime modules and feature-gated APIs, but it is not itself a runtime hot path
- Strategy-local: no
  - this is a cross-cutting crate facade
- Low-risk support code: no
  - it is support/facade code, but it has high compatibility blast radius

## Direct Interactions

### Backend interaction map

- binds the crate to almost every backend implementation module:
  - examples:
    - `double_ma`: `20`
    - `composite_cpu`: `11`
    - `composite_validate`: `16`
    - `double_ma_validate`: `26`
    - `iterative_sim`: `33`
    - `indicator_catalog`: `31`
    - `strategy_ir`: `68`
    - `strategy_gpu_runtime`: `67`
- re-exports public request/result/config types consumed by the Tauri layer:
  - `src-tauri/src/commands.rs:155`
  - `src-tauri/src/commands_campaign.rs:23`
  - `src-tauri/src/commands_dynamic.rs:2`
  - `src-tauri/src/commands_meta.rs:13`
  - `src-tauri/src/commands_ml_signal.rs:4`
  - `src-tauri/src/commands_options_data.rs:8`
  - `src-tauri/src/commands_options_expression.rs:16-20`
- supports nested-module imports that bypass the flat root re-export surface:
  - `ta_app_core::double_ma::GpuExecutionPolicy`:
    - `src-tauri/src/commands_campaign.rs:3773`
  - `ta_app_core::progress::ProgressSink`:
    - `src-tauri/src/commands_optimize.rs:32-49`
  - `ta_app_core::options_types::*`:
    - `src-tauri/src/commands_options_expression.rs:16`
    - plus many nested uses throughout that file
  - `ta_app_core::nvtx::scope`:
    - `src-tauri/src/commands_options_expression.rs:1431`
- bench and harness surfaces compile against this facade directly:
  - `crates/app_core/benches/double_ma_validation_workflow.rs:5`
  - `crates/app_core/benches/composite_validation_workflow.rs:5`
  - many additional benches and helper binaries under `crates/app_core/benches/**` and `src-tauri/src/bin/**`
- feature wiring comes from crate features:
  - `crates/app_core/Cargo.toml:6-9`

### Downstream verification impact if changed

- changing a root `pub use` can break:
  - Tauri command modules
  - benches
  - examples
  - helper binaries
- changing a `pub mod` visibility can break:
  - nested imports such as `ta_app_core::options_types`, `ta_app_core::progress`, `ta_app_core::double_ma`, `ta_app_core::composite_cpu`, and `ta_app_core::nvtx`
- changing feature-gated exports can break:
  - CUDA-enabled builds
  - CPU-only builds that assume additive feature behavior
  - downstream code that compiles under both feature sets

## Standards And Research Log

This packet was written after targeted web research focused on crate-root documentation expectations, public re-export semantics, visibility/privacy rules, and official Cargo guidance for feature-gated public APIs.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines the crate’s public API boundary
   - Standard informed:
     - public APIs should be intentional, documented, and minimize accidental compatibility promises

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - crate-root documentation is part of public API quality
   - Standard informed:
     - crate-level docs should orient users to the public surface and intended usage

3. Rust Reference, Use Declarations
   - Source: <https://doc.rust-lang.org/reference/items/use-declarations.html>
   - Why it was consulted:
     - this file relies heavily on `pub use` re-exports
   - Standard informed:
     - public `use` declarations are re-exports and therefore part of the public contract, even when they redirect to internal or cross-crate definitions

4. Rust Reference, Visibility And Privacy
   - Source: <https://doc.rust-lang.org/reference/visibility-and-privacy.html>
   - Why it was consulted:
     - this file mixes `pub mod` and `pub(crate) mod` across many modules
   - Standard informed:
     - visibility should be deliberate because private-by-default is the baseline and broader exposure creates broader maintenance burden

5. Cargo Book, Features
   - Source: <https://doc.rust-lang.org/cargo/reference/features.html>
   - Why it was consulted:
     - this file exposes materially different crate surfaces under `cuda` / `cuda-backtest-kernel`
   - Standard informed:
     - features should stay additive and should not casually introduce SemVer-incompatible or mutually hostile public API behavior

6. Rust Reference, Conditional Compilation
   - Source: <https://doc.rust-lang.org/reference/conditional-compilation.html>
   - Why it was consulted:
     - this file depends on `#[cfg(...)]` for module declarations and exports
   - Standard informed:
     - conditional compilation changes the compiled public surface and must therefore be treated as contract-shaping behavior, not just implementation detail

## Findings

## Correctness And Safety

### 1. No file-local runtime correctness bug is evident; the primary risk is contract drift

- Evidence:
  - this file contains module wiring and re-exports only:
    - `crates/app_core/src/lib.rs:1-296`
- Why it matters:
  - the risk profile here is not arithmetic or memory safety inside this file
  - the real risk is accidental API breakage, accidental public-surface growth, or undocumented feature-specific contract drift
- Review judgment:
  - no direct executable correctness defect identified in this file
  - public-contract stability is the main review concern

## API And Naming

### 1. The public surface is broader and less curated than the file structure suggests

- Evidence:
  - many modules are public rather than `pub(crate)`:
    - `crates/app_core/src/lib.rs:2`
    - `5`
    - `8`
    - `11`
    - `16-20`
    - `26`
    - `28`
    - `30-33`
    - `34`
    - `36-40`
    - `42-56`
    - `58`
    - `61`
    - `64-70`
    - `73`
    - `75-77`
    - `78`
    - `80-86`
  - several GPU/runtime-sensitive modules remain public:
    - `gpu_backtest_kernel`: `29-30`
    - `asr_gpu_kernel`: `3-4`
    - `kqsr_gpu_kernel`: `35-36`
    - `kvdf_gpu_kernel`: `38-39`
    - `mkcs_gpu_kernel`: `44-45`
    - `vcc_gpu_kernel`: `79-80`
    - `vcet_gpu_kernel`: `82-83`
    - `vemr_gpu_kernel`: `85-86`
- Why it matters:
  - every public module is a compatibility promise, whether or not the repo intended it that way
  - this is especially expensive when strategy-local GPU wrappers and runtime-heavy internals become part of the crate’s apparent supported surface
- Smallest justified fix:
  - do not broadly prune exports blindly
  - first document which modules are intentional public entry points versus incidental public exposure
  - then narrow only clearly internal modules after a consumer audit
- Review judgment:
  - high API-surface issue
  - semantics-sensitive because existing downstream code may depend on the current shape

### 2. The facade mixes flat root re-exports with still-public nested modules, which increases compatibility burden

- Evidence:
  - root-level `pub use` groups flatten many major APIs:
    - `103-118`
    - `131-142`
    - `144-158`
    - `160-180`
    - `182-244`
    - `246-296`
  - downstream callers also import nested public modules directly:
    - `ta_app_core::double_ma::GpuExecutionPolicy`:
      - `src-tauri/src/commands_campaign.rs:3773`
    - `ta_app_core::progress::ProgressSink`:
      - `src-tauri/src/commands_optimize.rs:32-49`
    - `ta_app_core::options_types::*`:
      - `src-tauri/src/commands_options_expression.rs:16`
    - `ta_app_core::nvtx::scope`:
      - `src-tauri/src/commands_options_expression.rs:1431`
    - bench imports of `ta_app_core::composite_cpu::*` and `ta_app_core::double_ma::*`:
      - `crates/app_core/benches/composite_validation_workflow.rs:47-54`
- Why it matters:
  - the crate is supporting two public styles at once:
    - curated root exports
    - direct module navigation
  - that makes later surface cleanup harder because consumers may rely on either path
- Smallest justified fix:
  - add crate-root documentation that explicitly distinguishes:
    - stable convenience exports
    - intentionally public nested modules
    - public-by-necessity internals that may later be tightened
- Review judgment:
  - medium-high API/facade-design issue
  - safe-now as documentation

### 3. Cross-crate re-exports amplify blast radius

- Evidence:
  - `ta_strategies` types are re-exported directly:
    - `crates/app_core/src/lib.rs:89-92`
- Why it matters:
  - a change in either `ta_app_core` export decisions or `ta_strategies` type placement can become a downstream break even when behavior is unchanged
- Smallest justified fix:
  - keep the re-exports if they are useful
  - document them as part of the supported facade instead of leaving them implicit
- Review judgment:
  - medium boundary/compatibility issue

## Error Handling

### 1. Error handling is not implemented here, but export shape still affects error-transport expectations

- Evidence:
  - this file re-exports many request/result APIs without any crate-root guidance on which surfaces are primary:
    - `103-296`
- Why it matters:
  - consumers need to know which entry points are intended to be stable when building command-layer error transport and higher-level orchestration
- Smallest justified fix:
  - document preferred public entry points rather than restructuring error types here
- Review judgment:
  - no direct file-local error-handling bug
  - documentation gap instead

## Module Boundaries And Technical Debt

### 1. The crate root currently behaves as both a public facade and an implementation directory index

- Evidence:
  - it begins with a raw module list:
    - `crates/app_core/src/lib.rs:1-88`
  - it then becomes a large flat export list:
    - `89-296`
  - there is no crate-level guidance separating:
    - facade entry points
    - advanced nested modules
    - internal-only modules
- Why it matters:
  - this structure is workable internally, but it shifts too much discovery burden onto downstream callers
  - it also makes future cleanup harder because “public for convenience” and “public by design” are not separated
- Smallest justified fix:
  - add a crate-root surface map
  - group exports by domain with short crate-doc sections
  - defer any broad structural redesign until consumer usage is inventoried
- Review judgment:
  - medium-high technical-debt issue
  - safe-now as documentation

### 2. Feature-gated exports create materially different crate surfaces that are not described at the crate root

- Evidence:
  - CUDA-gated modules:
    - `3-4`
    - `6-7`
    - `9-10`
    - `29-30`
    - `35-39`
    - `44-45`
    - `51-52`
    - `59-60`
    - `62-63`
    - `79-88`
  - CUDA-gated re-exports:
    - `99-100`
    - `169-173`
    - `288-296`
  - feature wiring:
    - `crates/app_core/Cargo.toml:6-9`
- Why it matters:
  - a consumer compiling CPU-only versus CUDA-enabled sees a different API surface
  - official Cargo guidance expects features to be additive and not casually hostile to compatibility
- Smallest justified fix:
  - add crate docs describing:
    - CPU-only baseline surface
    - CUDA-extended surface
    - which APIs require `cuda` versus `cuda-backtest-kernel`
- Review judgment:
  - medium contract/documentation issue
  - safe-now as documentation

## Docs And Examples

### 1. The crate root is missing crate-level rustdoc even though it defines the effective public contract

- Evidence:
  - the file starts immediately with module declarations:
    - `crates/app_core/src/lib.rs:1`
  - there is no `//!` crate header or public-surface guidance before the first item
- Why it matters:
  - for a facade file, missing crate-level docs is a real usability and maintainability gap
  - users cannot tell which APIs are recommended, advanced, internal-ish, or feature-gated
- Smallest justified fix:
  - add concise `//!` crate docs covering:
    - the main public workflows
    - how the crate is organized
    - which major surfaces are feature-gated
    - when to use root exports versus nested modules
- Review judgment:
  - high documentation issue
  - safe-now

### 2. Export groups would benefit from short orienting comments

- Evidence:
  - export blocks are large and domain-dense:
    - `103-118`
    - `131-142`
    - `150-180`
    - `182-244`
    - `250-296`
- Smallest justified fix:
  - add short comments or crate-doc subsections naming the export families:
    - optimization
    - validation
    - dynamic indicators
    - options
    - ML
    - GPU strategy runtime

## Tests And Missing Verification

### 1. Compile-only verification confirms the core crate surface, but full workspace verification is currently blocked by host/toolchain issues

- Evidence:
  - verification run:
    - `cargo test -p ta_app_core --lib --no-run`
      - passed
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel --lib --no-run`
      - passed
    - `cargo test --workspace --no-run`
      - failed outside this file due environment/toolchain issues in Tauri/bin compilation, including:
        - paging-file / metadata mapping failures for `ta_desktop_demo_app`
        - an ICE while compiling `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- Why it matters:
  - the crate-root facade is consumed broadly enough that true confidence after export changes still requires workspace-wide compile verification
  - however, the current workspace check is not a clean signal about `lib.rs` specifically
- Missing verification I would want before changing this file:
  - a successful workspace compile on a machine/session without the current paging-file and ICE issues
  - bench/example no-run compilation if root exports are renamed or narrowed
  - targeted compile checks for Tauri modules that use nested `ta_app_core::*` paths

## Performance, Allocation, And CUDA Concerns

### 1. This file is not a runtime hot path, so performance changes should not drive review decisions

- Evidence:
  - it contains only module declarations and re-exports:
    - `crates/app_core/src/lib.rs:1-296`
- Review judgment:
  - there is no direct runtime performance work to do here
  - do not justify risky public-surface changes as “cleanup for performance”

### 2. Feature-gated API shape still has CUDA-facing maintenance implications

- Evidence:
  - many GPU/runtime modules and exports are gated here:
    - `3-4`
    - `29-30`
    - `35-39`
    - `44-45`
    - `79-88`
    - `288-296`
- Review judgment:
  - preserve additive CPU/CUDA surface behavior
  - do not create mutually hostile feature shapes without explicit design intent

## Code-Quality Improvement Register

- Safe now:
  - add crate-level rustdoc
  - document export families and feature-gated surfaces
  - identify a shortlist of intentionally public nested modules
- Coordination required:
  - narrowing public modules
  - changing root export names
  - re-tiering nested-module versus root-export usage
- Defer until later pass:
  - any large redesign into multiple facades or prelude-style APIs

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing crate docs
  - undocumented feature-gated public surface
  - lack of public-surface map
- Coordination required:
  - visibility narrowing
  - cross-crate export reshaping
  - root-versus-nested API rationalization
- Useful debt that should not be “cleaned up” casually:
  - broad convenience exports that are actively consumed
  - intentionally public nested modules such as `options_types`, `progress`, `double_ma`, `composite_cpu`, and `nvtx` if downstream usage requires them

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent beyond documentation and export-family clarity
- Benchmark-sensitive candidates:
  - none in this file
- Not the right place to chase performance without evidence:
  - export pruning justified as runtime optimization

## Observability And Diagnostics Gaps

- the crate root does not document which exports are primary supported entry points
- the crate root does not document which nested modules are intentionally public
- the crate root does not document CPU-only versus CUDA-extended surface expectations

## Implementation Staging Guidance

- Safe now:
  - add `//!` crate docs
  - add short export-family comments
  - document feature-gated public surface expectations
- Coordination required:
  - narrowing public modules
  - renaming or removing root exports
  - reshaping nested-module access patterns
- Defer until later pass:
  - broader facade redesign

## Change Blast Radius

- Local only:
  - crate docs
  - export-family comments
- Same subsystem:
  - visibility changes can break Tauri commands, benches, examples, and helper binaries
- Cross-backend/runtime:
  - feature-gated export changes affect CPU-only and CUDA-enabled consumers across the repo

## Open Questions Or Assumptions

- assumption:
  - the broad current facade is partly intentional because many internal consumers rely on it
- open question:
  - which currently public nested modules are meant to be part of the long-term supported surface versus merely convenient today
- open question:
  - whether the project wants a formally curated stable facade, or whether broad internal-use re-exports are the intended long-term posture

## Recommended Action

`Small surgical refactor`

The right next pass is:

- add crate-level rustdoc and a public-surface map
- document CPU-only versus CUDA-gated APIs explicitly
- audit only the most obviously internal public modules before attempting any broader facade cleanup

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core --lib --no-run`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel --lib --no-run`
- `cargo test --workspace --no-run`
- if root exports or module visibility change:
  - `cargo test -p ta_desktop_demo_app --lib --no-run`
  - `cargo bench -p ta_app_core --no-run`

## Regression Risk And Feature-Preservation Constraints

- do not remove or rename nested public modules without auditing all downstream imports first
- do not change feature-gated export behavior in a way that makes features less additive without explicit design approval
- do not treat strategy-local GPU wrappers as safe to privatize until their actual consumers have been inventoried
